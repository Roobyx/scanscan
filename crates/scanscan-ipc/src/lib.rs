//! Wire protocol for the scanscan core daemon.
//!
//! This crate is the single source of truth for the IPC contract between the
//! Rust core and the TypeScript server/client. `packages/api-types` is
//! generated from (and must stay in sync with) these types.

pub mod docker;
pub mod rpc;
pub mod scan;
pub mod tree;

pub use docker::{ContainerInfo, DockerStats, MountInfo, MountKind};
pub use rpc::{RpcError, RpcNotification, RpcRequest, RpcResponse};
pub use scan::{ScanOptions, ScanProgress, ScanState, ScanSummary};
pub use tree::{NodeChildren, NodeKind, NodeRecord, Tile, TilesResponse};

/// Version of the IPC protocol. Bumped on any breaking change to the types
/// above; the server refuses to talk to a core with a different major version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Namespaced JSON-RPC method names exposed by the core daemon.
pub mod methods {
    pub const SCANS_CREATE: &str = "scans.create";
    pub const SCANS_GET: &str = "scans.get";
    pub const SCANS_LIST: &str = "scans.list";
    pub const SCANS_DELETE: &str = "scans.delete";
    pub const SCANS_CANCEL: &str = "scans.cancel";
    pub const SCANS_PROGRESS: &str = "scans.progress";

    pub const TREE_CHILDREN: &str = "tree.children";
    pub const TREE_TILES: &str = "tree.tiles";

    pub const QUERY_TOP: &str = "query.top";
    pub const QUERY_SEARCH: &str = "query.search";
    pub const QUERY_HISTOGRAM: &str = "query.histogram";
    pub const QUERY_DIFF: &str = "query.diff";

    pub const DOCKER_CONTAINERS: &str = "docker.containers";
    pub const DOCKER_STATS: &str = "docker.stats";
    pub const DOCKER_MOUNTS: &str = "docker.mounts";

    pub const EXPORT_RUN: &str = "export.run";
    pub const CONFIG_GET: &str = "config.get";
    pub const CONFIG_SET: &str = "config.set";
    pub const HEALTH: &str = "core.health";
}
