//! `scanscan` — CLI and daemon entry point.
//!
//! Every subcommand supports `--json` for scripting and agent use.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::json;

use scanscan_core::daemon::{serve, Daemon};
use scanscan_core::docker::DockerCollector;
use scanscan_core::index::{IndexReader, Kind, NO_PARENT};
use scanscan_core::query::{FindFilters, Metric, NodeView, QueryEngine, SortBy};
use scanscan_core::store::Store;
use scanscan_core::{Config as CoreConfig, CORE_VERSION};
use scanscan_ipc::{ScanOptions, ScanState, PROTOCOL_VERSION};

pub mod tui;

#[derive(Debug, Parser)]
#[command(
    name = "scanscan",
    version,
    about = "Fast, headless, web-accessible disk-space analyzer and indexer",
    propagate_version = true
)]
struct Cli {
    /// Emit machine-readable JSON on stdout.
    #[arg(long, global = true)]
    json: bool,

    /// Data directory holding snapshots (default: $SCANSCAN_DATA_DIR or ./data).
    #[arg(long = "data-dir", global = true)]
    data_dir: Option<String>,

    /// Operate on a specific snapshot instead of the latest completed one.
    #[arg(long, global = true)]
    snapshot: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan one or more paths and index them.
    Scan {
        #[arg(required = true)]
        paths: Vec<String>,
        #[arg(long)]
        incremental: bool,
        #[arg(long = "one-file-system", default_value_t = true, action = clap::ArgAction::Set)]
        one_file_system: bool,
        #[arg(long = "follow-symlinks")]
        follow_symlinks: bool,
        #[arg(long = "exclude")]
        exclude: Vec<String>,
        #[arg(long = "ignore-file")]
        ignore_file: Vec<String>,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long)]
        apparent: bool,
    },
    /// Run the Unix-domain-socket JSON-RPC daemon.
    Daemon {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long = "data-dir")]
        daemon_data_dir: Option<String>,
    },
    /// List the children of a path.
    Ls {
        path: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show subtree sizes.
    Du {
        path: Option<String>,
        #[arg(long)]
        depth: Option<u32>,
        #[arg(long)]
        apparent: bool,
    },
    /// Largest files or directories.
    Top {
        path: Option<String>,
        #[arg(short = 'n', long, default_value_t = 50)]
        n: usize,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        metric: Option<String>,
    },
    /// Find files by filter.
    Find {
        path: Option<String>,
        #[arg(long)]
        size: Option<String>,
        #[arg(long)]
        mtime: Option<String>,
        #[arg(long)]
        ext: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        regex: Option<String>,
        #[arg(long)]
        dupe: bool,
    },
    /// Print the directory hierarchy.
    Tree {
        path: Option<String>,
        #[arg(long)]
        depth: Option<u32>,
    },
    /// Interactively browse the children of a path in the terminal.
    Tui {
        path: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        metric: Option<String>,
        #[arg(long)]
        sort: Option<String>,
    },
    /// Extension breakdown.
    Ext { path: Option<String> },
    /// Compare two snapshots.
    Diff { a: String, b: String },
    /// Docker containers, sizes and mounts.
    Docker {
        #[arg(long = "stats")]
        stats: bool,
    },
    /// Export a snapshot.
    Export {
        snapshot: String,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// List, delete and garbage-collect snapshots.
    Snapshots {
        #[command(subcommand)]
        action: Option<SnapshotsAction>,
    },
    /// Get or set configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Generate shell completions.
    Completions { shell: clap_complete::Shell },
}

#[derive(Debug, Subcommand)]
enum SnapshotsAction {
    List,
    Delete { id: String },
    Gc {
        #[arg(long)]
        keep: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    Get,
    Set { key: String, value: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;

    let result: anyhow::Result<()> = match &cli.command {
        Command::Config {
            action: ConfigAction::Get,
        } => cmd_config_get(json, cli.data_dir.as_deref()),
        Command::Config {
            action: ConfigAction::Set { .. },
        } => cmd_not_implemented(json, "config set"),
        Command::Completions { shell } => cmd_completions(*shell),
        Command::Scan {
            paths,
            incremental,
            one_file_system,
            follow_symlinks,
            exclude,
            ignore_file,
            threads,
            apparent,
        } => {
            let options = ScanOptions {
                roots: paths.clone(),
                incremental: *incremental,
                one_file_system: *one_file_system,
                follow_symlinks: *follow_symlinks,
                exclusions: exclude.clone(),
                ignore_files: ignore_file.clone(),
                threads: *threads,
                allocated: !*apparent,
            };
            cmd_scan(json, cli.data_dir.as_deref(), options)
        }
        Command::Daemon {
            socket,
            daemon_data_dir,
        } => cmd_daemon(json, socket.clone(), daemon_data_dir.clone().or(cli.data_dir.clone())),
        Command::Ls { path, sort, limit } => cmd_ls(
            json,
            &cli,
            path.as_deref(),
            sort.as_deref().unwrap_or("size"),
            limit.unwrap_or(500),
        ),
        Command::Du {
            path,
            depth,
            apparent,
        } => cmd_du(json, &cli, path.as_deref(), depth.unwrap_or(1), *apparent),
        Command::Top {
            path,
            n,
            kind,
            metric,
        } => cmd_top(json, &cli, path.as_deref(), *n, kind.as_deref(), metric.as_deref()),
        Command::Find {
            path,
            size,
            mtime,
            ext,
            owner,
            regex,
            dupe,
        } => cmd_find(
            json,
            &cli,
            path.as_deref(),
            size.as_deref(),
            mtime.as_deref(),
            ext.as_deref(),
            owner.as_deref(),
            regex.as_deref(),
            *dupe,
        ),
        Command::Ext { path } => cmd_ext(json, &cli, path.as_deref()),
        Command::Tree { path, depth } => cmd_tree(json, &cli, path.as_deref(), depth.unwrap_or(2)),
        Command::Tui {
            path,
            limit,
            metric,
            sort,
        } => cmd_tui(
            json,
            &cli,
            path.as_deref(),
            limit.unwrap_or(40),
            metric.as_deref().unwrap_or("alloc"),
            sort.as_deref().unwrap_or("size"),
        ),
        Command::Docker { stats } => cmd_docker(json, *stats),
        Command::Diff { a, b } => cmd_diff(json, &cli, a, b),
        Command::Export { snapshot, format } => cmd_export(json, &cli, snapshot, format),
        Command::Snapshots { action } => match action {
            None | Some(SnapshotsAction::List) => cmd_snapshots_list(json, &cli),
            Some(SnapshotsAction::Delete { id }) => cmd_snapshots_delete(json, &cli, id),
            Some(SnapshotsAction::Gc { keep }) => cmd_snapshots_gc(json, &cli, *keep),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if json {
                println!("{}", json!({ "ok": false, "error": err.to_string() }));
            } else {
                eprintln!("error: {err}");
            }
            ExitCode::FAILURE
        }
    }
}

fn data_dir(cli: &Cli, override_dir: Option<&str>) -> String {
    override_dir
        .map(str::to_string)
        .or_else(|| cli.data_dir.clone())
        .or_else(|| std::env::var("SCANSCAN_DATA_DIR").ok())
        .unwrap_or_else(|| "./data".to_string())
}

fn open_store(cli: &Cli) -> anyhow::Result<Store> {
    Ok(Store::new(data_dir(cli, None))?)
}

/// Resolve the snapshot id to query: `--snapshot`, else the latest completed.
fn resolve_snapshot(cli: &Cli, store: &Store) -> anyhow::Result<String> {
    if let Some(id) = &cli.snapshot {
        return Ok(id.clone());
    }
    store
        .list()
        .into_iter()
        .find(|s| s.state == ScanState::Completed)
        .map(|s| s.id)
        .ok_or_else(|| anyhow::anyhow!("no completed snapshot; run `scanscan scan <path>` first"))
}

fn resolve_scope(reader: &IndexReader, path: Option<&str>) -> anyhow::Result<u32> {
    let Some(path) = path else {
        return Ok(0);
    };
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(0);
    }
    let q = QueryEngine::new(reader);
    let mut current = 0u32;
    for component in trimmed.split('/').filter(|c| !c.is_empty()) {
        if current == 0 && q.node(0).map(|n| n.name == component).unwrap_or(false) {
            continue;
        }
        let child = reader
            .children(current)
            .into_iter()
            .find(|c| reader.name(*c) == component)
            .ok_or_else(|| anyhow::anyhow!("path not found in snapshot: {path}"))?;
        current = child;
    }
    Ok(current)
}

fn cmd_scan(json: bool, dir: Option<&str>, options: ScanOptions) -> anyhow::Result<()> {
    let store = Store::new(data_dir_from(dir))?;
    let summary = store.start(options)?;
    if !json {
        eprintln!("scan {} started", summary.id);
    }
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let Some(progress) = store.progress(&summary.id) else {
            break;
        };
        if matches!(
            progress.state,
            ScanState::Completed | ScanState::Failed | ScanState::Cancelled
        ) {
            break;
        }
        if !json {
            eprint!(
                "\r{} files · {} dirs · {:.1} MB · {}",
                progress.files,
                progress.dirs,
                progress.bytes_alloc as f64 / 1_048_576.0,
                progress.current_path.as_deref().unwrap_or("")
            );
        }
    }
    let final_summary = store
        .get(&summary.id)
        .ok_or_else(|| anyhow::anyhow!("scan disappeared"))?;
    if json {
        println!("{}", serde_json::to_string(&final_summary)?);
    } else {
        eprintln!();
        println!(
            "scan {}: {:?} — {} files, {} dirs, {:.2} GB allocated, {} errors",
            final_summary.id,
            final_summary.state,
            final_summary.files,
            final_summary.dirs,
            final_summary.bytes_alloc as f64 / 1_073_741_824.0,
            final_summary.errors,
        );
    }
    if final_summary.state == ScanState::Failed {
        anyhow::bail!("scan failed");
    }
    Ok(())
}

fn data_dir_from(dir: Option<&str>) -> String {
    dir.map(str::to_string)
        .or_else(|| std::env::var("SCANSCAN_DATA_DIR").ok())
        .unwrap_or_else(|| "./data".to_string())
}

#[cfg(unix)]
fn cmd_daemon(json: bool, socket: Option<String>, dir: Option<String>) -> anyhow::Result<()> {
    let mut config = CoreConfig::from_env()?;
    if let Some(socket) = socket {
        config.socket = PathBuf::from(socket);
    }
    if let Some(dir) = dir {
        config.data_dir = PathBuf::from(dir);
    }
    let store = Arc::new(Store::new(config.data_dir.clone())?);
    let daemon = Arc::new(Daemon::new(store, config.clone()));
    if json {
        println!(
            "{}",
            json!({
                "status": "listening",
                "socket": config.socket.to_string_lossy(),
                "core_version": CORE_VERSION,
                "protocol": PROTOCOL_VERSION,
            })
        );
    } else {
        eprintln!("scanscan daemon listening on {}", config.socket.display());
    }
    serve(daemon, &config.socket)?;
    Ok(())
}

#[cfg(not(unix))]
fn cmd_daemon(_json: bool, _socket: Option<String>, _dir: Option<String>) -> anyhow::Result<()> {
    anyhow::bail!("the daemon requires a unix domain socket")
}

fn with_query<F>(json: bool, cli: &Cli, path: Option<&str>, f: F) -> anyhow::Result<()>
where
    F: FnOnce(&QueryEngine<'_>, u32) -> anyhow::Result<serde_json::Value>,
{
    let store = open_store(cli)?;
    let id = resolve_snapshot(cli, &store)?;
    let reader = store.open(&id)?;
    let scope = resolve_scope(&reader, path)?;
    let engine = QueryEngine::new(&reader);
    let value = f(&engine, scope)?;
    if json {
        println!("{}", serde_json::to_string(&value)?);
    } else {
        print_human(&value);
    }
    Ok(())
}

fn cmd_ls(json: bool, cli: &Cli, path: Option<&str>, sort: &str, limit: usize) -> anyhow::Result<()> {
    with_query(json, cli, path, |engine, scope| {
        let (total, items) = engine.children(scope, SortBy::parse(sort), limit, 0);
        let records: Vec<_> = items.iter().map(NodeView::to_record).collect();
        Ok(json!({ "parent": scope, "total": total, "offset": 0, "items": records }))
    })
}

fn cmd_top(
    json: bool,
    cli: &Cli,
    path: Option<&str>,
    n: usize,
    kind: Option<&str>,
    metric: Option<&str>,
) -> anyhow::Result<()> {
    with_query(json, cli, path, |engine, scope| {
        let kind = kind.and_then(parse_kind);
        let items = engine.top_n(scope, kind, Metric::parse(metric.unwrap_or("alloc")), n);
        let records: Vec<_> = items.iter().map(NodeView::to_record).collect();
        Ok(json!({ "items": records }))
    })
}

fn cmd_du(json: bool, cli: &Cli, path: Option<&str>, depth: u32, apparent: bool) -> anyhow::Result<()> {
    let metric = if apparent { Metric::Apparent } else { Metric::Alloc };
    with_query(json, cli, path, |engine, scope| {
        let node = engine
            .node(scope)
            .ok_or_else(|| anyhow::anyhow!("scope not found"))?;
        let mut rows: Vec<serde_json::Value> = Vec::new();
        collect_du(engine, scope, 0, depth, metric, &mut rows);
        let name = node.name.clone();
        let size = if apparent {
            node.size_apparent
        } else {
            node.size_alloc
        };
        Ok(json!({
            "scope": scope,
            "name": name,
            "size": size,
            "subtree_size": node.subtree_size,
            "rows": rows,
        }))
    })
}

fn collect_du(
    engine: &QueryEngine<'_>,
    scope: u32,
    level: u32,
    max_depth: u32,
    metric: Metric,
    out: &mut Vec<serde_json::Value>,
) {
    if level >= max_depth {
        return;
    }
    let (_, children) = engine.children(scope, SortBy::Size, 10_000, 0);
    for child in children {
        let size = match metric {
            Metric::Apparent => child.size_apparent,
            Metric::Items => u64::from(child.subtree_size),
            Metric::Alloc => child.size_alloc,
        };
        out.push(json!({
            "id": child.id,
            "name": child.name,
            "kind": child.kind.as_str(),
            "size": size,
            "level": level,
        }));
        if child.kind == Kind::Directory {
            collect_du(engine, child.id, level + 1, max_depth, metric, out);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_find(
    json: bool,
    cli: &Cli,
    path: Option<&str>,
    size: Option<&str>,
    mtime: Option<&str>,
    ext: Option<&str>,
    owner: Option<&str>,
    regex: Option<&str>,
    dupe: bool,
) -> anyhow::Result<()> {
    let (size_min, size_max) = parse_size_filter(size)?;
    let (mtime_after, mtime_before) = parse_mtime_filter(mtime)?;
    with_query(json, cli, path, |engine, scope| {
        if dupe {
            let groups = engine.duplicates(scope, "name+size", 200);
            return Ok(json!({ "groups": groups }));
        }
        let filters = FindFilters {
            size_min,
            size_max,
            mtime_before,
            mtime_after,
            ext: ext.map(str::to_string),
            name_contains: None,
            kind: None,
            limit: 1000,
        };
        let items = engine.find(scope, &filters);
        let records: Vec<_> = items.iter().map(NodeView::to_record).collect();
        Ok(json!({ "items": records, "owner": owner, "regex": regex }))
    })
}

fn cmd_ext(json: bool, cli: &Cli, path: Option<&str>) -> anyhow::Result<()> {
    with_query(json, cli, path, |engine, scope| {
        let buckets = engine.histogram_ext(scope);
        let items: Vec<_> = buckets
            .into_iter()
            .map(|(ext, count, size)| json!({ "ext": ext, "count": count, "size": size }))
            .collect();
        Ok(json!({ "items": items }))
    })
}

fn cmd_tree(json: bool, cli: &Cli, path: Option<&str>, depth: u32) -> anyhow::Result<()> {
    with_query(json, cli, path, |engine, scope| {
        let mut rows = Vec::new();
        collect_tree(engine, scope, 0, depth, &mut rows);
        Ok(json!({ "rows": rows }))
    })
}

fn collect_tree(
    engine: &QueryEngine<'_>,
    scope: u32,
    level: u32,
    max_depth: u32,
    out: &mut Vec<serde_json::Value>,
) {
    if level > max_depth {
        return;
    }
    let (_, children) = engine.children(scope, SortBy::Size, 10_000, 0);
    for child in children {
        out.push(json!({
            "id": child.id,
            "name": child.name,
            "kind": child.kind.as_str(),
            "size": child.size_alloc,
            "level": level,
        }));
        if child.kind == Kind::Directory {
            collect_tree(engine, child.id, level + 1, max_depth, out);
        }
    }
}

fn cmd_docker(json: bool, stats: bool) -> anyhow::Result<()> {
    let collector = DockerCollector::from_env();
    let status = collector.status();
    if !status.available {
        let message = status.error.unwrap_or_else(|| "unavailable".into());
        if json {
            println!("{}", json!({ "available": false, "error": message }));
        } else {
            eprintln!("docker unavailable: {message}");
        }
        return Ok(());
    }
    let containers = collector.containers()?;
    if json {
        println!(
            "{}",
            json!({ "available": true, "stats": stats, "containers": containers })
        );
    } else {
        for c in &containers {
            println!(
                "{}  {}  rw={}  mounts={}",
                c.name,
                c.image,
                c.size_rw.unwrap_or(0),
                c.mounts.len()
            );
        }
    }
    Ok(())
}

fn cmd_diff(json: bool, cli: &Cli, a: &str, b: &str) -> anyhow::Result<()> {
    let store = open_store(cli)?;
    let reader_a = store.open(a)?;
    let reader_b = store.open(b)?;
    let result = scanscan_core::query::diff(&reader_a, &reader_b);
    if json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!(
            "before {:.2} GB -> after {:.2} GB (delta {:+.2} GB)",
            result.totals.before as f64 / 1_073_741_824.0,
            result.totals.after as f64 / 1_073_741_824.0,
            result.totals.delta as f64 / 1_073_741_824.0,
        );
        for entry in result.grown.iter().take(20) {
            println!("  +{:>14}  {}", entry.delta, entry.path);
        }
        for entry in result.removed.iter().take(20) {
            println!("  -{:>14}  {}", entry.delta, entry.path);
        }
    }
    Ok(())
}

fn cmd_export(json: bool, cli: &Cli, snapshot: &str, format: &str) -> anyhow::Result<()> {
    let store = open_store(cli)?;
    let reader = store.open(snapshot)?;
    let engine = QueryEngine::new(&reader);
    match format {
        // CSV is written row by row: only the output buffer is held, not a copy
        // of every node as well.
        "csv" => println!("{}", export_csv(&engine, &reader)),
        "json" => {
            let mut rows: Vec<_> = Vec::new();
            for id in 0..reader.len() {
                if let Some(node) = engine.node(id) {
                    rows.push(node.to_record());
                }
            }
            let value = json!({ "snapshot": snapshot, "nodes": rows });
            if json {
                println!("{}", serde_json::to_string(&value)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
        other => anyhow::bail!(
            "export format '{other}' is not supported (use json or csv; parquet needs a parquet crate that resolves at build time)"
        ),
    }
    Ok(())
}

fn export_csv(engine: &QueryEngine<'_>, reader: &IndexReader) -> String {
    let mut out = String::new();
    out.push_str("id,parent,name,kind,sizeAlloc,sizeApparent,subtreeSize,mtimeMs\n");
    for id in 0..reader.len() {
        if let Some(node) = engine.node(id) {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                node.id,
                node.parent.map(|p| format!("{p}")).unwrap_or_default(),
                csv_escape(node.name.as_str()),
                node.kind.as_str(),
                node.size_alloc,
                node.size_apparent,
                node.subtree_size,
                node.mtime_ms,
            ));
        }
    }
    out
}

fn csv_escape(value: &str) -> String {
    let mut out = String::new();
    out.push_str("\"");
    let mut first = true;
    for c in value.chars() {
        // A cell that starts with one of these is evaluated as a formula by
        // spredsheet apps, so neutralize it before it leaves the process.
        if first && (c == '=' || c == '+' || c == '-' || c == '@' || c == '\t' || c == '\r') {
            out.push('\'');
        }
        first = false;
        if c == '"' {
            out.push_str("\"\"");
        } else {
            out.push(c);
        }
    }
    out.push_str("\"");
    out
}

fn cmd_tui(
    json: bool,
    cli: &Cli,
    path: Option<&str>,
    limit: usize,
    metric: &str,
    sort: &str,
) -> anyhow::Result<()> {
    use std::io::{BufRead, BufReader};

    let store = open_store(cli)?;
    let id = resolve_snapshot(cli, &store)?;
    let reader = store.open(&id)?;
    let engine = QueryEngine::new(&reader);
    let mut scope = resolve_scope(&reader, path)?;
    let mut metric = Metric::parse(metric);
    let mut sort = SortBy::parse(sort);
    let mut limit = limit;
    let mut offset = 0usize;
    let mut selected = 0usize;
    // The whole child list is fetched once per scope and reordered in memory, so
    // a keystroke never re-walks the subtrees just to rank one page.
    let mut children = ordered_children(&engine, scope, sort);
    let mut cached_scope = scope;
    let mut cached_sort = sort;
    let mut rows = page_of(&children, limit, offset);

    if json {
        let records: Vec<_> = rows.iter().map(NodeView::to_record).collect();
        let value = json!({ "scope": scope, "items": records });
        println!("{}", serde_json::to_string(&value)?);
        return Ok(());
    }

    render_tui(&reader, id.as_str(), scope, &children, &rows, metric, selected);

    let mut input = BufReader::new(std::io::stdin());
    for line in input.lines() {
        let Ok(line) = line else {
            break;
        };
        match tui::parse_nav(line) {
            tui::Nav::Quit => break,
            tui::Nav::Parent => {
                scope = parent_of(&reader, scope);
                offset = 0;
                selected = 0;
            }
            tui::Nav::Root => {
                scope = 0;
                offset = 0;
                selected = 0;
            }
            tui::Nav::Open => {
                if let Some(child) = child_scope(&rows, selected) {
                    scope = child;
                    offset = 0;
                    selected = 0;
                }
            }
            tui::Nav::Select(n) => {
                if n > 0 && n <= rows.len() {
                    let index = n - 1;
                    if let Some(child) = child_scope(&rows, index) {
                        scope = child;
                        offset = 0;
                        selected = 0;
                    } else {
                        selected = index;
                    }
                }
            }
            tui::Nav::PageDown => {
                offset += limit;
            }
            tui::Nav::PageUp => {
                if offset > limit {
                    offset -= limit;
                } else {
                    offset = 0;
                }
            }
            tui::Nav::Sort => {
                sort = next_sort(sort);
                offset = 0;
            }
            tui::Nav::Metric => {
                metric = next_metric(metric);
            }
            tui::Nav::Widen => {
                limit += 20;
            }
            tui::Nav::Narrow => {
                if limit > 20 {
                    limit -= 20;
                }
            }
            tui::Nav::Unnown => {}
        }
        if scope != cached_scope || sort != cached_sort {
            children = ordered_children(&engine, scope, sort);
            cached_scope = scope;
            cached_sort = sort;
        }
        rows = page_of(&children, limit, offset);
        render_tui(&reader, id.as_str(), scope, &children, &rows, metric, selected);
    }
    Ok(())
}

/// Every direct child of `scope`, ranked by `sort`.
fn ordered_children(engine: &QueryEngine<'_>, scope: u32, sort: SortBy) -> Vec<NodeView> {
    // Size order first: it is the only order that fills a directory's subtree
    // bytes, which the bars and the metrics need. The list is fetched whole so a
    // page can be sliced without walking the subtrees again.
    let (_, mut rows) = engine.children(scope, SortBy::Size, 1000000000usize, 0);
    match sort {
        SortBy::Name => {
            rows.sort_by(|a, b| a.name.cmp(&b.name));
        }
        SortBy::Mtime => {
            rows.sort_by(|a, b| b.mtime_ms.cmp(&a.mtime_ms));
        }
        SortBy::Size => {}
    }
    rows
}

/// One page of an already ordered child list.
fn page_of(all: &Vec<NodeView>, limit: usize, offset: usize) -> Vec<NodeView> {
    let mut page: Vec<NodeView> = Vec::new();
    let mut index = offset;
    while index < all.len() && page.len() < limit {
        if let Some(row) = all.get(index) {
            page.push(row.clone());
        }
        index += 1;
    }
    page
}

#[allow(clippy::too_many_arguments)]
fn render_tui(
    reader: &IndexReader,
    snapshot: &str,
    scope: u32,
    all: &Vec<NodeView>,
    rows: &Vec<NodeView>,
    metric: Metric,
    selected: usize,
) {
    let mut max = 0u64;
    let mut display: Vec<tui::Row> = Vec::new();
    for row in rows.iter() {
        let value = value_of(row, metric);
        if value > max {
            max = value;
        }
        display.push(tui::Row {
            label: row.name.clone(),
            value,
            detail: row.kind.as_str().to_string(),
        });
    }

    let crumb = path_of(reader, scope);
    let header = format!("scanscan tui · {snapshot}");
    let footer = format!(
        "{} children · {} shown · j/k page · u up · g root · <n> open · s sort · m metric · q quit",
        all.len(),
        rows.len(),
    );
    println!(
        "{}",
        tui::frame(
            header.as_str(),
            crumb.as_str(),
            &display,
            max,
            24,
            selected,
            footer.as_str(),
        )
    );
}

fn value_of(node: &NodeView, metric: Metric) -> u64 {
    match metric {
        Metric::Apparent => node.size_apparent,
        Metric::Items => u64::from(node.subtree_size),
        Metric::Alloc => node.size_alloc,
    }
}

fn child_scope(rows: &Vec<NodeView>, index: usize) -> Option<u32> {
    rows.get(index).and_then(|row| {
        if row.kind == Kind::Directory {
            Some(row.id)
        } else {
            None
        }
    })
}

fn parent_of(reader: &IndexReader, scope: u32) -> u32 {
    let parent = reader
        .record(scope)
        .map(|rec| rec.parent)
        .unwrap_or(NO_PARENT);
    if parent == NO_PARENT {
        0
    } else {
        parent
    }
}

fn path_of(reader: &IndexReader, scope: u32) -> String {
    let mut crumb = String::new();
    let mut current = scope;
    loop {
        let Some(rec) = reader.record(current) else {
            break;
        };
        if rec.parent == NO_PARENT {
            break;
        }
        let mut next = String::new();
        next.push('/');
        next.push_str(reader.name(current));
        next.push_str(crumb.as_str());
        crumb = next;
        current = rec.parent;
    }
    if crumb.is_empty() {
        "/".to_string()
    } else {
        crumb
    }
}

fn next_sort(sort: SortBy) -> SortBy {
    match sort {
        SortBy::Size => SortBy::Name,
        SortBy::Name => SortBy::Mtime,
        SortBy::Mtime => SortBy::Size,
    }
}

fn next_metric(metric: Metric) -> Metric {
    match metric {
        Metric::Alloc => Metric::Apparent,
        Metric::Apparent => Metric::Items,
        Metric::Items => Metric::Alloc,
    }
}

fn cmd_snapshots_list(json: bool, cli: &Cli) -> anyhow::Result<()> {
    let store = open_store(cli)?;
    let scans = store.list();
    if json {
        println!("{}", json!({ "scans": scans }));
    } else if scans.is_empty() {
        println!("no snapshots");
    } else {
        for s in scans {
            println!(
                "{}  {:?}  {} files  {:.2} GB",
                s.id,
                s.state,
                s.files,
                s.bytes_alloc as f64 / 1_073_741_824.0
            );
        }
    }
    Ok(())
}

fn cmd_snapshots_delete(json: bool, cli: &Cli, id: &str) -> anyhow::Result<()> {
    let store = open_store(cli)?;
    store.delete(id)?;
    if json {
        println!("{}", json!({ "deleted": id }));
    } else {
        println!("deleted snapshot {id}");
    }
    Ok(())
}

fn cmd_snapshots_gc(json: bool, cli: &Cli, keep: Option<usize>) -> anyhow::Result<()> {
    let store = open_store(cli)?;
    let keep = keep.unwrap_or(3);
    let mut scans = store.list();
    scans.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
    let mut removed = Vec::new();
    for scan in scans.into_iter().skip(keep) {
        store.delete(&scan.id)?;
        removed.push(scan.id);
    }
    if json {
        println!("{}", json!({ "removed": removed, "kept": keep }));
    } else {
        println!("removed {} snapshot(s)", removed.len());
    }
    Ok(())
}

fn cmd_config_get(json: bool, dir: Option<&str>) -> anyhow::Result<()> {
    let mut config = CoreConfig::from_env()?;
    if let Some(dir) = dir {
        config.data_dir = PathBuf::from(dir);
    }
    if json {
        println!("{}", serde_json::to_string(&config)?);
    } else {
        println!("data_dir: {}", config.data_dir.display());
        println!("socket:   {}", config.socket.display());
        println!("bind:     {}", config.bind);
        println!("roots:    {}", config.roots.join(", "));
        println!("docker:   {}", config.docker_enabled);
    }
    Ok(())
}

fn cmd_completions(shell: clap_complete::Shell) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

fn cmd_not_implemented(json: bool, command: &str) -> anyhow::Result<()> {
    if json {
        println!("{}", json!({ "command": command, "status": "not_implemented" }));
    } else {
        eprintln!("{command}: not implemented yet");
    }
    Ok(())
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

fn parse_size_filter(input: Option<&str>) -> anyhow::Result<(Option<u64>, Option<u64>)> {
    let Some(input) = input else {
        return Ok((None, None));
    };
    let (op, rest) = match input.chars().next() {
        Some('+') => ("+", &input[1..]),
        Some('-') => ("-", &input[1..]),
        _ => ("=", input),
    };
    let bytes = parse_size(rest)?;
    Ok(match op {
        "+" => (Some(bytes), None),
        "-" => (None, Some(bytes)),
        _ => (Some(bytes), Some(bytes)),
    })
}

fn parse_size(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    let (num, mult) = match s.chars().last() {
        Some('k') | Some('K') => (&s[..s.len() - 1], 1024u64),
        Some('m') | Some('M') => (&s[..s.len() - 1], 1024 * 1024),
        Some('g') | Some('G') => (&s[..s.len() - 1], 1024 * 1024 * 1024),
        Some('t') | Some('T') => (&s[..s.len() - 1], 1024u64.pow(4)),
        _ => (s, 1),
    };
    let value: f64 = num.parse()?;
    Ok((value * mult as f64) as u64)
}

fn parse_mtime_filter(input: Option<&str>) -> anyhow::Result<(Option<i64>, Option<i64>)> {
    let Some(input) = input else {
        return Ok((None, None));
    };
    let input = input.trim();
    let (sign, rest) = match input.chars().next() {
        Some('+') => (1i64, &input[1..]),
        Some('-') => (-1i64, &input[1..]),
        _ => (-1i64, input),
    };
    let (num, unit_ms) = match rest.chars().last() {
        Some('d') => (&rest[..rest.len() - 1], 86_400_000i64),
        Some('h') => (&rest[..rest.len() - 1], 3_600_000),
        Some('m') => (&rest[..rest.len() - 1], 60_000),
        _ => (rest, 86_400_000),
    };
    let amount: i64 = num.parse()?;
    let now = scanscan_core::index::now_ms();
    let boundary = now - sign * amount * unit_ms;
    if sign > 0 {
        Ok((None, Some(boundary)))
    } else {
        Ok((Some(boundary), None))
    }
}

fn print_human(value: &serde_json::Value) {
    if let Some(items) = value.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let size = item.get("sizeAlloc").and_then(|v| v.as_u64()).unwrap_or(0);
            let kind = item.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            println!("{:>14}  {:<10}  {}", tui::human_size(size), kind, name);
        }
        return;
    }
    println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_quotes_every_field() {
        assert_eq!(csv_escape("plain").as_str(), "\"plain\"");
        assert_eq!(csv_escape("a,b").as_str(), "\"a,b\"");
        assert_eq!(csv_escape("").as_str(), "\"\"");
    }

    #[test]
    fn csv_escape_neutralizes_formulas() {
        assert_eq!(csv_escape("=1+1").as_str(), "\"'=1+1\"");
        assert_eq!(csv_escape("+1").as_str(), "\"'+1\"");
        assert_eq!(csv_escape("-1").as_str(), "\"'-1\"");
        assert_eq!(csv_escape("@1").as_str(), "\"'@1\"");
        assert_eq!(csv_escape("a=1").as_str(), "\"a=1\"");
    }
}
