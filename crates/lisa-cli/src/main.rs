mod detect;
mod init;
mod templates;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lisa", about = "Lisa - DAG-driven concurrent task scheduling")]
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
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dry_run, path } => {
            let path = resolve_path(&path);
            if let Err(e) = init::run_init(&path, dry_run) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Validate { path } => {
            let path = resolve_path(&path);
            if let Err(e) = init::run_validate(&path) {
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
