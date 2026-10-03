//! Snapshot index format (v1).
//!
//! A snapshot is a directory `snapshots/<id>/` containing:
//! - `manifest.json` — metadata, stats, extension table, errors
//! - `nodes.bin`     — fixed-size node records; a node's id is its index
//! - `names.bin`     — concatenated name bytes (`name_off`/`name_len` point in)
//! - `subtree.bin`   — `u32` subtree size per node id
//!
//! Nodes are emitted in pre-order, so a subtree is the contiguous id range
//! `[id, id + subtree_size)`. Phase 1 keeps the columns uncompressed and
//! memory-mapped for O(1) access; zstd block compression + CAS block sharing
//! are layered on in Phase 3 without changing the query surface.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// Bump on any layout change; readers reject unknown versions.
pub const FORMAT_VERSION: u32 = 2;
/// Fixed node record size in bytes.
pub const RECORD_SIZE: usize = 64;
/// Bytes per node in the optional `inode.bin` column (`ino` + `dev`, both u64).
pub const INODE_SIZE: usize = 16;
/// Sentinel `parent` for top-level (root) nodes.
pub const NO_PARENT: u32 = u32::MAX;

/// Node flag bits.
pub mod flags {
    pub const DIR: u8 = 1 << 0;
    pub const SYMLINK: u8 = 1 << 1;
    pub const HARDLINK_DUP: u8 = 1 << 2;
    pub const SPARSE: u8 = 1 << 3;
    pub const MOUNT_POINT: u8 = 1 << 4;
    pub const DOCKER_MOUNT: u8 = 1 << 5;
    pub const ERROR: u8 = 1 << 6;
}

/// Kind of a node, encoded as one byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    File,
    Directory,
    Symlink,
    Special,
}

impl Kind {
    pub fn as_u8(self) -> u8 {
        match self {
            Kind::File => 0,
            Kind::Directory => 1,
            Kind::Symlink => 2,
            Kind::Special => 3,
        }
    }

    pub fn from_u8(v: u8) -> Kind {
        match v {
            1 => Kind::Directory,
            2 => Kind::Symlink,
            3 => Kind::Special,
            _ => Kind::File,
        }
    }

    pub fn is_dir(self) -> bool {
        matches!(self, Kind::Directory)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::File => "file",
            Kind::Directory => "directory",
            Kind::Symlink => "symlink",
            Kind::Special => "special",
        }
    }
}

/// A decoded node record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub parent: u32,
    pub name_off: u32,
    pub name_len: u16,
    pub kind: Kind,
    pub flags: u8,
    pub children: u32,
    pub ext_id: u32,
    pub subtree_size: u32,
    pub size_app: u64,
    pub size_alloc: u64,
    pub mtime_ms: i64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
}

impl Record {
    pub fn encode(&self) -> [u8; RECORD_SIZE] {
        let mut b = [0u8; RECORD_SIZE];
        b[0..4].copy_from_slice(&self.parent.to_le_bytes());
        b[4..8].copy_from_slice(&self.name_off.to_le_bytes());
        b[8..10].copy_from_slice(&self.name_len.to_le_bytes());
        b[10] = self.kind.as_u8();
        b[11] = self.flags;
        b[12..16].copy_from_slice(&self.children.to_le_bytes());
        b[16..20].copy_from_slice(&self.ext_id.to_le_bytes());
        b[20..24].copy_from_slice(&self.subtree_size.to_le_bytes());
        b[24..32].copy_from_slice(&self.size_app.to_le_bytes());
        b[32..40].copy_from_slice(&self.size_alloc.to_le_bytes());
        b[40..48].copy_from_slice(&self.mtime_ms.to_le_bytes());
        b[48..52].copy_from_slice(&self.uid.to_le_bytes());
        b[52..56].copy_from_slice(&self.gid.to_le_bytes());
        b[56..58].copy_from_slice(&self.mode.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8]) -> Record {
        let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let u64_at = |o: usize| {
            u64::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7]])
        };
        let i64_at = |o: usize| {
            i64::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7]])
        };
        Record {
            parent: u32_at(0),
            name_off: u32_at(4),
            name_len: u16::from_le_bytes([b[8], b[9]]),
            kind: Kind::from_u8(b[10]),
            flags: b[11],
            children: u32_at(12),
            ext_id: u32_at(16),
            subtree_size: u32_at(20),
            size_app: u64_at(24),
            size_alloc: u64_at(32),
            mtime_ms: i64_at(40),
            uid: u32_at(48),
            gid: u32_at(52),
            mode: u16::from_le_bytes([b[56], b[57]]),
        }
    }
}

/// Aggregate scan statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanStats {
    pub files: u64,
    pub dirs: u64,
    pub bytes_apparent: u64,
    pub bytes_alloc: u64,
    pub errors: u64,
}

