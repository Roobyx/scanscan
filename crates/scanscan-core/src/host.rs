//! Host mount discovery for the drive picker.
//!
//! When the host root is bind-mounted read-only (default `/host`), the host's
//! mount table is readable at `<host_root>/proc/mounts`. We filter it down to
//! real, block-backed filesystems and map each host mountpoint to its path
//! inside the container so the UI can offer a picker.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// A filesystem mount on the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMount {
    /// Mountpoint as seen on the host (e.g. `/`, `/mnt/data`).
    pub path: String,
    /// The same mount as seen inside the container (e.g. `/host/mnt/data`).
    pub container_path: String,
    /// Source device (e.g. `/dev/sda1`, `server:/export`).
    pub device: String,
    /// Filesystem type (e.g. `ext4`, `xfs`).
    pub fstype: String,
}

const REAL_FS: [&str; 16] = [
    "ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "vfat", "exfat", "ntfs", "ntfs3",
    "iso9660", "udf", "reiserfs", "jfs", "hfsplus",
];

/// Read the host mount table, keeping real filesystems.
///
/// `mounts_file` is an explicit host `/proc/mounts` mounted into the container
/// (the reliable source); otherwise we try `<host_root>/proc/mounts`, then the
/// container's own `/proc/mounts` as a last resort.
pub fn read_mounts(host_root: &Path, mounts_file: Option<&Path>) -> Vec<HostMount> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(file) = mounts_file {
        candidates.push(file.to_path_buf());
    }
    candidates.push(host_root.join("proc/mounts"));
    candidates.push(PathBuf::from("/proc/mounts"));
    let content = candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();

    let root = host_root.to_string_lossy();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for line in content.lines() {
        let mut fields = line.split_whitespace();
        let device = fields.next().unwrap_or("");
        let mountpoint = fields.next().unwrap_or("");
        let fstype = fields.next().unwrap_or("");
        if mountpoint.is_empty() || !REAL_FS.contains(&fstype) {
            continue;
        }
        let path = unescape(mountpoint);
        if !seen.insert(path.clone()) {
            continue;
        }
        let container_path = if path == "/" {
            root.to_string()
        } else {
            format!("{root}{path}")
        };
        out.push(HostMount {
            path,
            container_path,
            device: device.to_string(),
            fstype: fstype.to_string(),
        });
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// `/proc/mounts` escapes space, tab, newline and backslash as octal.
fn unescape(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_octal() {
        assert_eq!(unescape("/mnt/my\\040disk"), "/mnt/my disk");
    }

    #[test]
    fn keeps_only_real_filesystems() {
        let tmp = tempfile::tempdir().unwrap();
        let mounts = tmp.path().join("mounts");
        std::fs::write(
            &mounts,
            "/dev/sda1 / ext4 rw 0 0\nproc /proc proc rw 0 0\ntmpfs /run tmpfs rw 0 0\n/dev/sdb1 /mnt/data xfs rw 0 0\n",
        )
        .unwrap();
        let found = read_mounts(Path::new("/host"), Some(&mounts));
        let paths: Vec<&str> = found.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(paths, vec!["/", "/mnt/data"]);
        assert_eq!(found[0].container_path, "/host");
        assert_eq!(found[1].container_path, "/host/mnt/data");
    }
}
