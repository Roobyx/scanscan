use serde::{Deserialize, Serialize};

/// Options controlling a scan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Absolute roots to scan. Each is canonicalized before the walk.
    pub roots: Vec<String>,
    /// Reuse unchanged subtrees from the parent snapshot when possible.
    #[serde(default)]
    pub incremental: bool,
    /// Do not descend across filesystem boundaries (default: true).
    #[serde(default = "default_true")]
    pub one_file_system: bool,
    /// Follow symlinks, with loop detection.
    #[serde(default)]
    pub follow_symlinks: bool,
    /// Extra exclude globs (gitignore syntax).
    #[serde(default)]
    pub exclusions: Vec<String>,
    /// Paths to `.scanscanignore`-style files.
    #[serde(default)]
    pub ignore_files: Vec<String>,
    /// Worker threads; `None` lets the core choose based on available CPUs.
    #[serde(default)]
    pub threads: Option<usize>,
    /// Record allocated size (blocks) in addition to apparent size.
    #[serde(default = "default_true")]
    pub allocated: bool,
}

fn default_true() -> bool {
    true
}

impl ScanOptions {
    pub fn new(roots: Vec<String>) -> Self {
        Self {
            roots,
            incremental: false,
            one_file_system: true,
            follow_symlinks: false,
            exclusions: Vec::new(),
            ignore_files: Vec::new(),
            threads: None,
            allocated: true,
        }
    }
}

/// Lifecycle state of a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Immutable summary of a scan/snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanSummary {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub state: ScanState,
    pub roots: Vec<String>,
    pub started_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<i64>,
    pub files: u64,
    pub dirs: u64,
    pub bytes_apparent: u64,
    pub bytes_alloc: u64,
    pub errors: u64,
}

/// Throttled progress event emitted while a scan runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanProgress {
    pub scan_id: String,
    pub state: ScanState,
    pub files: u64,
    pub dirs: u64,
    pub bytes_apparent: u64,
    pub bytes_alloc: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_defaults_are_stable() {
        let opts = ScanOptions::new(vec!["/".into()]);
        assert!(opts.one_file_system);
        assert!(opts.allocated);
        assert!(!opts.incremental);
    }

    #[test]
    fn state_serializes_lowercase() {
        let json = serde_json::to_string(&ScanState::Running).unwrap();
        assert_eq!(json, "\"running\"");
    }
}
