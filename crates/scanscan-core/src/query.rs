//! Query engine over an [`IndexReader`].
//!
//! All operations scan contiguous subtree id ranges, so folder-scoped work is
//! bounded by the subtree rather than the whole snapshot.

use scanscan_ipc::{MountInfo, NodeKind, NodeRecord, Tile, TilesResponse};
use serde::Serialize;

use crate::index::{flags, IndexReader, Kind};

/// Which size metric a query ranks by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Alloc,
    Apparent,
    Items,
}

impl Metric {
    pub fn parse(s: &str) -> Metric {
        match s {
            "apparent" => Metric::Apparent,
            "items" => Metric::Items,
            _ => Metric::Alloc,
        }
    }

    fn value(self, rec: &crate::index::Record, subtree_size: u32) -> u64 {
        match self {
            Metric::Alloc => rec.size_alloc,
            Metric::Apparent => rec.size_app,
            Metric::Items => u64::from(subtree_size),
        }
    }
}

/// Sort order for child listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Size,
    Name,
    Mtime,
}

impl SortBy {
    pub fn parse(s: &str) -> SortBy {
        match s {
            "name" => SortBy::Name,
            "mtime" => SortBy::Mtime,
            _ => SortBy::Size,
        }
    }
}

/// A decoded node plus its name and extension, ready to serialise.
#[derive(Debug, Clone)]
pub struct NodeView {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub kind: Kind,
    pub flags: u8,
    pub size_apparent: u64,
    pub size_alloc: u64,
    pub mtime_ms: i64,
    pub subtree_size: u32,
    pub children: u32,
    pub ext: String,
    /// Optional display label (e.g. the Docker container owning an overlay2 layer).
    pub label: String,
}

impl NodeView {
    pub fn to_record(&self) -> NodeRecord {
        NodeRecord {
            id: self.id,
            parent: self.parent,
            name: self.name.clone(),
            kind: kind_to_ipc(self.kind),
            size_apparent: self.size_apparent,
            size_alloc: self.size_alloc,
            mtime_ms: self.mtime_ms,
            subtree_size: self.subtree_size,
            children: self.children,
            has_children: self.children > 0,
            docker_mount: self.flags & flags::DOCKER_MOUNT != 0,
            label: opt_label(&self.label),
            error: self.flags & flags::ERROR != 0,
            path: None,
        }
    }
}

/// Map the empty label sentinel to `None` for the wire (0 = "no label").
fn opt_label(label: &str) -> Option<String> {
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

fn kind_to_ipc(kind: Kind) -> NodeKind {
    match kind {
        Kind::File => NodeKind::File,
        Kind::Directory => NodeKind::Directory,
        Kind::Symlink => NodeKind::Symlink,
        Kind::Special => NodeKind::Special,
    }
}

/// Filters for `find`.
#[derive(Debug, Clone, Default)]
pub struct FindFilters {
    pub size_min: Option<u64>,
    pub size_max: Option<u64>,
    pub mtime_before: Option<i64>,
    pub mtime_after: Option<i64>,
    pub ext: Option<String>,
    pub name_contains: Option<String>,
    pub kind: Option<Kind>,
    pub limit: usize,
}

/// Read-only query facade.
pub struct QueryEngine<'a> {
    reader: &'a IndexReader,
}

impl<'a> QueryEngine<'a> {
    pub fn new(reader: &'a IndexReader) -> Self {
        Self { reader }
    }

    pub fn reader(&self) -> &IndexReader {
        self.reader
    }

    pub fn node(&self, id: u32) -> Option<NodeView> {
        let rec = self.reader.record(id)?;
        Some(NodeView {
            id,
            parent: if rec.parent == crate::index::NO_PARENT {
                None
            } else {
                Some(rec.parent)
            },
            name: self.reader.name(id).to_string(),
            kind: rec.kind,
            flags: rec.flags,
            size_apparent: rec.size_app,
            size_alloc: rec.size_alloc,
            mtime_ms: rec.mtime_ms,
            subtree_size: self.reader.subtree_size(id),
            children: rec.children,
            ext: self.reader.ext(id).to_string(),
            label: self.reader.label_of(rec.label_id).to_string(),
        })
    }

