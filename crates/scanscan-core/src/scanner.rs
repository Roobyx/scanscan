//! Filesystem traversal.
//!
//! `scan()` performs a streaming pre-order walk: it emits each node to the
//! [`IndexWriter`] as it is visited and keeps only O(depth) traversal state
//! plus the hardlink set. Directory entries are read and `stat`-ed in
//! parallel; the walk itself is sequential so output order (and therefore the
//! contiguous-subtree invariant) is deterministic.
//!
//! `LinuxFs` (statx) and fully parallel traversal (jwalk-style) are later
//! optimisations; the portable implementation below is correct on every
//! platform.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use crate::error::{CoreError, Result};
use crate::index::{flags, IndexReader, IndexWriter, Kind, NodeInput, NO_PARENT};
use scanscan_ipc::ScanOptions;

/// Metadata captured for a single directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMeta {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size_apparent: u64,
    pub size_alloc: u64,
    pub mtime_ms: i64,
    pub nlink: u64,
    pub dev: u64,
    pub ino: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    /// Set when this entry could not be `stat`-ed; the walk records it and moves on.
    pub error: Option<String>,
}

impl EntryMeta {
    fn errored(path: &Path, message: String) -> Self {
        Self {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_path_buf(),
            is_dir: false,
            is_symlink: false,
            size_apparent: 0,
            size_alloc: 0,
            mtime_ms: 0,
            nlink: 1,
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            mode: 0,
            error: Some(message),
        }
    }
}

/// Directory path -> display label, resolved while walking. Used to annotate
/// e.g. Docker overlay2 layer directories with their owning container. Empty
/// when Docker attribution is unavailable; lookups are then a no-op.
#[derive(Debug, Default, Clone)]
pub struct LabelIndex {
    by_path: HashMap<String, String>,
}

impl LabelIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: impl Into<String>, label: impl Into<String>) {
        self.by_path.insert(path.into(), label.into());
    }

    /// Label for a directory path, if any. Cheap no-op when no labels exist.
    pub fn label_for(&self, path: &Path) -> Option<&str> {
        if self.is_empty() {
            return None;
        }
        self.by_path.get(path.to_string_lossy().as_ref()).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }
}

/// A filesystem implementation: the Linux fast path or the portable fallback.
pub trait Filesystem: Send + Sync {
    /// Read the direct entries of one directory. `stat` failures are reported
    /// per-entry via [`EntryMeta::error`] rather than failing the directory.
    fn read_dir(&self, dir: &Path) -> Result<Vec<EntryMeta>>;

    /// Metadata for a single path (does not follow symlinks).
    fn metadata(&self, path: &Path) -> Result<EntryMeta>;

    /// Metadata following symlinks (used when `--follow-symlinks` is set).
    fn metadata_follow(&self, path: &Path) -> Result<EntryMeta> {
        self.metadata(path)
    }
}

/// Portable walker built on `std::fs`.
#[derive(Debug, Default, Clone, Copy)]
pub struct PortableFs;

impl PortableFs {
    fn meta_for(path: &Path, meta: std::fs::Metadata) -> EntryMeta {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_symlink = meta.file_type().is_symlink();
        let is_dir = meta.is_dir();

        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        #[cfg(unix)]
        let (size_alloc, nlink, dev, ino, uid, gid, mode) = {
            use std::os::unix::fs::MetadataExt;
            (
                meta.blocks().saturating_mul(512),
                meta.nlink(),
                meta.dev(),
                meta.ino(),
                meta.uid(),
                meta.gid(),
                meta.mode() as u16,
            )
        };
        #[cfg(not(unix))]
        let (size_alloc, nlink, dev, ino, uid, gid, mode) =
            (meta.len(), 1u64, 0u64, 0u64, 0u32, 0u32, 0u16);

        EntryMeta {
            path: path.to_path_buf(),
            name,
            is_dir,
            is_symlink,
            size_apparent: meta.len(),
            size_alloc,
            mtime_ms,
            nlink,
            dev,
            ino,
            uid,
            gid,
            mode,
            error: None,
        }
    }
}

