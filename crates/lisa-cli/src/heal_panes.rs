//! `lisa heal-panes` — ask a running loop to put back a pane it lost.
//!
//! ## The problem this answers
//!
//! `.lisa-layout.kdl` declares the coding panes and Zellij creates them once, at
//! `lisa loop`. A pane that dies — fenced by Lisa, crashed with its terminal,
//! closed by hand — used to be gone for the session's lifetime, and the board
//! quietly ran on fewer seats than it said it had. Measured on `screen-design`
//! on 2026-08-13:
//!
//! ```text
//! lisa-5            → 4 children    healthy
//! screen-design-4   → 2 children    running on half its panes
//! ```
//!
//! The loop now notices that on its own, off the pane event Zellij already
//! sends. This is the door for whoever noticed first — `rail`, a monitoring
//! script, a person reading the tab — so the answer does not have to wait for
//! the next thing that changes a pane.
//!
//! ## This command creates nothing
//!
//! Creating geometry inside Zellij is the plugin's job and nobody else's:
//! `rail` forbids itself from splitting panes (`no-zellij-split`) and it is
//! right to. So this writes a request and reads the answer. The one thing that
//! can put a pane back where the layout wanted it is the plugin running inside
//! the Zellij server, and it decides.
//!
//! ## Three answers
//!
//! - **healed** — the board was short; a pane was made and joined the stack.
//! - **already fine** — every pane the layout declared is there. Nothing was
//!   created and asking again will say the same thing.
//! - **refused** — the loop will not, and says why: it has spent its
//!   regeneration budget, or it was launched from a layout that never said how
//!   many panes it made.
//!
//! A fourth outcome is not an answer: **nothing replied**. That means no loop is
//! running on this board, or its plugin is not ticking, and it is reported as
//! its own thing rather than dressed up as a refusal.

use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lisa_core::pane_heal::{
    publish_request, read_receipt, PaneHealAnswer, PaneHealReceipt, PaneHealRequest,
};
use lisa_core::schedulers;

use crate::config;

/// How long to wait for a scheduler's receipt, by default.
///
/// The plugin reads the request on its poll tick and answers a whole board
/// immediately; a board that is short waits for the pane to actually exist,
/// which is one more Zellij round trip. Four poll intervals covers both with
/// room for a tick that lands badly.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// How often the receipt is looked for while waiting.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// What the ask came back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealPanesOutcome {
    /// A scheduler answered.
    Answered(PaneHealReceipt),
    /// Nothing answered inside the timeout.
    Unanswered {
        /// Seconds waited.
        waited_secs: u64,
        /// The running schedulers Lisa could see, for the sentence that
        /// explains the silence.
        schedulers: Vec<String>,
    },
}

impl HealPanesOutcome {
    /// Whether the caller should treat the ask as successful.
    pub fn is_satisfied(&self) -> bool {
        match self {
            Self::Answered(receipt) => receipt.answer.is_satisfied(),
            Self::Unanswered { .. } => false,
        }
    }
}

/// Ask, wait, and report.
pub fn run_heal_panes(
    root: &Path,
    asked_by: &str,
    timeout_secs: Option<u64>,
    json: bool,
) -> Result<HealPanesOutcome, String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    run_heal_panes_with(root, asked_by, timeout_secs, json, &mut out, &sleep)
}

fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}

/// The command, with its clock and its writer supplied, so a test can run it
/// without waiting in real time.
pub(crate) fn run_heal_panes_with(
    root: &Path,
    asked_by: &str,
    timeout_secs: Option<u64>,
    json: bool,
    out: &mut dyn Write,
    sleep: &dyn Fn(Duration),
) -> Result<HealPanesOutcome, String> {
    // Refuse to ask a directory that is not a Lisa project rather than time out
    // in it: a mistyped `--path` and a dead loop must not read the same.
    if !root.join(".lisa.toml").exists() {
        return Err(format!(
            "{} is not a Lisa project — there is no .lisa.toml here, so there is no loop to ask.",
            root.display()
        ));
    }
    config::load_config(root)?;

    let nonce = nonce_for(root)?;
    let asked_at = now_secs().unwrap_or_default();
    publish_request(root, &PaneHealRequest::new(&nonce, asked_at, asked_by))?;

    let budget = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let deadline = SystemTime::now() + Duration::from_secs(budget);
    let outcome = loop {
        if let Some(receipt) = read_receipt(root, &nonce) {
            break HealPanesOutcome::Answered(receipt);
        }
        if SystemTime::now() >= deadline {
            break HealPanesOutcome::Unanswered {
                waited_secs: budget,
                schedulers: live_schedulers(root),
            };
        }
        sleep(POLL_INTERVAL);
    };

    if json {
        writeln!(out, "{}", render_json(&outcome)).map_err(|error| error.to_string())?;
    } else {
        writeln!(out, "{}", render(&outcome)).map_err(|error| error.to_string())?;
    }
    Ok(outcome)
}