/// A recorded scan error (unreadable directory, I/O failure, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

/// The per-snapshot manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub roots: Vec<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
    pub node_count: u32,
    pub record_size: u32,
    pub stats: ScanStats,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub errors: Vec<ScanError>,
    /// True when an `inode.bin` column (ino+dev per node) is present.
    #[serde(default)]
    pub has_inodes: bool,
}

/// Input for a single node handed to [`IndexWriter::push`].
pub struct NodeInput<'a> {
    pub parent: u32,
    pub name: &'a str,
    pub kind: Kind,
    pub flags: u8,
    pub children: u32,
    pub size_app: u64,
    pub size_alloc: u64,
    pub mtime_ms: i64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    pub ino: u64,
    pub dev: u64,
}

struct Frame {
    id: u32,
    acc: u32,
    remaining: u32,
}

/// Streaming snapshot writer. Emits nodes in pre-order and computes
/// `subtree_size` on the way back up using an O(depth) stack.
pub struct IndexWriter {
    dir: PathBuf,
    nodes: BufWriter<File>,
    names: BufWriter<File>,
    inodes: BufWriter<File>,
    names_len: u64,
    subtree: Vec<u32>,
    count: u32,
    stack: Vec<Frame>,
    stats: ScanStats,
    errors: Vec<ScanError>,
    ext_ids: HashMap<String, u32>,
    extensions: Vec<String>,
}

impl IndexWriter {
    /// Create the snapshot directory and open the column files.
    pub fn create(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let nodes = File::create(dir.join("nodes.bin"))?;
        let names = File::create(dir.join("names.bin"))?;
        let inodes = File::create(dir.join("inode.bin"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            nodes: BufWriter::with_capacity(1 << 20, nodes),
            names: BufWriter::with_capacity(1 << 20, names),
            inodes: BufWriter::with_capacity(1 << 20, inodes),
            names_len: 0,
            subtree: Vec::new(),
            count: 0,
            stack: Vec::new(),
            stats: ScanStats::default(),
            errors: Vec::new(),
            ext_ids: HashMap::new(),
            extensions: vec![String::new()],
        })
    }

    pub fn node_count(&self) -> u32 {
        self.count
    }

    pub fn stats(&self) -> &ScanStats {
        &self.stats
    }

    /// Record a scan error and continue.
    pub fn push_error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.stats.errors += 1;
        if self.errors.len() < 10_000 {
            self.errors.push(ScanError {
                path: path.into(),
                message: message.into(),
            });
        }
    }

    fn ext_id(&mut self, name: &str) -> u32 {
        match extension_of(name) {
            None => 0,
            Some(ext) => {
                if let Some(id) = self.ext_ids.get(&ext) {
                    *id
                } else {
                    let id = self.extensions.len() as u32;
                    self.extensions.push(ext.clone());
                    self.ext_ids.insert(ext, id);
                    id
                }
            }
        }
    }

    /// Push a node in pre-order and return its id.
    pub fn push(&mut self, node: NodeInput<'_>) -> Result<u32> {
        let id = self.count;
        let name_off = self.names_len as u32;
        self.names.write_all(node.name.as_bytes())?;
        self.names_len += node.name.len() as u64;

        let ext_id = self.ext_id(node.name);
        let rec = Record {
            parent: node.parent,
            name_off,
            name_len: node.name.len() as u16,
            kind: node.kind,
            flags: node.flags,
            children: node.children,
            ext_id,
            subtree_size: 0,
            size_app: node.size_app,
            size_alloc: node.size_alloc,
            mtime_ms: node.mtime_ms,
            uid: node.uid,
            gid: node.gid,
            mode: node.mode,
        };
        self.nodes.write_all(&rec.encode())?;

        let mut inode_bytes = [0u8; INODE_SIZE];
        inode_bytes[0..8].copy_from_slice(&node.ino.to_le_bytes());
        inode_bytes[8..16].copy_from_slice(&node.dev.to_le_bytes());
        self.inodes.write_all(&inode_bytes)?;

        self.subtree.push(0);
        self.count += 1;

        match node.kind {
            Kind::Directory => self.stats.dirs += 1,
            _ => self.stats.files += 1,
        }
        self.stats.bytes_apparent = self.stats.bytes_apparent.saturating_add(node.size_app);
        self.stats.bytes_alloc = self.stats.bytes_alloc.saturating_add(node.size_alloc);

        if node.kind.is_dir() && node.children > 0 {
            self.stack.push(Frame {
                id,
                acc: 1,
                remaining: node.children,
            });
        } else {
            self.subtree[id as usize] = 1;
            self.propagate(1);
        }

        Ok(id)
    }

    fn propagate(&mut self, mut size: u32) {
        while let Some(top) = self.stack.last_mut() {
            top.acc = top.acc.saturating_add(size);
            top.remaining = top.remaining.saturating_sub(1);
            if top.remaining == 0 {
                let frame = self.stack.pop().expect("frame present");
                self.subtree[frame.id as usize] = frame.acc;
                size = frame.acc;
                continue;
            }
            break;
        }
    }

    /// Flush columns and write the manifest. Returns the manifest.
    pub fn finish(mut self, manifest_seed: ManifestSeed) -> Result<Manifest> {
        // Close any still-open frames (defensive; a well-formed walk leaves none).
        while let Some(frame) = self.stack.pop() {
            self.subtree[frame.id as usize] = frame.acc;
        }

        self.nodes.flush()?;
        self.names.flush()?;
        self.inodes.flush()?;

        let mut subtree_file = BufWriter::new(File::create(self.dir.join("subtree.bin"))?);
        for size in &self.subtree {
            subtree_file.write_all(&size.to_le_bytes())?;
        }
        subtree_file.flush()?;

        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            id: manifest_seed.id,
            parent_id: manifest_seed.parent_id,
            roots: manifest_seed.roots,
            started_at_ms: manifest_seed.started_at_ms,
            finished_at_ms: now_ms(),
            node_count: self.count,
            record_size: RECORD_SIZE as u32,
            stats: self.stats,
            extensions: self.extensions,
            errors: self.errors,
            has_inodes: true,
        };

        let manifest_path = self.dir.join("manifest.json");
        let tmp = self.dir.join("manifest.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&manifest)?)?;
        std::fs::rename(&tmp, &manifest_path)?;
        Ok(manifest)
    }
}

