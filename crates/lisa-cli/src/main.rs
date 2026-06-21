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
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "lisa",
    about = "Lisa - DAG-driven concurrent task scheduling",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a project for lisa-loop completion
    Init {
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Validate ticket DAG and project setup
    Validate {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Also check that zellij and claude are on PATH
        #[arg(long)]
        check_tools: bool,
    },
    /// Show DAG status: tickets, dependencies, execution waves, scheduling readiness
    Status {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Output LLM-friendly setup instructions for this project
    SetupGuide {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Output the hooks setup guide for agents configuring Claude Code hooks
    HooksGuide,
    /// Check that all runtime dependencies are installed
    Doctor {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Print version information
    Version,
    /// Launch zellij with the Lisa plugin for DAG-driven task scheduling
    Loop {
        /// Path to the project root (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Maximum concurrent Claude sessions (overrides .lisa.toml)
        #[arg(long)]
        max_threads: Option<usize>,

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
            dry_run,
        } => {
            let path = resolve_path(&path);
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
            let resolved = config::resolve_config(&validation.config, max_threads);
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
