mod agent_exec;
mod already_done;
mod capture_usage;
mod check_disposition;
mod check_run;
mod claim;
mod clean;
mod codex_launcher;
mod completion_seal;
mod config;
mod currency;
mod detect;
mod doctor;
mod hooks_guide;
mod init;
mod json_guide;
mod json_output;
mod legacy_context;
mod loop_cmd;
mod notes;
mod preownership_status;
mod proposal;
mod run_summary;
mod runtime;
mod seats;
mod setup_guide;
mod status;
mod templates;
mod triage_agent;
mod unblock;

use clap::{Parser, Subcommand};
use lisa_cli::commit_transaction;
use lisa_core::claim::AssignmentClaim;
use lisa_core::client::AgentClient;
use lisa_core::completion::{AttemptId, CompletionGenerationId, CompletionId};
use std::path::{Path, PathBuf};

const SETUP_FIRST_LINE: &str = "This folder isn't set up yet. Run: lisa init";

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
        after_help = "Example: lisa validate --path ./my-project --check-tools\n\nFor another program to read: lisa validate --json. What the fields mean and which ones you can rely on: lisa json-guide"
    )]
    Validate {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Also check that zellij and claude are on PATH
        #[arg(long)]
        check_tools: bool,

        /// Print one JSON document instead of prose, for another program to read
        #[arg(long)]
        json: bool,
    },
    /// Show which tickets are ready to run and which are waiting, and why.
    #[command(
        display_order = 2,
        after_help = "Example: lisa status --path ./my-project\n\nFor another program to read: lisa status --json. What the fields mean and which ones you can rely on: lisa json-guide"
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

        /// Print one JSON document instead of prose, for another program to read
        #[arg(long)]
        json: bool,
    },
    /// Read or acknowledge updates from work that kept moving.
    #[command(
        display_order = 3,
        after_help = "Examples:\n  lisa notes --path ./my-project\n  lisa notes ack T-001 --path ./my-project\n  lisa notes ack T-001 --generation 2 --path ./my-project"
    )]
    Notes {
        #[command(subcommand)]
        action: Option<NotesCommands>,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".", global = true)]
        path: PathBuf,
    },
    /// Verify what changed and let a waiting ticket run again.
    #[command(
        display_order = 4,
        after_help = "Example: lisa unblock T-001 --path ./my-project"
    )]
    Unblock {
        /// Ticket to let run again
        ticket_id: String,

        /// Let the ticket run again even when its check says no, and record that you overrode it
        #[arg(long)]
        override_check: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Finish a ticket whose work is already recorded in history.
    #[command(
        display_order = 5,
        after_help = "Example: lisa already-done T-001 --path ./my-project"
    )]
    AlreadyDone {
        /// Ticket whose work is already saved
        ticket_id: String,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Remove what an older Lisa left behind, once you have read the list.
    #[command(
        display_order = 7,
        after_help = "A bare run prints the list and changes nothing. Add --remove to carry it out.\n\nExample: lisa clean --path ./my-project"
    )]
    Clean {
        /// Remove the listed files instead of only listing them
        #[arg(long, conflicts_with = "dry_run")]
        remove: bool,

        /// Print the list and change nothing, which is what a bare run does
        #[arg(long, conflicts_with = "remove")]
        dry_run: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Free the seats a run left behind when it stopped without shutting down.
    ///
    /// Last in the everyday list on purpose: it is the one command here an
    /// operator reaches for only after something went wrong.
    #[command(
        display_order = 10,
        after_help = "A bare run prints the list and changes nothing. Add --release to carry it \
                      out.\n\nExample: lisa release-seats --path ./my-project"
    )]
    ReleaseSeats {
        /// Release the listed seats instead of only listing them
        #[arg(long, conflicts_with = "dry_run")]
        release: bool,

        /// Print the list and change nothing, which is what a bare run does
        #[arg(long, conflicts_with = "release")]
        dry_run: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Settle a first-responder proposal for a waiting ticket.
    #[command(display_order = 8)]
    Proposal {
        #[command(subcommand)]
        action: ProposalCommands,
    },
    /// Verify observable world-owned waits without operator involvement.
    #[command(hide = true)]
    RecheckWorld {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Run one bounded read-only first-responder pass.
    #[command(hide = true)]
    TriageAgent {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        client: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        ticket_path: PathBuf,
        #[arg(long)]
        disposition_path: PathBuf,
        #[arg(long)]
        timeout_secs: u64,
        #[arg(long)]
        agent_bin: Option<PathBuf>,
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
    /// Print the guide to Lisa's `--json` documents and their stability rules.
    #[command(hide = true)]
    JsonGuide,
    /// Check that the tools Lisa needs are installed.
    #[command(
        display_order = 6,
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
    /// Check this attempt's Review disposition and name every required fix.
    #[command(display_order = 23, hide = true)]
    CheckDisposition {
        /// Ticket whose Review disposition was just written.
        ticket_id: String,

        /// Project root containing the attempt work directory.
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Commit this ticket's own files without touching the repo's ordinary git index.
    #[command(display_order = 24, hide = true)]
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
    #[command(display_order = 25, hide = true)]
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
        display_order = 9,
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

#[derive(Subcommand)]
enum ProposalCommands {
    /// Run the prepared steps and let the ticket re-review.
    Apply {
        ticket_id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Discard the advice while keeping the original park.
    Dismiss {
        ticket_id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum NotesCommands {
    /// Mark a ticket's oldest or selected note as read.
    Ack {
        /// Ticket whose note has been read
        ticket_id: String,

        /// Mark this listed generation instead of the oldest
        #[arg(long)]
        generation: Option<u64>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor { path } => {
            let path = resolve_path(&path);
            require_lisa_project(&path);
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
        Commands::CheckDisposition { ticket_id, path } => {
            let path = resolve_path(&path);
            match check_disposition::run_check_disposition(&path, &ticket_id) {
                Ok(message) => println!("{message}"),
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
        Commands::Clean {
            remove,
            dry_run: _,
            path,
        } => {
            // `--dry-run` is the default said out loud, so there is nothing to
            // read here. It earns its place by conflicting with `--remove`, which
            // Clap enforces before this arm runs.
            let path = resolve_path(&path);
            require_lisa_project(&path);
            if let Err(e) = clean::run_clean(&path, remove) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::ReleaseSeats {
            release,
            dry_run: _,
            path,
        } => {
            // Same shape as `clean`: `--dry-run` names the default out loud and
            // earns its place by conflicting with the acting flag.
            let path = resolve_path(&path);
            require_lisa_project(&path);
            if let Err(e) = seats::run_release_seats(&path, release) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Validate {
            path,
            check_tools,
            json,
        } => {
            let path = resolve_path(&path);
            if json {
                std::process::exit(match lisa_project_gate(&path) {
                    Ok(()) => init::run_validate_json(&path, check_tools),
                    Err(message) => {
                        json_output::emit("validate", json_output::Outcome::Failure(message))
                    }
                });
            }
            require_lisa_project(&path);
            if let Err(e) = init::run_validate(&path, check_tools) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status {
            path,
            ticket,
            ledger,
            json,
        } => {
            let path = resolve_path(&path);
            if json {
                // `--ticket` answers a different question with a different
                // shape. Saying so in a document beats letting a caller parse
                // one shape and receive another.
                let gate = match ticket {
                    Some(_) => Err(JSON_WITH_TICKET.to_string()),
                    None => lisa_project_gate(&path),
                };
                std::process::exit(match gate {
                    Ok(()) => status::run_status_json(&path),
                    Err(message) => {
                        json_output::emit("status", json_output::Outcome::Failure(message))
                    }
                });
            }
            let result = if let Some(ticket_id) = ticket {
                let ledger_path = match ledger {
                    Some(ledger) if ledger.is_absolute() => ledger,
                    Some(ledger) => path.join(ledger),
                    None => path.join(".lisa/provenance.jsonl"),
                };
                preownership_status::run_preownership_status(&ledger_path, &ticket_id)
            } else {
                require_lisa_project(&path);
                status::run_status(&path)
            };
            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Notes { action, path } => {
            let path = resolve_path(&path);
            let result = match action {
                None => notes::run_list(&path),
                Some(NotesCommands::Ack {
                    ticket_id,
                    generation,
                }) => notes::run_ack(&path, &ticket_id, generation),
            };
            if let Err(error) = result {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        Commands::Unblock {
            ticket_id,
            override_check,
            path,
        } => {
            let path = resolve_path(&path);
            match unblock::run_unblock(&path, &ticket_id, override_check) {
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
        Commands::AlreadyDone { ticket_id, path } => {
            let path = resolve_path(&path);
            require_lisa_project(&path);
            let validation = match config::load_config(&path) {
                Ok(validation) => validation,
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            };
            let resolved = config::resolve_config(&validation.config, None, None);
            let request = already_done::AlreadyDoneRequest {
                project_root: &path,
                ticket_dir: &path.join(&resolved.ticket_dir),
                journal_path: &path
                    .join(lisa_core::completion_journal::COMPLETION_JOURNAL_RELATIVE_PATH),
            };
            match already_done::run_already_done(request, &ticket_id) {
                Ok(already_done::AlreadyDoneOutcome::Recovered {
                    ticket_id,
                    commit_id,
                    ticket_file_rewritten,
                }) => {
                    let short: String = commit_id.chars().take(8).collect();
                    println!("{ticket_id} is finished — its work was already saved in {short}.");
                    if ticket_file_rewritten {
                        println!("Its ticket file now reads done, and is not committed yet.");
                    }
                }
                Ok(already_done::AlreadyDoneOutcome::Declined(message)) => {
                    eprintln!("{message}");
                    std::process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Proposal { action } => {
            let (ticket_id, path, action) = match action {
                ProposalCommands::Apply { ticket_id, path } => {
                    (ticket_id, path, proposal::OperatorProposalAction::Apply)
                }
                ProposalCommands::Dismiss { ticket_id, path } => {
                    (ticket_id, path, proposal::OperatorProposalAction::Dismiss)
                }
            };
            match proposal::run_proposal_action(&resolve_path(&path), &ticket_id, action) {
                Ok(message) => println!("{message}"),
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
        Commands::TriageAgent {
            path,
            client,
            model,
            ticket_path,
            disposition_path,
            timeout_secs,
            agent_bin,
        } => {
            let client = match AgentClient::parse(&client) {
                Ok(client) => client,
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            };
            let args = triage_agent::TriageAgentArgs {
                root: resolve_path(&path),
                client,
                model,
                ticket_path,
                disposition_path,
                timeout_secs,
                agent_bin,
            };
            match triage_agent::run_triage_agent(&args) {
                Ok(proposal) => println!("{proposal}"),
                Err(triage_agent::TriageAgentError::TimedOut) => {
                    eprintln!("triage timed out after {timeout_secs}s");
                    std::process::exit(124);
                }
                Err(triage_agent::TriageAgentError::Failed(error)) => {
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
        Commands::JsonGuide => {
            if let Err(e) = json_guide::run_json_guide() {
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
            require_lisa_project(&path);
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

/// What `lisa status --json --ticket <id>` says instead of guessing.
const JSON_WITH_TICKET: &str = "--json reports the whole board and is not available with --ticket. Run `lisa status --json` for the board, or `lisa status --ticket <id>` without --json for one ticket's retained failures.";

/// The gate `require_lisa_project` enforces, as a sentence a caller can read.
///
/// Same predicate, one message: a JSON caller must not learn a different rule
/// from the one the prose path applies.
fn lisa_project_gate(root: &Path) -> Result<(), String> {
    if is_lisa_project(root) {
        return Ok(());
    }
    Err(format!(
        "{SETUP_FIRST_LINE} — Lisa couldn't find .lisa.toml or docs/active/tickets/ in {}.",
        root.display()
    ))
}

fn is_lisa_project(root: &Path) -> bool {
    root.join(".lisa.toml").exists() || root.join("docs/active/tickets").is_dir()
}

fn require_lisa_project(root: &Path) {
    if is_lisa_project(root) {
        return;
    }

    eprintln!(
        "{SETUP_FIRST_LINE}\n\nTechnical detail: Lisa couldn't find .lisa.toml or docs/active/tickets/ in {}.",
        root.display()
    );
    std::process::exit(1);
}

#[cfg(test)]
mod flag_audit_tests {
    use super::*;
    use clap::{ArgAction, Command, CommandFactory};
    use std::collections::{BTreeMap, BTreeSet};

    const FLAG_AUDIT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/knowledge/flag-audit.md"
    ));
    const MISSING_ROW_FIXTURE: &str = include_str!("../tests/fixtures/flag-audit-missing-row.md");

    const PROMPT_IDS: [&str; 4] = [
        "prompt:init-project-history",
        "prompt:dashboard-mark-done",
        "prompt:dashboard-reset-ticket",
        "prompt:dashboard-quit-pending",
    ];
    const ALLOWED_ASK_CATEGORIES: [&str; 2] = ["destructive/irreversible", "expert override"];
    const BANNED_VOICE: [&str; 9] = [
        "dag",
        "orchestrat",
        "scheduling",
        "leverage",
        "solutions",
        "deployment",
        "case study",
        "build log",
        "research release",
    ];

    #[derive(Debug)]
    struct AuditRow {
        id: String,
        surface: String,
        bar: String,
        rule: String,
        fixture: String,
        category: String,
    }

    fn parse_audit_rows(markdown: &str) -> Result<BTreeMap<String, AuditRow>, String> {
        let mut rows = BTreeMap::new();

        for (index, line) in markdown.lines().enumerate() {
            let cells: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            let Some(raw_id) = cells.first() else {
                continue;
            };
            let id = raw_id
                .strip_prefix('`')
                .and_then(|value| value.strip_suffix('`'))
                .unwrap_or(raw_id);
            if !id.starts_with("flag:") && !id.starts_with("config:") && !id.starts_with("prompt:")
            {
                continue;
            }
            if cells.len() != 6 {
                return Err(format!(
                    "audit row {} ({id}) has {} cells; expected 6",
                    index + 1,
                    cells.len()
                ));
            }

            let row = AuditRow {
                id: id.to_string(),
                surface: cells[1].to_string(),
                bar: cells[2].to_string(),
                rule: cells[3].to_string(),
                fixture: cells[4].to_string(),
                category: cells[5].to_string(),
            };
            if rows.insert(row.id.clone(), row).is_some() {
                return Err(format!("audit contains duplicate row {id}"));
            }
        }

        Ok(rows)
    }

    fn collect_flags() -> BTreeMap<String, bool> {
        fn visit(command: &Command, path: &str, flags: &mut BTreeMap<String, bool>) {
            for argument in command.get_arguments() {
                if matches!(
                    argument.get_action(),
                    ArgAction::Help
                        | ArgAction::HelpShort
                        | ArgAction::HelpLong
                        | ArgAction::Version
                ) {
                    continue;
                }
                if let Some(long) = argument.get_long() {
                    flags.insert(format!("flag:{path}:--{long}"), argument.is_required_set());
                }
            }

            for subcommand in command.get_subcommands() {
                if subcommand.get_name() == "help" {
                    continue;
                }
                visit(
                    subcommand,
                    &format!("{path}/{}", subcommand.get_name()),
                    flags,
                );
            }
        }

        let mut command = Cli::command();
        command.build();
        let mut flags = BTreeMap::new();
        visit(&command, command.get_name(), &mut flags);
        flags
    }

    fn collect_config_ids() -> Result<BTreeSet<String>, String> {
        let ids: BTreeSet<String> = config::CONFIG_KEYS
            .iter()
            .map(|key| format!("config:{}", key.path))
            .collect();
        if ids.len() != config::CONFIG_KEYS.len() {
            return Err("CONFIG_KEYS contains a duplicate dotted path".to_string());
        }
        Ok(ids)
    }

    fn validate_row_policy(row: &AuditRow) -> Result<(), String> {
        if row.surface.is_empty() || row.surface.contains(['\n', '\r']) {
            return Err(format!("{} needs a one-line surface description", row.id));
        }
        if row.rule.is_empty()
            || row.rule.contains(['\n', '\r'])
            || !row.rule.ends_with(['.', '?', '!'])
        {
            return Err(format!(
                "{} needs a complete one-line default or justification",
                row.id
            ));
        }

        match row.bar.as_str() {
            "working default" => {
                if row.fixture.is_empty() || row.fixture == "—" {
                    return Err(format!("{} default needs a pinning fixture", row.id));
                }
                if row.category != "—" {
                    return Err(format!("{} default category must be —", row.id));
                }
            }
            "justified ask" => {
                if !ALLOWED_ASK_CATEGORIES.contains(&row.category.as_str()) {
                    return Err(format!(
                        "{} ask category must be destructive/irreversible or expert override",
                        row.id
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "{} bar must be working default or justified ask",
                    row.id
                ));
            }
        }

        let voiced = format!("{} {}", row.surface, row.rule).to_ascii_lowercase();
        if let Some(term) = BANNED_VOICE.iter().find(|term| voiced.contains(**term)) {
            return Err(format!(
                "{} operator-facing copy contains banned term {term:?}",
                row.id
            ));
        }
        Ok(())
    }

    fn coverage_error(
        kind: &str,
        expected: &BTreeSet<String>,
        actual: &BTreeSet<String>,
    ) -> Option<String> {
        let missing: Vec<&String> = expected.difference(actual).collect();
        let unexpected: Vec<&String> = actual.difference(expected).collect();
        if missing.is_empty() && unexpected.is_empty() {
            return None;
        }
        Some(format!(
            "{kind} rows differ; missing: [{}]; unexpected: [{}]",
            missing
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            unexpected
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    fn verify_audit(markdown: &str) -> Result<(), String> {
        let rows = parse_audit_rows(markdown)?;
        let mut errors = Vec::new();
        for row in rows.values() {
            if let Err(error) = validate_row_policy(row) {
                errors.push(error);
            }
        }

        let actual_flags: BTreeSet<String> = rows
            .keys()
            .filter(|id| id.starts_with("flag:"))
            .cloned()
            .collect();
        let flags = collect_flags();
        let expected_flags = flags.keys().cloned().collect();
        if let Some(error) = coverage_error("flag", &expected_flags, &actual_flags) {
            errors.push(error);
        }
        for (id, required) in flags {
            let Some(row) = rows.get(&id) else {
                continue;
            };
            let expected_bar = if required {
                "justified ask"
            } else {
                "working default"
            };
            if row.bar != expected_bar {
                errors.push(format!(
                    "{id} must use {expected_bar:?} because Clap required={required}"
                ));
            }
        }

        let actual_config: BTreeSet<String> = rows
            .keys()
            .filter(|id| id.starts_with("config:"))
            .cloned()
            .collect();
        if let Some(error) = coverage_error("config", &collect_config_ids()?, &actual_config) {
            errors.push(error);
        }

        let expected_prompts: BTreeSet<String> =
            PROMPT_IDS.iter().map(|id| id.to_string()).collect();
        let actual_prompts: BTreeSet<String> = rows
            .keys()
            .filter(|id| id.starts_with("prompt:"))
            .cloned()
            .collect();
        if let Some(error) = coverage_error("prompt", &expected_prompts, &actual_prompts) {
            errors.push(error);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    #[test]
    fn flag_audit_covers_live_cli_config_and_prompts() {
        verify_audit(FLAG_AUDIT).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn flag_audit_missing_row_fixture_names_every_gap() {
        let error = verify_audit(MISSING_ROW_FIXTURE).unwrap_err();
        assert!(error.contains("flag:lisa/loop:--client"), "{error}");
        assert!(error.contains("config:agent.client"), "{error}");
    }
}
