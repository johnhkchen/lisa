//! `lisa reset-ticket` — put a stalled ticket back on the board.
//!
//! ## The problem this answers
//!
//! Lisa owns `phase:`. Every assignment tells the agent so, in the same
//! sentence every time: *do not edit the ticket's phase or status fields*. And
//! in the field failure of `T-062-01-04` the only way out was for an operator
//! to open three ticket files and edit exactly that field by hand, because the
//! scheduler had advanced them to `implement` at dispatch and then never
//! started a session. `lisa clean` is about a finished ticket's litter,
//! `lisa unblock` is about a ticket waiting on a check, and `lisa release-seats`
//! is about markers a dead run left behind. None of them is about a ticket that
//! says it is being worked on when nothing is working on it.
//!
//! ## What it does, and what it deliberately does not
//!
//! It moves `implement` or `review` back to `ready`, and `status` back to
//! `open`. That is all. It deletes no work, no commits, and no attempt history:
//! a reset ticket is scheduled again with everything its earlier attempts
//! committed still in the tree.
//!
//! `done` is never a candidate. Finished work is finished, and the command that
//! disagrees with a completion record is not this one.
//!
//! ## Why it refuses while a run is live
//!
//! A running scheduler holds this board in memory — threads, seats, leases —
//! and would not learn about a phase edited underneath it until its next scan,
//! by which time it may have scheduled or completed the ticket. The dashboard's
//! own reset (`r`) does the whole job at once because it is inside that state.
//! So a live run gets pointed at the key, and a stopped run gets the command.

use std::io::{self, Write};
use std::path::Path;

use lisa_core::ticket::{scan_tickets, update_ticket_phase, update_ticket_status};
use lisa_core::types::{Phase, Ticket, TicketStatus};

use crate::config;
use crate::seats::{assess_run_at, now_secs, RunLiveness};

/// What Lisa proposes to do about one named ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResetPlan {
    /// Its phase will move back to Ready.
    Reset { phase: Phase },
    /// It is already schedulable; nothing to do.
    AlreadyReady,
    /// It is finished, and this command does not undo that.
    Finished,
    /// No ticket with this id is on the board.
    Unknown,
}

impl ResetPlan {
    fn line(&self, ticket_id: &str, attempts: &AttemptTally) -> String {
        match self {
            Self::Reset { phase } => format!(
                "  {ticket_id}  {} → ready{}",
                phase_word(*phase),
                attempts.suffix()
            ),
            Self::AlreadyReady => format!("  {ticket_id}  already on the board"),
            Self::Finished => format!("  {ticket_id}  finished — left alone"),
            Self::Unknown => format!("  {ticket_id}  no ticket with that id"),
        }
    }

    fn acts(&self) -> bool {
        matches!(self, Self::Reset { .. })
    }
}

fn phase_word(phase: Phase) -> &'static str {
    match phase {
        Phase::Ready => "ready",
        Phase::Implement => "implement",
        Phase::Review => "review",
        Phase::Done => "done",
    }
}

/// How many attempt numbers a ticket has spent, and how many of those bought no
/// session at all.
///
/// The second number is the one the field failure needed: `T-062-01-03` reached
/// *attempt 4* having never started once, and nothing on the board said so.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AttemptTally {
    pub(crate) total: usize,
    pub(crate) void: usize,
}

impl AttemptTally {
    fn suffix(&self) -> String {
        match (self.total, self.void) {
            (0, _) => String::new(),
            (total, 0) => format!("   ({total} attempt{})", plural_s(total)),
            (total, void) => format!(
                "   ({total} attempt{}, {void} of which never started a session)",
                plural_s(total)
            ),
        }
    }
}

fn plural_s(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// Count attempt directories for one ticket, and the void records among them.
pub(crate) fn tally_attempts(root: &Path, ticket_id: &str) -> AttemptTally {
    let dir = root.join(".lisa/attempts").join(ticket_id);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return AttemptTally::default();
    };
    let mut tally = AttemptTally::default();
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        if entry.file_name().to_string_lossy().parse::<u64>().is_err() {
            continue;
        }
        tally.total += 1;
        if entry.path().join("void.json").is_file() {
            tally.void += 1;
        }
    }
    tally
}

fn plan_for(ticket: Option<&Ticket>) -> ResetPlan {
    match ticket.map(|ticket| ticket.phase) {
        None => ResetPlan::Unknown,
        Some(Phase::Ready) => ResetPlan::AlreadyReady,
        Some(Phase::Done) => ResetPlan::Finished,
        Some(phase) => ResetPlan::Reset { phase },
    }
}

