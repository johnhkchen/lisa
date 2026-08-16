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
    run_schedulers_with(
        root,
        stop,
        &mut out,
        now,
        &current_session(),
        &kill_session,
        &crate::presence::Machine::read(root),
    )
}

/// The Zellij session this process is sitting in, when it is in one.
pub(crate) fn current_session() -> Option<String> {
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
#[allow(clippy::too_many_arguments)]
fn run_schedulers_with(
    root: &Path,
    stop: Option<&str>,
    out: &mut impl Write,
    now: u64,
    current_session: &Option<String>,
    stopper: &dyn Fn(&Path, &str) -> Result<(), String>,
    machine: &crate::presence::Machine,
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
            machine,
        ),
        None => list(root, &roster, out, now, &resolved, machine),
    }
}

/// Whether this record counts as a scheduler that is here now.
///
/// Two questions, and the second one is the one this command was missing. A
/// fresh stamp says the process was alive when it wrote; [`crate::presence`]
/// asks this machine whether it is alive now, and a record whose Zellij server
/// is provably gone is not a running scheduler however recently it stamped.
/// A machine that cannot be asked answers nothing, and the stamp stands alone
/// exactly as it always did.
fn is_live(
    record: &SchedulerRecord,
    now: u64,
    resolved: &config::ResolvedConfig,
    machine: &crate::presence::Machine,
) -> bool {
    record.is_live(now, record.live_window_secs(resolved.wind_down_secs))
        && !machine.look(record, now).is_gone()
}

/// One scheduler, as an operator reads it.
fn describe(
    root: &Path,
    record: &SchedulerRecord,
    now: u64,
    resolved: &config::ResolvedConfig,
    machine: &crate::presence::Machine,
) -> Vec<String> {
    let presence = machine.look(record, now);
    let live = record.is_live(now, record.live_window_secs(resolved.wind_down_secs))
        && !presence.is_gone();
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
        // The live one carries the word. A run that ended is described by what
        // it did — `stopped stamping 52m ago` — rather than by a label, because
        // a listing where the only running scheduler is the one *without* a
        // marker asks the reader to notice an absence.
        if live { "  — running" } else { "" },
        match record.zellij_pid {
            Some(pid) => format!(", zellij server pid {pid}"),
            None => String::new(),
        }
    )];

    // What the name says about itself, when it says the run started before
    // today. `started 19h ago` is the same fact off the record; this one is off
    // the *name*, which is what `zellij list-sessions` shows, what the tab
    // shows, and what an operator types — so a stale session reads as stale
    // wherever they meet it, not only here.
    if let Some(note) = record
        .session_name
        .as_deref()
        .zip(crate::session_name::day_at(now))
        .and_then(|(session, today)| crate::session_name::previous_day_note(session, today))
    {
        lines.push(format!("    {note}"));
    }

    // The shell the run was started from. An operator asking *why did that run
    // fail to push when the identical one yesterday did not* is asking about
    // this line: a run that came in over ssh reaches no login keychain and
    // usually has no agent, and nothing else on the board remembers which kind
    // of run this was. A record that never observed it says so in those words
    // rather than going quiet, because a missing line and a bare shell are
    // different answers.
    lines.push(format!(
        "    started from: {}",
        match record.launched_from {
            Some(shell) => shell.describe(),
            None => lisa_core::launch_shell::NOT_RECORDED.to_string(),
        }
    ));

    // What this machine was asked and what it said, whenever asking settled it.
    // `stopped stamping 87s ago` and `pid 15340 is not a process on this
    // machine any more` are the same verdict with very different standing, and
    // the second is the one that ends an argument about whether to wait.
    if presence.is_gone() {
        lines.push(format!("    {}", presence.because()));
    }

    match (live, record.stop_command()) {
        // A remedy that can work. `zellij kill-session` is right while the
        // session is there and is the whole trap once it is not, so the run
        // that has already ended is offered the command that retires its
        // record instead (S-070-01).
        (true, Some(command)) => lines.push(format!("    stop it with: {command}")),
        (true, None) => lines.push(
            "    Lisa was never told this one's session name; find it with `zellij \
             list-sessions`"
                .to_string(),
        ),
        (false, _) => lines.push(format!(
            "    nothing to stop; clear its record with: lisa schedulers --stop {}",
            record.scheduler_id
        )),
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

/// The order an operator reads them in: whatever is running first, then the
/// runs that ended, most recently stopped first.
///
/// This command is reached for when something is holding the board, and the
/// answer is the live run. A listing that ends with it is a listing that
/// `head` truncates into history — measured on the `renderer` board, where the
/// first eight lines were two dead schedulers and a count that disagreed with
/// them.
fn reading_order<'a>(
    roster: &'a [SchedulerRecord],
    now: u64,
    resolved: &config::ResolvedConfig,
    machine: &crate::presence::Machine,
) -> Vec<&'a SchedulerRecord> {
    let mut ordered: Vec<&SchedulerRecord> = roster.iter().collect();
    ordered.sort_by_key(|record| {
        (
            !is_live(record, now, resolved, machine),
            std::cmp::Reverse(record.stamped_at),
        )
    });
    ordered
}

