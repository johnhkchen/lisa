//! Native interactive Codex launcher.
//!
//! This boundary deliberately constructs child argv with [`std::process::Command`]
//! instead of composing a shell line. The assignment path is Codex's single
//! positional prompt and remains one operating-system argument regardless of shell
//! metacharacters in the path.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

pub(crate) struct CodexLauncherArgs {
    pub(crate) assignment_path: PathBuf,
    pub(crate) codex_bin: PathBuf,
    pub(crate) model: Option<String>,
}

fn build_codex_argv(args: &CodexLauncherArgs) -> Vec<OsString> {
    let mut argv = vec![
        OsString::from("--dangerously-bypass-approvals-and-sandbox"),
        OsString::from("--dangerously-bypass-hook-trust"),
    ];
    if let Some(model) = &args.model {
        argv.push(OsString::from("--model"));
        argv.push(OsString::from(model));
    }
    argv.push(OsString::from("--"));
    argv.push(args.assignment_path.as_os_str().to_os_string());
    argv
}

pub(crate) fn run_codex_launcher(args: CodexLauncherArgs) -> Result<ExitStatus, String> {
    if !args.assignment_path.is_file() {
        return Err(format!(
            "assignment is not a regular file: {}",
            args.assignment_path.display()
        ));
    }

    let argv = build_codex_argv(&args);
    Command::new(&args.codex_bin)
        .args(argv)
        .status()
        .map_err(|error| {
            format!(
                "failed to run Codex child {}: {error}",
                args.codex_bin.display()
            )
        })
}