/// Execute the command, writing operator-facing output to stdout.
pub fn run_reset_ticket(root: &Path, ticket_ids: &[String], apply: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    run_reset_ticket_with_writer(root, ticket_ids, apply, &mut out)
}

fn write_line(out: &mut impl Write, args: std::fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}").map_err(|error| format!("Failed to write reset-ticket output: {error}"))
}

fn run_reset_ticket_with_writer(
    root: &Path,
    ticket_ids: &[String],
    apply: bool,
    out: &mut impl Write,
) -> Result<(), String> {
    if ticket_ids.is_empty() {
        return Err("Name at least one ticket to reset.".to_string());
    }

    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let tickets = scan_tickets(root.join(&resolved.ticket_dir))
        .map_err(|error| format!("Failed to read the board: {error}"))?;

    let planned: Vec<(String, ResetPlan, AttemptTally)> = ticket_ids
        .iter()
        .map(|ticket_id| {
            let ticket = tickets.iter().find(|ticket| &ticket.id == ticket_id);
            (
                ticket_id.clone(),
                plan_for(ticket),
                tally_attempts(root, ticket_id),
            )
        })
        .collect();

    write_line(out, format_args!("Planned actions:"))?;
    for (ticket_id, plan, attempts) in &planned {
        write_line(out, format_args!("{}", plan.line(ticket_id, attempts)))?;
    }
    write_line(out, format_args!(""))?;
    write_line(
        out,
        format_args!(
            "A reset changes the ticket's phase and nothing else: committed work, attempt \
             history, and finished tickets are left exactly as they are."
        ),
    )?;
    write_line(out, format_args!(""))?;

    if let Some(unknown) = planned
        .iter()
        .find(|(_, plan, _)| *plan == ResetPlan::Unknown)
    {
        return Err(format!(
            "No ticket with id {} is on the board, so nothing was changed. Check the id and run it again.",
            unknown.0
        ));
    }

    if !planned.iter().any(|(_, plan, _)| plan.acts()) {
        write_line(out, format_args!("Nothing to reset."))?;
        return Ok(());
    }

    if !apply {
        write_line(
            out,
            format_args!("Dry run complete. No changes made. Add --apply to carry this list out."),
        )?;
        return Ok(());
    }

    // The live-run guard sits after the plan and before the first write, so an
    // operator always sees what they were asking for even when Lisa says no.
    if let Some(now) = now_secs() {
        let report = assess_run_at(root, &resolved, now);
        if report.holds_the_board() {
            // Two refusals, not one. A run that is working wants the operator
            // to use the pane; a scheduler that is only stamping wants them to
            // find out which scheduler that is — the case where this message
            // used to send an operator round a loop with no exit (S-063-01).
            let next_step = match report.liveness {
                RunLiveness::Stamping => format!(
                    "Press r in the Lisa pane if you can still see it. If you cannot, that run is \
                     going without a window: {}",
                    report.how_to_stop().unwrap_or_else(|| {
                        "run `lisa schedulers` to see what is holding this board.".to_string()
                    })
                ),
                _ => "Press r in the Lisa pane to reset a ticket while the run is going, or stop \
                      the run and try again."
                    .to_string(),
            };
            return Err(format!(
                "{} is using this board, so nothing was changed. {next_step}\nEvidence: {}",
                match report.liveness {
                    RunLiveness::Stamping => "A Lisa scheduler",
                    _ => "A Lisa run",
                },
                report.evidence
            ));
        }
    }

    let mut reset = 0usize;
    for (ticket_id, plan, _) in &planned {
        if !plan.acts() {
            continue;
        }
        let Some(ticket) = tickets.iter().find(|ticket| &ticket.id == ticket_id) else {
            continue;
        };
        update_ticket_phase(&ticket.file_path, Phase::Ready)
            .map_err(|error| format!("Failed to reset {ticket_id}: {error}"))?;
        update_ticket_status(&ticket.file_path, TicketStatus::Open)
            .map_err(|error| format!("Failed to reset {ticket_id}: {error}"))?;
        reset += 1;
        write_line(out, format_args!("  reset {ticket_id}"))?;
    }

    write_line(out, format_args!(""))?;
    write_line(
        out,
        format_args!(
            "{} back on the board. Start a run and {} will be picked up again.",
            if reset == 1 {
                "1 ticket".to_string()
            } else {
                format!("{reset} tickets")
            },
            if reset == 1 { "it" } else { "they" }
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(phase: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        let tickets = dir.path().join("docs/active/tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("T-062-01-03-the-plugin-pane.md"),
            format!(
                "---\nid: T-062-01-03\ntitle: stalled\ntype: bug\nstatus: open\npriority: high\nphase: {phase}\ndepends_on: []\n---\n\nBody.\n"
            ),
        )
        .unwrap();
        dir
    }

    fn ticket_body(dir: &Path) -> String {
        std::fs::read_to_string(dir.join("docs/active/tickets/T-062-01-03-the-plugin-pane.md"))
            .unwrap()
    }

    fn run(dir: &Path, apply: bool) -> (Result<(), String>, String) {
        let mut out = Vec::new();
        let result =
            run_reset_ticket_with_writer(dir, &["T-062-01-03".to_string()], apply, &mut out);
        (result, String::from_utf8(out).unwrap())
    }

    /// The field failure's step 3, as a command: `implement` with nothing
    /// working on it goes back to `ready` without anyone opening the file.
    #[test]
    fn a_stalled_ticket_goes_back_to_ready_without_hand_editing_phase() {
        let dir = project("implement");
        let (result, output) = run(dir.path(), true);

        assert!(result.is_ok(), "{result:?}");
        assert!(output.contains("implement → ready"));
        let body = ticket_body(dir.path());
        assert!(body.contains("phase: ready"), "{body}");
        assert!(body.contains("status: open"), "{body}");
        assert!(body.contains("Body."), "the ticket's prose is untouched");
    }

    #[test]
    fn a_bare_run_lists_the_plan_and_changes_nothing() {
        let dir = project("implement");
        let (result, output) = run(dir.path(), false);

        assert!(result.is_ok());
        assert!(output.contains("Dry run complete"));
        assert!(ticket_body(dir.path()).contains("phase: implement"));
    }

    #[test]
    fn a_finished_ticket_is_left_alone() {
        let dir = project("done");
        let (result, output) = run(dir.path(), true);

        assert!(result.is_ok());
        assert!(output.contains("finished — left alone"));
        assert!(output.contains("Nothing to reset."));
        assert!(ticket_body(dir.path()).contains("phase: done"));
    }

    #[test]
    fn an_unknown_id_changes_nothing_and_says_so() {
        let dir = project("implement");
        let mut out = Vec::new();
        let result =
            run_reset_ticket_with_writer(dir.path(), &["T-NOPE".to_string()], true, &mut out);

        let error = result.unwrap_err();
        assert!(error.contains("T-NOPE"));
        assert!(error.contains("nothing was changed"));
        assert!(ticket_body(dir.path()).contains("phase: implement"));
    }

    /// A live run owns this board in memory; editing underneath it is the race
    /// the dashboard key exists to avoid.
    #[test]
    fn a_live_run_is_told_to_use_the_dashboard_instead() {
        let dir = project("implement");
        std::fs::create_dir_all(dir.path().join(".lisa")).unwrap();
        let now = now_secs().unwrap();
        std::fs::write(
            dir.path().join(lisa_core::liveness::SCHEDULER_ALIVE_FILE),
            serde_json::to_vec(&lisa_core::liveness::SchedulerAlive::new(now, 5)).unwrap(),
        )
        .unwrap();

        let (result, _) = run(dir.path(), true);

        let error = result.unwrap_err();
        assert!(error.contains("Press r in the Lisa pane"));
        assert!(ticket_body(dir.path()).contains("phase: implement"));
    }

    /// Attempt numbers that bought nothing are counted separately, so
    /// "attempt 4" cannot be read as four tries.
    #[test]
    fn the_plan_says_how_many_attempts_never_started() {
        let dir = project("implement");
        for attempt in 1..=4 {
            let attempt_dir = dir
                .path()
                .join(".lisa/attempts/T-062-01-03")
                .join(attempt.to_string());
            std::fs::create_dir_all(&attempt_dir).unwrap();
            if attempt < 4 {
                std::fs::write(attempt_dir.join("void.json"), "{}").unwrap();
            }
        }

        assert_eq!(
            tally_attempts(dir.path(), "T-062-01-03"),
            AttemptTally { total: 4, void: 3 }
        );

        let (_, output) = run(dir.path(), false);
        assert!(
            output.contains("4 attempts, 3 of which never started a session"),
            "{output}"
        );
    }
}
