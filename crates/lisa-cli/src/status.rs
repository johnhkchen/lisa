use std::path::Path;

use crate::config;
use crate::json_output::{BoardCounts, ConfigView};
use lisa_core::completion::{CompletionSeal, CompletionState};
use lisa_core::completion_journal::{CompletionJournalAggregate, COMPLETION_JOURNAL_RELATIVE_PATH};
use lisa_core::dag::{CycleDetectionResult, Dag, DagError, DagStats};
use lisa_core::disposition::RemedyOwner;
use lisa_core::notes::{collect_notes, QueuedNote};
use lisa_core::parking::{collect_parked_remedies, ParkedRemedy};
use lisa_core::provenance::RunOutcome;
use lisa_core::provenance::{
    correct_usage, latest_world_rechecks, lost_seat_usage_gap, usage_gap, ProvenanceLedgerRecord,
    WorldRecheckObservation,
};
use lisa_core::types::{AttemptLease, Phase, Ticket};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// How many recorded non-passes make a world remedy worth naming as stuck.
///
/// Rows land on a doubling schedule, so this is the fourth one. Below it a
/// world remedy that has not cleared is simply waiting for the world, which is
/// what a world remedy is *for*; at it, the check has kept saying no long enough
/// that the check itself is the likelier problem.
const STUCK_NON_PASS_COUNT: u64 = 8;

/// Group a token count with thousands separators so big numbers stay legible.
fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (bytes.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(*byte));
    }
    out
}

/// Measured token usage per ticket, from the **corrected view** — totals
/// layered from append-only usage corrections, never the raw first-write row.
/// A completed ticket whose capture never joined stays a visible, counted gap
/// rather than a fabricated zero (T-051-03-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TokenUsageView {
    /// One entry per ticket whose capture joined, in ticket-id order.
    tickets: Vec<TicketUsageView>,
    tickets_joined: usize,
    tokens_in: u64,
    tokens_out: u64,
    /// Completed tickets whose usage capture is pending or never arrived.
    not_yet_joined: Vec<String>,
    /// Tickets whose seat was taken away with no tokens joined.
    ///
    /// Separate from `not_yet_joined` because the prognosis differs. A
    /// completed ticket's capture is normally late, not absent — the Stop hook
    /// fired and the sweep will join it. A seat lost before its session ever
    /// stopped produced no capture at all, and nothing later can invent one.
    /// Counting them apart is what keeps that spend from disappearing.
    lost_with_the_seat: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TicketUsageView {
    ticket_id: String,
    tokens_in: u64,
    tokens_out: u64,
}

fn token_usage_view(records: &[ProvenanceLedgerRecord]) -> TokenUsageView {
    let corrected = correct_usage(records);
    let mut tickets = Vec::new();
    let mut tokens_in = 0u64;
    let mut tokens_out = 0u64;
    for (ticket_id, usage) in &corrected {
        if let (Some(ticket_in), Some(ticket_out)) = (usage.tokens_in, usage.tokens_out) {
            tokens_in = tokens_in.saturating_add(ticket_in);
            tokens_out = tokens_out.saturating_add(ticket_out);
            tickets.push(TicketUsageView {
                ticket_id: ticket_id.clone(),
                tokens_in: ticket_in,
                tokens_out: ticket_out,
            });
        }
    }
    TokenUsageView {
        tickets_joined: tickets.len(),
        tickets,
        tokens_in,
        tokens_out,
        not_yet_joined: usage_gap(records),
        lost_with_the_seat: lost_seat_usage_gap(records),
    }
}

/// Render the "Token usage" block from the same view `--json` serialises, so
/// the two cannot say different things about the same ledger.
fn token_usage_lines(records: &[ProvenanceLedgerRecord]) -> Vec<String> {
    let view = token_usage_view(records);

    let mut lines = vec!["Token usage".to_string()];
    for ticket in &view.tickets {
        lines.push(format!(
            "  {:<12}  {} in / {} out",
            ticket.ticket_id,
            group_thousands(ticket.tokens_in),
            group_thousands(ticket.tokens_out),
        ));
    }

    if view.tickets_joined == 0 {
        lines.push("  Nothing measured yet.".to_string());
    } else {
        lines.push(format!(
            "  Joined {} ticket{}: {} in / {} out",
            view.tickets_joined,
            if view.tickets_joined == 1 { "" } else { "s" },
            group_thousands(view.tokens_in),
            group_thousands(view.tokens_out),
        ));
    }

    let lost = view.lost_with_the_seat.len();
    if lost > 0 {
        lines.push(format!(
            "  Lost with the seat: {} ticket{} — the pane went before its session could report \
             what it spent, so those tokens are not recoverable.",
            lost,
            if lost == 1 { "" } else { "s" },
        ));
    }

    let gap = view.not_yet_joined.len();
    if gap > 0 {
        lines.push(format!(
            "  Not yet joined: {} completed ticket{} — usage capture pending or never arrived.",
            gap,
            if gap == 1 { "" } else { "s" },
        ));
    }

    lines
}