/// The first line, written to survive being read on its own.
///
/// A bare count is the bug: `1 scheduler is running` above four records is a
/// contradiction to anybody who reads the first line and the next two. Every
/// shape here either names both numbers or says outright that the records
/// below are history.
fn headline(live: usize, total: usize) -> String {
    match (live, total) {
        (0, 1) => "No scheduler is running on this board. The 1 record below is a run that ended."
            .to_string(),
        (0, total) => format!(
            "No scheduler is running on this board. All {total} records below are runs that ended."
        ),
        (_, 1) => "1 scheduler is running on this board.".to_string(),
        (live, total) if live == total => {
            format!("All {total} schedulers on this board are running.")
        }
        (1, total) => format!("1 of {total} schedulers on this board is running."),
        (live, total) => format!("{live} of {total} schedulers on this board are running."),
    }
}

/// Print the board's schedulers, whatever is running first.
fn list(
    root: &Path,
    roster: &[SchedulerRecord],
    out: &mut impl Write,
    now: u64,
    resolved: &config::ResolvedConfig,
    machine: &crate::presence::Machine,
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
        .filter(|record| is_live(record, now, resolved, machine))
        .collect();
    let headline = headline(live.len(), roster.len());
    write_line(out, format_args!("{headline}"))?;
    write_line(out, format_args!(""))?;
    for record in reading_order(roster, now, resolved, machine) {
        for line in describe(root, record, now, resolved, machine) {
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
    machine: &crate::presence::Machine,
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

    // Ending a run that has already ended is the ordinary case after a crash,
    // not an error: it is what an operator is doing at the moment they need
    // this command most. So Lisa says what it found and what it cleaned, and
    // whether `zellij kill-session` had anything left to do is a detail of the
    // report rather than the difference between success and refusal.
    let presence = machine.look(record, now);
    let stamping = record.is_live(now, record.live_window_secs(resolved.wind_down_secs));
    if !stamping || presence.is_gone() {
        let found = if presence.is_gone() {
            presence.because().to_string()
        } else {
            match record.age_secs(now) {
                Some(age) => format!("it stopped stamping {} ago", humanize(age)),
                None => "its last stamp is dated ahead of this machine's clock".to_string(),
            }
        };
        // Asked anyway, and its answer is not the verdict: a session that is
        // somehow still there is worth ending, and one that is not costs
        // nothing to try.
        let killed = stopper(root, &session).is_ok();
        schedulers::forget(&schedulers::roster_dir(root), &record.scheduler_id);
        let stamp = retire_orphan_stamp(root, roster, &record.scheduler_id, now, resolved, machine);
        write_line(
            out,
            format_args!(
                "{} was not running any more: {found}.\n\nCleaned: its record in {}/{}\
                 .alive{}{}. Its seats stay where they are — run `lisa release-seats` to see them.",
                record.label(),
                schedulers::SCHEDULER_DIR,
                record.scheduler_id,
                if killed {
                    format!(", and the session {session}, which was still there")
                } else {
                    String::new()
                },
                match stamp {
                    Some(path) => format!(", and {path}, which only that run could have written"),
                    None => String::new(),
                }
            ),
        )?;
        return Ok(());
    }

    if let Err(error) = stopper(root, &session) {
        // The remedy Lisa prints and the remedy Lisa runs used to be one
        // failing command with nothing after it. A kill that fails while this
        // machine still says the run is here is a real failure and still
        // stops; anything else has already been handled above.
        return Err(format!(
            "{error}\n\nNothing was stopped and nothing was changed. Lisa still reads {} as \
             running here: {}. If that is wrong — the session ended without its scheduler \
             noticing — `lisa schedulers` will say so once the stamp goes cold.",
            record.label(),
            presence.because()
        ));
    }
    // The scheduler cannot withdraw its own record — a run that has just been
    // stopped is exactly the run that cannot — so the caller who established it
    // is gone does it here.
    schedulers::forget(&schedulers::roster_dir(root), &record.scheduler_id);
    let stamp = retire_orphan_stamp(root, roster, &record.scheduler_id, now, resolved, machine);
    write_line(
        out,
        format_args!(
            "Stopped {} with `zellij kill-session {session}`.\n\nCleaned: its record in {}/{}\
             .alive{}. Its seats stay where they are — run `lisa release-seats` to see them.",
            record.label(),
            schedulers::SCHEDULER_DIR,
            record.scheduler_id,
            match stamp {
                Some(path) => format!(", and {path}, which no scheduler is left to rewrite"),
                None => String::new(),
            }
        ),
    )?;
    Ok(())
}

/// Retire the board's shared stamp once the run just ended was the last thing
/// that could have written it.
///
/// `.lisa/scheduler.alive` has one name and as many writers as there are
/// schedulers, so nobody can withdraw it and a dead run's last word keeps
/// reading as *a scheduler is here* — the half of the S-070-01 deadlock that
/// survived clearing the registry, and the reason a board was recovered by
/// moving files by hand. Left alone whenever any other scheduler is still
/// running, because that one is rewriting it every few seconds and its stamp is
/// telling the truth.
fn retire_orphan_stamp(
    root: &Path,
    roster: &[SchedulerRecord],
    stopped: &str,
    now: u64,
    resolved: &config::ResolvedConfig,
    machine: &crate::presence::Machine,
) -> Option<String> {
    let others_running = roster
        .iter()
        .filter(|record| record.scheduler_id != stopped)
        .any(|record| is_live(record, now, resolved, machine));
    if others_running {
        return None;
    }
    let stamp = root.join(lisa_core::liveness::SCHEDULER_ALIVE_FILE);
    if !stamp.exists() {
        return None;
    }
    std::fs::remove_file(&stamp)
        .ok()
        .map(|()| lisa_core::liveness::SCHEDULER_ALIVE_FILE.to_string())
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

    /// A machine nobody could ask — every record is judged by its stamp alone,
    /// exactly as it was before this command learned to ask.
    fn unasked() -> crate::presence::Machine {
        crate::presence::Machine::unknown()
    }

    /// A machine that says the recorded pid is not a process any more, and no
    /// session by that name is running either.
    fn without_the_server() -> crate::presence::Machine {
        crate::presence::Machine::fixed(Some(Vec::new()), |_| crate::presence::Server::Missing)
    }

    fn listed(root: &Path, now: u64) -> String {
        let mut out = Vec::new();
        run_schedulers_with(root, None, &mut out, now, &None, &never_stops(), &unasked()).unwrap();
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

        assert!(printed.starts_with("All 3 schedulers on this board are running."));
        assert!(printed.contains("blossoming-cymbal (cymbal)  — running"));
        assert!(printed.contains("zellij kill-session inventive-triceratops"));
        assert!(printed.contains("started 17h ago, last seen 5s ago, zellij server pid 9450"));
        assert!(printed.contains("Stop all but one"));
    }

    /// The staleness check, read off the session name rather than off the
    /// record: `renderer-0814-3` seen on the 15th is a run from a previous day,
    /// and the listing says so in those words.
    #[test]
    fn a_session_named_for_a_previous_day_is_called_one() {
        let dir = project();
        let now = 1_786_000_000;
        let yesterday = crate::session_name::day_at(now - 86_400).unwrap().stamp();
        let today = crate::session_name::day_at(now).unwrap().stamp();
        register(
            dir.path(),
            "renderer-49ded6ab",
            Some(&format!("renderer-{yesterday}-3")),
            now - 20 * 3600,
            now - 17 * 3600,
        );
        register(
            dir.path(),
            "renderer-c1133916",
            Some(&format!("renderer-{today}")),
            now - 600,
            now - 2,
        );

        let printed = listed(dir.path(), now);

        assert!(
            printed.contains("this session is from a previous day"),
            "the previous day's session was not called one:\n{printed}"
        );
        assert_eq!(
            printed
                .matches("this session is from a previous day")
                .count(),
            1,
            "today's own run must not be called stale:\n{printed}"
        );
    }

    /// One scheduler is the ordinary case, and it gets no scolding.
    #[test]
    fn one_scheduler_is_listed_without_a_warning() {
        let dir = project();
        let now = 1_786_000_000;
        register(dir.path(), "lisa-1", Some("lisa"), now - 600, now - 2);

        let printed = listed(dir.path(), now);

        assert!(printed.starts_with("1 scheduler is running on this board."));
        assert!(printed.contains("lisa (lisa-1)  — running"));
        assert!(!printed.contains("Stop all but one"));
    }

    /// A record whose scheduler stopped is history, not a running scheduler,
    /// and the listing has to say which it is — in the count as well as on the
    /// line, and without asking anyone to read a missing word.
    #[test]
    fn a_record_left_by_a_run_that_ended_is_shown_as_history() {
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

        assert!(printed.starts_with(
            "No scheduler is running on this board. The 1 record below is a run that ended."
        ));
        assert!(printed.contains("stopped stamping 17h ago"));
        assert!(
            !printed.contains("— running"),
            "nothing here is running: {printed}"
        );
    }

    /// The `renderer` board of 2026-08-14, which said `1 scheduler is running`
    /// over four records and put the live one last. The eight lines an operator
    /// actually reads have to be true and enough on their own.
    #[test]
    fn the_first_eight_lines_answer_which_run_is_holding_the_board() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "renderer-4cd787ad",
            Some("renderer"),
            now - 4 * 3600,
            now - 4 * 3600,
        );
        register(
            dir.path(),
            "renderer-0933474d",
            Some("renderer"),
            now - 4 * 3600,
            now - 4 * 3600,
        );
        register(
            dir.path(),
            "renderer-2-93ea8bf4",
            Some("renderer-2"),
            now - 4 * 3600,
            now - 52 * 60,
        );
        register(
            dir.path(),
            "renderer-3-49ded6ab",
            Some("renderer-3"),
            now - 45 * 60,
            now - 87,
        );

        let printed = listed(dir.path(), now);
        let head: Vec<&str> = printed.lines().take(8).collect();
        let head = head.join("\n");

        assert!(
            head.starts_with("1 of 4 schedulers on this board is running."),
            "the count has to name both numbers: {head}"
        );
        assert!(
            head.contains("renderer-3 (renderer-3-49ded6ab)  — running"),
            "the live run has to be inside the truncation: {head}"
        );
        assert!(
            head.contains("started 45m ago, last seen 87s ago"),
            "and its detail with it: {head}"
        );
        assert!(
            head.contains("stop it with: zellij kill-session renderer-3"),
            "and what to do about it: {head}"
        );
        assert!(
            !head.contains("stopped stamping 4h ago"),
            "the oldest history is what `head` is allowed to cut: {head}"
        );

        // The rest is history, most recently stopped first.
        let order: Vec<usize> = ["renderer-3-49ded6ab", "renderer-2-93ea8bf4"]
            .iter()
            .map(|id| printed.find(id).expect("every record is listed"))
            .collect();
        assert!(order[0] < order[1], "{printed}");
        assert!(printed.contains("stopped stamping 52m ago"));
        assert!(printed.contains("stopped stamping 4h ago"));
    }

    /// Each shape of the first line, read with nothing under it.
    #[test]
    fn the_count_says_what_it_is_counting() {
        assert_eq!(headline(1, 1), "1 scheduler is running on this board.");
        assert_eq!(
            headline(1, 4),
            "1 of 4 schedulers on this board is running."
        );
        assert_eq!(
            headline(2, 4),
            "2 of 4 schedulers on this board are running."
        );
        assert_eq!(
            headline(3, 3),
            "All 3 schedulers on this board are running."
        );
        assert_eq!(
            headline(0, 1),
            "No scheduler is running on this board. The 1 record below is a run that ended."
        );
        assert_eq!(
            headline(0, 3),
            "No scheduler is running on this board. All 3 records below are runs that ended."
        );
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

    /// The question this listing has to be able to answer the morning after:
    /// *the same board, started two ways — which one was this?*
    #[test]
    fn the_listing_says_which_kind_of_shell_each_run_was_started_from() {
        let dir = project();
        let now = 1_786_000_000;
        for (id, session, shell) in [
            (
                "desk",
                "lisa",
                lisa_core::launch_shell::LaunchShell {
                    ssh_connection: false,
                    ssh_agent: true,
                    tty: true,
                },
            ),
            (
                "overnight",
                "lisa-2",
                lisa_core::launch_shell::LaunchShell {
                    ssh_connection: true,
                    ssh_agent: false,
                    tty: false,
                },
            ),
        ] {
            schedulers::write_record(
                &schedulers::roster_dir(dir.path()),
                &SchedulerRecord::new(
                    id,
                    Some(session.to_string()),
                    Some(9450),
                    now - 600,
                    now - 2,
                    5,
                )
                .started_from(shell),
            )
            .unwrap();
        }

        let printed = listed(dir.path(), now);

        assert!(
            printed.contains("started from: not over ssh, with an ssh-agent, with a terminal"),
            "the desk's run: {printed}"
        );
        assert!(
            printed.contains("started from: over ssh, with no ssh-agent, with no terminal"),
            "the overnight run: {printed}"
        );
        assert!(
            !printed.contains("SSH_AUTH_SOCK") && !printed.contains("/private/tmp"),
            "presence is the fact; the value is a secret: {printed}"
        );
    }

    /// A run nobody observed and a run observed to have nothing must not read
    /// the same, or every record from before this existed quietly acquires the
    /// answer.
    #[test]
    fn a_run_whose_shell_was_never_observed_says_so_rather_than_going_quiet() {
        let dir = project();
        let now = 1_786_000_000;
        register(dir.path(), "old", Some("lisa"), now - 600, now - 2);
        schedulers::write_record(
            &schedulers::roster_dir(dir.path()),
            &SchedulerRecord::new(
                "bare",
                Some("lisa-2".to_string()),
                Some(9450),
                now - 600,
                now - 2,
                5,
            )
            .started_from(lisa_core::launch_shell::LaunchShell {
                ssh_connection: false,
                ssh_agent: false,
                tty: false,
            }),
        )
        .unwrap();

        let printed = listed(dir.path(), now);

        assert!(
            printed.contains("started from: not recorded (an older Lisa started this run)"),
            "the run nobody looked at: {printed}"
        );
        assert!(
            printed.contains("started from: not over ssh, with no ssh-agent, with no terminal"),
            "the run measured to have nothing: {printed}"
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
            &unasked(),
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
            &unasked(),
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
            &unasked(),
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
            &unasked(),
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
            &unasked(),
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
        run_schedulers_with(
            dir.path(),
            Some("gone"),
            &mut out,
            now,
            &None,
            &|_, _| Err("no such session".to_string()),
            &unasked(),
        )
        .unwrap();

        assert!(String::from_utf8(out)
            .unwrap()
            .contains("was not running any more"));
        assert!(schedulers::read_roster(dir.path()).is_empty());
    }

    /// The command that could not succeed. `renderer-3` stamped 87 seconds
    /// ago, its Zellij server is gone, and `--stop` used to be implemented as
    /// the one sentence that fails on exactly this board.
    #[test]
    fn stopping_a_scheduler_whose_session_is_already_gone_succeeds_and_says_what_it_cleaned() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "renderer-3-49ded6ab",
            Some("renderer-3"),
            now - 45 * 60,
            now - 87,
        );
        std::fs::create_dir_all(dir.path().join(".lisa")).unwrap();
        std::fs::write(
            dir.path().join(lisa_core::liveness::SCHEDULER_ALIVE_FILE),
            serde_json::to_vec(&lisa_core::liveness::SchedulerAlive::new(now - 87, 5)).unwrap(),
        )
        .unwrap();

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            Some("renderer-3-49ded6ab"),
            &mut out,
            now,
            &None,
            &|_, session| {
                Err(format!(
                    "`zellij kill-session {session}` exited exit status: 1"
                ))
            },
            &without_the_server(),
        )
        .unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(
            printed.contains("was not running any more: the Zellij server it recorded, pid 9450"),
            "it has to say what it found: {printed}"
        );
        assert!(
            printed.contains(".lisa/schedulers/renderer-3-49ded6ab.alive"),
            "and what it cleaned: {printed}"
        );
        assert!(
            printed.contains(".lisa/scheduler.alive"),
            "including the shared stamp only that run could have written: {printed}"
        );
        assert!(schedulers::read_roster(dir.path()).is_empty());
        assert!(
            !dir.path()
                .join(lisa_core::liveness::SCHEDULER_ALIVE_FILE)
                .exists(),
            "the board was recovered by moving this file by hand; that is the bug"
        );
    }

    /// The listing has to say the same thing the recovery does, and offer the
    /// command that can work rather than the one that cannot.
    #[test]
    fn a_scheduler_whose_server_is_gone_is_listed_as_not_running_with_the_reason() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "renderer-3-49ded6ab",
            Some("renderer-3"),
            now - 45 * 60,
            now - 87,
        );

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            None,
            &mut out,
            now,
            &None,
            &never_stops(),
            &without_the_server(),
        )
        .unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(
            printed.starts_with("No scheduler is running on this board."),
            "{printed}"
        );
        assert!(!printed.contains("— running"), "{printed}");
        assert!(
            printed.contains("pid 9450, is not a process on this machine any more"),
            "the pid it recorded is the proof: {printed}"
        );
        assert!(
            printed.contains("clear its record with: lisa schedulers --stop renderer-3-49ded6ab"),
            "a remedy that can work: {printed}"
        );
        assert!(
            !printed.contains("stop it with: zellij kill-session"),
            "the command that cannot succeed is not offered: {printed}"
        );
    }

    /// The guard this must not weaken. A run whose server answers keeps every
    /// protection it had, and stopping it is still the command that ends its
    /// session.
    #[test]
    fn a_scheduler_whose_server_is_still_up_is_still_running_and_still_protected() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "renderer-3-49ded6ab",
            Some("renderer-3"),
            now - 45 * 60,
            now - 87,
        );
        let live = crate::presence::Machine::fixed(Some(vec!["renderer-3".to_string()]), |_| {
            crate::presence::Server::Zellij {
                up_secs: Some(3 * 3600),
            }
        });

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            None,
            &mut out,
            now,
            &None,
            &never_stops(),
            &live,
        )
        .unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.starts_with("1 scheduler is running on this board."));
        assert!(printed.contains("  — running"));
        assert!(printed.contains("stop it with: zellij kill-session renderer-3"));

        // And a kill that fails while the machine still says it is here is a
        // real failure, not a cleanup.
        let error = run_schedulers_with(
            dir.path(),
            Some("renderer-3-49ded6ab"),
            &mut Vec::new(),
            now,
            &None,
            &|_, _| Err("no such session".to_string()),
            &live,
        )
        .unwrap_err();
        assert!(error.contains("Nothing was stopped"));
        assert_eq!(schedulers::read_roster(dir.path()).len(), 1);
    }

    /// A stamp another scheduler is still rewriting is not litter, and the
    /// clean-up after one dead run must not take it.
    #[test]
    fn the_shared_stamp_survives_while_any_other_scheduler_is_still_running() {
        let dir = project();
        let now = 1_786_000_000;
        register(dir.path(), "gone", Some("renderer-3"), now - 3600, now - 87);
        schedulers::write_record(
            &schedulers::roster_dir(dir.path()),
            &SchedulerRecord::new(
                "here",
                Some("renderer-4".to_string()),
                Some(9451),
                now - 600,
                now - 3,
                5,
            ),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join(".lisa")).unwrap();
        std::fs::write(
            dir.path().join(lisa_core::liveness::SCHEDULER_ALIVE_FILE),
            serde_json::to_vec(&lisa_core::liveness::SchedulerAlive::new(now - 3, 5)).unwrap(),
        )
        .unwrap();
        // The dead one's pid is gone; the live one's session is listed.
        let mixed = crate::presence::Machine::fixed(Some(vec!["renderer-4".to_string()]), |pid| {
            if pid == 9450 {
                crate::presence::Server::Missing
            } else {
                crate::presence::Server::Zellij { up_secs: None }
            }
        });

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            Some("gone"),
            &mut out,
            now,
            &None,
            &|_, _| Err("no such session".to_string()),
            &mixed,
        )
        .unwrap();

        assert!(String::from_utf8(out)
            .unwrap()
            .contains("was not running any more"));
        assert!(
            dir.path()
                .join(lisa_core::liveness::SCHEDULER_ALIVE_FILE)
                .exists(),
            "renderer-4 is still rewriting it every few seconds"
        );
        assert_eq!(schedulers::read_roster(dir.path()).len(), 1);
    }

    /// The listing is where the id is read from, so every id it prints has to
    /// be one `--stop` takes — including the dead ones now that the listing no
    /// longer leads with them.
    #[test]
    fn a_dead_record_can_be_stopped_while_a_live_scheduler_holds_the_board() {
        let dir = project();
        let now = 1_786_000_000;
        register(
            dir.path(),
            "live",
            Some("renderer-3"),
            now - 45 * 60,
            now - 87,
        );
        register(
            dir.path(),
            "dead",
            Some("renderer"),
            now - 4 * 3600,
            now - 4 * 3600,
        );

        let mut out = Vec::new();
        run_schedulers_with(
            dir.path(),
            Some("dead"),
            &mut out,
            now,
            &None,
            &|_, _| Err("no such session".to_string()),
            &unasked(),
        )
        .unwrap();

        assert!(String::from_utf8(out)
            .unwrap()
            .contains("was not running any more"));
        let left = schedulers::read_roster(dir.path());
        assert_eq!(left.len(), 1, "the live run is untouched");
        assert_eq!(left[0].scheduler_id, "live");
    }
}
