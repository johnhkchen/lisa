mod agent_exec;
mod capture_usage;
mod commit_transaction;
mod config;
mod detect;
mod doctor;
mod hooks_guide;
mod init;
mod loop_cmd;
mod setup_guide;
mod status;
mod templates;

use clap::{Parser, Subcommand};
use lisa_core::client::AgentClient;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "lisa",
    about = "Runs your coding agents through a project's tickets.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a project for lisa-loop completion
    #[command(display_order = 0)]
    Init {
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Validate ticket DAG and project setup
    #[command(display_order = 1)]
    Validate {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Also check that zellij and claude are on PATH
        #[arg(long)]
        check_tools: bool,
    },
    /// Show DAG status: tickets, dependencies, execution waves, scheduling readiness
    #[command(display_order = 2)]
    Status {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Output LLM-friendly setup instructions for this project
    #[command(hide = true)]
    SetupGuide {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Output the hooks setup guide for agents configuring Claude Code hooks
    #[command(hide = true)]
    HooksGuide,
    /// Check that all runtime dependencies are installed
    #[command(display_order = 3)]
    Doctor {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Print version information
    #[command(hide = true)]
    Version,
    /// Run Codex under Lisa's legacy JSON signal/rendering wrapper.
    ///
    /// Reads LISA_PANE_ID / LISA_TICKET_ID from the environment (inherited from
    /// the pane launch) for signal attribution. Runs `codex exec --json …`,
    /// translates its event stream into `.lisa/signals/pane-<id>.*` files, and
    /// renders the conversation to stdout. `lisa loop` uses the native Codex TUI;
    /// this remains available for diagnostics and headless automation.
    #[command(display_order = 20)]
    AgentExec {
        /// The prompt to send to codex.
        prompt: String,

        /// Resume this ticket's persisted thread (falls back to codex --last).
        #[arg(long)]
        resume: bool,

        /// Codex binary to invoke.
        #[arg(long, default_value = "codex")]
        codex_bin: String,

        /// Working tree passed to `codex -C`.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,

        /// Use `--dangerously-bypass-approvals-and-sandbox` instead of the
        /// default `-a never -s workspace-write`.
        #[arg(long)]
        bypass_sandbox: bool,

        /// Extra flag passed through to `codex exec` (repeatable).
        #[arg(long = "codex-arg")]
        codex_args: Vec<String>,

        /// Signal directory (override for tests).
        #[arg(long, default_value = ".lisa/signals")]
        signal_dir: PathBuf,
    },
    /// Capture Claude session token usage from a Stop-hook payload on stdin,
    /// writing `.lisa/claude/<ticket>.usage.json` for the provenance ledger.
    #[command(display_order = 21)]
    CaptureUsage {
        /// Project root the `.lisa/claude` artifact is written under.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Commit ticket-owned paths without using the repository's ordinary index.
    #[command(display_order = 22)]
    CommitTicket {
        /// Repository root containing the ticket changes.
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Ticket identifier used for transaction diagnostics.
        #[arg(long)]
        ticket_id: String,

        /// Commit message for the ticket completion commit.
        #[arg(long)]
        message: String,

        /// Repository-relative ticket-owned path to include (repeatable).
        #[arg(long = "include", required = true)]
        includes: Vec<PathBuf>,
    },
    /// Mark a ticket done and commit its loop-owned files atomically.
    #[command(display_order = 23)]
    CompleteTicket {
        /// Repository root containing the ticket changes.
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Ticket identifier used for transaction diagnostics.
        #[arg(long)]
        ticket_id: String,

        /// Commit message for the ticket completion commit.
        #[arg(long)]
        message: String,

        /// Repository-relative path to the ticket's real Markdown file.
        #[arg(long)]
        ticket_file: PathBuf,

        /// Repository-relative path to this ticket's work artifact directory.
        #[arg(long)]
        work_dir: PathBuf,
    },
    /// Launch zellij with the Lisa plugin for DAG-driven task scheduling
    #[command(display_order = 4)]
    Loop {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Maximum concurrent Claude sessions (overrides .lisa.toml)
        #[arg(long)]
        max_threads: Option<usize>,

        /// Agent client to drive (claude | codex); overrides .lisa.toml [agent].client
        #[arg(long)]
        client: Option<String>,

        /// Show what would be done without launching zellij
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor { path } => {
            let path = resolve_path(&path);
            if let Err(e) = doctor::run_doctor(&path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Version => {
            println!("lisa {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::AgentExec {
            prompt,
            resume,
            codex_bin,
            cwd,
            bypass_sandbox,
            codex_args,
            signal_dir,
        } => {
            let cwd = resolve_path(&cwd);
            let args = agent_exec::AgentExecArgs {
                prompt,
                resume,
                codex_bin,
                cwd,
                bypass_sandbox,
                codex_args,
                signal_dir,
            };
            if let Err(e) = agent_exec::run_agent_exec(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::CaptureUsage { cwd } => {
            // Best-effort: a hook must never fail the session. Errors (e.g. an
            // unwritable `.lisa/claude`) are swallowed; tokens stay null.
            let cwd = resolve_path(&cwd);
            let _ = capture_usage::run_capture_usage(&cwd);
        }
        Commands::CommitTicket {
            path,
            ticket_id,
            message,
            includes,
        } => {
            let request = commit_transaction::CommitTransactionRequest {
                repo_root: resolve_path(&path),
                ticket_id,
                message,
                includes,
            };
            match commit_transaction::commit_ticket(request) {
                Ok(result) => println!("{}", result.commit_id),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::CompleteTicket {
            path,
            ticket_id,
            message,
            ticket_file,
            work_dir,
        } => {
            let request = commit_transaction::CompleteTicketRequest {
                repo_root: resolve_path(&path),
                ticket_id,
                message,
                ticket_file,
                work_dir,
            };
            match commit_transaction::complete_ticket(request) {
                Ok(result) => println!("{}", result.commit_id),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Init { dry_run, path } => {
            let path = resolve_path(&path);
            if let Err(e) = init::run_init(&path, dry_run) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Validate { path, check_tools } => {
            let path = resolve_path(&path);
            if let Err(e) = init::run_validate(&path, check_tools) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status { path } => {
            let path = resolve_path(&path);
            if let Err(e) = status::run_status(&path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::SetupGuide { path } => {
            let path = resolve_path(&path);
            if let Err(e) = setup_guide::run_setup_guide(&path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::HooksGuide => {
            if let Err(e) = hooks_guide::run_hooks_guide() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Loop {
            path,
            max_threads,
            client,
            dry_run,
        } => {
            let path = resolve_path(&path);
            // Parse the --client override up front so an invalid value fails fast
            // with an actionable message, before loading the project config.
            let cli_client = match client.as_deref().map(AgentClient::parse).transpose() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let validation = match config::load_config(&path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            for w in &validation.warnings {
                eprintln!("Warning: {}", w);
            }
            let resolved = config::resolve_config(&validation.config, max_threads, cli_client);
            if let Err(e) = loop_cmd::run_loop(&path, &resolved, dry_run) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn resolve_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