fn read_ledger(ledger_path: &Path) -> Vec<ProvenanceLedgerRecord> {
    std::fs::read_to_string(ledger_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn print_token_usage(records: &[ProvenanceLedgerRecord]) {
    for line in token_usage_lines(records) {
        println!("{line}");
    }
    println!();
}

/// The two lines a world remedy earns once its check has kept saying no.
///
/// Automation never acts on a non-pass — that policy is unchanged — so the only
/// thing that can change is the silence. This says what Lisa has seen and names
/// the one command that ends the wait, addressed to the person who can decide
/// the check is wrong.
fn stuck_world_lines(remedy: &ParkedRemedy, seen: &WorldRecheckObservation) -> Vec<String> {
    // "at least": rows are sampled, so the last recorded total is a floor rather
    // than a live count, and saying otherwise would overstate what Lisa knows.
    vec![
        format!(
            "       Lisa has checked at least {} times and it still isn't passing.",
            seen.non_pass_count
        ),
        format!(
            "       If you have checked this yourself, run: lisa unblock {} --override-check",
            remedy.ticket_id
        ),
    ]
}

fn waiting_on_you_lines(
    remedies: &[ParkedRemedy],
    rechecks: &HashMap<String, WorldRecheckObservation>,
) -> Vec<String> {
    remedies
        .iter()
        .flat_map(|remedy| {
            let lead = match remedy.remedy_owner {
                RemedyOwner::Operator => format!("{}  {}", remedy.ticket_id, remedy.ask),
                RemedyOwner::World => format!(
                    "{}  {} — Lisa checks on its own.",
                    remedy.ticket_id, remedy.ask
                ),
                RemedyOwner::Agent => return Vec::new(),
            };
            // The count belongs to one check. A rewritten disposition is a
            // different claim about the world, and inherits nothing.
            let stuck = (remedy.remedy_owner == RemedyOwner::World)
                .then(|| rechecks.get(&remedy.ticket_id))
                .flatten()
                .filter(|seen| {
                    remedy.check.as_deref() == Some(seen.check.as_str())
                        && seen.non_pass_count >= STUCK_NON_PASS_COUNT
                });
            let mut lines = Vec::new();
            if let Some(proposal) = &remedy.proposal {
                lines.push(format!(
                    "{}  First responder: {}",
                    remedy.ticket_id, proposal.summary
                ));
                lines.push(format!("       Suggested: {}", proposal.recommendation));
                lines.extend(
                    proposal
                        .prepared_steps
                        .iter()
                        .map(|step| format!("       Prepared: {}", step.display())),
                );
                lines.push(format!("       Original ask: {}", remedy.ask));
            } else {
                lines.push(lead);
            }
            if let Some(seen) = stuck {
                lines.extend(stuck_world_lines(remedy, seen));
            }
            lines.push(format!("       Reviewer's note: {}", remedy.reason));
            lines
        })
        .collect()
}

fn print_waiting_on_you(
    remedies: &[ParkedRemedy],
    rechecks: &HashMap<String, WorldRecheckObservation>,
) {
    let lines = waiting_on_you_lines(remedies, rechecks);
    if lines.is_empty() {
        println!("Waiting on you");
        println!("Nothing waiting.\n");
        return;
    }

    println!("Waiting on you");
    for line in lines {
        println!("{line}");
    }
    println!();
}

/// A ticket the board says is under way that nothing is working.
///
/// `implement` and `review` both mean *an agent has this*. When no pane lease
/// marker names the ticket, nothing does, and the board is describing work that
/// is not happening. That state is reachable honestly — the scheduler fences a
/// seat and withdraws its marker in the same breath — so the answer is not to
/// stop it happening but to stop it being silent about it.
///
/// The word each one gets comes from the ledger, and the whole point of the
/// row-writing changes beside this is that there is now something to read.
/// `evidence` is the last thing the ledger has to say about this ticket, in a
/// sentence, including the case where it says nothing at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StrandedTicket {
    ticket_id: String,
    /// `implement` or `review`, spelled as the board spells it.
    phase: String,
    /// The last attempt the ledger names for this ticket, or `null` when it
    /// names none.
    attempt_id: Option<u64>,
    /// The ledger's last word, as one sentence.
    evidence: String,
}

/// The last thing the ledger says about one ticket, in a sentence.
///
/// Ledger order is append order, so the last matching row is the latest fact.
/// Only rows that speak to *this attempt's fate* count: a launch row that is
/// never answered is itself the finding, which is why it is folded in here
/// rather than filtered out.
fn ledger_last_word(ticket_id: &str, ledger: &[ProvenanceLedgerRecord]) -> (Option<u64>, String) {
    let mut last: Option<(Option<u64>, String)> = None;
    for record in ledger {
        let word = match record {
            ProvenanceLedgerRecord::AttemptLaunch(launch) if launch.ticket_id == ticket_id => (
                Some(launch.attempt_lease.attempt_id),
                format!(
                    "attempt {} was launched and nothing has recorded how it ended",
                    launch.attempt_lease.attempt_id
                ),
            ),
            ProvenanceLedgerRecord::Execution(exec) if exec.ticket_id == ticket_id => {
                let attempt = Some(exec.attempt_lease.attempt_id);
                let sentence = match exec.outcome {
                    RunOutcome::SeatLost => format!(
                        "attempt {} lost its seat: {}",
                        exec.attempt_lease.attempt_id,
                        exec.reason.as_deref().unwrap_or("no reason was recorded"),
                    ),
                    RunOutcome::Failed => {
                        format!("attempt {} failed", exec.attempt_lease.attempt_id)
                    }
                    RunOutcome::TimedOut => {
                        format!("attempt {} timed out", exec.attempt_lease.attempt_id)
                    }
                    RunOutcome::Done => format!(
                        "attempt {} recorded done, but the board still says it is under way",
                        exec.attempt_lease.attempt_id
                    ),
                };
                (attempt, sentence)
            }
            ProvenanceLedgerRecord::AssignmentTransition(transition)
                if transition.ticket_id == ticket_id =>
            {
                (
                    Some(transition.attempt_lease.attempt_id),
                    format!(
                        "attempt {} ended before an agent owned it ({}): {}",
                        transition.attempt_lease.attempt_id,
                        serde_json::to_string(&transition.state)
                            .unwrap_or_default()
                            .trim_matches('"'),
                        transition.reason,
                    ),
                )
            }
            _ => continue,
        };
        last = Some(word);
    }
    last.unwrap_or((
        None,
        "the ledger has no record of an attempt on this ticket".to_string(),
    ))
}

fn stranded_tickets(
    tickets: &[Ticket],
    attempts: &[SeatAttempt],
    ledger: &[ProvenanceLedgerRecord],
) -> Vec<StrandedTicket> {
    tickets
        .iter()
        .filter(|ticket| matches!(ticket.phase, Phase::Implement | Phase::Review))
        .filter(|ticket| {
            !attempts
                .iter()
                .any(|attempt| attempt.ticket_id == ticket.id)
        })
        .map(|ticket| {
            let (attempt_id, evidence) = ledger_last_word(&ticket.id, ledger);
            StrandedTicket {
                ticket_id: ticket.id.clone(),
                phase: ticket.phase.to_string(),
                attempt_id,
                evidence,
            }
        })
        .collect()
}

/// Silent when there are none, for the same reason the abandoned-seat report
/// is: a healthy board should not have to read a line saying nothing is wrong.
fn stranded_ticket_lines(stranded: &[StrandedTicket]) -> Vec<String> {
    if stranded.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["Tickets nobody is working".to_string()];
    for ticket in stranded {
        lines.push(format!(
            "  {:<12} {:<10} {}",
            ticket.ticket_id, ticket.phase, ticket.evidence
        ));
    }
    lines.push("  The board says these are under way and no pane holds them.".to_string());
    lines.push("  To hand one back: lisa reset-ticket <ticket-id> --apply".to_string());
    lines
}

fn print_stranded_tickets(data: &StatusData) {
    let lines = stranded_ticket_lines(&data.stranded);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        println!("{line}");
    }
    println!();
}

/// A ticket the board calls ready that no run will take.
///
/// The scheduler declines a ready ticket whose completion is still masking its
/// durable Done, and until this existed the board went on counting that ticket
/// as ready anyway: `1 ready, 0 blocked` beside a live run with four idle
/// slots, taking nothing, for ten minutes. Both halves of that sentence were
/// false. The condition here is the scheduler's own, so the two cannot drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct UnschedulableTicket {
    ticket_id: String,
    /// What is holding it, in one sentence.
    reason: String,
    /// The command that ends it, or `null` when only waiting does.
    command: Option<String>,
}

/// Every ticket the DAG calls ready whose completion is not settled.
fn unschedulable_tickets(
    ready: &[String],
    aggregates: &HashMap<String, CompletionJournalAggregate>,
) -> Vec<UnschedulableTicket> {
    ready
        .iter()
        .filter_map(|ticket_id| {
            let aggregate = aggregates.get(ticket_id)?;
            if !aggregate.masks_durable_done() {
                return None;
            }
            let (reason, command) = match aggregate.state() {
                CompletionState::Rejected { .. } => (
                    "Lisa could not record its finished work, and nothing has settled that yet."
                        .to_string(),
                    Some(format!("lisa already-done {ticket_id}")),
                ),
                _ => (
                    "Lisa is recording its finished work right now.".to_string(),
                    None,
                ),
            };
            Some(UnschedulableTicket {
                ticket_id: ticket_id.clone(),
                reason,
                command,
            })
        })
        .collect()
}

/// Silent when there are none, like every other fault report here.
fn unschedulable_ticket_lines(unschedulable: &[UnschedulableTicket]) -> Vec<String> {
    if unschedulable.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["Tickets no run will take".to_string()];
    for ticket in unschedulable {
        lines.push(format!("  {:<12} {}", ticket.ticket_id, ticket.reason));
        if let Some(command) = &ticket.command {
            lines.push(format!("               To finish it: {command}"));
        }
    }
    lines.push(
        "  These are counted as blocked above: a ticket nothing can schedule is not ready."
            .to_string(),
    );
    lines
}

fn print_unschedulable_tickets(data: &StatusData) {
    let lines = unschedulable_ticket_lines(&data.unschedulable);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        println!("{line}");
    }
    println!();
}