impl Filesystem for PortableFs {
    fn read_dir(&self, dir: &Path) -> Result<Vec<EntryMeta>> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            match entry {
                Ok(e) => paths.push(e.path()),
                Err(e) => paths.push(PathBuf::from(format!("{}/<unreadable:{e}>", dir.display()))),
            }
        }

        let mut metas: Vec<EntryMeta> = paths
            .par_iter()
            .map(|p| match std::fs::symlink_metadata(p) {
                Ok(meta) => PortableFs::meta_for(p, meta),
                Err(e) => EntryMeta::errored(p, e.to_string()),
            })
            .collect();
        // Parallel collection preserves order.
        metas.shrink_to_fit();
        Ok(metas)
    }

    fn metadata(&self, path: &Path) -> Result<EntryMeta> {
        let meta = std::fs::symlink_metadata(path)?;
        Ok(PortableFs::meta_for(path, meta))
    }

    fn metadata_follow(&self, path: &Path) -> Result<EntryMeta> {
        let meta = std::fs::metadata(path)?;
        Ok(PortableFs::meta_for(path, meta))
    }
}

/// Resolve the platform-appropriate filesystem implementation.
pub fn platform_fs() -> Box<dyn Filesystem> {
    Box::new(PortableFs)
}

/// Canonicalize a root, rejecting paths that do not exist.
pub fn canonical_root(root: &str) -> Result<PathBuf> {
    let path = PathBuf::from(root);
    std::fs::canonicalize(&path)
        .map_err(|e| CoreError::Config(format!("invalid root '{}': {e}", path.display())))
}

/// A throttled progress sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub files: u64,
    pub dirs: u64,
    pub bytes_apparent: u64,
    pub bytes_alloc: u64,
    pub errors: u64,
    pub current_path: String,
    pub elapsed_ms: u64,
}

/// Build the exclusion matcher from options, adding pseudo-fs defaults when
/// scanning `/`.
pub fn build_exclusions(options: &ScanOptions, roots: &[PathBuf]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &options.exclusions {
        builder.add(Glob::new(pattern).map_err(|e| CoreError::Config(format!("bad glob '{pattern}': {e}")))?);
    }
    // Exclude pseudo-filesystems under every root (e.g. /proc, or /host/proc
    // when the host is mounted at /host).
    for root in roots {
        for name in ["proc", "sys", "dev", "run", "snap"] {
            let path = root.join(name);
            let path = path.to_string_lossy();
            builder.add(Glob::new(&path).expect("static glob"));
            builder.add(Glob::new(&format!("{path}/**")).expect("static glob"));
        }
    }
    builder
        .build()
        .map_err(|e| CoreError::Config(format!("invalid exclusions: {e}")))
}

/// Stream a pre-order scan of `roots` into `writer`.
pub fn scan(
    writer: &mut IndexWriter,
    roots: &[PathBuf],
    options: &ScanOptions,
    fs: &dyn Filesystem,
    cancel: &AtomicBool,
    previous: Option<&Previous>,
    labels: &LabelIndex,
    progress: &mut dyn FnMut(&Progress),
) -> Result<()> {
    let exclude = build_exclusions(options, roots)?;
    let started = Instant::now();
    let mut walker = Walker {
        fs,
        options,
        exclude,
        writer,
        hardlinks: HashSet::new(),
        visited: HashSet::new(),
        cancel,
        previous,
        labels,
        progress,
        started,
        last_emit: Instant::now(),
        current_path: String::new(),
    };

    for root in roots {
        walker.walk_root(root)?;
    }
    walker.tick(Path::new(""));
    Ok(())
}

/// Reusable directory index from a previous snapshot, for `--incremental`.
pub struct Previous {
    reader: IndexReader,
    dirs: std::collections::HashMap<String, (i64, u64, u32)>,
}

