//! Filesystem traversal abstraction.
//!
//! `LinuxFs` (statx fast path) arrives in Phase 1; Phase 0 ships the trait and
//! a portable `std::fs` implementation used on macOS/Windows and as a fallback.

use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

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
}

/// A filesystem implementation: the Linux fast path or the portable fallback.
pub trait Filesystem: Send + Sync {
    /// Read the direct entries of one directory. Order is unspecified.
    fn read_dir(&self, dir: &Path) -> Result<Vec<EntryMeta>>;

    /// Metadata for a single path (does not follow symlinks).
    fn metadata(&self, path: &Path) -> Result<EntryMeta>;
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
        let (size_alloc, nlink, dev, ino) = {
            use std::os::unix::fs::MetadataExt;
            (meta.blocks().saturating_mul(512), meta.nlink(), meta.dev(), meta.ino())
        };
        #[cfg(not(unix))]
        let (size_alloc, nlink, dev, ino) = (meta.len(), 1, 0, 0);

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
        }
    }
}

impl Filesystem for PortableFs {
    fn read_dir(&self, dir: &Path) -> Result<Vec<EntryMeta>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            out.push(PortableFs::meta_for(&path, meta));
        }
        Ok(out)
    }

    fn metadata(&self, path: &Path) -> Result<EntryMeta> {
        let meta = std::fs::symlink_metadata(path)?;
        Ok(PortableFs::meta_for(path, meta))
    }
}

/// Resolve the platform-appropriate filesystem implementation.
pub fn platform_fs() -> Result<Box<dyn Filesystem>> {
    #[cfg(target_os = "linux")]
    {
        // Phase 1 swaps this for `LinuxFs` (statx). Until then the portable
        // walker keeps behaviour correct on every platform.
        Ok(Box::new(PortableFs))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Box::new(PortableFs))
    }
}

/// Canonicalize a root, rejecting paths that do not exist.
pub fn canonical_root(root: &str) -> Result<PathBuf> {
    let path = PathBuf::from(root);
    std::fs::canonicalize(&path).map_err(|e| {
        CoreError::Config(format!("invalid root '{}': {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_current_directory() {
        let fs = PortableFs;
        let entries = fs.read_dir(Path::new(".")).expect("read_dir");
        assert!(!entries.is_empty());
    }

    #[test]
    fn missing_root_is_rejected() {
        let err = canonical_root("/definitely/not/here/scanscan").unwrap_err();
        assert!(matches!(err, CoreError::Config(_)));
    }
}