/// The seats a run left behind, if there are any.
///
/// Silent otherwise. This is a fault report, and a project where nothing is
/// wrong should not have to read a line saying so — the `Status:` counts above
/// already describe a healthy board. When it does speak it names the ticket in
/// each seat, the evidence, and the one command that ends it, because the whole
/// complaint this answers was that the state was undiscoverable rather than
/// unrecoverable.
fn abandoned_seat_lines(attempts: &[SeatAttempt], run: &crate::seats::RunReport) -> Vec<String> {
    let abandoned: Vec<&SeatAttempt> = attempts
        .iter()
        .filter(|attempt| attempt.abandoned)
        .collect();
    if abandoned.is_empty() {
        return Vec::new();
    }

    let mut lines = vec!["Seats held by a run that is gone".to_string()];
    for attempt in abandoned {
        lines.push(format!(
            "  pane {:<3} {:<12}  attempt {}  ({})",
            attempt.pane_id,
            attempt.ticket_id,
            attempt.attempt_id,
            attempt
                .ticket_phase
                .as_deref()
                .unwrap_or("not on the board"),
        ));
    }
    lines.push(format!("  {}", run.evidence));
    lines.push("  To free them, read the list first: lisa release-seats".to_string());
    lines
}

fn print_abandoned_seats(data: &StatusData) {
    let lines = abandoned_seat_lines(&data.attempts, &data.run);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        println!("{line}");
    }
    println!();
}

/// Who is running this board, when that is not exactly one scheduler.
///
/// Silent for the ordinary case, loud for the one that cost a day: `lisa
/// status` read perfectly normally while three schedulers held the board, and
/// nothing an operator could type would have told them otherwise (S-063-01).
fn contested_board_lines(run: &crate::seats::RunReport) -> Vec<String> {
    if run.schedulers.len() < 2 {
        return Vec::new();
    }
    let mut lines = vec![format!(
        "{} are running on this board",
        run.schedulers.len()
    )];
    for record in &run.schedulers {
        lines.push(format!(
            "  {}{}",
            record.label(),
            match record.stop_command() {
                Some(command) => format!("  —  stop it with: {command}"),
                None => String::new(),
            }
        ));
    }
    lines.push(
        "  They split the signals your panes write between them, so each one sees part of what \
         is happening and the others take the rest."
            .to_string(),
    );
    lines.push("  To see them and stop all but one: lisa schedulers".to_string());
    lines
}

fn print_contested_board(data: &StatusData) {
    let lines = contested_board_lines(&data.run);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        println!("{line}");
    }
    println!();
}

fn print_status_notes(notes: &[QueuedNote]) {
    if notes.is_empty() {
        println!("Notes for you");
        println!("Nothing to read.\n");
    } else {
        crate::notes::print_notes(notes);
    }
}

fn format_config_summary(resolved: &config::ResolvedConfig, seal: CompletionSeal) -> String {
    let timeout_str = if resolved.session_timeout_secs == 0 {
        "disabled".to_string()
    } else {
        format!("{}s", resolved.session_timeout_secs)
    };
    let mut output = format!(
        "Config: max_threads={}, session_timeout={}\n",
        resolved.max_threads, timeout_str
    );
    if !resolved.phase_timeouts.is_empty() {
        let mut entries: Vec<_> = resolved.phase_timeouts.iter().collect();
        entries.sort_by_key(|(key, _)| (*key).clone());
        let parts: Vec<String> = entries
            .iter()
            .map(|(key, value)| format!("{}={}s", key, value))
            .collect();
        output.push_str(&format!("  phase_timeouts: {}\n", parts.join(" ")));
    }
    output.push_str(crate::completion_seal::visibility_line(seal));
    output.push('\n');
    output
}

/// One seat the scheduler placed an attempt into.
///
/// Read from `.lisa/signals/pane-<id>.lease`, which the plugin publishes
/// atomically for each launch and is the scheduler's own record of the
/// placement — not a re-derivation from the ledger. The marker is deliberately
/// *not* revoked when a seat is released, so an entry names the attempt the
/// scheduler most recently placed in that pane. `ticket_phase` is that ticket's
/// phase on the board right now, which is how a consumer tells a running seat
/// from a finished one without re-deriving residency. `lisa json-guide` states
/// this in the same words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SeatAttempt {
    pane_id: u32,
    ticket_id: String,
    attempt_id: u64,
    /// `None` when the lease names a ticket that is no longer on the board.
    ticket_phase: Option<String>,
    /// Whether a later attempt for the same ticket holds another seat.
    ///
    /// `AttemptLease::mint` is strictly monotonic per ticket, so between two
    /// markers naming one ticket the lower attempt is definitively the older
    /// one. That is a fact about the markers themselves, which is why it can be
    /// stated here without reconstructing residency from the ledgers.
    superseded: bool,
    /// Whether the run that placed this seat has stopped without withdrawing
    /// it.
    ///
    /// True only when Lisa can say so from two agreeing facts — see
    /// [`crate::seats`]. Doubt reads as `false`, because a seat wrongly called
    /// free can put a second agent on a ticket somebody is working.
    abandoned: bool,
    /// The evidence behind `abandoned`, in the same words `lisa status` prints,
    /// or `None` when the seat is not abandoned.
    abandoned_reason: Option<String>,
}

/// Read every pane lease marker the scheduler has published, in pane order.
///
/// A marker that cannot be read or parsed is skipped rather than reported: a
/// half-written file is the scheduler's business, and this reader must never
/// turn one into a failure for a consumer asking what is running.
fn read_seat_attempts(
    signal_dir: &Path,
    tickets: &[Ticket],
    run: &crate::seats::RunReport,
) -> Vec<SeatAttempt> {
    let phases: HashMap<&str, String> = tickets
        .iter()
        .map(|ticket| (ticket.id.as_str(), ticket.phase.to_string()))
        .collect();
    let Ok(entries) = std::fs::read_dir(signal_dir) else {
        return Vec::new();
    };

    let mut attempts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("lease") {
            continue;
        }
        let Some(pane_id) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.strip_prefix("pane-"))
            .and_then(|pane| pane.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(lease) = serde_json::from_str::<AttemptLease>(&body) else {
            continue;
        };
        attempts.push(SeatAttempt {
            pane_id,
            ticket_phase: phases.get(lease.ticket_id.as_str()).cloned(),
            ticket_id: lease.ticket_id,
            attempt_id: lease.attempt_id,
            superseded: false,
            // The verdict is about the run, not the pane: every marker in the
            // directory predates the moment that run stopped, so they are
            // abandoned together or not at all.
            abandoned: run.seats_are_abandoned(),
            abandoned_reason: run.seats_are_abandoned().then(|| run.evidence.clone()),
        });
    }

    let newest: HashMap<String, u64> =
        attempts.iter().fold(HashMap::new(), |mut newest, attempt| {
            let seen = newest.entry(attempt.ticket_id.clone()).or_default();
            *seen = (*seen).max(attempt.attempt_id);
            newest
        });
    for attempt in &mut attempts {
        attempt.superseded = newest
            .get(&attempt.ticket_id)
            .is_some_and(|newest| *newest > attempt.attempt_id);
    }

    attempts.sort_by_key(|attempt| attempt.pane_id);
    attempts
}