    /// Root node ids (nodes with no parent).
    pub fn roots(&self) -> Vec<u32> {
        (0..self.reader.len())
            .filter(|id| {
                self.reader
                    .record(*id)
                    .map(|r| r.parent == crate::index::NO_PARENT)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Direct children of `id`, sorted and paginated.
    pub fn children(&self, id: u32, sort: SortBy, limit: usize, offset: usize) -> (u32, Vec<NodeView>) {
        let mut views: Vec<NodeView> = self
            .reader
            .children(id)
            .into_iter()
            .filter_map(|c| self.node(c))
            .collect();

        match sort {
            SortBy::Size => {
                // A directory carries no size of its own: rank it by the bytes in
                // its subtree so folders sort alongside files, matching the
                // hierarchy views and `du`.
                for view in &mut views {
                    if view.kind == Kind::Directory {
                        view.size_alloc = self.subtree_bytes(view.id, Metric::Alloc);
                        view.size_apparent = self.subtree_bytes(view.id, Metric::Apparent);
                    }
                }
                views.sort_by(|a, b| {
                    b.size_alloc
                        .cmp(&a.size_alloc)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
            SortBy::Name => views.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Mtime => views.sort_by(|a, b| b.mtime_ms.cmp(&a.mtime_ms)),
        }

        let total = views.len() as u32;
        let items = views.into_iter().skip(offset).take(limit).collect();
        (total, items)
    }

    /// Largest nodes within `scope` (inclusive), optionally filtered by kind.
    pub fn top_n(&self, scope: u32, kind: Option<Kind>, metric: Metric, n: usize) -> Vec<NodeView> {
        let mut heap: Vec<NodeView> = Vec::new();
        for id in self.reader.subtree_range(scope) {
            let Some(view) = self.node(id) else { continue };
            if let Some(k) = kind {
                if view.kind != k {
                    continue;
                }
            }
            heap.push(view);
        }
        heap.sort_by(|a, b| {
            let av = metric.value(
                &self.reader.record(a.id).unwrap_or_else(dummy_record),
                a.subtree_size,
            );
            let bv = metric.value(
                &self.reader.record(b.id).unwrap_or_else(dummy_record),
                b.subtree_size,
            );
            bv.cmp(&av).then_with(|| a.name.cmp(&b.name))
        });
        heap.truncate(n);
        heap
    }

    /// Aggregate size/count per extension within `scope`.
    pub fn histogram_ext(&self, scope: u32) -> Vec<(String, u64, u64)> {
        let mut map: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else { continue };
            if rec.kind.is_dir() {
                continue;
            }
            let ext = self.reader.ext(id).to_string();
            let entry = map.entry(ext).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.saturating_add(rec.size_alloc);
        }
        let mut out: Vec<(String, u64, u64)> = map
            .into_iter()
            .map(|(ext, (count, size))| (ext, count, size))
            .collect();
        out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Depth-limited treemap tiles for `scope`.
    pub fn tiles(&self, scope: u32, depth: u32, metric: Metric, color: &str) -> TilesResponse {
        let target = depth.max(1);
        let mut buckets: Vec<(u32, u64)> = Vec::new();
        let mut stack: Vec<u32> = Vec::new();
        let range = self.reader.subtree_range(scope);
        let mut i = range.start;

        while i < range.end {
            while let Some(&top) = stack.last() {
                let end = top.saturating_add(self.reader.subtree_size(top));
                if i >= end {
                    stack.pop();
                } else {
                    break;
                }
            }

            let depth_rel = stack.len() as u32;
            let rec = self.reader.record(i);

            if depth_rel >= target {
                buckets.push((i, self.bucket_size(i, metric)));
                i = i.saturating_add(self.reader.subtree_size(i).max(1));
                continue;
            }

            match rec {
                Some(r) if r.kind.is_dir() && self.reader.subtree_size(i) > 1 => {
                    stack.push(i);
                    i += 1;
                }
                _ => {
                    buckets.push((i, self.bucket_size(i, metric)));
                    i += 1;
                }
            }
        }

        buckets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let values: Vec<f64> = buckets.iter().map(|(_, s)| *s as f64).collect();
        let rects = squarify(&values);

        let mut tiles: Vec<Tile> = Vec::with_capacity(buckets.len());
        for ((id, size), rect) in buckets.iter().zip(rects.iter()) {
            let name = self.reader.name(*id).to_string();
            let color_key = match color {
                "ext" => self.reader.ext(*id).to_string(),
                _ => String::new(),
            };
            let label = self.reader.label(*id).to_string();
            tiles.push(Tile {
                node: *id,
                name,
                size: *size,
                x: rect[0] as f32,
                y: rect[1] as f32,
                w: rect[2] as f32,
                h: rect[3] as f32,
                color_key: if color_key.is_empty() {
                    None
                } else {
                    Some(color_key)
                },
                label: opt_label(&label),
            });
        }

        TilesResponse {
            snapshot: self.reader.manifest().id.clone(),
            scope,
            depth: target,
            tiles,
            truncated: false,
        }
    }

    fn bucket_size(&self, id: u32, metric: Metric) -> u64 {
        match self.reader.record(id) {
            // A directory's own size is zero; a tile must show the bytes it
            // contains, otherwise folders vanish from the treemap.
            Some(rec) if rec.kind.is_dir() && metric != Metric::Items => {
                self.subtree_bytes(id, metric)
            }
            Some(rec) => metric.value(&rec, self.reader.subtree_size(id)),
            None => 0,
        }
    }

    /// Filter nodes within `scope` by size/mtime/extension/name/kind.
    pub fn find(&self, scope: u32, filters: &FindFilters) -> Vec<NodeView> {
        let mut out = Vec::new();
        for id in self.reader.subtree_range(scope) {
            if out.len() >= filters.limit.max(1) {
                break;
            }
            let Some(view) = self.node(id) else { continue };
            if let Some(k) = filters.kind {
                if view.kind != k {
                    continue;
                }
            }
            if let Some(min) = filters.size_min {
                if view.size_alloc < min {
                    continue;
                }
            }
            if let Some(max) = filters.size_max {
                if view.size_alloc > max {
                    continue;
                }
            }
            if let Some(before) = filters.mtime_before {
                if view.mtime_ms >= before {
                    continue;
                }
            }
            if let Some(after) = filters.mtime_after {
                if view.mtime_ms <= after {
                    continue;
                }
            }
            if let Some(ext) = &filters.ext {
                if !view.ext.eq_ignore_ascii_case(ext) {
                    continue;
                }
            }
            if let Some(needle) = &filters.name_contains {
                if !view.name.to_lowercase().contains(&needle.to_lowercase()) {
                    continue;
                }
            }
            out.push(view);
        }
        out
    }

    /// Aggregate count/size over a dimension (`ext`, `age`, `owner`, `size`).
    pub fn histogram(&self, scope: u32, dim: &str) -> Vec<Bucket> {
        match dim {
            "owner" => self.histogram_owner(scope),
            "age" => self.histogram_age(scope),
            "size" => self.histogram_size(scope),
            _ => self.histogram_ext_buckets(scope),
        }
    }

    fn histogram_ext_buckets(&self, scope: u32) -> Vec<Bucket> {
        self.histogram_ext(scope)
            .into_iter()
            .map(|(ext, count, size)| Bucket {
                label: if ext.is_empty() {
                    "(none)".to_string()
                } else {
                    ext.clone()
                },
                key: ext,
                count,
                size,
            })
            .collect()
    }

    fn histogram_owner(&self, scope: u32) -> Vec<Bucket> {
        let mut map: std::collections::HashMap<(u32, u32), (u64, u64)> =
            std::collections::HashMap::new();
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else {
                continue;
            };
            if rec.kind.is_dir() {
                continue;
            }
            let entry = map.entry((rec.uid, rec.gid)).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.saturating_add(rec.size_alloc);
        }
        let mut out: Vec<Bucket> = map
            .into_iter()
            .map(|((uid, gid), (count, size))| Bucket {
                key: format!("{uid}:{gid}"),
                label: format!("{uid}:{gid}"),
                count,
                size,
            })
            .collect();
        out.sort_by(|a, b| b.size.cmp(&a.size));
        out
    }

    fn histogram_age(&self, scope: u32) -> Vec<Bucket> {
        const DAY: i64 = 86_400_000;
        let edges: [(i64, &str); 7] = [
            (DAY, "<1d"),
            (7 * DAY, "<7d"),
            (30 * DAY, "<30d"),
            (90 * DAY, "<90d"),
            (365 * DAY, "<1y"),
            (1095 * DAY, "<3y"),
            (i64::MAX, ">3y"),
        ];
        let now = crate::index::now_ms();
        let mut counts = vec![(0u64, 0u64); edges.len()];
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else {
                continue;
            };
            if rec.kind.is_dir() {
                continue;
            }
            let age = (now - rec.mtime_ms).max(0);
            if let Some(slot) = edges.iter().position(|(limit, _)| age < *limit) {
                counts[slot].0 += 1;
                counts[slot].1 = counts[slot].1.saturating_add(rec.size_alloc);
            }
        }
        edges
            .iter()
            .enumerate()
            .map(|(i, (_, label))| Bucket {
                key: (*label).to_string(),
                label: (*label).to_string(),
                count: counts[i].0,
                size: counts[i].1,
            })
            .collect()
    }

    fn histogram_size(&self, scope: u32) -> Vec<Bucket> {
        const KB: u64 = 1024;
        let edges: [(u64, &str); 8] = [
            (KB, "0-1 KB"),
            (10 * KB, "1-10 KB"),
            (100 * KB, "10-100 KB"),
            (KB * KB, "100 KB-1 MB"),
            (10 * KB * KB, "1-10 MB"),
            (100 * KB * KB, "10-100 MB"),
            (KB * KB * KB, "100 MB-1 GB"),
            (u64::MAX, ">1 GB"),
        ];
        let mut counts = vec![(0u64, 0u64); edges.len()];
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else {
                continue;
            };
            if rec.kind.is_dir() {
                continue;
            }
            if let Some(slot) = edges.iter().position(|(limit, _)| rec.size_alloc < *limit) {
                counts[slot].0 += 1;
                counts[slot].1 = counts[slot].1.saturating_add(rec.size_alloc);
            }
        }
        edges
            .iter()
            .enumerate()
            .map(|(i, (_, label))| Bucket {
                key: (*label).to_string(),
                label: (*label).to_string(),
                count: counts[i].0,
                size: counts[i].1,
            })
            .collect()
    }

    /// Total of `metric` over the subtree rooted at `scope`.
    pub fn subtree_bytes(&self, scope: u32, metric: Metric) -> u64 {
        let mut total = 0u64;
        for id in self.reader.subtree_range(scope) {
            if let Some(rec) = self.reader.record(id) {
                total = total.saturating_add(metric.value(&rec, self.reader.subtree_size(id)));
            }
        }
        total
    }

    /// Bounded nested tree for sunburst/icicle/bubble layouts.
    pub fn hierarchy(&self, scope: u32, depth: u32, limit: usize) -> HierarchyNode {
        let mut budget = limit.max(1);
        self.build_hierarchy(scope, depth, &mut budget)
    }

    fn build_hierarchy(&self, id: u32, depth: u32, budget: &mut usize) -> HierarchyNode {
        let view = self.node(id);
        let (name, kind, size, is_dir) = match &view {
            Some(v) => {
                let size = if v.kind == Kind::Directory {
                    self.subtree_bytes(id, Metric::Alloc)
                } else {
                    v.size_alloc
                };
                (
                    v.name.clone(),
                    kind_name(v.kind).to_string(),
                    size,
                    v.kind == Kind::Directory,
                )
            }
            None => (String::new(), "special".to_string(), 0, false),
        };
        let mut children = Vec::new();
        if depth > 0 && is_dir {
            let mut kids: Vec<(u32, u64)> = self
                .reader
                .children(id)
                .into_iter()
                .map(|child| (child, self.subtree_bytes(child, Metric::Alloc)))
                .collect();
            kids.sort_by(|a, b| b.1.cmp(&a.1));
            for (child, _) in kids {
                if *budget == 0 {
                    break;
                }
                *budget -= 1;
                children.push(self.build_hierarchy(child, depth - 1, budget));
            }
        }
        HierarchyNode {
            id,
            name,
            kind,
            size,
            children,
        }
    }

    /// Groups of metadata-identical files (name+size, optionally +mtime).
    pub fn duplicates(&self, scope: u32, mode: &str, limit: usize) -> Vec<DupGroup> {
        let use_mtime = mode.contains("mtime");
        // Pass 1: count occurrences per (name, size[, mtime]). Keying on borrowed
        // names (stable in the reader's mmap) avoids a String per file, and
        // counting first avoids a Vec per file (most files are unique).
        let mut counts: std::collections::HashMap<(&str, u64, i64), u32> =
            std::collections::HashMap::new();
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else {
                continue;
            };
            if rec.kind.is_dir() || rec.size_alloc == 0 {
                continue;
            }
            let key = (
                self.reader.name(id),
                rec.size_alloc,
                if use_mtime { rec.mtime_ms } else { 0 },
            );
            *counts.entry(key).or_insert(0) += 1;
        }

        // Only keys seen more than once matter; index them for pass 2.
        let mut index: std::collections::HashMap<(&str, u64, i64), usize> =
            std::collections::HashMap::new();
        let mut groups: Vec<DupGroup> = Vec::new();
        for (&key, &count) in &counts {
            if count > 1 {
                index.insert(key, groups.len());
                groups.push(DupGroup {
                    key: format!("{} ({} B)", key.0, key.1),
                    count: count as u64,
                    size: key.1,
                    wasted: key.1.saturating_mul((count as u64).saturating_sub(1)),
                    items: Vec::new(),
                });
            }
        }

        // Pass 2: collect up to 50 example nodes per duplicate group, resolving
        // each one's absolute path so the UI can show and copy it.
        if !groups.is_empty() {
            let root_ids = self.root_node_ids();
            for id in self.reader.subtree_range(scope) {
                let Some(rec) = self.reader.record(id) else {
                    continue;
                };
                if rec.kind.is_dir() || rec.size_alloc == 0 {
                    continue;
                }
                let key = (
                    self.reader.name(id),
                    rec.size_alloc,
                    if use_mtime { rec.mtime_ms } else { 0 },
                );
                if let Some(&gi) = index.get(&key) {
                    if groups[gi].items.len() < 50 {
                        if let Some(view) = self.node(id) {
                            let mut record = view.to_record();
                            record.path = self.absolute_path(id, &root_ids);
                            groups[gi].items.push(record);
                        }
                    }
                }
            }
        }

        groups.sort_by(|a, b| b.wasted.cmp(&a.wasted));
        groups.truncate(limit.max(1));
        groups
    }

    /// Node ids with no parent (snapshot roots), in id order.
    ///
    /// Roots are contiguous pre-order, so each next root starts right after the
    /// previous root's subtree; this walks roots, not every node.
    fn root_node_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        let len = self.reader.len();
        let mut id = 0u32;
        while id < len {
            ids.push(id);
            id = id.saturating_add(self.reader.subtree_size(id).max(1));
        }
        ids
    }

    /// Absolute filesystem path for a node, or `None` when it cannot be resolved.
    ///
    /// A root's node name is only its basename, so the path is rebuilt from the
    /// manifest's canonical root path plus the names of the nodes below it.
    fn absolute_path(&self, id: u32, root_ids: &[u32]) -> Option<String> {
        let mut names: Vec<&str> = Vec::new();
        let mut current = id;
        loop {
            let rec = self.reader.record(current)?;
            if rec.parent == crate::index::NO_PARENT {
                let root_index = root_ids.iter().position(|&root| root == current)?;
                let base = self.reader.manifest().roots.get(root_index)?;
                let mut path = base.clone();
                for name in names.iter().rev() {
                    if !path.ends_with('/') {
                        path.push('/');
                    }
                    path.push_str(name);
                }
                return Some(path);
            }
            names.push(self.reader.name(current));
            current = rec.parent;
        }
    }

    /// Age × size heatmap of file mass.
    pub fn heatmap(&self, scope: u32) -> Heatmap {
        const DAY: i64 = 86_400_000;
        let age_edges: [(i64, &str); 7] = [
            (DAY, "<1d"),
            (7 * DAY, "<7d"),
            (30 * DAY, "<30d"),
            (90 * DAY, "<90d"),
            (365 * DAY, "<1y"),
            (1095 * DAY, "<3y"),
            (i64::MAX, ">3y"),
        ];
        const KB: u64 = 1024;
        let size_edges: [(u64, &str); 8] = [
            (KB, "0-1 KB"),
            (10 * KB, "1-10 KB"),
            (100 * KB, "10-100 KB"),
            (KB * KB, "100 KB-1 MB"),
            (10 * KB * KB, "1-10 MB"),
            (100 * KB * KB, "10-100 MB"),
            (KB * KB * KB, "100 MB-1 GB"),
            (u64::MAX, ">1 GB"),
        ];

        let mut cells = vec![vec![(0u64, 0u64); size_edges.len()]; age_edges.len()];
        let now = crate::index::now_ms();
        for id in self.reader.subtree_range(scope) {
            let Some(rec) = self.reader.record(id) else {
                continue;
            };
            if rec.kind.is_dir() {
                continue;
            }
            let age = (now - rec.mtime_ms).max(0);
            let age_idx = age_edges
                .iter()
                .position(|(limit, _)| age < *limit)
                .unwrap_or(age_edges.len() - 1);
            let size_idx = size_edges
                .iter()
                .position(|(limit, _)| rec.size_alloc < *limit)
                .unwrap_or(size_edges.len() - 1);
            let cell = &mut cells[age_idx][size_idx];
            cell.0 += 1;
            cell.1 = cell.1.saturating_add(rec.size_alloc);
        }

        Heatmap {
            age_buckets: age_edges.iter().map(|(_, label)| label.to_string()).collect(),
            size_buckets: size_edges.iter().map(|(_, label)| label.to_string()).collect(),
            cells: cells
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|(count, bytes)| HeatmapCell { count, bytes })
                        .collect()
                })
                .collect(),
        }
    }
}