/// The sentence a person reads.
pub(crate) fn render(outcome: &HealPanesOutcome) -> String {
    match outcome {
        HealPanesOutcome::Answered(receipt) => {
            let mut line = match receipt.answer {
                PaneHealAnswer::Healed => "Healed.".to_string(),
                PaneHealAnswer::AlreadyFine => "Already fine.".to_string(),
                PaneHealAnswer::Refused => "Not this time.".to_string(),
            };
            line.push(' ');
            line.push_str(&receipt.detail);
            if let Some(scheduler) = &receipt.scheduler {
                line.push_str(&format!("\n  Answered by {scheduler}."));
            }
            line
        }
        HealPanesOutcome::Unanswered {
            waited_secs,
            schedulers,
        } => {
            let mut line = format!("Nothing answered in {waited_secs}s.");
            if schedulers.is_empty() {
                line.push_str(
                    " No loop is running on this board — start one with `lisa loop`, and it will \
                     make all of its panes.",
                );
            } else {
                line.push_str(&format!(
                    " {} is running here but did not answer, so its dashboard is not ticking. \
                     `lisa schedulers` says how to stop it.",
                    schedulers.join(", ")
                ));
            }
            line
        }
    }
}

/// The same outcome for another program to read.
pub(crate) fn render_json(outcome: &HealPanesOutcome) -> String {
    let document = match outcome {
        HealPanesOutcome::Answered(receipt) => serde_json::json!({
            "answered": true,
            "answer": receipt.answer.id(),
            "detail": receipt.detail,
            "declared": receipt.declared,
            "present": receipt.present,
            "scheduler": receipt.scheduler,
        }),
        HealPanesOutcome::Unanswered {
            waited_secs,
            schedulers,
        } => serde_json::json!({
            "answered": false,
            "waited_secs": waited_secs,
            "schedulers": schedulers,
        }),
    };
    document.to_string()
}

/// Identify this ask, so its answer cannot be confused with another one's.
///
/// Built from the clock and this process's own pid: two asks from one machine
/// in the same second are two processes, and an ask that races a stale receipt
/// is the failure the nonce exists to stop.
fn nonce_for(_root: &Path) -> Result<String, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "This machine's clock is set before 1970.".to_string())?;
    Ok(format!("{}-{}", stamp.as_nanos(), std::process::id()))
}

fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// The schedulers that are running on this board.
///
/// A stamp and a process, both, for the reason every other reader now asks for
/// both: this sentence is read by somebody whose panes did not come back, and
/// telling them a run they cannot find is here — because a dead one's last
/// note is fresh — sends them looking for something to stop instead of
/// starting a loop (S-070-01).
fn live_schedulers(root: &Path) -> Vec<String> {
    let Some(now) = now_secs() else {
        return Vec::new();
    };
    let wind_down = config::load_config(root)
        .map(|validation| config::resolve_config(&validation.config, None, None).wind_down_secs)
        .unwrap_or_default();
    let machine = crate::presence::Machine::read(root);
    schedulers::read_roster(root)
        .into_iter()
        .filter(|record| {
            record.is_live(now, record.live_window_secs(wind_down))
                && !machine.look(record, now).is_gone()
        })
        .map(|record| record.label())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::pane_heal::{publish_receipt, take_request};

    fn project(root: &Path) {
        std::fs::create_dir_all(root.join(".lisa")).unwrap();
        std::fs::write(
            root.join(".lisa.toml"),
            format!(
                "version = \"{}\"\n\n[dirs]\ntickets = \"docs/active/tickets\"\n\
                 stories = \"docs/active/stories\"\nwork = \"docs/active/work\"\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .unwrap();
    }

    /// The scheduler side, run inline: take the ask and leave the receipt this
    /// answer describes.
    fn answering<'a>(
        root: &'a Path,
        answer: PaneHealAnswer,
        detail: &str,
    ) -> impl Fn(Duration) + 'a {
        let detail = detail.to_string();
        move |_| {
            if let Some(request) = take_request(root) {
                publish_receipt(
                    root,
                    &PaneHealReceipt::new(
                        request.nonce,
                        answer,
                        detail.clone(),
                        Some(4),
                        if answer == PaneHealAnswer::Healed {
                            4
                        } else {
                            3
                        },
                        Some("lisa-1".to_string()),
                        0,
                    ),
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn asked_and_healed_reads_as_success() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let mut out = Vec::new();
        let outcome = run_heal_panes_with(
            dir.path(),
            "rail",
            Some(5),
            false,
            &mut out,
            &answering(dir.path(), PaneHealAnswer::Healed, "Put a pane back."),
        )
        .unwrap();

        assert!(outcome.is_satisfied());
        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("Healed."), "{printed}");
        assert!(printed.contains("Answered by lisa-1."), "{printed}");
    }

    #[test]
    fn asked_and_already_fine_is_not_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let mut out = Vec::new();
        let outcome = run_heal_panes_with(
            dir.path(),
            "rail",
            Some(5),
            false,
            &mut out,
            &answering(
                dir.path(),
                PaneHealAnswer::AlreadyFine,
                "Nothing to do — all 4 coding panes are already there.",
            ),
        )
        .unwrap();

        assert!(
            outcome.is_satisfied(),
            "a board that was already whole is the answer the caller wanted"
        );
        assert!(String::from_utf8(out).unwrap().contains("Already fine."));
    }

    #[test]
    fn asked_and_refused_says_so_and_fails() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let mut out = Vec::new();
        let outcome = run_heal_panes_with(
            dir.path(),
            "rail",
            Some(5),
            false,
            &mut out,
            &answering(
                dir.path(),
                PaneHealAnswer::Refused,
                "Lisa has stopped asking. Restart the loop.",
            ),
        )
        .unwrap();

        assert!(!outcome.is_satisfied());
        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("Not this time."), "{printed}");
        assert!(printed.contains("Restart the loop."), "{printed}");
    }

    /// Silence is its own outcome, and it names the thing to do about it.
    #[test]
    fn nothing_answering_is_not_reported_as_a_refusal() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let mut out = Vec::new();
        let outcome =
            run_heal_panes_with(dir.path(), "rail", Some(0), false, &mut out, &|_| {}).unwrap();

        assert!(matches!(outcome, HealPanesOutcome::Unanswered { .. }));
        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("Nothing answered"), "{printed}");
        assert!(printed.contains("No loop is running"), "{printed}");
        assert!(
            !printed.contains("Not this time"),
            "silence is not a scheduler's verdict: {printed}"
        );
    }

    #[test]
    fn a_receipt_for_somebody_elses_ask_is_not_this_ones_answer() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        publish_receipt(
            dir.path(),
            &PaneHealReceipt::new(
                "somebody-else",
                PaneHealAnswer::Healed,
                "not yours",
                Some(4),
                4,
                None,
                0,
            ),
        )
        .unwrap();

        let mut out = Vec::new();
        let outcome =
            run_heal_panes_with(dir.path(), "rail", Some(0), false, &mut out, &|_| {}).unwrap();

        assert!(matches!(outcome, HealPanesOutcome::Unanswered { .. }));
    }

    #[test]
    fn a_directory_that_is_not_a_lisa_project_is_refused_rather_than_waited_on() {
        let dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        let error =
            run_heal_panes_with(dir.path(), "rail", Some(0), false, &mut out, &|_| {}).unwrap_err();
        assert!(!error.is_empty());
        assert!(out.is_empty(), "nothing is printed for a bad path");
    }

    #[test]
    fn the_json_document_carries_the_answer_and_the_counts() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let mut out = Vec::new();
        run_heal_panes_with(
            dir.path(),
            "rail",
            Some(5),
            true,
            &mut out,
            &answering(dir.path(), PaneHealAnswer::Healed, "Put a pane back."),
        )
        .unwrap();

        let document: serde_json::Value =
            serde_json::from_slice(String::from_utf8(out).unwrap().trim().as_bytes()).unwrap();
        assert_eq!(document["answered"], serde_json::json!(true));
        assert_eq!(document["answer"], serde_json::json!("healed"));
        assert_eq!(document["declared"], serde_json::json!(4));
        assert_eq!(document["present"], serde_json::json!(4));
    }
}
