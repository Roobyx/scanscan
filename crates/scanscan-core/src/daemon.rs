//! Unix-domain-socket JSON-RPC 2.0 daemon.
//!
//! Newline-delimited requests; each yields one newline-delimited response.
//! The protocol is versioned in `scanscan-ipc` and mirrored by the TS server.

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use scanscan_ipc::rpc::{codes, RpcError, RpcRequest, RpcResponse};
use scanscan_ipc::{methods, ScanOptions, PROTOCOL_VERSION};

use crate::config::Config;
use crate::docker::DockerCollector;
use crate::index::Kind;
use crate::query::{FindFilters, Metric, QueryEngine, SortBy};
use crate::store::Store;
use crate::CORE_VERSION;

/// JSON-RPC method dispatcher over a [`Store`].
pub struct Daemon {
    store: Arc<Store>,
    config: Config,
    docker: DockerCollector,
}

impl Daemon {
    pub fn new(store: Arc<Store>, config: Config) -> Self {
        let target = std::env::var("SCANSCAN_DOCKER_SOCKET")
            .or_else(|_| std::env::var("DOCKER_HOST"))
            .unwrap_or_else(|_| "/var/run/docker.sock".to_string());
        Self {
            store,
            config,
            docker: DockerCollector::new(target),
        }
    }

    /// Handle one request, returning a response envelope.
    pub fn dispatch(&self, req: &RpcRequest) -> RpcResponse {
        match self.route(req) {
            Ok(value) => RpcResponse::success(req.id, value),
            Err(err) => RpcResponse::failure(req.id, err),
        }
    }