/// A group of metadata-identical files.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub key: String,
    pub count: u64,
    pub size: u64,
    pub wasted: u64,
    pub items: Vec<NodeRecord>,
}

/// One path in a snapshot diff.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    pub path: String,
    pub before: u64,
    pub after: u64,
    pub delta: i64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffTotals {
    pub before: u64,
    pub after: u64,
    pub delta: i64,
    pub added: u64,
    pub removed: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub a: String,
    pub b: String,
    pub grown: Vec<DiffEntry>,
    pub shrunk: Vec<DiffEntry>,
    pub added: Vec<DiffEntry>,
    pub removed: Vec<DiffEntry>,
    pub totals: DiffTotals,
}

const PATH_MAP_CAP: usize = 2_000_000;

/// Compare two snapshots by relative path (allocated bytes).
pub fn diff(a: &IndexReader, b: &IndexReader) -> DiffResult {
    let pa = path_sizes(a);
    let pb = path_sizes(b);

    let mut grown = Vec::new();
    let mut shrunk = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for (path, after) in &pb {
        match pa.get(path) {
            Some(before) if after > before => grown.push(DiffEntry {
                path: path.clone(),
                before: *before,
                after: *after,
                delta: (*after - *before) as i64,
                kind: "grown".into(),
            }),
            Some(before) if after < before => shrunk.push(DiffEntry {
                path: path.clone(),
                before: *before,
                after: *after,
                delta: (*before - *after) as i64,
                kind: "shrunk".into(),
            }),
            Some(_) => {}
            None => added.push(DiffEntry {
                path: path.clone(),
                before: 0,
                after: *after,
                delta: *after as i64,
                kind: "added".into(),
            }),
        }
    }
    for (path, before) in &pa {
        if !pb.contains_key(path) {
            removed.push(DiffEntry {
                path: path.clone(),
                before: *before,
                after: 0,
                delta: *before as i64,
                kind: "removed".into(),
            });
        }
    }

    grown.sort_by(|x, y| y.delta.cmp(&x.delta));
    shrunk.sort_by(|x, y| y.delta.cmp(&x.delta));
    added.sort_by(|x, y| y.after.cmp(&x.after));
    removed.sort_by(|x, y| y.before.cmp(&x.before));

    let before_total: u64 = pa.values().copied().sum();
    let after_total: u64 = pb.values().copied().sum();
    let added_total: u64 = added.iter().map(|e| e.after).sum();
    let removed_total: u64 = removed.iter().map(|e| e.before).sum();

    let cap = 500;
    grown.truncate(cap);
    shrunk.truncate(cap);
    added.truncate(cap);
    removed.truncate(cap);

    DiffResult {
        a: a.manifest().id.clone(),
        b: b.manifest().id.clone(),
        grown,
        shrunk,
        added,
        removed,
        totals: DiffTotals {
            before: before_total,
            after: after_total,
            delta: after_total as i64 - before_total as i64,
            added: added_total,
            removed: removed_total,
        },
    }
}