impl Previous {
    /// Build a directory index: container path -> (mtime_ms, ino, node_id).
    pub fn build(reader: IndexReader) -> Self {
        let mut child_index: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        for id in 0..reader.len() {
            if let Some(rec) = reader.record(id) {
                if rec.parent != NO_PARENT {
                    child_index.entry(rec.parent).or_default().push(id);
                }
            }
        }
        let mut dirs: std::collections::HashMap<String, (i64, u64, u32)> =
            std::collections::HashMap::new();
        let roots = reader.manifest().roots.clone();
        let mut root_index = 0usize;
        for root in 0..reader.len() {
            let is_root = reader
                .record(root)
                .map(|rec| rec.parent == NO_PARENT)
                .unwrap_or(false);
            if is_root {
                let abs = roots
                    .get(root_index)
                    .cloned()
                    .unwrap_or_else(|| reader.name(root).to_string());
                collect_dirs(&reader, &child_index, root, abs, &mut dirs);
                root_index += 1;
            }
        }
        Self { reader, dirs }
    }

    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }
}

fn collect_dirs(
    reader: &IndexReader,
    index: &std::collections::HashMap<u32, Vec<u32>>,
    id: u32,
    abs_path: String,
    out: &mut std::collections::HashMap<String, (i64, u64, u32)>,
) {
    if let Some(rec) = reader.record(id) {
        if rec.kind.is_dir() {
            let ino = reader.inode(id).map(|(ino, _)| ino).unwrap_or(0);
            out.insert(abs_path.clone(), (rec.mtime_ms, ino, id));
        }
    }
    if let Some(children) = index.get(&id) {
        for child in children {
            let name = reader.name(*child);
            collect_dirs(reader, index, *child, format!("{abs_path}/{name}"), out);
        }
    }
}

/// Re-emit an old snapshot's subtree into the new writer, remapping parent ids.
///
/// Labels are resolved from the *current* scan's [`LabelIndex`] (by rebuilding
/// each node's path as we walk pre-order), not copied from the old snapshot, so
/// an unchanged subtree still picks up labels that are newly known.
fn copy_subtree(
    writer: &mut IndexWriter,
    old: &IndexReader,
    old_root: u32,
    new_root: u32,
    root_path: &str,
    labels: &LabelIndex,
) -> Result<()> {
    let size = old.subtree_size(old_root);
    // Pre-order ancestor stack: (exclusive subtree end, parent path length).
    let mut stack: Vec<(u32, usize)> = Vec::new();
    let mut path = root_path.to_string();
    for old_id in (old_root + 1)..(old_root + size) {
        while let Some(&(end, parent_len)) = stack.last() {
            if old_id >= end {
                stack.pop();
                path.truncate(parent_len);
            } else {
                break;
            }
        }
        let Some(rec) = old.record(old_id) else {
            continue;
        };
        let parent_new = if rec.parent == old_root {
            new_root
        } else {
            new_root + (rec.parent - old_root)
        };
        let name = old.name(old_id);
        let parent_len = path.len();
        path.push('/');
        path.push_str(name);
        let label = labels.label_for(Path::new(path.as_str()));
        let (ino, dev) = old.inode(old_id).unwrap_or((0, 0));
        writer.push_labeled(
            NodeInput {
                parent: parent_new,
                name,
                kind: rec.kind,
                flags: rec.flags,
                children: rec.children,
                size_app: rec.size_app,
                size_alloc: rec.size_alloc,
                mtime_ms: rec.mtime_ms,
                uid: rec.uid,
                gid: rec.gid,
                mode: rec.mode,
                ino,
                dev,
            },
            label,
        )?;
        let subtree = old.subtree_size(old_id);
        if subtree > 1 {
            stack.push((old_id + subtree, parent_len));
        } else {
            path.truncate(parent_len);
        }
    }
    Ok(())
}

