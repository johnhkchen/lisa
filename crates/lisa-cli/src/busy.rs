//! Whether this machine has a run on it right now.
//!
//! An upgrade replaces the `lisa` on this box. A run is a Zellij session with
//! Lisa's scheduler inside it, spawning `claude`, `codex` and `lisa` itself for
//! hours at a time, so swapping the binary underneath one is how you get a
//! half-old, half-new run and a failure nobody can read afterwards. Being one
//! release behind is the cheaper mistake, every time.
//!
//! So the question this module answers is deliberately blunt: **is any Zellij
//! session still running on this machine?** Not "is it one of Lisa's" — a
//! machine-level command has no project to compare against, and the cost of the
//! two mistakes is not symmetric. Refusing an upgrade because an unrelated
//! Zellij is up costs a skipped cycle the next one picks up; upgrading into a
//! live run costs the run.
//!
//! Both Zellijs a Lisa box can have are asked: the one on `PATH` (`brew install
//! zellij` on macOS) and the pinned `/usr/libexec/lisa/zellij` the Debian
//! package puts down. A Zellij that cannot be run holds nothing, which is the
//! same answer as no sessions.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session_name;

/// The Zellij the Debian package pins, which is never on `PATH`.
const PACKAGED_ZELLIJ: &str = "/usr/libexec/lisa/zellij";

/// What this machine is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Busy {
    /// Nothing is running. An upgrade can land.
    Idle,
    /// These sessions are still up, and each one may hold a scheduler.
    Running(Vec<String>),
}

impl Busy {
    /// Whether an upgrade would land under something.
    pub(crate) fn is_busy(&self) -> bool {
        matches!(self, Busy::Running(sessions) if !sessions.is_empty())
    }

    /// The sentence a person or a health record reads.
    pub(crate) fn describe(&self) -> String {
        match self {
            Busy::Idle => "no Zellij session is running on this machine".to_string(),
            Busy::Running(sessions) => format!(
                "{} still running on this machine: {}",
                if sessions.len() == 1 {
                    "one Zellij session is".to_string()
                } else {
                    format!("{} Zellij sessions are", sessions.len())
                },
                sessions.join(", ")
            ),
        }
    }
}

/// Ask every Zellij this machine could be running work under.
pub(crate) fn look() -> Busy {
    let mut sessions: Vec<String> = Vec::new();

    for zellij in candidates() {
        for name in running_under(&zellij) {
            if !sessions.contains(&name) {
                sessions.push(name);
            }
        }
    }

    if sessions.is_empty() {
        Busy::Idle
    } else {
        Busy::Running(sessions)
    }
}

/// The Zellij executables to ask: the one on `PATH`, and the packaged one.
fn candidates() -> Vec<PathBuf> {
    let mut found = vec![PathBuf::from("zellij")];
    let packaged = PathBuf::from(PACKAGED_ZELLIJ);
    if packaged.exists() {
        found.push(packaged);
    }
    found
}

/// The sessions one Zellij is still running.
///
/// `list-sessions` exits non-zero when there are none at all, so the exit
/// status carries nothing this needs — the lines it printed do. A Zellij that
/// will not run is a Zellij holding nothing.
fn running_under(zellij: &Path) -> Vec<String> {
    match Command::new(zellij)
        .arg("list-sessions")
        .arg("--no-formatting")
        .output()
    {
        Ok(output) => session_name::running_session_names(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_idle_machine_says_so_in_one_sentence() {
        assert!(!Busy::Idle.is_busy());
        assert_eq!(
            Busy::Idle.describe(),
            "no Zellij session is running on this machine"
        );
    }

    #[test]
    fn a_busy_machine_names_the_sessions_holding_it() {
        let busy = Busy::Running(vec!["lisa-6".to_string(), "overseer".to_string()]);
        assert!(busy.is_busy());
        assert_eq!(
            busy.describe(),
            "2 Zellij sessions are still running on this machine: lisa-6, overseer"
        );
        let one = Busy::Running(vec!["lisa".to_string()]);
        assert_eq!(
            one.describe(),
            "one Zellij session is still running on this machine: lisa"
        );
    }

    #[test]
    fn an_empty_running_list_is_not_busy() {
        // Belt and braces: `look` never builds this, but a `Running(vec![])`
        // must never read as a reason to hold an upgrade.
        assert!(!Busy::Running(Vec::new()).is_busy());
    }

    #[test]
    fn only_sessions_that_are_still_up_hold_an_upgrade() {
        let listing = "lisa [Created 1day ago] (EXITED - attach to resurrect)\n\
                       overseer [Created 23h ago] \n\
                       lisa-6 [Created 34m ago] (current)\n";
        assert_eq!(
            session_name::running_session_names(listing),
            ["overseer", "lisa-6"]
        );
    }
}