/// File path -> allocated bytes, built in a single pre-order pass.
///
/// Nodes are contiguous pre-order, so one ancestor stack reconstructs each
/// path in O(path length). Avoids `IndexReader::children`, which scans a whole
/// subtree per directory and made this O(n·depth) on large snapshots.
fn path_sizes(reader: &IndexReader) -> std::collections::HashMap<String, u64> {
    let mut map = std::collections::HashMap::new();
    // (exclusive subtree end, parent path length)
    let mut stack: Vec<(u32, usize)> = Vec::new();
    let mut path = String::new();
    for id in 0..reader.len() {
        if map.len() >= PATH_MAP_CAP {
            break;
        }
        while let Some(&(end, parent_len)) = stack.last() {
            if id >= end {
                stack.pop();
                path.truncate(parent_len);
            } else {
                break;
            }
        }
        let Some(rec) = reader.record(id) else {
            continue;
        };
        let name = reader.name(id);
        let parent_len = path.len();
        if rec.parent == crate::index::NO_PARENT {
            path.clear();
            path.push_str(name);
        } else {
            path.push('/');
            path.push_str(name);
        }
        if !rec.kind.is_dir() {
            map.insert(path.clone(), rec.size_alloc);
        }
        let size = reader.subtree_size(id);
        if size > 1 {
            stack.push((id + size, parent_len));
        } else {
            path.truncate(parent_len);
        }
    }
    map
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::File => "file",
        Kind::Directory => "directory",
        Kind::Symlink => "symlink",
        Kind::Special => "special",
    }
}

