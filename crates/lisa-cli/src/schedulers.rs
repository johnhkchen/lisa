//! `lisa schedulers` — who is running this board, and how to stop one.
//!
//! ## The problem this answers
//!
//! `lisa loop` starts a Zellij *client*; the plugin that schedules the run
//! lives in the Zellij *server*, and the server outlives the client. Closing
//! the pane, closing the window, killing the `lisa loop` process — every
//! ordinary way of stopping a loop stops the client and leaves the scheduler
//! running. Measured on this repository on 2026-08-12: three schedulers on one
//! board, two of them seventeen hours old, while `ps aux | grep 'lisa loop'`
//! returned nothing at all.
//!
//! Nothing on the desk could say so, because the only thing a scheduler wrote
//! about itself was a single shared file that all three kept fresh. This
//! command reads the per-scheduler registry instead — one file per scheduler,
//! written by the scheduler — so the count is a count, and each line names the
//! session an operator would have to stop.
//!
//! ## Stopping one
//!
//! `kill <pid>` on the Zellij server did not work; `zellij kill-session <name>`
//! did. Finding the name was the hard part — it took `lsof` across every Zellij
//! server on the machine to see which ones held the project directory. The
//! session name is in the record, so `--stop` is that command with the name
//! already filled in.
//!
//! Stopping a scheduler ends its whole Zellij session, agent panes included, so
//! the id has to be named explicitly: there is no "stop them all". The one
//! refusal is the session the caller is sitting in, because a command that
//! closes the terminal it is printing to cannot report what it did.

use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use lisa_core::schedulers::{self, SchedulerRecord};

use crate::config;
use crate::seats::{humanize, now_secs};

/// How far back the listing summarises what a scheduler took out of
/// `.lisa/signals/`. An hour is long enough to cover the window an operator is
/// asking about and short enough that the line stays one line.
const RECEIPT_WINDOW_SECS: u64 = 3600;

/// How many recent signals are named per scheduler before the line is cut.
const RECEIPTS_SHOWN: usize = 4;

/// Execute the command, writing operator-facing output to stdout.
pub fn run_schedulers(root: &Path, stop: Option<&str>) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let Some(now) = now_secs() else {
        return Err(
            "This machine's clock is set before 1970, so Lisa cannot tell how old a scheduler is."
                .to_string(),
        );
    };
    run_schedulers_with(root, stop, &mut out, now, &current_session(), &kill_session)
}

/// The Zellij session this process is sitting in, when it is in one.
fn current_session() -> Option<String> {
    std::env::var("ZELLIJ_SESSION_NAME")
        .ok()
        .filter(|name| !name.is_empty())
}

/// Stop one Zellij session by name, using whichever Zellij this project runs.
fn kill_session(root: &Path, session: &str) -> Result<(), String> {
    let zellij = match config::load_config(root) {
        Ok(validation) => {
            let resolved = config::resolve_config(&validation.config, None, None);
            crate::runtime::resolve_zellij_runtime(&resolved.zellij_runtime)
                .map(|runtime| runtime.path)
                .unwrap_or_else(|_| std::path::PathBuf::from("zellij"))
        }
        Err(_) => std::path::PathBuf::from("zellij"),
    };
    let output = Command::new(&zellij)
        .arg("kill-session")
        .arg(session)
        .output()
        .map_err(|error| format!("Could not run {}: {error}", zellij.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    Err(if detail.is_empty() {
        format!("`zellij kill-session {session}` exited {}", output.status)
    } else {
        format!("`zellij kill-session {session}` said: {detail}")
    })
}

fn write_line(out: &mut impl Write, args: fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}").map_err(|error| format!("Failed to write schedulers output: {error}"))
}

/// The whole command, as a function of the tree, one clock reading, and the two
/// pieces of the outside world it touches.
fn run_schedulers_with(
    root: &Path,
    stop: Option<&str>,
    out: &mut impl Write,
    now: u64,
    current_session: &Option<String>,
    stopper: &dyn Fn(&Path, &str) -> Result<(), String>,
) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let roster = schedulers::read_roster(root);

    match stop {
        Some(target) => stop_one(
            root,
            target,
            &roster,
            out,
            now,
            &resolved,
            current_session,
            stopper,
        ),
        None => list(root, &roster, out, now, &resolved),
    }
}

/// Whether this record counts as a scheduler that is here now.
fn is_live(record: &SchedulerRecord, now: u64, resolved: &config::ResolvedConfig) -> bool {
    record.is_live(now, record.live_window_secs(resolved.wind_down_secs))
}