    fn route(&self, req: &RpcRequest) -> Result<Value, RpcError> {
        match req.method.as_str() {
            methods::HEALTH => Ok(json!({
                "status": "ok",
                "core_version": CORE_VERSION,
                "protocol": PROTOCOL_VERSION,
            })),
            methods::CONFIG_GET => Ok(json!({
                "data_dir": self.config.data_dir.to_string_lossy(),
                "socket": self.config.socket.to_string_lossy(),
                "bind": self.config.bind,
                "roots": self.config.roots,
                "docker_enabled": self.config.docker_enabled,
            })),
            methods::HOST_MOUNTS => {
                let host_root = std::env::var("SCANSCAN_HOST_ROOT")
                    .unwrap_or_else(|_| "/host".to_string());
                to_value(json!({
                    "hostRoot": host_root,
                    "mounts": crate::host::read_mounts(std::path::Path::new(&host_root)),
                }))
            }
            methods::SCANS_CREATE => {
                let p: ScansCreate = params(req)?;
                let mut options = p.options.unwrap_or_else(|| ScanOptions::new(Vec::new()));
                if options.roots.is_empty() {
                    options.roots = p.roots.unwrap_or_default();
                }
                let summary = self
                    .store
                    .start(options)
                    .map_err(|e| RpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                to_value(summary)
            }
            methods::SCANS_GET => {
                let p: IdParam = params(req)?;
                self.store
                    .get(&p.id)
                    .ok_or_else(|| RpcError::new(codes::NOT_FOUND, "scan not found"))
                    .and_then(to_value)
            }
            methods::SCANS_LIST => {
                let scans = self.store.list();
                to_value(json!({ "scans": scans }))
            }
            methods::SCANS_CANCEL => {
                let p: IdParam = params(req)?;
                Ok(json!({ "cancelled": self.store.cancel(&p.id) }))
            }
            methods::SCANS_DELETE => {
                let p: IdParam = params(req)?;
                self.store
                    .delete(&p.id)
                    .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))?;
                Ok(json!({ "deleted": true }))
            }
            methods::SCANS_PROGRESS => {
                let p: IdParam = params(req)?;
                self.store
                    .progress(&p.id)
                    .ok_or_else(|| RpcError::new(codes::NOT_FOUND, "scan not found"))
                    .and_then(to_value)
            }
            methods::TREE_CHILDREN => {
                let p: ChildrenParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let scope = p.scope.unwrap_or(0);
                let (total, items) = q.children(
                    scope,
                    SortBy::parse(p.sort.as_deref().unwrap_or("size")),
                    p.limit.unwrap_or(500),
                    p.offset.unwrap_or(0),
                );
                let records: Vec<_> = items.iter().map(|v| v.to_record()).collect();
                to_value(json!({
                    "parent": scope,
                    "total": total,
                    "offset": p.offset.unwrap_or(0),
                    "items": records,
                }))
            }
            methods::TREE_TILES => {
                let p: TilesParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let tiles = q.tiles(
                    p.scope.unwrap_or(0),
                    p.depth.unwrap_or(2),
                    Metric::parse(p.metric.as_deref().unwrap_or("alloc")),
                    p.color.as_deref().unwrap_or("ext"),
                );
                to_value(tiles)
            }
            methods::QUERY_TOP => {
                let p: TopParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let items = q.top_n(
                    p.scope.unwrap_or(0),
                    p.kind.as_deref().and_then(parse_kind),
                    Metric::parse(p.metric.as_deref().unwrap_or("alloc")),
                    p.n.unwrap_or(50),
                );
                let records: Vec<_> = items.iter().map(|v| v.to_record()).collect();
                to_value(json!({ "items": records }))
            }
            methods::QUERY_HISTOGRAM => {
                let p: HistParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let dim = p.dim.as_deref().unwrap_or("ext");
                let buckets = q.histogram(p.scope.unwrap_or(0), dim);
                Ok(json!({ "dim": dim, "items": buckets }))
            }
            methods::TREE_HIERARCHY => {
                let p: HierarchyParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let scope = p.scope.unwrap_or(0);
                let depth = p.depth.unwrap_or(2);
                let root = q.hierarchy(scope, depth, p.limit.unwrap_or(2000));
                Ok(json!({ "scope": scope, "depth": depth, "root": root }))
            }
            methods::QUERY_DUPLICATES => {
                let p: DupParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let mode = p.mode.unwrap_or_else(|| "name+size".to_string());
                let groups = q.duplicates(p.scope.unwrap_or(0), &mode, p.limit.unwrap_or(200));
                Ok(json!({ "mode": mode, "groups": groups }))
            }
            methods::QUERY_DIFF => {
                let p: DiffParam = params(req)?;
                let a = self.open(&p.a)?;
                let b = self.open(&p.b)?;
                to_value(crate::query::diff(&a, &b))
            }
            methods::QUERY_HEATMAP => {
                let p: HistParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                to_value(q.heatmap(p.scope.unwrap_or(0)))
            }
            methods::SNAPSHOTS_GC => {
                let p: GcParam = params(req)?;
                let removed = self
                    .store
                    .gc(p.keep.unwrap_or(3))
                    .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))?;
                Ok(json!({ "removed": removed }))
            }
            methods::QUERY_SEARCH => {
                let p: FindParam = params(req)?;
                let reader = self.open(&p.id)?;
                let q = QueryEngine::new(&reader);
                let filters = FindFilters {
                    size_min: p.size_min,
                    size_max: p.size_max,
                    mtime_before: p.mtime_before,
                    mtime_after: p.mtime_after,
                    ext: p.ext,
                    name_contains: p.name,
                    kind: p.kind.as_deref().and_then(parse_kind),
                    limit: p.limit.unwrap_or(500),
                };
                let items = q.find(p.scope.unwrap_or(0), &filters);
                let records: Vec<_> = items.iter().map(|v| v.to_record()).collect();
                to_value(json!({ "items": records }))
            }
            methods::DOCKER_CONTAINERS => to_value(json!({ "containers": self.docker.containers().unwrap_or_default() })),
            methods::DOCKER_MOUNTS => to_value(json!({ "mounts": self.docker.mounts().unwrap_or_default() })),
            methods::DOCKER_STATS => to_value(json!({ "containers": self.docker.stats().unwrap_or_default() })),
            methods::DOCKER_IMAGES => to_value(json!({ "images": self.docker.images().unwrap_or_default() })),
            methods::DOCKER_VOLUMES => to_value(json!({ "volumes": self.docker.volumes().unwrap_or_default() })),
            methods::DOCKER_MOUNTS_FOR => {
                let p: IdParam = params(req)?;
                let reader = self.open(&p.id)?;
                let host_root = std::env::var("SCANSCAN_HOST_ROOT")
                    .unwrap_or_else(|_| "/host".to_string());
                let mounts = crate::query::correlate_mounts(
                    &reader,
                    self.docker.mounts().unwrap_or_default(),
                    &host_root,
                );
                to_value(json!({ "mounts": mounts }))
            }
            "docker.status" => to_value(self.docker.status()),
            _ => Err(RpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("unknown method '{}'", req.method),
            )),
        }
    }

    fn open(&self, id: &str) -> Result<crate::index::IndexReader, RpcError> {
        self.store
            .open(id)
            .map_err(|e| RpcError::new(codes::NOT_FOUND, e.to_string()))
    }
}

#[derive(Deserialize)]
struct ScansCreate {
    #[serde(default)]
    roots: Option<Vec<String>>,
    #[serde(default)]
    options: Option<ScanOptions>,
}

#[derive(Deserialize)]
struct IdParam {
    id: String,
}

#[derive(Deserialize)]
struct ChildrenParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct TilesParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    depth: Option<u32>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    metric: Option<String>,
}

#[derive(Deserialize)]
struct TopParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    n: Option<usize>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    metric: Option<String>,
}

#[derive(Deserialize)]
struct HistParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    dim: Option<String>,
}

#[derive(Deserialize)]
struct HierarchyParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    depth: Option<u32>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct DupParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct DiffParam {
    a: String,
    b: String,
}

