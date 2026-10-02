use serde::{Deserialize, Serialize};

/// Kind of a filesystem node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    File,
    Directory,
    Symlink,
    Special,
}

/// A single indexed node. Node ids are stable within a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    pub name: String,
    pub kind: NodeKind,
    pub size_apparent: u64,
    pub size_alloc: u64,
    pub mtime_ms: i64,
    pub subtree_size: u32,
    pub children: u32,
    #[serde(default)]
    pub has_children: bool,
    #[serde(default)]
    pub docker_mount: bool,
    #[serde(default)]
    pub error: bool,
}

/// One page of a directory listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeChildren {
    pub parent: u32,
    pub total: u32,
    pub offset: u32,
    pub items: Vec<NodeRecord>,
}

/// A single rectangle in a squarified treemap tile set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub node: u32,
    pub name: String,
    pub size: u64,
    /// Normalized [0,1] rectangle within the tile canvas.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_key: Option<String>,
}

/// A bounded tile set for one treemap viewport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TilesResponse {
    pub snapshot: String,
    pub scope: u32,
    pub depth: u32,
    pub tiles: Vec<Tile>,
    /// True when the server truncated deeper nodes into an aggregate.
    pub truncated: bool,
}