/// One scheduler, as an operator reads it.
fn describe(
    root: &Path,
    record: &SchedulerRecord,
    now: u64,
    resolved: &config::ResolvedConfig,
) -> Vec<String> {
    let live = is_live(record, now, resolved);
    let seen = match record.age_secs(now) {
        Some(age) if live => format!("last seen {} ago", humanize(age)),
        Some(age) => format!("stopped stamping {} ago", humanize(age)),
        None => "stamped ahead of this machine's clock".to_string(),
    };
    let started = match now.checked_sub(record.started_at) {
        Some(age) => format!("started {} ago", humanize(age)),
        None => "start time is ahead of this machine's clock".to_string(),
    };
    let mut lines = vec![format!(
        "  {}{}\n    {started}, {seen}{}",
        record.label(),
        if live { "" } else { "  — not running" },
        match record.zellij_pid {
            Some(pid) => format!(", zellij server pid {pid}"),
            None => String::new(),
        }
    )];

    match record.stop_command() {
        Some(command) => lines.push(format!("    stop it with: {command}")),
        None => lines.push(
            "    Lisa was never told this one's session name; find it with `zellij \
             list-sessions`"
                .to_string(),
        ),
    }

    if let Some(taken) = recent_receipts(root, &record.scheduler_id, now) {
        lines.push(format!("    took from .lisa/signals/: {taken}"));
    }
    lines
}

/// What this scheduler took out of the signal directory in the last hour.
fn recent_receipts(root: &Path, scheduler_id: &str, now: u64) -> Option<String> {
    let since = now.saturating_sub(RECEIPT_WINDOW_SECS);
    let mut taken: Vec<String> = schedulers::read_receipts(&schedulers::roster_dir(root))
        .into_iter()
        .filter(|receipt| receipt.scheduler_id == scheduler_id && receipt.at >= since)
        .map(|receipt| receipt.signal)
        .collect();
    if taken.is_empty() {
        return None;
    }
    let total = taken.len();
    taken.reverse();
    taken.truncate(RECEIPTS_SHOWN);
    Some(if total > RECEIPTS_SHOWN {
        format!(
            "{} in the last hour, most recently {}",
            total,
            taken.join(", ")
        )
    } else {
        taken.join(", ")
    })
}

/// Print the board's schedulers, newest run last.
fn list(
    root: &Path,
    roster: &[SchedulerRecord],
    out: &mut impl Write,
    now: u64,
    resolved: &config::ResolvedConfig,
) -> Result<(), String> {
    if roster.is_empty() {
        write_line(
            out,
            format_args!(
                "No scheduler has written itself down here. Either none has run since this \
                 version of Lisa was installed, or none is running."
            ),
        )?;
        return Ok(());
    }

    let live: Vec<&SchedulerRecord> = roster
        .iter()
        .filter(|record| is_live(record, now, resolved))
        .collect();
    write_line(
        out,
        format_args!(
            "{} on this board.",
            match live.len() {
                0 => "No scheduler is running".to_string(),
                1 => "1 scheduler is running".to_string(),
                count => format!("{count} schedulers are running"),
            }
        ),
    )?;
    write_line(out, format_args!(""))?;
    for record in roster {
        for line in describe(root, record, now, resolved) {
            write_line(out, format_args!("{line}"))?;
        }
        write_line(out, format_args!(""))?;
    }

    if live.len() > 1 {
        write_line(
            out,
            format_args!(
                "Two schedulers on one board split .lisa/signals/ between them: each sees some \
                 of what the panes write and the others take the rest. Stop all but one with \
                 `lisa schedulers --stop <id>`."
            ),
        )?;
    }
    Ok(())
}

