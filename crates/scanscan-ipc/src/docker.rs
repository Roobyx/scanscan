use serde::{Deserialize, Serialize};

/// How a container mount maps onto the host filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountKind {
    Bind,
    Volume,
    OverlayUpper,
    Tmpfs,
}

/// A container mount correlated with an index node where possible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MountInfo {
    pub container_id: String,
    pub container_name: String,
    pub kind: MountKind,
    pub source: String,
    pub destination: String,
    pub read_write: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// A container summary derived from the read-only Docker Engine API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_rw: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_root_fs: Option<u64>,
    #[serde(default)]
    pub mounts: Vec<MountInfo>,
}

/// A single non-streaming Docker stats sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockerStats {
    pub container_id: String,
    pub cpu_percent: f64,
    pub mem_used: u64,
    pub mem_limit: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub blk_read: u64,
    pub blk_write: u64,
}