#[derive(Deserialize)]
struct GcParam {
    #[serde(default)]
    keep: Option<usize>,
}

#[derive(Deserialize)]
struct FindParam {
    id: String,
    #[serde(default)]
    scope: Option<u32>,
    #[serde(default)]
    size_min: Option<u64>,
    #[serde(default)]
    size_max: Option<u64>,
    #[serde(default)]
    mtime_before: Option<i64>,
    #[serde(default)]
    mtime_after: Option<i64>,
    #[serde(default)]
    ext: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

fn params<T: DeserializeOwned>(req: &RpcRequest) -> Result<T, RpcError> {
    let value = req.params.clone().unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|e| RpcError::new(codes::INVALID_PARAMS, e.to_string()))
}

fn to_value<T: serde::Serialize>(value: T) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))
}

fn parse_kind(s: &str) -> Option<Kind> {
    match s {
        "file" => Some(Kind::File),
        "dir" | "directory" => Some(Kind::Directory),
        "symlink" => Some(Kind::Symlink),
        "special" => Some(Kind::Special),
        _ => None,
    }
}

/// Serve the daemon on a Unix socket until the process is stopped.
#[cfg(unix)]
pub fn serve(daemon: Arc<Daemon>, socket: &std::path::Path) -> crate::error::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    if let Some(parent) = socket.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(socket);
    let listener = UnixListener::bind(socket)?;
    tracing::info!(socket = %socket.display(), "scanscan core daemon listening");

    for connection in listener.incoming() {
        let Ok(stream) = connection else { continue };
        let daemon = daemon.clone();
        std::thread::spawn(move || {
            let Ok(write_stream) = stream.try_clone() else {
                return;
            };
            let reader = BufReader::new(stream);
            let mut writer = write_stream;
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let response = match serde_json::from_str::<RpcRequest>(&line) {
                    Ok(req) => daemon.dispatch(&req),
                    Err(e) => RpcResponse::failure(0, RpcError::new(codes::PARSE_ERROR, e.to_string())),
                };
                let mut bytes = match serde_json::to_vec(&response) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                bytes.push(b'\n');
                if writer.write_all(&bytes).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        });
    }
    Ok(())
}

/// Non-Unix builds cannot host a Unix-socket daemon.
#[cfg(not(unix))]
pub fn serve(_daemon: Arc<Daemon>, _socket: &std::path::Path) -> crate::error::Result<()> {
    Err(crate::error::CoreError::Config(
        "the daemon requires a unix domain socket".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexWriter, Kind as IKind, ManifestSeed, NodeInput, NO_PARENT};

    fn seeded_store() -> Arc<Store> {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path()).unwrap();
        let dir = store.snapshot_path("snap1");
        let mut w = IndexWriter::create(&dir).unwrap();
        let root = w
            .push(NodeInput {
                parent: NO_PARENT,
                name: "root",
                kind: IKind::Directory,
                flags: 0,
                children: 1,
                size_app: 0,
                size_alloc: 0,
                mtime_ms: 0,
                uid: 0,
                gid: 0,
                mode: 0,
                ino: 0,
                dev: 0,
            })
            .unwrap();
        w.push(NodeInput {
            parent: root,
            name: "big.iso",
            kind: IKind::File,
            flags: 0,
            children: 0,
            size_app: 4096,
            size_alloc: 8192,
            mtime_ms: 0,
            uid: 0,
            gid: 0,
            mode: 0,
            ino: 0,
            dev: 0,
        })
        .unwrap();
        w.finish(ManifestSeed {
            id: "snap1".into(),
            parent_id: None,
            roots: vec!["/x".into()],
            started_at_ms: 0,
        })
        .unwrap();
        // Leak the tempdir so the store remains valid for the test body.
        std::mem::forget(tmp);
        Arc::new(store)
    }

    #[test]
    fn dispatches_health_and_queries() {
        let store = seeded_store();
        let daemon = Daemon::new(store, Config::default());

        let health = daemon.dispatch(&RpcRequest::new(1, methods::HEALTH, None));
        assert!(health.error.is_none());

        let top = daemon.dispatch(&RpcRequest::new(
            2,
            methods::QUERY_TOP,
            Some(json!({ "id": "snap1", "scope": 0, "n": 5 })),
        ));
        let result = top.result.expect("top result");
        assert_eq!(result["items"][0]["name"], "big.iso");

        let tiles = daemon.dispatch(&RpcRequest::new(
            3,
            methods::TREE_TILES,
            Some(json!({ "id": "snap1", "scope": 0, "depth": 1 })),
        ));
        assert!(tiles.error.is_none());

        let missing = daemon.dispatch(&RpcRequest::new(4, methods::SCANS_GET, Some(json!({ "id": "nope" }))));
        assert_eq!(missing.error.unwrap().code, codes::NOT_FOUND);
    }
}
