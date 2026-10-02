//! Host mount discovery for the drive picker.
//!
//! `/proc/mounts` cannot be used from inside a container: procfs generates its
//! content from the *reader's* mount namespace, so even a bind-mounted host
//! copy shows the container's mounts. Instead we walk the read-only host root
//! (default `/host`) and detect mount points by device-id changes, which is
//! namespace-independent. The walk is depth- and budget-bounded.

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

/// A filesystem mount on the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMount {
    /// Mountpoint as seen on the host (e.g. `/`, `/mnt/data`).
    pub path: String,
    /// The same mount as seen inside the container (e.g. `/host/mnt/data`).
    pub container_path: String,
    /// Source device, when known.
    pub device: String,
    /// Filesystem type, when known.
    pub fstype: String,
}

/// Directories that never contain interesting mounts and can be very large.
const NO_DESCEND: [&str; 13] = [
    "proc", "sys", "dev", "run", "snap", "usr", "lib", "lib32", "lib64", "libx32", "bin", "sbin",
    "etc",
];

const MAX_DEPTH: u32 = 3;
const MAX_VISITS: usize = 10_000;

/// Enumerate the host's filesystems by walking the read-only host root.
pub fn read_mounts(host_root: &Path) -> Vec<HostMount> {
    let root_dev = dev_of(host_root);
    let mut out = vec![HostMount {
        path: "/".to_string(),
        container_path: host_root.to_string_lossy().into_owned(),
        device: String::new(),
        fstype: String::new(),
    }];
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(host_root.to_string_lossy().into_owned());
    let mut budget = MAX_VISITS;
    walk(host_root, host_root, 0, root_dev, &mut out, &mut seen, &mut budget);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: u32,
    parent_dev: u64,
    out: &mut Vec<HostMount>,
    seen: &mut HashSet<String>,
    budget: &mut usize,
) {
    if depth >= MAX_DEPTH || *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            break;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if NO_DESCEND.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        // symlink_metadata: do not follow symlinks (a symlink is not a mount).
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        *budget -= 1;
        let dev = dev_of(&path);
        if dev != parent_dev {
            if seen.insert(path.to_string_lossy().into_owned()) {
                out.push(HostMount {
                    path: relative(root, &path),
                    container_path: path.to_string_lossy().into_owned(),
                    device: String::new(),
                    fstype: String::new(),
                });
            }
            // Do not descend into a different filesystem.
            continue;
        }
        walk(root, &path, depth + 1, dev, out, seen, budget);
    }
}

fn relative(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rest) => format!("/{}", rest.to_string_lossy()),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

fn dev_of(path: &Path) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::symlink_metadata(path).map(|m| m.dev()).unwrap_or(0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_root_entry() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("mnt/data")).unwrap();
        let mounts = read_mounts(tmp.path());
        assert_eq!(mounts[0].path, "/");
        assert_eq!(mounts[0].container_path, tmp.path().to_string_lossy());
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative(Path::new("/host"), Path::new("/host/boot/efi")), "/boot/efi");
    }
}