/// One histogram bucket (`key` is the stable identity, `label` is for display).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub key: String,
    pub label: String,
    pub count: u64,
    pub size: u64,
}

/// A bounded nested node for hierarchy layouts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyNode {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<HierarchyNode>,
}

/// A single age × size heatmap cell.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapCell {
    pub count: u64,
    pub bytes: u64,
}

/// An age × size heatmap matrix.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    pub age_buckets: Vec<String>,
    pub size_buckets: Vec<String>,
    /// `cells[age][size]`
    pub cells: Vec<Vec<HeatmapCell>>,
}

/// Resolve a slash-separated path (relative to the snapshot root) to a node id.
///
/// The root node's own name may be included or omitted (e.g. `/host/mnt/data`
/// or `/mnt/data` both resolve against a snapshot whose root is `host`).
pub fn resolve_path(reader: &IndexReader, path: &str) -> Option<u32> {
    let index = child_index(reader);
    resolve_with(&index, reader, path)
}

/// Correlate Docker mounts with snapshot nodes, filling `node` and `size`.
///
/// Builds a parent→children index once (so resolving N mounts is O(n + N·depth)
/// rather than O(N·n)) and computes each distinct node's subtree bytes once.
pub fn correlate_mounts(
    reader: &IndexReader,
    mounts: Vec<MountInfo>,
    host_root: &str,
) -> Vec<MountInfo> {
    let index = child_index(reader);
    let engine = QueryEngine::new(reader);
    let mut size_cache: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();

    mounts
        .into_iter()
        .map(|mut mount| {
            let container_path = if mount.source == "/" {
                host_root.to_string()
            } else {
                format!("{host_root}{}", mount.source)
            };
            if let Some(node) = resolve_with(&index, reader, &container_path) {
                mount.node = Some(node);
                let size = *size_cache
                    .entry(node)
                    .or_insert_with(|| engine.subtree_bytes(node, Metric::Alloc));
                mount.size = Some(size);
            }
            mount
        })
        .collect()
}