/// Stop exactly one named scheduler.
#[allow(clippy::too_many_arguments)]
fn stop_one(
    root: &Path,
    target: &str,
    roster: &[SchedulerRecord],
    out: &mut impl Write,
    now: u64,
    resolved: &config::ResolvedConfig,
    current_session: &Option<String>,
    stopper: &dyn Fn(&Path, &str) -> Result<(), String>,
) -> Result<(), String> {
    let matches: Vec<&SchedulerRecord> = roster
        .iter()
        .filter(|record| {
            record.scheduler_id == target || record.session_name.as_deref() == Some(target)
        })
        .collect();
    let record = match matches.as_slice() {
        [record] => *record,
        [] => {
            return Err(format!(
                "No scheduler here is called {target}. Run `lisa schedulers` for the list."
            ))
        }
        many => {
            return Err(format!(
                "{target} names {} schedulers here. Use the id in brackets: {}.",
                many.len(),
                many.iter()
                    .map(|record| record.scheduler_id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    };

    let Some(session) = record.session_name.clone() else {
        return Err(format!(
            "Lisa was never told which Zellij session {} runs in, so it cannot stop it. Find it \
             with `zellij list-sessions` and stop it with `zellij kill-session <name>`.",
            record.scheduler_id
        ));
    };
    if current_session.as_deref() == Some(session.as_str()) {
        return Err(format!(
            "{session} is the session this terminal is in. Press q in its Lisa pane to stop it, \
             or run this from another terminal."
        ));
    }

    if !is_live(record, now, resolved) {
        stopper(root, &session).ok();
        schedulers::forget(&schedulers::roster_dir(root), &record.scheduler_id);
        write_line(
            out,
            format_args!(
                "{} was not running any more. Lisa forgot its record; nothing else changed.",
                record.label()
            ),
        )?;
        return Ok(());
    }

    stopper(root, &session)
        .map_err(|error| format!("{}\n\nNothing was stopped and nothing was changed.", error))?;
    // The scheduler cannot withdraw its own record — a run that has just been
    // stopped is exactly the run that cannot — so the caller who established it
    // is gone does it here.
    schedulers::forget(&schedulers::roster_dir(root), &record.scheduler_id);
    write_line(
        out,
        format_args!(
            "Stopped {} with `zellij kill-session {session}`. Its seats stay where they are — \
             run `lisa release-seats` to see them.",
            record.label()
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        dir
    }

    fn register(root: &Path, id: &str, session: Option<&str>, started: u64, stamped: u64) {
        schedulers::write_record(
            &schedulers::roster_dir(root),
            &SchedulerRecord::new(
                id,
                session.map(str::to_string),
                Some(9450),
                started,
                stamped,
                5,
            ),
        )
        .unwrap();
    }

    fn never_stops() -> impl Fn(&Path, &str) -> Result<(), String> {
        |_, session| panic!("nothing should have stopped {session}")
    }

    fn listed(root: &Path, now: u64) -> String {
        let mut out = Vec::new();
        run_schedulers_with(root, None, &mut out, now, &None, &never_stops()).unwrap();
        String::from_utf8(out).unwrap()
    }

    /// The listing an operator needed on the day and did not have.
    #[test]
    fn three_schedulers_are_counted_named_and_each_given_its_stop_command() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "cymbal",
            Some("blossoming-cymbal"),
            now - 4 * 3600,
            now - 3,
        );
        register(
            dir.path(),
            "triceratops",
            Some("inventive-triceratops"),
            now - 17 * 3600,
            now - 4,
        );
        register(
            dir.path(),
            "drum",
            Some("fascinating-drum"),
            now - 17 * 3600,
            now - 5,
        );

        let printed = listed(dir.path(), now);

        assert!(printed.starts_with("3 schedulers are running on this board."));
        assert!(printed.contains("blossoming-cymbal (cymbal)"));
        assert!(printed.contains("zellij kill-session inventive-triceratops"));
        assert!(printed.contains("started 17h ago, last seen 5s ago, zellij server pid 9450"));
        assert!(printed.contains("Stop all but one"));
    }

    /// One scheduler is the ordinary case, and it gets no scolding.
    #[test]
    fn one_scheduler_is_listed_without_a_warning() {
        let dir = project();
        let now = 1_786_000_000;
        register(dir.path(), "lisa-1", Some("lisa"), now - 600, now - 2);

        let printed = listed(dir.path(), now);

        assert!(printed.starts_with("1 scheduler is running on this board."));
        assert!(!printed.contains("Stop all but one"));
    }

    /// A record whose scheduler stopped is history, not a running scheduler,
    /// and the listing has to say which it is.
    #[test]
    fn a_record_left_by_a_run_that_ended_is_shown_as_not_running() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "gone",
            Some("lisa"),
            now - 20 * 3600,
            now - 17 * 3600,
        );

        let printed = listed(dir.path(), now);

        assert!(printed.starts_with("No scheduler is running on this board."));
        assert!(printed.contains("— not running"));
        assert!(printed.contains("stopped stamping 17h ago"));
    }

    #[test]
    fn a_board_no_scheduler_has_written_itself_on_says_exactly_that() {
        let dir = project();
        assert!(listed(dir.path(), 1_786_000_000).contains("No scheduler has written itself down"));
    }

    /// What each scheduler took out of the shared directory is the whole
    /// evidence for "the other one ate it", so the listing carries it.
    #[test]
    fn the_listing_says_what_each_scheduler_took_from_the_signal_directory() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "drum",
            Some("fascinating-drum"),
            now - 3600,
            now - 3,
        );
        for (at, signal) in [
            (now - 40, "pane-1.started"),
            (now - 30, "pane-1.claim"),
            (now - 2 * 3600, "pane-0.started"),
        ] {
            schedulers::append_receipt(
                &schedulers::roster_dir(dir.path()),
                &schedulers::SignalReceipt::new(at, "drum", signal),
            )
            .unwrap();
        }

        let printed = listed(dir.path(), now);

        assert!(printed.contains("took from .lisa/signals/: pane-1.claim, pane-1.started"));
        assert!(
            !printed.contains("pane-0.started"),
            "an hour-old receipt is not what the operator is asking about"
        );
    }

    #[test]
    fn stopping_a_live_scheduler_runs_kill_session_and_forgets_its_record() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "drum",
            Some("fascinating-drum"),
            now - 17 * 3600,
            now - 3,
        );
        let stopped = std::cell::RefCell::new(Vec::new());

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            Some("drum"),
            &mut out,
            now,
            &None,
            &|_, session| {
                stopped.borrow_mut().push(session.to_string());
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(stopped.into_inner(), vec!["fascinating-drum"]);
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("Stopped fascinating-drum (drum) with `zellij kill-session"));
        assert!(schedulers::read_roster(dir.path()).is_empty());
    }

    /// The session name is what an operator has in front of them, so it is
    /// accepted as well as the id.
    #[test]
    fn a_scheduler_can_be_named_by_its_session() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "drum",
            Some("fascinating-drum"),
            now - 3600,
            now - 3,
        );

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            Some("fascinating-drum"),
            &mut out,
            now,
            &None,
            &|_, _| Ok(()),
        )
        .unwrap();

        assert!(String::from_utf8(out).unwrap().contains("Stopped"));
    }

    /// A command that closes the terminal it is printing to cannot tell anybody
    /// what it did.
    #[test]
    fn lisa_refuses_to_stop_the_session_it_is_running_inside() {
        let dir = project();
        let now = 1_786_000_000;
        register(dir.path(), "here", Some("lisa"), now - 600, now - 2);

        let error = run_schedulers_with(
            dir.path(),
            Some("here"),
            &mut Vec::new(),
            now,
            &Some("lisa".to_string()),
            &never_stops(),
        )
        .unwrap_err();

        assert!(error.contains("the session this terminal is in"));
        assert_eq!(schedulers::read_roster(dir.path()).len(), 1);
    }

    #[test]
    fn a_failed_kill_changes_nothing_and_says_what_zellij_said() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "drum",
            Some("fascinating-drum"),
            now - 600,
            now - 2,
        );

        let error = run_schedulers_with(
            dir.path(),
            Some("drum"),
            &mut Vec::new(),
            now,
            &None,
            &|_, _| Err("`zellij kill-session fascinating-drum` said: no such session".to_string()),
        )
        .unwrap_err();

        assert!(error.contains("no such session"));
        assert!(error.contains("Nothing was stopped"));
        assert_eq!(
            schedulers::read_roster(dir.path()).len(),
            1,
            "a record is forgotten only once its scheduler really is gone"
        );
    }

    #[test]
    fn an_unknown_name_is_refused_with_the_command_that_lists_the_real_ones() {
        let dir = project();
        register(dir.path(), "drum", Some("fascinating-drum"), 1, 2);

        let error = run_schedulers_with(
            dir.path(),
            Some("cymbal"),
            &mut Vec::new(),
            1_786_000_000,
            &None,
            &never_stops(),
        )
        .unwrap_err();

        assert!(error.contains("No scheduler here is called cymbal"));
    }

    /// A scheduler that already stopped leaves a record nobody can withdraw.
    /// Naming it is how an operator clears it.
    #[test]
    fn stopping_an_already_dead_scheduler_just_forgets_it() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "gone",
            Some("lisa"),
            now - 20 * 3600,
            now - 17 * 3600,
        );

        let mut out = Vec::new();
        run_schedulers_with(dir.path(), Some("gone"), &mut out, now, &None, &|_, _| {
            Err("no such session".to_string())
        })
        .unwrap();

        assert!(String::from_utf8(out)
            .unwrap()
            .contains("was not running any more"));
        assert!(schedulers::read_roster(dir.path()).is_empty());
    }
}
