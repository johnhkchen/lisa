mod agent_exec;
mod capture_usage;
mod claim;
mod codex_launcher;
mod completion_seal;
mod config;
mod detect;
mod doctor;
mod hooks_guide;
mod init;
mod loop_cmd;
mod preownership_status;
mod run_summary;
mod runtime;
mod setup_guide;
mod status;
mod templates;
mod unblock;

use clap::{Parser, Subcommand};
use lisa_cli::commit_transaction;
use lisa_core::claim::AssignmentClaim;
use lisa_core::client::AgentClient;
use lisa_core::completion::{AttemptId, CompletionGenerationId, CompletionId};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "lisa",
    about = "Runs coding agents through your ticket board, so you don't have to approve every step by hand.",
    before_help = "Everyday path: init → validate → status → loop",
    after_help = "Plumbing commands (called by Lisa and agent hooks):
  agent-exec       Run Codex and turn its output into Lisa's pane signals
  capture-usage    Record a native session's token usage from its Stop-hook payload on stdin
  claim            Assert ownership of one exact ticket assignment
  commit-ticket    Commit this ticket's own files without touching the repo's ordinary git index
  complete-ticket  Mark a ticket done and commit its files in one step",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up a project to run with Lisa.
    #[command(
        display_order = 0,
        after_help = "Example: lisa init --path ./my-project"
    )]
    Init {
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Bring project history along for undo and a record of finished work
        #[arg(long, conflicts_with = "no_history")]
        with_history: bool,

        /// Continue without project history
        #[arg(long, conflicts_with = "with_history")]
        no_history: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Check your tickets and project setup for problems before a run.
    #[command(
        display_order = 1,
        after_help = "Example: lisa validate --path ./my-project --check-tools"
    )]
    Validate {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Also check that zellij and claude are on PATH
        #[arg(long)]
        check_tools: bool,
    },
    /// Show which tickets are ready to run and which are waiting, and why.
    #[command(
        display_order = 2,
        after_help = "Example: lisa status --path ./my-project"
    )]
    Status {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Show retained pre-ownership failures for this ticket
        #[arg(long)]
        ticket: Option<String>,

        /// Provenance ledger to read (defaults to .lisa/provenance.jsonl)
        #[arg(long, requires = "ticket")]
        ledger: Option<PathBuf>,
    },
    /// Verify what changed and let a waiting ticket run again.
    #[command(
        display_order = 3,
        after_help = "Example: lisa unblock T-001 --path ./my-project"
    )]
    Unblock {
        /// Ticket to let run again
        ticket_id: String,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Verify observable world-owned waits without operator involvement.
    #[command(hide = true)]
    RecheckWorld {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Print setup instructions for an agent to follow.
    #[command(hide = true)]
    SetupGuide {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Print the guide for wiring up Claude Code hooks.
    #[command(hide = true)]
    HooksGuide,
    /// Check that the tools Lisa needs are installed.
    #[command(
        display_order = 4,
        after_help = "Example: lisa doctor --path ./my-project"
    )]
    Doctor {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Print Lisa's version.
    #[command(hide = true)]
    Version,
    /// Run Codex and turn its output into Lisa's pane signals.
    ///
    /// Reads LISA_PANE_ID / LISA_TICKET_ID from the environment (inherited from
    /// the pane launch) for signal attribution. Runs `codex exec --json …`,
    /// translates its event stream into `.lisa/signals/pane-<id>.*` files, and
    /// renders the conversation to stdout. `lisa loop` uses the native Codex TUI;
    /// this remains available for diagnostics and headless automation.
    #[command(display_order = 20, hide = true)]
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
    /// Record a native session's token usage from its Stop-hook payload on stdin,
    /// appending observed facts to `.lisa/<client>/captures.jsonl`.
    #[command(display_order = 21, hide = true)]
    CaptureUsage {
        /// Project root the `.lisa/<client>` capture ledger is written under.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Start interactive Codex with one exact assignment-file argument.
    #[command(hide = true)]
    LaunchCodex {
        /// Exact atomically published assignment file used as Codex's initial prompt.
        assignment: PathBuf,

        /// Codex executable to invoke.
        #[arg(long, default_value = "codex")]
        codex_bin: PathBuf,

        /// Routed Codex model; omit to use Codex's configured default.
        #[arg(long)]
        model: Option<String>,
    },
    /// Assert ownership of one exact nonce-bearing ticket assignment.
    #[command(display_order = 22, hide = true)]
    Claim {
        /// Project root containing Lisa's attempt and signal directories.
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Ticket identifier carried by the assignment.
        #[arg(long)]
        ticket_id: String,

        /// E-034 attempt generation carried by the assignment.
        #[arg(long)]
        attempt_id: u64,

        /// Opaque nonce identifying the exact assignment file.
        #[arg(long)]
        nonce: u128,
    },
    /// Commit this ticket's own files without touching the repo's ordinary git index.
    #[command(display_order = 23, hide = true)]
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
    /// Mark a ticket done and commit its files in one step.
    #[command(display_order = 24, hide = true)]
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

        /// Attempt identity authorized to complete this ticket.
        #[arg(long)]
        attempt_id: String,

        /// Idempotency generation for this attempt's completion transaction.
        #[arg(long)]
        completion_generation: u64,
    },
    /// Start a run: work through the ready tickets, in parallel where they don't collide.
    #[command(
        display_order = 5,
        after_help = "Example: lisa loop --path ./my-project --max-threads 3"
    )]
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
            // Keep the release-pinned runtime manifest in every platform build.
            // Without this OS-neutral reference, fat LTO can remove the Linux-only
            // managed acquisition path (and its manifest) from Darwin artifacts.
            std::hint::black_box(runtime::MANAGED_RUNTIME_SHA256_MANIFEST);
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
            let cwd = resolve_path(&cwd);
            if let Err(e) = capture_usage::run_capture_usage(&cwd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::LaunchCodex {
            assignment,
            codex_bin,
            model,
        } => {
            let args = codex_launcher::CodexLauncherArgs {
                assignment_path: assignment,
                codex_bin,
                model,
            };
            match codex_launcher::run_codex_launcher(args) {
                Ok(status) if status.success() => {}
                Ok(status) => std::process::exit(status.code().unwrap_or(1)),
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Claim {
            path,
            ticket_id,
            attempt_id,
            nonce,
        } => {
            let request = claim::ClaimRequest {
                project_root: resolve_path(&path),
                pane_id: std::env::var("LISA_PANE_ID").ok(),
                claim: AssignmentClaim {
                    ticket_id,
                    attempt_id,
                    nonce,
                },
            };
            match claim::claim_assignment(request) {
                Ok(receipt) => {
                    let claim = receipt.claim;
                    println!(
                        "Claim accepted: {} attempt {} nonce {}",
                        claim.ticket_id, claim.attempt_id, claim.nonce
                    );
                }
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            }
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
            attempt_id,
            completion_generation,
        } => {
            let completion_key = CompletionGenerationId::new(
                CompletionId::new(ticket_id.clone()),
                AttemptId::new(attempt_id),
                completion_generation,
            );
            let request = commit_transaction::CompleteTicketRequest {
                repo_root: resolve_path(&path),
                ticket_id,
                message,
                ticket_file,
                work_dir,
                completion_key,
            };
            match commit_transaction::complete_ticket(request) {
                Ok(result) => println!("{}", result.commit_id),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Init {
            dry_run,
            with_history,
            no_history,
            path,
        } => {
            let path = resolve_path(&path);
            let history = if with_history {
                init::HistoryPreference::WithHistory
            } else if no_history {
                init::HistoryPreference::NoHistory
            } else {
                init::HistoryPreference::Ask
            };
            if let Err(e) = init::run_init(&path, dry_run, history) {
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
        Commands::Status {
            path,
            ticket,
            ledger,
        } => {
            let path = resolve_path(&path);
            let result = if let Some(ticket_id) = ticket {
                let ledger_path = match ledger {
                    Some(ledger) if ledger.is_absolute() => ledger,
                    Some(ledger) => path.join(ledger),
                    None => path.join(".lisa/provenance.jsonl"),
                };
                preownership_status::run_preownership_status(&ledger_path, &ticket_id)
            } else {
                status::run_status(&path)
            };
            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Unblock { ticket_id, path } => {
            let path = resolve_path(&path);
            match unblock::run_unblock(&path, &ticket_id) {
                Ok(unblock::UnblockOutcome::Reopened(message)) => println!("{message}"),
                Ok(unblock::UnblockOutcome::Declined(message)) => {
                    eprintln!("{message}");
                    std::process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Commands::RecheckWorld { path } => {
            let path = resolve_path(&path);
            match unblock::run_world_rechecks(&path) {
                Ok(reopened) => {
                    for ticket_id in reopened {
                        println!("{ticket_id}");
                    }
                }
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
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