fn child_index(reader: &IndexReader) -> std::collections::HashMap<u32, Vec<u32>> {
    let mut index: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for id in 0..reader.len() {
        if let Some(rec) = reader.record(id) {
            if rec.parent != crate::index::NO_PARENT {
                index.entry(rec.parent).or_default().push(id);
            }
        }
    }
    index
}

fn resolve_with(
    index: &std::collections::HashMap<u32, Vec<u32>>,
    reader: &IndexReader,
    path: &str,
) -> Option<u32> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Some(0);
    }
    let mut current = 0u32;
    for component in trimmed.split('/').filter(|c| !c.is_empty()) {
        if current == 0 && reader.name(0) == component {
            continue;
        }
        let kids = index.get(&current)?;
        let next = kids.iter().copied().find(|child| reader.name(*child) == component)?;
        current = next;
    }
    Some(current)
}

fn dummy_record() -> crate::index::Record {
    crate::index::Record {
        parent: crate::index::NO_PARENT,
        name_off: 0,
        name_len: 0,
        kind: Kind::File,
        flags: 0,
        children: 0,
        ext_id: 0,
        subtree_size: 0,
        size_app: 0,
        size_alloc: 0,
        mtime_ms: 0,
        uid: 0,
        gid: 0,
        mode: 0,
        label_id: 0,
    }
}