struct Walker<'a> {
    fs: &'a dyn Filesystem,
    options: &'a ScanOptions,
    exclude: GlobSet,
    writer: &'a mut IndexWriter,
    hardlinks: HashSet<(u64, u64)>,
    visited: HashSet<(u64, u64)>,
    cancel: &'a AtomicBool,
    previous: Option<&'a Previous>,
    labels: &'a LabelIndex,
    progress: &'a mut dyn FnMut(&Progress),
    started: Instant,
    last_emit: Instant,
    current_path: String,
}

impl Walker<'_> {
    fn check_cancel(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(CoreError::Scan("scan cancelled".into()))
        } else {
            Ok(())
        }
    }

    fn tick(&mut self, path: &Path) {
        if !path.as_os_str().is_empty() {
            self.current_path = path.to_string_lossy().into_owned();
        }
        if self.last_emit.elapsed().as_millis() < 200 {
            return;
        }
        self.last_emit = Instant::now();
        let s = self.writer.stats();
        (self.progress)(&Progress {
            files: s.files,
            dirs: s.dirs,
            bytes_apparent: s.bytes_apparent,
            bytes_alloc: s.bytes_alloc,
            errors: s.errors,
            current_path: self.current_path.clone(),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        });
    }

    fn excluded(&self, path: &Path) -> bool {
        self.exclude.is_match(path)
    }

    fn walk_root(&mut self, path: &Path) -> Result<()> {
        let meta = self
            .fs
            .metadata(path)
            .map_err(|e| CoreError::Scan(format!("cannot stat root '{}': {e}", path.display())))?;
        let root_dev = meta.dev;
        self.walk_entry(meta, NO_PARENT, root_dev)
    }

    fn walk_entry(&mut self, entry: EntryMeta, parent: u32, root_dev: u64) -> Result<()> {
        self.check_cancel()?;
        if let Some(err) = &entry.error {
            self.writer.push_error(entry.path.to_string_lossy(), err.clone());
            self.writer.push(NodeInput {
                parent,
                name: &entry.name,
                kind: Kind::Special,
                flags: flags::ERROR,
                children: 0,
                size_app: 0,
                size_alloc: 0,
                mtime_ms: 0,
                uid: entry.uid,
                gid: entry.gid,
                mode: entry.mode,
                ino: entry.ino,
                dev: entry.dev,
            })?;
            self.tick(&entry.path);
            return Ok(());
        }

        if entry.is_symlink {
            if !self.options.follow_symlinks {
                return self.emit_symlink(&entry, parent);
            }
            match self.fs.metadata_follow(&entry.path) {
                Ok(target) => {
                    let mut followed = target;
                    followed.name = entry.name.clone();
                    followed.path = entry.path.clone();
                    if followed.is_dir {
                        if !self.visited.insert((followed.dev, followed.ino)) {
                            return self.emit_symlink(&entry, parent);
                        }
                        return self.emit_dir(followed, parent, root_dev, flags::DIR | flags::SYMLINK);
                    }
                    return self.emit_file(followed, parent);
                }
                Err(e) => {
                    self.writer
                        .push_error(entry.path.to_string_lossy(), e.to_string());
                    return self.emit_symlink(&entry, parent);
                }
            }
        }

        if entry.is_dir {
            let mut extra = flags::DIR;
            if entry.dev != root_dev {
                extra |= flags::MOUNT_POINT;
                if self.options.one_file_system {
                    self.writer.push(NodeInput {
                        parent,
                        name: &entry.name,
                        kind: Kind::Directory,
                        flags: extra,
                        children: 0,
                        size_app: 0,
                        size_alloc: 0,
                        mtime_ms: entry.mtime_ms,
                        uid: entry.uid,
                        gid: entry.gid,
                        mode: entry.mode,
                        ino: entry.ino,
                        dev: entry.dev,
                    })?;
                    self.tick(&entry.path);
                    return Ok(());
                }
            }
            self.emit_dir(entry, parent, root_dev, extra)
        } else {
            self.emit_file(entry, parent)
        }
    }

    fn emit_symlink(&mut self, entry: &EntryMeta, parent: u32) -> Result<()> {
        self.writer.push(NodeInput {
            parent,
            name: &entry.name,
            kind: Kind::Symlink,
            flags: flags::SYMLINK,
            children: 0,
            size_app: 0,
            size_alloc: 0,
            mtime_ms: entry.mtime_ms,
            uid: entry.uid,
            gid: entry.gid,
            mode: entry.mode,
            ino: entry.ino,
            dev: entry.dev,
        })?;
        self.tick(&entry.path);
        Ok(())
    }

    fn emit_dir(&mut self, entry: EntryMeta, parent: u32, root_dev: u64, extra: u8) -> Result<()> {
        if let Some(previous) = self.previous {
            if entry.ino != 0 {
                let key = entry.path.to_string_lossy();
                if let Some(&(mtime, ino, old_id)) = previous.dirs.get(key.as_ref()) {
                    if mtime == entry.mtime_ms && ino == entry.ino {
                        // Directory unchanged: reuse its whole subtree.
                        let old_children = previous
                            .reader
                            .record(old_id)
                            .map(|rec| rec.children)
                            .unwrap_or(0);
                        let label = self.labels.label_for(&entry.path);
                        let new_id = self.writer.push_labeled(
                            NodeInput {
                                parent,
                                name: &entry.name,
                                kind: Kind::Directory,
                                flags: extra,
                                children: old_children,
                                size_app: 0,
                                size_alloc: 0,
                                mtime_ms: entry.mtime_ms,
                                uid: entry.uid,
                                gid: entry.gid,
                                mode: entry.mode,
                                ino: entry.ino,
                                dev: entry.dev,
                            },
                            label,
                        )?;
                        let root_path = entry.path.to_string_lossy();
                        copy_subtree(
                            self.writer,
                            &previous.reader,
                            old_id,
                            new_id,
                            root_path.as_ref(),
                            self.labels,
                        )?;
                        self.tick(&entry.path);
                        return Ok(());
                    }
                }
            }
        }

        let entries = match self.fs.read_dir(&entry.path) {
            Ok(e) => e,
            Err(e) => {
                self.writer.push_error(entry.path.to_string_lossy(), e.to_string());
                let label = self.labels.label_for(&entry.path);
                self.writer.push_labeled(
                    NodeInput {
                        parent,
                        name: &entry.name,
                        kind: Kind::Directory,
                        flags: extra | flags::ERROR,
                        children: 0,
                        size_app: 0,
                        size_alloc: 0,
                        mtime_ms: entry.mtime_ms,
                        uid: entry.uid,
                        gid: entry.gid,
                        mode: entry.mode,
                        ino: entry.ino,
                        dev: entry.dev,
                    },
                    label,
                )?;
                self.tick(&entry.path);
                return Ok(());
            }
        };

        let kept: Vec<EntryMeta> = entries
            .into_iter()
            .filter(|e| !self.excluded(&e.path))
            .collect();
        let child_count = kept.len() as u32;

        let label = self.labels.label_for(&entry.path);
        let id = self.writer.push_labeled(
            NodeInput {
                parent,
                name: &entry.name,
                kind: Kind::Directory,
                flags: extra,
                children: child_count,
                size_app: 0,
                size_alloc: 0,
                mtime_ms: entry.mtime_ms,
                uid: entry.uid,
                gid: entry.gid,
                mode: entry.mode,
                ino: entry.ino,
                dev: entry.dev,
            },
            label,
        )?;
        self.tick(&entry.path);

        for child in kept {
            self.walk_entry(child, id, root_dev)?;
        }
        Ok(())
    }

    fn emit_file(&mut self, entry: EntryMeta, parent: u32) -> Result<()> {
        let mut flags = 0u8;
        let mut size_alloc = entry.size_alloc;
        let mut size_app = entry.size_apparent;

        if entry.nlink > 1 && !self.hardlinks.insert((entry.dev, entry.ino)) {
            // Later hardlinks own no bytes so totals count the inode once.
            flags |= flags::HARDLINK_DUP;
            size_alloc = 0;
            size_app = 0;
        }
        if entry.size_alloc < entry.size_apparent {
            flags |= flags::SPARSE;
        }

        self.writer.push(NodeInput {
            parent,
            name: &entry.name,
            kind: Kind::File,
            flags,
            children: 0,
            size_app,
            size_alloc,
            mtime_ms: entry.mtime_ms,
            uid: entry.uid,
            gid: entry.gid,
            mode: entry.mode,
            ino: entry.ino,
            dev: entry.dev,
        })?;
        self.tick(&entry.path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexReader, ManifestSeed};

    fn fixture(dir: &Path) {
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("sub/b.bin"), vec![0u8; 2000]).unwrap();
    }

    #[test]
    fn scans_fixture_tree() {
        let tmp = tempfile::tempdir().unwrap();
        // Keep the snapshot outside the scanned root so the walk cannot index
        // its own output.
        let snap_tmp = tempfile::tempdir().unwrap();
        fixture(tmp.path());
        let root = tmp.path().to_path_buf();

        let snap = snap_tmp.path().join("snap");
        let mut writer = IndexWriter::create(&snap).unwrap();
        let options = ScanOptions::new(vec![root.to_string_lossy().into_owned()]);
        let fs = PortableFs;
        let mut progress = |_: &Progress| {};
        scan(
            &mut writer,
            std::slice::from_ref(&root),
            &options,
            &fs,
            &AtomicBool::new(false),
            None,
            &LabelIndex::new(),
            &mut progress,
        )
        .unwrap();
        let manifest = writer
            .finish(ManifestSeed {
                id: "t".into(),
                parent_id: None,
                roots: vec![root.to_string_lossy().into_owned()],
                started_at_ms: 0,
            })
            .unwrap();

        // root dir + sub dir + 2 files
        assert_eq!(manifest.node_count, 4);
        assert_eq!(manifest.stats.files, 2);
        assert_eq!(manifest.stats.dirs, 2);
        assert_eq!(manifest.stats.bytes_apparent, 3000);

        let reader = IndexReader::open(&snap).unwrap();
        assert_eq!(reader.subtree_size(0), 4);
    }

    #[test]
    fn applies_labels_to_matching_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let snap_tmp = tempfile::tempdir().unwrap();
        fixture(tmp.path());
        let root = tmp.path().to_path_buf();

        let mut labels = LabelIndex::new();
        labels.insert(
            root.join("sub").to_string_lossy().into_owned(),
            "web (nginx:latest) · myapp/web",
        );

        let snap = snap_tmp.path().join("snap");
        let mut writer = IndexWriter::create(&snap).unwrap();
        let options = ScanOptions::new(vec![root.to_string_lossy().into_owned()]);
        let mut progress = |_: &Progress| {};
        scan(
            &mut writer,
            std::slice::from_ref(&root),
            &options,
            &PortableFs,
            &AtomicBool::new(false),
            None,
            &labels,
            &mut progress,
        )
        .unwrap();
        writer
            .finish(ManifestSeed {
                id: "t".into(),
                parent_id: None,
                roots: vec![root.to_string_lossy().into_owned()],
                started_at_ms: 0,
            })
            .unwrap();

        let reader = IndexReader::open(&snap).unwrap();
        let sub = reader
            .children(0)
            .into_iter()
            .find(|id| reader.name(*id) == "sub")
            .expect("sub dir");
        assert_eq!(reader.label(sub), "web (nginx:latest) · myapp/web");
        let file = reader
            .children(0)
            .into_iter()
            .find(|id| reader.name(*id) == "a.txt")
            .expect("a.txt");
        assert_eq!(reader.label(file), "");
    }

    #[test]
    fn missing_root_is_rejected() {
        let err = canonical_root("/definitely/not/here/scanscan").unwrap_err();
        assert!(matches!(err, CoreError::Config(_)));
    }
}