/// Everything `lisa status` computes, assembled once.
///
/// The prose and the JSON document are two renderings of this one value, which
/// is the only way to keep them from disagreeing.
struct StatusData {
    ticket_dir_rel: String,
    resolved: config::ResolvedConfig,
    completion_seal: CompletionSeal,
    parked_remedies: Vec<ParkedRemedy>,
    rechecks: HashMap<String, WorldRecheckObservation>,
    notes: Vec<QueuedNote>,
    tickets: Vec<Ticket>,
    ledger: Vec<ProvenanceLedgerRecord>,
    attempts: Vec<SeatAttempt>,
    /// Tickets the board says are under way that no seat holds.
    stranded: Vec<StrandedTicket>,
    /// Tickets the DAG calls ready that no run will take.
    unschedulable: Vec<UnschedulableTicket>,
    /// Why the completion journal could not be folded, when it could not be.
    ///
    /// `status` is the command an operator reaches for when the board is
    /// wrong, so a journal it cannot read is a line in the report rather than
    /// a reason to print nothing at all.
    journal_error: Option<String>,
    /// What Lisa can say about the run that placed those seats.
    run: crate::seats::RunReport,
    /// `None` when the ticket directory holds no tickets: there is no board to
    /// describe, which is a fact rather than a failure.
    board: Option<StatusBoard>,
    run_summary: Option<crate::run_summary::RunSummary>,
}

/// The dependency structure `status` prints, computed once.
struct StatusBoard {
    dag: Dag,
    stats: DagStats,
    waves: Vec<Vec<String>>,
    /// Sorted, exactly as the "Ready to schedule" line lists them.
    ready: Vec<String>,
}

/// Scan tickets, build the DAG, and gather everything status reports.
fn collect_status(root: &Path) -> Result<StatusData, String> {
    // Load config to get ticket directory and scheduling settings
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let ticket_dir_rel = resolved.ticket_dir.clone();
    let work_dir_rel = resolved.work_dir.clone();
    let completion_seal =
        crate::completion_seal::resolve_for_inspection(root, resolved.completion_mode);

    let ticket_dir = root.join(&ticket_dir_rel);
    if !ticket_dir.exists() {
        return Err(format!(
            "Ticket directory not found: {}",
            ticket_dir.display()
        ));
    }

    // Scan tickets
    let tickets = lisa_core::ticket::scan_tickets(&ticket_dir)
        .map_err(|e| format!("Failed to scan tickets: {}", e))?;

    let ledger_path = root.join(".lisa/provenance.jsonl");
    let parked_remedies =
        collect_parked_remedies(tickets.iter(), &root.join(&work_dir_rel), &ledger_path);
    let rechecks = latest_world_rechecks(&ledger_path);
    let notes = collect_notes(
        &root.join(".lisa/completion-journal.jsonl"),
        &root.join(".lisa/provenance.jsonl"),
    )?;
    let run = crate::seats::assess_run(root, &resolved);
    let attempts = read_seat_attempts(&root.join(".lisa/signals"), &tickets, &run);

    let board = if tickets.is_empty() {
        None
    } else {
        // Build DAG
        let dag = Dag::from_tickets(tickets.clone()).map_err(|e| match e {
            DagError::MissingDependency {
                ticket_id,
                missing_dep,
            } => format!(
                "Ticket {} depends on {} which does not exist",
                ticket_id, missing_dep
            ),
            DagError::CycleDetected(nodes) => {
                format!("Cycle detected involving: {}", nodes.join(", "))
            }
        })?;

        // Check for cycles
        match dag.detect_cycles() {
            CycleDetectionResult::NoCycle => {}
            CycleDetectionResult::Cycle(nodes) => {
                return Err(format!("Cycle detected involving: {}", nodes.join(", ")));
            }
        }

        let stats = dag.stats();
        let waves = dag
            .execution_waves()
            .map_err(|_| "Failed to compute execution waves".to_string())?;
        let mut ready = dag.get_ready_tickets();
        ready.sort();

        Some(StatusBoard {
            dag,
            stats,
            waves,
            ready,
        })
    };

    let (aggregates, journal_error) =
        match lisa_core::completion_journal::load(&root.join(COMPLETION_JOURNAL_RELATIVE_PATH)) {
            Ok(aggregates) => (aggregates, None),
            Err(error) => (HashMap::new(), Some(error)),
        };
    let unschedulable = board
        .as_ref()
        .map(|board| unschedulable_tickets(&board.ready, &aggregates))
        .unwrap_or_default();
    // A ticket nothing can schedule is not ready, and the counts have to say
    // the same thing the section below them says.
    let board = board.map(|mut board| {
        board.ready.retain(|ticket_id| {
            !unschedulable
                .iter()
                .any(|held| &held.ticket_id == ticket_id)
        });
        board.stats.ready_tickets = board
            .stats
            .ready_tickets
            .saturating_sub(unschedulable.len());
        // Re-derived rather than incremented, by the same subtraction the DAG
        // uses: blocked is whatever is left over, and a ticket already counted
        // as under way must not be counted a second time as blocked.
        board.stats.blocked_tickets = board
            .stats
            .total_tickets
            .saturating_sub(board.stats.done_tickets)
            .saturating_sub(board.stats.ready_tickets)
            .saturating_sub(board.stats.in_progress_tickets);
        board
    });

    let run_summary =
        crate::run_summary::collect_run_summary(root, &tickets, Path::new(&work_dir_rel));

    let ledger = read_ledger(&ledger_path);
    let stranded = stranded_tickets(&tickets, &attempts, &ledger);

    Ok(StatusData {
        ticket_dir_rel,
        resolved,
        completion_seal,
        parked_remedies,
        rechecks,
        notes,
        tickets,
        ledger,
        attempts,
        stranded,
        unschedulable,
        journal_error,
        run,
        board,
        run_summary,
    })
}

/// Print the board for a person. Unchanged by `--json` existing.
fn print_status(data: &StatusData) -> Result<(), String> {
    print_waiting_on_you(&data.parked_remedies, &data.rechecks);
    print_status_notes(&data.notes);

    let Some(board) = &data.board else {
        println!("No tickets found in {}", data.ticket_dir_rel);
        return Ok(());
    };

    // Print summary header
    println!(
        "DAG: {} tickets, {} edges, no cycles",
        board.dag.len(),
        board.dag.edge_count()
    );
    println!(
        "Critical path: {} tickets",
        board.stats.critical_path_length
    );
    println!(
        "Status: {} done, {} in progress, {} ready, {} blocked",
        board.stats.done_tickets,
        board.stats.in_progress_tickets,
        board.stats.ready_tickets,
        board.stats.blocked_tickets
    );
    print!(
        "{}",
        format_config_summary(&data.resolved, data.completion_seal)
    );
    println!();

    // Directly under the counts it qualifies: "2 in progress" on a board where
    // the run that placed those seats is gone is the exact sentence that misled
    // an operator for a day.
    print_abandoned_seats(data);

    // The same qualification from the other side. An abandoned seat is a marker
    // outliving its run; this is a ticket outliving its marker, which is what a
    // scheduler that fenced a live pane leaves behind.
    print_stranded_tickets(data);

    // And the third way those counts can lie: a ticket the DAG calls ready
    // that every scheduling pass declines.
    print_unschedulable_tickets(data);
    if let Some(error) = &data.journal_error {
        println!("Lisa could not read this board's completion journal: {error}");
        println!("  Until that is fixed, the counts above cannot account for finished work Lisa");
        println!("  has not managed to record. Run: lisa doctor");
        println!();
    }

    // And beside it, the other way those counts can lie: a board being worked
    // by more schedulers than anybody started.
    print_contested_board(data);

    print_token_usage(&data.ledger);

    // Print execution waves
    for (i, wave) in board.waves.iter().enumerate() {
        let label = if i == 0 {
            "no dependencies".to_string()
        } else {
            format!("depends on wave {}", i - 1)
        };
        println!("Wave {} ({}):", i, label);

        for id in wave {
            if let Some(ticket) = board.dag.get_ticket(id) {
                let deps = sorted_ids(board.dag.get_dependencies(id));
                let blocks = sorted_ids(board.dag.get_blocked_by(id));

                let deps_str = if deps.is_empty() {
                    String::new()
                } else {
                    format!("  deps: {}", deps.join(", "))
                };

                let blocks_str = if blocks.is_empty() {
                    String::new()
                } else {
                    format!("  blocks: {}", blocks.join(", "))
                };

                println!(
                    "  {:<12}  {:<12}  {:<12}  {}{}{}",
                    ticket.id, ticket.phase, ticket.status, ticket.title, deps_str, blocks_str
                );
            }
        }
        println!();
    }

    // Print ready-to-schedule summary
    if board.ready.is_empty() {
        println!("No tickets ready to schedule.");
    } else {
        println!("Ready to schedule: {}", board.ready.join(", "));
    }

    if let Some(summary) = &data.run_summary {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        crate::run_summary::render_run_summary(summary, &mut output)
            .map_err(|error| format!("Failed to print run summary: {error}"))?;
    }

    Ok(())
}

