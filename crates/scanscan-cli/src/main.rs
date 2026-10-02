//! `scanscan` — CLI and daemon entry point.
//!
//! Every subcommand supports `--json` for scripting and agent use. Phase 0
//! wires the argument surface and implements `config get`; scan/query commands
//! are filled in during Phase 1.

use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::json;

use scanscan_core::{Config as CoreConfig, CORE_VERSION};
use scanscan_ipc::PROTOCOL_VERSION;

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
        data_dir: Option<String>,
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

    let result = match cli.command {
        Command::Config {
            action: ConfigAction::Get,
        } => cmd_config_get(cli.json),
        Command::Config {
            action: ConfigAction::Set { key, value },
        } => cmd_not_implemented(cli.json, "config set", &json!({ "key": key, "value": value })),
        Command::Completions { shell } => cmd_completions(shell),
        Command::Daemon { socket, data_dir } => cmd_not_implemented(
            cli.json,
            "daemon",
            &json!({ "socket": socket, "data_dir": data_dir }),
        ),
        Command::Scan { paths, .. } => {
            cmd_not_implemented(cli.json, "scan", &json!({ "roots": paths }))
        }
        Command::Ls { path, .. } => cmd_not_implemented(cli.json, "ls", &json!({ "path": path })),
        Command::Du { path, .. } => cmd_not_implemented(cli.json, "du", &json!({ "path": path })),
        Command::Top { path, n, .. } => {
            cmd_not_implemented(cli.json, "top", &json!({ "path": path, "n": n }))
        }
        Command::Find { path, .. } => cmd_not_implemented(cli.json, "find", &json!({ "path": path })),
        Command::Tree { path, .. } => cmd_not_implemented(cli.json, "tree", &json!({ "path": path })),
        Command::Ext { path } => cmd_not_implemented(cli.json, "ext", &json!({ "path": path })),
        Command::Diff { a, b } => {
            cmd_not_implemented(cli.json, "diff", &json!({ "a": a, "b": b }))
        }
        Command::Docker { stats } => {
            cmd_not_implemented(cli.json, "docker", &json!({ "stats": stats }))
        }
        Command::Export { snapshot, format } => {
            cmd_not_implemented(cli.json, "export", &json!({ "snapshot": snapshot, "format": format }))
        }
        Command::Snapshots { action } => {
            let name = match action {
                Some(SnapshotsAction::List) | None => "snapshots list",
                Some(SnapshotsAction::Delete { .. }) => "snapshots delete",
                Some(SnapshotsAction::Gc { .. }) => "snapshots gc",
            };
            cmd_not_implemented(cli.json, name, &json!({}))
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if cli.json {
                let payload = json!({ "ok": false, "error": err.to_string() });
                println!("{payload}");
            } else {
                eprintln!("error: {err}");
            }
            ExitCode::FAILURE
        }
    }
}

fn cmd_config_get(as_json: bool) -> anyhow::Result<()> {
    let config = CoreConfig::from_env()?;
    if as_json {
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

fn cmd_not_implemented(
    as_json: bool,
    command: &str,
    args: &serde_json::Value,
) -> anyhow::Result<()> {
    if as_json {
        let payload = json!({
            "command": command,
            "status": "not_implemented",
            "core_version": CORE_VERSION,
            "protocol_version": PROTOCOL_VERSION,
            "args": args,
        });
        println!("{payload}");
    } else {
        eprintln!("{command}: not implemented yet (planned for Phase 1)");
    }
    Ok(())
}