/// Squarified treemap layout over the unit square.
///
/// Returns one `[x, y, w, h]` rectangle per input value, in the input order.
/// Values are laid out largest-first internally; the output preserves input
/// indexing so callers can zip it back to their items.
pub fn squarify(values: &[f64]) -> Vec<[f64; 4]> {
    let mut rects = vec![[0.0f64; 4]; values.len()];
    if values.is_empty() {
        return rects;
    }

    let total: f64 = values.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 {
        return rects;
    }

    // Order by value descending, remembering original indices.
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[b].partial_cmp(&values[a]).unwrap_or(std::cmp::Ordering::Equal));

    let scale = 1.0 / total; // unit square area == 1
    let areas: Vec<f64> = order.iter().map(|&i| values[i].max(0.0) * scale).collect();

    let (mut x, mut y, mut w, mut h) = (0.0f64, 0.0f64, 1.0f64, 1.0f64);
    let mut idx = 0usize;

    while idx < areas.len() {
        let side = w.min(h);
        let mut row: Vec<f64> = Vec::new();
        let mut best = f64::INFINITY;
        while idx < areas.len() {
            let mut candidate = row.clone();
            candidate.push(areas[idx]);
            let ratio = worst_ratio(&candidate, side);
            if ratio <= best {
                best = ratio;
                row.push(areas[idx]);
                idx += 1;
            } else {
                break;
            }
        }

        let row_sum: f64 = row.iter().sum();
        if w >= h {
            let strip_w = if h > 0.0 { row_sum / h } else { 0.0 };
            let mut cursor = y;
            for (k, area) in row.iter().enumerate() {
                let rh = if strip_w > 0.0 { area / strip_w } else { 0.0 };
                rects[order[idx - row.len() + k]] = [x, cursor, strip_w, rh];
                cursor += rh;
            }
            x += strip_w;
            w = (w - strip_w).max(0.0);
        } else {
            let strip_h = if w > 0.0 { row_sum / w } else { 0.0 };
            let mut cursor = x;
            for (k, area) in row.iter().enumerate() {
                let rw = if strip_h > 0.0 { area / strip_h } else { 0.0 };
                rects[order[idx - row.len() + k]] = [cursor, y, rw, strip_h];
                cursor += rw;
            }
            y += strip_h;
            h = (h - strip_h).max(0.0);
        }
    }

    rects
}

