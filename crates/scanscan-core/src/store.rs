//! Snapshot catalog and background scan jobs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use scanscan_ipc::{ScanOptions, ScanProgress, ScanState, ScanSummary};

use crate::error::{CoreError, Result};
use crate::index::{now_ms, IndexReader, IndexWriter, Manifest, ManifestSeed};
use crate::scanner::{canonical_root, platform_fs, scan, Previous, Progress};

struct Job {
    summary: ScanSummary,
    progress: ScanProgress,
    cancel: Arc<AtomicBool>,
}

/// Owns the data directory and the running/completed scans.
pub struct Store {
    data_dir: PathBuf,
    snapshots_dir: PathBuf,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
}

impl Store {
    pub fn new(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        let snapshots_dir = data_dir.join("snapshots");
        std::fs::create_dir_all(&snapshots_dir)?;
        Ok(Self {
            data_dir,
            snapshots_dir,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn snapshots_dir(&self) -> &Path {
        &self.snapshots_dir
    }

    pub fn snapshot_path(&self, id: &str) -> PathBuf {
        self.snapshots_dir.join(id)
    }

    /// All known scans: completed snapshots on disk plus live jobs.
    pub fn list(&self) -> Vec<ScanSummary> {
        let mut out: Vec<ScanSummary> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.snapshots_dir) {
            for entry in entries.flatten() {
                let manifest = entry.path().join("manifest.json");
                if let Ok(bytes) = std::fs::read(&manifest) {
                    if let Ok(m) = serde_json::from_slice::<Manifest>(&bytes) {
                        out.push(manifest_to_summary(&m));
                    }
                }
            }
        }
        if let Ok(jobs) = self.jobs.lock() {
            for job in jobs.values() {
                if job.summary.state == ScanState::Running || job.summary.state == ScanState::Queued {
                    out.push(job.summary.clone());
                }
            }
        }
        out.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
        out
    }

    pub fn get(&self, id: &str) -> Option<ScanSummary> {
        if let Ok(jobs) = self.jobs.lock() {
            if let Some(job) = jobs.get(id) {
                return Some(job.summary.clone());
            }
        }
        let bytes = std::fs::read(self.snapshot_path(id).join("manifest.json")).ok()?;
        let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
        Some(manifest_to_summary(&manifest))
    }

    pub fn progress(&self, id: &str) -> Option<ScanProgress> {
        if let Ok(jobs) = self.jobs.lock() {
            if let Some(job) = jobs.get(id) {
                return Some(job.progress.clone());
            }
        }
        let summary = self.get(id)?;
        Some(ScanProgress {
            scan_id: summary.id,
            state: summary.state,
            files: summary.files,
            dirs: summary.dirs,
            bytes_apparent: summary.bytes_apparent,
            bytes_alloc: summary.bytes_alloc,
            errors: summary.errors,
            elapsed_ms: (summary.finished_at_ms.unwrap_or(summary.started_at_ms)
                - summary.started_at_ms)
                .max(0) as u64,
            eta_ms: None,
            current_path: None,
        })
    }

    pub fn open(&self, id: &str) -> Result<IndexReader> {
        IndexReader::open(&self.snapshot_path(id))
    }

    /// Start a scan in the background and return its initial summary.
    pub fn start(&self, options: ScanOptions) -> Result<ScanSummary> {
        if options.roots.is_empty() {
            return Err(CoreError::Config("scan requires at least one root".into()));
        }
        let mut options = options;
        // Never index the store's own data directory, even when a root contains it.
        let data_dir = self.data_dir.to_string_lossy().into_owned();
        options.exclusions.push(data_dir.clone());
        options.exclusions.push(format!("{data_dir}/**"));
        let roots: Vec<PathBuf> = options
            .roots
            .iter()
            .map(|r| canonical_root(r))
            .collect::<Result<_>>()?;

        let id = new_id();
        let snap = self.snapshot_path(&id);
        let started = now_ms();

        let summary = ScanSummary {
            id: id.clone(),
            parent_id: None,
            state: ScanState::Running,
            roots: options.roots.clone(),
            started_at_ms: started,
            finished_at_ms: None,
            files: 0,
            dirs: 0,
            bytes_apparent: 0,
            bytes_alloc: 0,
            errors: 0,
        };
        let progress = ScanProgress {
            scan_id: id.clone(),
            state: ScanState::Running,
            files: 0,
            dirs: 0,
            bytes_apparent: 0,
            bytes_alloc: 0,
            errors: 0,
            elapsed_ms: 0,
            eta_ms: None,
            current_path: None,
        };
        let cancel = Arc::new(AtomicBool::new(false));

        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.insert(
                id.clone(),
                Job {
                    summary: summary.clone(),
                    progress,
                    cancel: cancel.clone(),
                },
            );
        }

        let jobs = self.jobs.clone();
        let job_id = id.clone();
        let roots_for_thread = roots;
        let options_for_thread = options.clone();
        let snapshots_dir = self.snapshots_dir.clone();
        thread::spawn(move || {
            run_scan(
                jobs,
                job_id,
                snap,
                snapshots_dir,
                roots_for_thread,
                options_for_thread,
                started,
                cancel,
            );
        });

        Ok(summary)
    }

    pub fn cancel(&self, id: &str) -> bool {
        if let Ok(jobs) = self.jobs.lock() {
            if let Some(job) = jobs.get(id) {
                job.cancel.store(true, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        if let Ok(mut jobs) = self.jobs.lock() {
            if let Some(job) = jobs.get(id) {
                job.cancel.store(true, Ordering::Relaxed);
                jobs.remove(id);
            }
        }
        let path = self.snapshot_path(id);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    /// Delete completed snapshots beyond the newest `keep`.
    pub fn gc(&self, keep: usize) -> Result<Vec<String>> {
        let mut scans: Vec<ScanSummary> = self
            .list()
            .into_iter()
            .filter(|s| s.state == ScanState::Completed)
            .collect();
        scans.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
        let mut removed = Vec::new();
        for scan in scans.into_iter().skip(keep) {
            self.delete(&scan.id)?;
            removed.push(scan.id);
        }
        Ok(removed)
    }
}

fn run_scan(
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    id: String,
    snap: PathBuf,
    snapshots_dir: PathBuf,
    roots: Vec<PathBuf>,
    options: ScanOptions,
    started: i64,
    cancel: Arc<AtomicBool>,
) {
    let fs = platform_fs();
    let mut writer = match IndexWriter::create(&snap) {
        Ok(w) => w,
        Err(e) => return finish_failed(&jobs, &id, e.to_string()),
    };

    let previous = if options.incremental {
        latest_completed(&snapshots_dir, &snap)
            .and_then(|dir| IndexReader::open(&dir).ok())
            .map(Previous::build)
    } else {
        None
    };

    let jobs_cb = jobs.clone();
    let id_cb = id.clone();
    let mut on_progress = move |p: &Progress| {
        if let Ok(mut map) = jobs_cb.lock() {
            if let Some(job) = map.get_mut(&id_cb) {
                job.progress.files = p.files;
                job.progress.dirs = p.dirs;
                job.progress.bytes_apparent = p.bytes_apparent;
                job.progress.bytes_alloc = p.bytes_alloc;
                job.progress.errors = p.errors;
                job.progress.elapsed_ms = p.elapsed_ms;
                job.progress.current_path = Some(p.current_path.clone());
                job.summary.files = p.files;
                job.summary.dirs = p.dirs;
                job.summary.bytes_apparent = p.bytes_apparent;
                job.summary.bytes_alloc = p.bytes_alloc;
                job.summary.errors = p.errors;
            }
        }
    };

    let result = scan(
        &mut writer,
        &roots,
        &options,
        fs.as_ref(),
        &cancel,
        previous.as_ref(),
        &mut on_progress,
    );
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&snap);
        let state = if cancel.load(Ordering::Relaxed) {
            ScanState::Cancelled
        } else {
            ScanState::Failed
        };
        return finish_state(&jobs, &id, state, Some(e.to_string()));
    }

    match writer.finish(ManifestSeed {
        id: id.clone(),
        parent_id: None,
        roots: roots.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        started_at_ms: started,
    }) {
        Ok(manifest) => {
            if let Ok(mut map) = jobs.lock() {
                if let Some(job) = map.get_mut(&id) {
                    job.summary.state = ScanState::Completed;
                    job.summary.finished_at_ms = Some(manifest.finished_at_ms);
                    job.summary.files = manifest.stats.files;
                    job.summary.dirs = manifest.stats.dirs;
                    job.summary.bytes_apparent = manifest.stats.bytes_apparent;
                    job.summary.bytes_alloc = manifest.stats.bytes_alloc;
                    job.summary.errors = manifest.stats.errors;
                    job.progress.state = ScanState::Completed;
                }
            }
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&snap);
            finish_failed(&jobs, &id, e.to_string());
        }
    }
}

fn finish_failed(jobs: &Arc<Mutex<HashMap<String, Job>>>, id: &str, message: String) {
    finish_state(jobs, id, ScanState::Failed, Some(message));
}

fn finish_state(
    jobs: &Arc<Mutex<HashMap<String, Job>>>,
    id: &str,
    state: ScanState,
    _message: Option<String>,
) {
    if let Ok(mut map) = jobs.lock() {
        if let Some(job) = map.get_mut(id) {
            job.summary.state = state;
            job.summary.finished_at_ms = Some(now_ms());
            job.progress.state = state;
        }
    }
}

fn manifest_to_summary(m: &Manifest) -> ScanSummary {
    ScanSummary {
        id: m.id.clone(),
        parent_id: m.parent_id.clone(),
        state: ScanState::Completed,
        roots: m.roots.clone(),
        started_at_ms: m.started_at_ms,
        finished_at_ms: Some(m.finished_at_ms),
        files: m.stats.files,
        dirs: m.stats.dirs,
        bytes_apparent: m.stats.bytes_apparent,
        bytes_alloc: m.stats.bytes_alloc,
        errors: m.stats.errors,
    }
}

/// Newest completed snapshot directory other than `current`, for incremental reuse.
fn latest_completed(snapshots_dir: &Path, current: &Path) -> Option<PathBuf> {
    let mut best: Option<(i64, PathBuf)> = None;
    for entry in std::fs::read_dir(snapshots_dir).ok()?.flatten() {
        let dir = entry.path();
        if dir == current {
            continue;
        }
        let Ok(bytes) = std::fs::read(dir.join("manifest.json")) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
            continue;
        };
        if best
            .as_ref()
            .map(|(ms, _)| manifest.finished_at_ms > *ms)
            .unwrap_or(true)
        {
            best = Some((manifest.finished_at_ms, dir));
        }
    }
    best.map(|(_, dir)| dir)
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}