/// Manifest fields supplied by the scanner.
pub struct ManifestSeed {
    pub id: String,
    pub parent_id: Option<String>,
    pub roots: Vec<String>,
    pub started_at_ms: i64,
}

/// Read-only view over a snapshot, backed by memory-mapped columns.
pub struct IndexReader {
    dir: PathBuf,
    manifest: Manifest,
    nodes: Option<Mmap>,
    names: Option<Mmap>,
    subtree: Option<Mmap>,
    inodes: Option<Mmap>,
}

impl IndexReader {
    pub fn open(dir: &Path) -> Result<Self> {
        let manifest_bytes = std::fs::read(dir.join("manifest.json"))?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
        if manifest.format_version > FORMAT_VERSION {
            return Err(CoreError::Format(format!(
                "snapshot format {} is not supported (this build reads up to {})",
                manifest.format_version, FORMAT_VERSION
            )));
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
            nodes: map_opt(&dir.join("nodes.bin"))?,
            names: map_opt(&dir.join("names.bin"))?,
            subtree: map_opt(&dir.join("subtree.bin"))?,
            inodes: map_opt(&dir.join("inode.bin"))?,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn len(&self) -> u32 {
        self.manifest.node_count
    }

    pub fn is_empty(&self) -> bool {
        self.manifest.node_count == 0
    }

    pub fn record(&self, id: u32) -> Option<Record> {
        let nodes = self.nodes.as_ref()?;
        let start = id as usize * RECORD_SIZE;
        let end = start + RECORD_SIZE;
        if end > nodes.len() {
            return None;
        }
        Some(Record::decode(&nodes[start..end]))
    }

    pub fn name(&self, id: u32) -> &str {
        let Some(rec) = self.record(id) else {
            return "";
        };
        let Some(names) = self.names.as_ref() else {
            return "";
        };
        let start = rec.name_off as usize;
        let end = start + rec.name_len as usize;
        if end > names.len() {
            return "";
        }
        std::str::from_utf8(&names[start..end]).unwrap_or("")
    }

    pub fn ext(&self, id: u32) -> &str {
        let Some(rec) = self.record(id) else {
            return "";
        };
        self.manifest
            .extensions
            .get(rec.ext_id as usize)
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn subtree_size(&self, id: u32) -> u32 {
        match self.subtree.as_ref() {
            Some(mm) => {
                let start = id as usize * 4;
                if start + 4 > mm.len() {
                    return 0;
                }
                u32::from_le_bytes([mm[start], mm[start + 1], mm[start + 2], mm[start + 3]])
            }
            None => 0,
        }
    }

    /// Contiguous id range covering `id` and all of its descendants.
    pub fn subtree_range(&self, id: u32) -> std::ops::Range<u32> {
        let start = id;
        let end = id.saturating_add(self.subtree_size(id));
        start..end
    }

    /// `(ino, dev)` for a node, when the snapshot carries an inode column.
    pub fn inode(&self, id: u32) -> Option<(u64, u64)> {
        let mm = self.inodes.as_ref()?;
        let start = id as usize * INODE_SIZE;
        if start + INODE_SIZE > mm.len() {
            return None;
        }
        let mut ino = [0u8; 8];
        let mut dev = [0u8; 8];
        ino.copy_from_slice(&mm[start..start + 8]);
        dev.copy_from_slice(&mm[start + 8..start + 16]);
        Some((u64::from_le_bytes(ino), u64::from_le_bytes(dev)))
    }

    /// Direct children of `id`, in id order.
    pub fn children(&self, id: u32) -> Vec<u32> {
        let range = self.subtree_range(id);
        let mut out = Vec::new();
        for child in range.start.saturating_add(1)..range.end {
            if let Some(rec) = self.record(child) {
                if rec.parent == id {
                    out.push(child);
                }
            }
        }
        out
    }
}

/// Compute the lowercase extension (without the dot) of a path/name.
pub fn extension_of(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() && ext.len() <= 32 => {
            Some(ext.to_ascii_lowercase())
        }
        _ => None,
    }
}

fn map_opt(path: &Path) -> Result<Option<Mmap>> {
    let file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        // Optional columns (e.g. inode.bin on older snapshots) may be absent.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if file.metadata()?.len() == 0 {
        return Ok(None);
    }
    // SAFETY: the snapshot files are immutable once written.
    let mmap = unsafe { Mmap::map(&file)? };
    Ok(Some(mmap))
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_small(dir: &Path) -> Manifest {
        let mut w = IndexWriter::create(dir).unwrap();
        // root dir with 2 children
        let root = w
            .push(NodeInput {
                parent: NO_PARENT,
                name: "root",
                kind: Kind::Directory,
                flags: flags::DIR,
                children: 2,
                size_app: 0,
                size_alloc: 0,
                mtime_ms: 0,
                uid: 0,
                gid: 0,
                mode: 0o755,
                ino: 0,
                dev: 0,
            })
            .unwrap();
        let _f1 = w
            .push(NodeInput {
                parent: root,
                name: "a.txt",
                kind: Kind::File,
                flags: 0,
                children: 0,
                size_app: 100,
                size_alloc: 200,
                mtime_ms: 1,
                uid: 0,
                gid: 0,
                mode: 0o644,
                ino: 0,
                dev: 0,
            })
            .unwrap();
        let _sub = w
            .push(NodeInput {
                parent: root,
                name: "sub",
                kind: Kind::Directory,
                flags: flags::DIR,
                children: 1,
                size_app: 0,
                size_alloc: 0,
                mtime_ms: 2,
                uid: 0,
                gid: 0,
                mode: 0o755,
                ino: 0,
                dev: 0,
            })
            .unwrap();
        let _f2 = w
            .push(NodeInput {
                parent: _sub,
                name: "b.bin",
                kind: Kind::File,
                flags: 0,
                children: 0,
                size_app: 300,
                size_alloc: 400,
                mtime_ms: 3,
                uid: 0,
                gid: 0,
                mode: 0o644,
                ino: 0,
                dev: 0,
            })
            .unwrap();
        w.finish(ManifestSeed {
            id: "s1".into(),
            parent_id: None,
            roots: vec!["root".into()],
            started_at_ms: 0,
        })
        .unwrap()
    }

    #[test]
    fn record_roundtrips() {
        let rec = Record {
            parent: 7,
            name_off: 11,
            name_len: 5,
            kind: Kind::Directory,
            flags: flags::DIR | flags::MOUNT_POINT,
            children: 3,
            ext_id: 2,
            subtree_size: 9,
            size_app: 123456789,
            size_alloc: 987654321,
            mtime_ms: -42,
            uid: 1000,
            gid: 1000,
            mode: 0o755,
        };
        assert_eq!(Record::decode(&rec.encode()), rec);
    }

    #[test]
    fn writer_computes_subtree_sizes_and_children() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = write_small(tmp.path());
        assert_eq!(manifest.node_count, 4);
        assert_eq!(manifest.stats.files, 2);
        assert_eq!(manifest.stats.dirs, 2);

        let reader = IndexReader::open(tmp.path()).unwrap();
        assert_eq!(reader.name(0), "root");
        assert_eq!(reader.name(3), "b.bin");
        assert_eq!(reader.subtree_size(0), 4);
        assert_eq!(reader.subtree_size(2), 2);
        assert_eq!(reader.children(0), vec![1, 2]);
        assert_eq!(reader.children(2), vec![3]);
        assert_eq!(reader.ext(1), "txt");
        assert_eq!(reader.ext(3), "bin");
    }

    #[test]
    fn extension_parsing() {
        assert_eq!(extension_of("a.tar.gz").as_deref(), Some("gz"));
        assert_eq!(extension_of("noext").as_deref(), None);
        assert_eq!(extension_of(".bashrc").as_deref(), None);
        assert_eq!(extension_of("/x/y/Z.PNG").as_deref(), Some("png"));
    }
}