fn worst_ratio(row: &[f64], side: f64) -> f64 {
    let sum: f64 = row.iter().sum();
    if sum <= 0.0 || side <= 0.0 {
        return f64::INFINITY;
    }
    let max = row.iter().cloned().fold(f64::MIN, f64::max);
    let min = row.iter().cloned().fold(f64::MAX, f64::min);
    let s2 = sum * sum;
    let side2 = side * side;
    (side2 * max / s2).max(s2 / (side2 * min))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squarify_conserves_area() {
        let values = vec![6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let rects = squarify(&values);
        let area: f64 = rects.iter().map(|r| r[2] * r[3]).sum();
        assert!((area - 1.0).abs() < 1e-6, "area was {area}");
    }

    #[test]
    fn squarify_has_no_overlap() {
        let values = vec![5.0, 4.0, 3.0, 2.0, 1.0, 1.0, 1.0, 1.0];
        let rects = squarify(&values);
        for (i, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(i + 1) {
                let overlap_x = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
                let overlap_y = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
                assert!(overlap_x <= 1e-9 || overlap_y <= 1e-9, "overlap {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn squarify_handles_zero_total() {
        let rects = squarify(&[0.0, 0.0]);
        assert_eq!(rects.len(), 2);
    }

    /// root/
    ///   big/        (3 MB total)
    ///     a.bin     1 MB
    ///     b.bin     2 MB
    ///   small.txt   1 KB
    fn build_reader(dir: &std::path::Path) -> IndexReader {
        use crate::index::{IndexWriter, Kind as IKind, ManifestSeed, NodeInput, NO_PARENT};

        fn push(
            w: &mut IndexWriter,
            parent: u32,
            name: &str,
            kind: IKind,
            children: u32,
            size: u64,
        ) -> u32 {
            w.push(NodeInput {
                parent,
                name,
                kind,
                flags: 0,
                children,
                size_app: size,
                size_alloc: size,
                mtime_ms: 0,
                uid: 0,
                gid: 0,
                mode: 0,
                ino: 0,
                dev: 0,
            })
            .unwrap()
        }

        let mut w = IndexWriter::create(dir).unwrap();
        let root = push(&mut w, NO_PARENT, "root", IKind::Directory, 2, 0);
        let big = push(&mut w, root, "big", IKind::Directory, 2, 0);
        push(&mut w, big, "a.bin", IKind::File, 0, 1_000_000);
        push(&mut w, big, "b.bin", IKind::File, 0, 2_000_000);
        push(&mut w, root, "small.txt", IKind::File, 0, 1_000);
        w.finish(ManifestSeed {
            id: "t".into(),
            parent_id: None,
            roots: vec!["root".into()],
            started_at_ms: 0,
        })
        .unwrap();
        IndexReader::open(dir).unwrap()
    }

    #[test]
    fn tiles_size_directories_by_subtree() {
        let tmp = tempfile::tempdir().unwrap();
        let reader = build_reader(&tmp.path().join("snap"));
        let q = QueryEngine::new(&reader);

        // depth 1 == the root's direct children, sized by their subtree.
        let response = q.tiles(0, 1, Metric::Alloc, "ext");
        let big = response.tiles.iter().find(|t| t.name == "big").expect("big tile");
        assert_eq!(big.size, 3_000_000);
        let small = response
            .tiles
            .iter()
            .find(|t| t.name == "small.txt")
            .expect("small tile");
        assert_eq!(small.size, 1_000);
    }

    #[test]
    fn children_rank_directories_by_subtree() {
        let tmp = tempfile::tempdir().unwrap();
        let reader = build_reader(&tmp.path().join("snap"));
        let q = QueryEngine::new(&reader);

        let (total, items) = q.children(0, SortBy::Size, 10, 0);
        assert_eq!(total, 2);
        assert_eq!(items[0].name, "big");
        assert_eq!(items[0].size_alloc, 3_000_000);
        assert_eq!(items[1].name, "small.txt");
    }

    #[test]
    fn path_sizes_maps_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let reader = build_reader(&tmp.path().join("snap"));
        let sizes = path_sizes(&reader);
        assert_eq!(sizes.len(), 3);
        assert_eq!(sizes.get("root/big/a.bin"), Some(&1_000_000));
        assert_eq!(sizes.get("root/big/b.bin"), Some(&2_000_000));
        assert_eq!(sizes.get("root/small.txt"), Some(&1_000));
    }

    #[test]
    fn duplicates_items_carry_absolute_paths() {
        use crate::index::{IndexWriter, Kind as IKind, ManifestSeed, NodeInput, NO_PARENT};

        fn push(
            w: &mut IndexWriter,
            parent: u32,
            name: &str,
            kind: IKind,
            children: u32,
            size: u64,
        ) -> u32 {
            w.push(NodeInput {
                parent,
                name,
                kind,
                flags: 0,
                children,
                size_app: size,
                size_alloc: size,
                mtime_ms: 0,
                uid: 0,
                gid: 0,
                mode: 0,
                ino: 0,
                dev: 0,
            })
            .unwrap()
        }

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snap");
        let mut w = IndexWriter::create(&dir).unwrap();
        let root = push(&mut w, NO_PARENT, "root", IKind::Directory, 2, 0);
        let a = push(&mut w, root, "a", IKind::Directory, 1, 0);
        let b = push(&mut w, root, "b", IKind::Directory, 1, 0);
        push(&mut w, a, "dup.bin", IKind::File, 0, 1_000);
        push(&mut w, b, "dup.bin", IKind::File, 0, 1_000);
        w.finish(ManifestSeed {
            id: "t".into(),
            parent_id: None,
            roots: vec!["root".into()],
            started_at_ms: 0,
        })
        .unwrap();
        let reader = IndexReader::open(&dir).unwrap();

        let q = QueryEngine::new(&reader);
        let groups = q.duplicates(0, "name+size", 10);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 2);
        let mut paths: Vec<String> = groups[0]
            .items
            .iter()
            .filter_map(|item| item.path.clone())
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["root/a/dup.bin", "root/b/dup.bin"]);
    }

    #[test]
    fn diff_of_identical_snapshots_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let a = build_reader(&tmp.path().join("a"));
        let b = build_reader(&tmp.path().join("b"));
        let result = diff(&a, &b);
        assert!(result.grown.is_empty());
        assert!(result.shrunk.is_empty());
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
        assert_eq!(result.totals.delta, 0);
    }
}