fn sorted_ids(ids: std::collections::HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = ids.into_iter().collect();
    sorted.sort();
    sorted
}

/// Run the status command: scan tickets, build DAG, print scheduling state.
pub fn run_status(root: &Path) -> Result<(), String> {
    print_status(&collect_status(root)?)
}

/// Run the status command and print one JSON document. Returns the exit code.
pub fn run_status_json(root: &Path) -> i32 {
    crate::json_output::emit_result(
        "status",
        collect_status(root).map(|data| status_payload(&data)),
    )
}

// ---------------------------------------------------------------------------
// The JSON body. Every field below is stated in `lisa json-guide`; adding one
// here without adding it there leaves a shape nobody can find.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct TicketView {
    id: String,
    title: String,
    /// The same spelling the prose prints, e.g. `in_progress`.
    status: String,
    phase: String,
    depends_on: Vec<String>,
    blocks: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WaveView {
    index: usize,
    /// The wave this one waits on, or `null` for the wave with no dependencies.
    depends_on_wave: Option<usize>,
    ticket_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct NoteView {
    ticket_id: String,
    attempt_id: String,
    generation: u64,
    summary: String,
    criterion_quote: String,
    evidence_citation: String,
}

#[derive(Debug, Clone, Serialize)]
struct WaitingView {
    ticket_id: String,
    remedy_owner: RemedyOwner,
    ask: String,
    reason: String,
    steps: Vec<String>,
    check: Option<String>,
    check_timeout_secs: Option<u64>,
    origin: lisa_core::disposition::DispositionOrigin,
    proposal: Option<ProposalView>,
}

#[derive(Debug, Clone, Serialize)]
struct ProposalView {
    summary: String,
    recommendation: String,
}

fn status_payload(data: &StatusData) -> serde_json::Value {
    let (counts, critical_path_length, edge_count) = match &data.board {
        Some(board) => (
            BoardCounts {
                total: board.stats.total_tickets,
                done: board.stats.done_tickets,
                in_progress: board.stats.in_progress_tickets,
                ready: board.stats.ready_tickets,
                blocked: board.stats.blocked_tickets,
            },
            board.stats.critical_path_length,
            board.dag.edge_count(),
        ),
        None => (
            BoardCounts {
                total: 0,
                done: 0,
                in_progress: 0,
                ready: 0,
                blocked: 0,
            },
            0,
            0,
        ),
    };

    let tickets: Vec<TicketView> = data
        .tickets
        .iter()
        .map(|ticket| TicketView {
            id: ticket.id.clone(),
            title: ticket.title.clone(),
            status: ticket.status.to_string(),
            phase: ticket.phase.to_string(),
            depends_on: match &data.board {
                Some(board) => sorted_ids(board.dag.get_dependencies(&ticket.id)),
                None => {
                    let mut deps = ticket.depends_on.clone();
                    deps.sort();
                    deps
                }
            },
            blocks: match &data.board {
                Some(board) => sorted_ids(board.dag.get_blocked_by(&ticket.id)),
                None => Vec::new(),
            },
        })
        .collect();

    let waves: Vec<WaveView> = data
        .board
        .as_ref()
        .map(|board| {
            board
                .waves
                .iter()
                .enumerate()
                .map(|(index, wave)| WaveView {
                    index,
                    depends_on_wave: index.checked_sub(1),
                    ticket_ids: wave.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let notes: Vec<NoteView> = data
        .notes
        .iter()
        .map(|note| NoteView {
            ticket_id: note.key.ticket_id.clone(),
            attempt_id: note.key.attempt_id.clone(),
            generation: note.key.generation,
            summary: note.note.summary().to_string(),
            criterion_quote: note.note.criterion_quote().to_string(),
            evidence_citation: note.note.evidence_citation().to_string(),
        })
        .collect();

    let waiting_on_you: Vec<WaitingView> = data
        .parked_remedies
        .iter()
        .map(|remedy| WaitingView {
            ticket_id: remedy.ticket_id.clone(),
            remedy_owner: remedy.remedy_owner,
            ask: remedy.ask.clone(),
            reason: remedy.reason.clone(),
            steps: remedy.steps.clone(),
            check: remedy.check.clone(),
            check_timeout_secs: remedy.check_timeout_secs,
            origin: remedy.origin,
            proposal: remedy.proposal.as_ref().map(|proposal| ProposalView {
                summary: proposal.summary.clone(),
                recommendation: proposal.recommendation.clone(),
            }),
        })
        .collect();

    let mut phase_timeouts: BTreeMap<String, u64> = BTreeMap::new();
    phase_timeouts.extend(
        data.resolved
            .phase_timeouts
            .iter()
            .map(|(phase, seconds)| (phase.clone(), *seconds)),
    );

    serde_json::json!({
        "ticket_dir": data.ticket_dir_rel,
        "completion_seal": match data.completion_seal {
            CompletionSeal::Commit => "commit",
            CompletionSeal::Journal => "journal",
        },
        "counts": counts,
        "critical_path_length": critical_path_length,
        "edge_count": edge_count,
        "tickets": tickets,
        "waves": waves,
        "ready": data.board.as_ref().map(|board| board.ready.clone()).unwrap_or_default(),
        "notes": notes,
        "waiting_on_you": waiting_on_you,
        "attempts": data.attempts,
        // The complement of `attempts`: tickets the board says are under way
        // that no entry in `attempts` names. Empty on a healthy board.
        "stranded": data.stranded,
        // Tickets removed from `ready` and counted as blocked because no run
        // will take them. Empty on a healthy board.
        "unschedulable": data.unschedulable,
        // Present only when the completion journal could not be folded, in
        // which case `unschedulable` cannot be trusted to be complete.
        "completion_journal_error": data.journal_error,
        // One entry per scheduler stamping this board. A reader counting these
        // is asking the question the shared heartbeat could never answer, so
        // the array is present even when it holds one — and empty on a board
        // whose scheduler predates the registry.
        "schedulers": data.run.schedulers.iter().map(|record| serde_json::json!({
            "id": record.scheduler_id,
            "session_name": record.session_name,
            "zellij_pid": record.zellij_pid,
            "started_at": record.started_at,
            "stamped_at": record.stamped_at,
            "stop_command": record.stop_command(),
        })).collect::<Vec<_>>(),
        // Beside the summary of what a run did, where the run is. A consumer
        // that had to inspect panes, or guess a session name, or `lsof` across
        // a whole desk, was going around the envelope for a fact Lisa held.
        "run_location": data.run.location(),
        "token_usage": token_usage_view(&data.ledger),
        "run_summary": data.run_summary,
        "config": ConfigView {
            max_threads: data.resolved.max_threads,
            session_timeout_secs: data.resolved.session_timeout_secs,
            phase_timeouts,
            client: data.resolved.client.as_str(),
            model: data.resolved.model.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_valid_project(dir: &Path) {
        fs::write(dir.join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.join("docs/active/tickets")).unwrap();
    }

    fn write_ticket(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join("docs/active/tickets").join(filename), content).unwrap();
    }

    fn under_way(id: &str, phase: Phase) -> Ticket {
        let mut ticket = Ticket::new(id, format!("ticket-{id}"));
        ticket.phase = phase;
        ticket
    }

    fn held_seat(ticket_id: &str) -> SeatAttempt {
        SeatAttempt {
            pane_id: 1,
            ticket_id: ticket_id.to_string(),
            attempt_id: 1,
            ticket_phase: Some("review".to_string()),
            superseded: false,
            abandoned: false,
            abandoned_reason: None,
        }
    }

    fn ledger_rows(raw: &str) -> Vec<ProvenanceLedgerRecord> {
        raw.lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// The field failure, read back from `lisa status`: two tickets advanced to
    /// `review`, no seat holding either, and — before this — nothing on the
    /// board that could say why.
    #[test]
    fn a_ticket_under_way_with_no_seat_is_named_and_carries_the_ledgers_reason() {
        let ledger = ledger_rows(concat!(
            r#"{"schema_version":11,"seal":"commit","record_type":"attempt-launch","ticket_id":"T-019-01","attempt_lease":{"ticket_id":"T-019-01","attempt_id":1},"pane_id":1,"provider":"anthropic","assignment":"assignment-1-77.md","occurred_at":1786650742}"#,
            "\n",
            r#"{"schema_version":11,"seal":"commit","ticket_id":"T-019-01","attempt_lease":{"ticket_id":"T-019-01","attempt_id":1},"outcome":"seat-lost","reason":"positive shell readiness was never proven","authoritative":false,"fenced":true,"requested":{"method":"claude","provider":"anthropic","model":null},"actual":{"method":"claude","provider":"anthropic","model":null},"started_at":1786650000,"ended_at":1786650720,"wall_clock_secs":720,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":1,"pane_id":1}"#,
            "\n",
            r#"{"schema_version":11,"seal":"commit","record_type":"attempt-launch","ticket_id":"T-019-03","attempt_lease":{"ticket_id":"T-019-03","attempt_id":2},"pane_id":2,"provider":"anthropic","assignment":"assignment-2-91.md","occurred_at":1786650742}"#,
            "\n",
        ));

        let stranded = stranded_tickets(
            &[
                under_way("T-019-01", Phase::Review),
                under_way("T-019-03", Phase::Review),
                under_way("T-020-01", Phase::Implement),
                under_way("T-021-01", Phase::Ready),
                under_way("T-022-01", Phase::Done),
            ],
            &[held_seat("T-020-01")],
            &ledger,
        );

        let named: Vec<&str> = stranded
            .iter()
            .map(|entry| entry.ticket_id.as_str())
            .collect();
        assert_eq!(
            named,
            vec!["T-019-01", "T-019-03"],
            "a held ticket, a ready one and a finished one are not stranded"
        );

        assert_eq!(stranded[0].attempt_id, Some(1));
        assert!(
            stranded[0].evidence.contains("lost its seat")
                && stranded[0]
                    .evidence
                    .contains("positive shell readiness was never proven"),
            "{}",
            stranded[0].evidence
        );

        assert_eq!(stranded[1].attempt_id, Some(2));
        assert!(
            stranded[1]
                .evidence
                .contains("nothing has recorded how it ended"),
            "a launch with no answer is itself the finding: {}",
            stranded[1].evidence
        );

        let lines = stranded_ticket_lines(&stranded).join("\n");
        assert!(lines.contains("Tickets nobody is working"));
        assert!(lines.contains("lisa reset-ticket <ticket-id> --apply"));
    }

    /// The pre-schema-11 board, which is the one the bug was found on: an
    /// operator still gets a named state, and the sentence says the ledger is
    /// the thing that is empty.
    #[test]
    fn a_stranded_ticket_with_no_ledger_row_at_all_still_says_so() {
        let stranded = stranded_tickets(&[under_way("T-019-01", Phase::Review)], &[], &[]);

        assert_eq!(stranded.len(), 1);
        assert_eq!(stranded[0].attempt_id, None);
        assert_eq!(
            stranded[0].evidence,
            "the ledger has no record of an attempt on this ticket"
        );
    }

    /// Silence is the healthy board's report. Nothing to say, nothing printed.
    #[test]
    fn a_board_whose_seats_all_hold_prints_nothing_about_stranded_tickets() {
        let stranded = stranded_tickets(
            &[
                under_way("T-020-01", Phase::Implement),
                under_way("T-022-01", Phase::Done),
            ],
            &[held_seat("T-020-01")],
            &[],
        );

        assert!(stranded.is_empty());
        assert!(stranded_ticket_lines(&stranded).is_empty());
    }

    /// The two token gaps are different facts and must not be summed into one.
    #[test]
    fn tokens_lost_with_a_seat_are_counted_apart_from_a_late_capture() {
        let ledger = ledger_rows(concat!(
            r#"{"schema_version":11,"seal":"commit","ticket_id":"T-019-01","attempt_lease":{"ticket_id":"T-019-01","attempt_id":1},"outcome":"seat-lost","reason":"pane fenced","authoritative":false,"fenced":true,"requested":{"method":"claude","provider":"anthropic","model":null},"actual":{"method":"claude","provider":"anthropic","model":null},"started_at":1786650000,"ended_at":1786650720,"wall_clock_secs":720,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":1,"pane_id":1}"#,
            "\n",
            r#"{"schema_version":11,"seal":"commit","ticket_id":"T-018-01","attempt_lease":{"ticket_id":"T-018-01","attempt_id":1},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"claude","provider":"anthropic","model":null},"actual":{"method":"claude","provider":"anthropic","model":null},"started_at":1786640000,"ended_at":1786640600,"wall_clock_secs":600,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":1,"pane_id":2}"#,
            "\n",
        ));

        let view = token_usage_view(&ledger);
        assert_eq!(view.lost_with_the_seat, vec!["T-019-01".to_string()]);
        assert_eq!(view.not_yet_joined, vec!["T-018-01".to_string()]);

        let lines = token_usage_lines(&ledger).join("\n");
        assert!(lines.contains("Lost with the seat: 1 ticket"));
        assert!(lines.contains("not recoverable"));
        assert!(lines.contains("Not yet joined: 1 completed ticket"));
    }

    #[test]
    fn test_status_no_tickets() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_single_ticket() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: first-ticket\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\n## Context\nTest.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn status_completion_fixtures_show_both_plain_language_tiers() {
        let resolved = config::ResolvedConfig::default();
        let commit = format_config_summary(&resolved, CompletionSeal::Commit);
        assert!(commit.contains("completion seal: commit-sealed — finished work lands as history"));

        let journal = format_config_summary(&resolved, CompletionSeal::Journal);
        assert!(journal.contains(
            "completion seal: journal-only — finished work is recorded but not undoable"
        ));
        assert!(!journal.to_ascii_lowercase().contains("git"));
    }

    #[test]
    fn test_status_dependency_chain() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone.\n",
        );
        write_ticket(
            dir.path(),
            "T-002.md",
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nReady.\n",
        );
        write_ticket(
            dir.path(),
            "T-003.md",
            "---\nid: T-003\ntitle: third\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-002]\n---\n\nBlocked.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_cycle_error() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-002]\n---\n\nA.\n",
        );
        write_ticket(
            dir.path(),
            "T-002.md",
            "---\nid: T-002\ntitle: b\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nB.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_status_missing_dep_error() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-999]\n---\n\nA.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_status_missing_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Don't create the ticket directory

        let result = run_status(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_status_respects_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("custom/tickets")).unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[dirs]\ntickets = \"custom/tickets\"\n",
        )
        .unwrap();

        // Write ticket to the custom directory
        fs::write(
            dir.path().join("custom/tickets/T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nTest.\n",
        )
        .unwrap();

        // Should find the ticket in the custom directory
        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn waiting_lines_preserve_operator_ask_and_explain_world_waiting() {
        let remedies = vec![
            ParkedRemedy {
                ticket_id: "T-001".to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: "Run the checkout test exactly once.".to_string(),
                reason: "The checkout evidence is missing.".to_string(),
                steps: Vec::new(),
                check: None,
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
            ParkedRemedy {
                ticket_id: "T-002".to_string(),
                remedy_owner: RemedyOwner::World,
                ask: "Wait for the release link.".to_string(),
                reason: "The release has not reached the mirror.".to_string(),
                steps: Vec::new(),
                check: Some("test -f release".to_string()),
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
            ParkedRemedy {
                ticket_id: "T-003".to_string(),
                remedy_owner: RemedyOwner::Agent,
                ask: "Agent retry exhausted.".to_string(),
                reason: "The agent can retry this work.".to_string(),
                steps: Vec::new(),
                check: None,
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
        ];

        assert_eq!(
            waiting_on_you_lines(&remedies, &HashMap::new()),
            vec![
                "T-001  Run the checkout test exactly once.",
                "       Reviewer's note: The checkout evidence is missing.",
                "T-002  Wait for the release link. — Lisa checks on its own.",
                "       Reviewer's note: The release has not reached the mirror.",
            ]
        );
    }

    /// A world remedy whose check keeps saying no stops being silent — but only
    /// once it has said so enough times to mean something.
    #[test]
    fn a_world_remedy_that_never_clears_says_so_and_names_the_way_through() {
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-WORLD".to_string(),
            remedy_owner: RemedyOwner::World,
            ask: "Wait for the release link.".to_string(),
            reason: "The release has not reached the mirror.".to_string(),
            steps: Vec::new(),
            check: Some("curl -fsS https://example.test/release".to_string()),
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let observation = |count, check: &str| {
            HashMap::from([(
                "T-WORLD".to_string(),
                WorldRecheckObservation {
                    check: check.to_string(),
                    result: lisa_core::provenance::WorldRecheckOutcome::Failed,
                    non_pass_count: count,
                    occurred_at: 1_752_900_000,
                },
            )])
        };
        let quiet = vec![
            "T-WORLD  Wait for the release link. — Lisa checks on its own.".to_string(),
            "       Reviewer's note: The release has not reached the mirror.".to_string(),
        ];

        // Below the threshold, and for a check the count does not belong to,
        // nothing changes: waiting for the world is what a world remedy does.
        assert_eq!(
            waiting_on_you_lines(&remedies, &HashMap::new()),
            quiet,
            "no observation"
        );
        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(
                    STUCK_NON_PASS_COUNT - 1,
                    "curl -fsS https://example.test/release"
                )
            ),
            quiet,
            "under the threshold"
        );
        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(STUCK_NON_PASS_COUNT, "test -f some-older-check")
            ),
            quiet,
            "a count belonging to a different check"
        );

        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(
                    STUCK_NON_PASS_COUNT,
                    "curl -fsS https://example.test/release"
                )
            ),
            vec![
                "T-WORLD  Wait for the release link. — Lisa checks on its own.",
                "       Lisa has checked at least 8 times and it still isn't passing.",
                "       If you have checked this yourself, run: lisa unblock T-WORLD --override-check",
                "       Reviewer's note: The release has not reached the mirror.",
            ]
        );
    }

    /// The count is a world remedy's fact. An operator-owned park is theirs to
    /// clear, and Lisa never rechecks one, so it never accumulates a count.
    #[test]
    fn an_operator_owned_remedy_never_shows_a_recheck_count() {
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-OPERATOR".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: "Run the checkout test.".to_string(),
            reason: "The checkout evidence is missing.".to_string(),
            steps: Vec::new(),
            check: Some("test -f evidence".to_string()),
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let rechecks = HashMap::from([(
            "T-OPERATOR".to_string(),
            WorldRecheckObservation {
                check: "test -f evidence".to_string(),
                result: lisa_core::provenance::WorldRecheckOutcome::Failed,
                non_pass_count: 64,
                occurred_at: 1_752_900_000,
            },
        )]);

        assert_eq!(
            waiting_on_you_lines(&remedies, &rechecks),
            vec![
                "T-OPERATOR  Run the checkout test.",
                "       Reviewer's note: The checkout evidence is missing.",
            ]
        );
    }

    #[test]
    fn legacy_field_block_leads_with_the_standard_plain_ask() {
        const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-046-06-03".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
            reason: FIELD_REASON.to_string(),
            steps: Vec::new(),
            check: None,
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];

        let lines = waiting_on_you_lines(&remedies, &HashMap::new());

        assert_eq!(
            lines,
            vec![
                format!("T-046-06-03  {}", lisa_core::parking::LEGACY_BLOCK_ASK),
                format!("       Reviewer's note: {FIELD_REASON}"),
            ]
        );
        assert!(!lines[0].contains(FIELD_REASON));
    }

    #[test]
    fn field_proposal_leads_with_gap_amendment_and_prepared_edit() {
        use lisa_core::triage::{PreparedStep, TriageProposal};

        let remedies = vec![ParkedRemedy {
            ticket_id: "T-046-06-03".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
            reason: "The 225 MiB measurement conflicts with the approximately 200 MiB criterion."
                .to_string(),
            steps: Vec::new(),
            check: None,
            check_timeout_secs: None,
            proposal: Some(TriageProposal {
                summary: "The written criteria conflict with the measured evidence.".to_string(),
                recommendation: "Amend the stale criteria.".to_string(),
                prepared_steps: vec![PreparedStep::FileEdit {
                    description: "Use the calibrated bound.".to_string(),
                    path: std::path::PathBuf::from("docs/active/tickets/T-046-06-03.md"),
                    old: "approximately 200 MiB".to_string(),
                    new: "the calibrated 300 MiB bound".to_string(),
                }],
            }),
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let lines = waiting_on_you_lines(&remedies, &HashMap::new());
        assert!(lines[0].contains("First responder"));
        assert!(lines[0].contains("criteria"));
        assert!(lines[0].contains("evidence"));
        assert!(lines[1].contains("Suggested: Amend"));
        assert!(lines[2].contains("Prepared:"));
        assert!(lines[3].contains("Original ask:"));
        assert!(lines[4].contains("Reviewer's note:"));
    }

    fn parse_ledger(jsonl: &str) -> Vec<ProvenanceLedgerRecord> {
        jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    // A null-token Done row and a matching usage-correction, as written to the
    // ledger by the plugin.
    fn null_done(ticket: &str) -> String {
        format!(
            r#"{{"schema_version":9,"seal":"commit","ticket_id":"{ticket}","attempt_lease":{{"ticket_id":"{ticket}","attempt_id":1}},"outcome":"done","authoritative":true,"fenced":false,"requested":{{"method":"codex","provider":"openai","model":null}},"actual":{{"method":"codex","provider":"openai","model":null}},"started_at":1000,"ended_at":1100,"wall_clock_secs":100,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":0,"pane_id":1}}"#
        )
    }

    fn correction(ticket: &str, tokens_in: u64, tokens_out: u64) -> String {
        format!(
            r#"{{"schema_version":9,"record_type":"usage-correction","ticket_id":"{ticket}","attempt_lease":{{"ticket_id":"{ticket}","attempt_id":1}},"method":"codex","session_id":"s","pane_id":1,"source_line":1,"captured_at":1150,"tokens_in":{tokens_in},"tokens_out":{tokens_out},"occurred_at":1200}}"#
        )
    }

    #[test]
    fn token_usage_reads_the_corrected_view_not_the_raw_row() {
        let ledger = format!(
            "{}\n{}\n",
            null_done("T-A"),
            correction("T-A", 1_234_567, 8_900)
        );
        let lines = token_usage_lines(&parse_ledger(&ledger));
        assert_eq!(lines[0], "Token usage");
        // The raw row is null; the corrected total comes from the correction,
        // rendered with thousands separators.
        assert!(lines.iter().any(|line| line.contains("T-A")
            && line.contains("1,234,567 in")
            && line.contains("8,900 out")));
        assert!(lines.iter().any(|line| line.contains("Joined 1 ticket:")));
        assert!(!lines.iter().any(|line| line.contains("Not yet joined")));
    }

    #[test]
    fn token_usage_counts_the_capture_never_gap() {
        // T-A joined; T-B completed but never joined a capture.
        let ledger = format!(
            "{}\n{}\n{}\n",
            null_done("T-A"),
            correction("T-A", 100, 10),
            null_done("T-B"),
        );
        let lines = token_usage_lines(&parse_ledger(&ledger));
        assert!(lines.iter().any(|line| line.contains("Joined 1 ticket:")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Not yet joined: 1 completed ticket")));
        // The gap ticket is never printed with a fabricated zero.
        assert!(!lines
            .iter()
            .any(|line| line.contains("T-B") && line.contains("0 in")));
    }

    fn seat(pane_id: u32, ticket_id: &str, phase: &str, abandoned: bool) -> SeatAttempt {
        SeatAttempt {
            pane_id,
            ticket_id: ticket_id.to_string(),
            attempt_id: 1,
            ticket_phase: Some(phase.to_string()),
            superseded: false,
            abandoned,
            abandoned_reason: abandoned.then(|| "the run stopped 18h ago.".to_string()),
        }
    }

    fn ended_run() -> crate::seats::RunReport {
        crate::seats::RunReport {
            liveness: crate::seats::RunLiveness::Ended,
            evidence: "the run stopped 18h ago.".to_string(),
            schedulers: Vec::new(),
            stamped: false,
        }
    }

    /// The section exists because "2 in progress" on a board where nothing is
    /// running is the sentence that misled an operator for a day. It names the
    /// seats, the evidence, and the way out.
    #[test]
    fn abandoned_seats_are_named_with_their_evidence_and_the_command() {
        let attempts = vec![
            seat(2, "T-008-06", "review", true),
            seat(3, "T-008-07", "done", true),
        ];

        let lines = abandoned_seat_lines(&attempts, &ended_run());

        assert_eq!(lines[0], "Seats held by a run that is gone");
        assert!(lines[1].contains("pane 2") && lines[1].contains("T-008-06"));
        assert!(lines[2].contains("pane 3") && lines[2].contains("T-008-07"));
        assert!(lines[3].contains("the run stopped 18h ago."));
        assert_eq!(
            lines[4],
            "  To free them, read the list first: lisa release-seats"
        );
    }

    fn stamping_run(sessions: &[&str]) -> crate::seats::RunReport {
        crate::seats::RunReport {
            liveness: crate::seats::RunLiveness::Stamping,
            evidence: "2 schedulers are stamping this board.".to_string(),
            schedulers: sessions
                .iter()
                .map(|session| {
                    lisa_core::schedulers::SchedulerRecord::new(
                        format!("{session}-id"),
                        Some((*session).to_string()),
                        Some(9450),
                        1_000,
                        2_000,
                        5,
                    )
                })
                .collect(),
            stamped: true,
        }
    }

    /// The line `lisa status` could not print on the day: how many schedulers
    /// are on this board, which, and how to stop each one.
    #[test]
    fn a_board_with_more_than_one_scheduler_names_every_one_of_them() {
        let lines =
            contested_board_lines(&stamping_run(&["blossoming-cymbal", "fascinating-drum"]));

        assert_eq!(lines[0], "2 are running on this board");
        assert!(lines[1].contains("blossoming-cymbal"));
        assert!(lines[1].contains("zellij kill-session blossoming-cymbal"));
        assert!(lines[2].contains("fascinating-drum"));
        assert!(lines.last().unwrap().contains("lisa schedulers"));
    }

    /// One scheduler is what a run looks like, and a report that fires on the
    /// healthy case is a report nobody reads.
    #[test]
    fn one_scheduler_prints_no_section_at_all() {
        assert!(contested_board_lines(&stamping_run(&["lisa"])).is_empty());
        assert!(contested_board_lines(&ended_run()).is_empty());
    }

    /// A healthy board says nothing at all. The counts above it already
    /// describe one, and a fault report that fires when there is no fault
    /// teaches an operator to skip it.
    #[test]
    fn a_board_with_no_abandoned_seat_prints_no_section() {
        let attempts = vec![seat(0, "T-008-08", "implement", false)];
        assert!(abandoned_seat_lines(&attempts, &ended_run()).is_empty());
        assert!(abandoned_seat_lines(&[], &ended_run()).is_empty());
    }

    /// `attempts[]` has to carry the same truth the prose does — a consumer
    /// reading the documented contract must not be the last to know.
    #[test]
    fn the_json_entry_carries_the_verdict_and_its_reason() {
        let live = serde_json::to_value(seat(0, "T-008-08", "implement", false)).unwrap();
        assert_eq!(live["abandoned"], serde_json::json!(false));
        assert_eq!(live["abandoned_reason"], serde_json::Value::Null);

        let gone = serde_json::to_value(seat(2, "T-008-06", "review", true)).unwrap();
        assert_eq!(gone["abandoned"], serde_json::json!(true));
        assert_eq!(gone["abandoned_reason"], "the run stopped 18h ago.");
        // The fields a consumer already reads keep their meaning.
        assert_eq!(gone["superseded"], serde_json::json!(false));
        assert_eq!(gone["ticket_phase"], "review");
    }

    /// The verdict belongs to the run, so a lease Lisa cannot match to a ticket
    /// still gets it — an entry left out of the diagnosis is one an operator
    /// cannot act on.
    #[test]
    fn every_marker_from_an_ended_run_is_marked_abandoned() {
        let dir = tempfile::tempdir().unwrap();
        let signals = dir.path().join("signals");
        fs::create_dir_all(&signals).unwrap();
        fs::write(
            signals.join("pane-0.lease"),
            r#"{"ticket_id":"T-001","attempt_id":1}"#,
        )
        .unwrap();
        fs::write(
            signals.join("pane-1.lease"),
            r#"{"ticket_id":"T-GONE","attempt_id":4}"#,
        )
        .unwrap();

        let ticket_path = dir.path().join("T-001.md");
        fs::write(
            &ticket_path,
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nA.\n",
        )
        .unwrap();
        let tickets = vec![lisa_core::ticket::parse_ticket(&ticket_path).unwrap()];

        let attempts = read_seat_attempts(&signals, &tickets, &ended_run());
        assert!(attempts.iter().all(|attempt| attempt.abandoned));
        assert_eq!(attempts[1].ticket_phase, None);
        assert!(attempts[1].abandoned_reason.is_some());
    }

    #[test]
    fn token_usage_empty_ledger_says_nothing_yet() {
        let lines = token_usage_lines(&[]);
        assert_eq!(
            lines,
            vec![
                "Token usage".to_string(),
                "  Nothing measured yet.".to_string()
            ]
        );
    }
}
