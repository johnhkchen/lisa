//! Two attempts, twelve minutes, and a ledger that said the day contained
//! neither.
//!
//! On 2026-08-13 `T-019-01` and `T-019-03` were each assigned twice on
//! `screen-design`. Both agents worked. Startup was never observed on either
//! pane, recovery gave up, and both panes were fenced. What the desk could read
//! afterwards was this:
//!
//! ```text
//! grep 'T-019-01' .lisa/provenance.jsonl         → nothing
//! grep 'T-019-01' .lisa/completion-journal.jsonl → nothing
//! ```
//!
//! Not a failure row, not a timeout row, nothing — beside two tickets sitting
//! at `phase: review` with no seat holding them. Every tool that answers *what
//! happened here* reads those files, so two agent sessions of real tokens and
//! real wall clock were spent and nothing on the desk could say so.
//!
//! What these tests pin:
//!
//! 1. An attempt that was launched leaves a row, written between the assignment
//!    file becoming real and the pane hearing about it. The ledger cannot
//!    disagree with the existence of that file.
//! 2. A seat that is lost is recorded as lost, under its own name, with the
//!    scheduler's reason — and the row is durable before the fence destroys
//!    everything it is built from.
//! 3. Both rows name the exact attempt, so the grep above answers.

use super::*;

use lisa_core::provenance::{ProvenanceLedgerRecord, RunOutcome};

const TICKET: &str = "T-NAME";
const PANE: u32 = 10;

/// A scheduled board whose ledger is a real file, so the rows are readable
/// rather than merely attempted.
fn dispatched_board() -> (State, tempfile::TempDir) {
    let (mut state, dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
    state.ledger_path = dir.path().join(".lisa/provenance.jsonl");
    state.schedule_ready_tickets();
    (state, dir)
}

fn ledger_rows(state: &State) -> Vec<ProvenanceLedgerRecord> {
    std::fs::read_to_string(&state.ledger_path)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("unreadable ledger row {line:?}: {error}"))
        })
        .collect()
}

/// Every assignment file published under the attempt root, by file name.
fn published_assignments(state: &State) -> Vec<String> {
    fn walk(dir: &std::path::Path, found: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                if name.starts_with("assignment-") {
                    found.push(name.to_string());
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(&state.attempt_root(), &mut found);
    found.sort();
    found
}

/// The first half of the invariant: the file exists, so the row does.
///
/// The assignment file is the moment an attempt becomes real — it is published
/// atomically, before any provider lifecycle input, and it is the exact bytes
/// the agent is told to read. Nothing may be able to produce one without the
/// ledger saying so.
#[test]
fn a_launched_attempt_leaves_a_row_naming_the_assignment_file_it_was_given() {
    let (state, _dir) = dispatched_board();

    let lease = state.current_leases[TICKET].clone();
    let assignment = state.assignment_refs[TICKET].clone();
    assert!(
        assignment.path.is_file(),
        "the attempt was dispatched with a published assignment"
    );

    let launches: Vec<_> = ledger_rows(&state)
        .into_iter()
        .filter_map(|row| match row {
            ProvenanceLedgerRecord::AttemptLaunch(launch) => Some(launch),
            _ => None,
        })
        .collect();

    assert_eq!(launches.len(), 1, "one launch, one row");
    assert_eq!(launches[0].ticket_id, TICKET);
    assert_eq!(launches[0].attempt_lease, lease);
    assert_eq!(launches[0].pane_id, PANE);
    assert_eq!(launches[0].provider, "anthropic");
    assert_eq!(
        launches[0].assignment,
        assignment.path.file_name().unwrap().to_string_lossy(),
        "the row names the exact file the agent was handed"
    );

    // Stated as the property rather than the instance: the ledger cannot
    // disagree with what is on disk.
    let mut named: Vec<String> = launches
        .iter()
        .map(|launch| launch.assignment.clone())
        .collect();
    named.sort();
    assert_eq!(
        named,
        published_assignments(&state),
        "every published assignment is named by a launch row"
    );
}

/// The second half: the pane dies mid-attempt and the ledger says so.
///
/// The seat here has already spent its one same-pane relaunch, which is the
/// shape recovery is in when it gives up — the pane is fenced, the lease is
/// revoked, the marker is withdrawn, and the ticket keeps whatever phase its
/// agent had already reached. That is precisely the state that produced two
/// tickets nobody was working and nothing to read about it.
#[test]
fn a_seat_lost_mid_attempt_is_recorded_as_lost_with_the_reason() {
    let (mut state, _dir) = dispatched_board();
    let lease = state.current_leases[TICKET].clone();

    state.seat_assignments.insert(
        PANE,
        SeatAssignmentState::Starting {
            generation: lease.attempt_id,
            start_deadline: None,
            relaunches: MAX_SAME_PANE_STARTUP_RELAUNCHES,
        },
    );

    let reason = "startup was never observed and the pane proved nothing";
    let outcome = state.fail_startup_recovery(PANE, reason);

    assert_eq!(
        outcome,
        Some(FailureTransitionOutcome::StartupRecoveryFailed {
            pane_id: PANE,
            ticket_id: TICKET.to_string(),
        })
    );
    // This really is the pane-death path: the seat is fenced and its lease is
    // gone, which is what leaves the board with a ticket and no seat.
    assert_eq!(
        state.agent_slots[0].transition_state,
        TransitionState::Fenced
    );
    assert_eq!(state.current_leases.get(TICKET), None);

    let terminal: Vec<_> = ledger_rows(&state)
        .into_iter()
        .filter_map(|row| match row {
            ProvenanceLedgerRecord::Execution(exec) => Some(exec),
            _ => None,
        })
        .collect();

    assert_eq!(terminal.len(), 1, "one lost seat, one terminal row");
    let record = &terminal[0];
    assert_eq!(record.ticket_id, TICKET);
    assert_eq!(
        record.attempt_lease, lease,
        "the row names the exact attempt"
    );
    assert_eq!(
        record.outcome,
        RunOutcome::SeatLost,
        "not `failed` and not `timed-out`: the work said nothing wrong and outran nothing"
    );
    assert_eq!(record.reason.as_deref(), Some(reason));
    assert!(!record.authoritative);
    assert!(record.fenced);
    assert_eq!(record.pane_id, PANE);
    // Never fabricated: the session died before it could report what it spent.
    assert_eq!(record.tokens_in, None);
    assert_eq!(record.tokens_out, None);
}

/// The check an operator actually ran, and the answer it used to give.
#[test]
fn grepping_the_ledger_for_the_ticket_no_longer_comes_back_empty() {
    let (mut state, _dir) = dispatched_board();
    let lease = state.current_leases[TICKET].clone();
    state.seat_assignments.insert(
        PANE,
        SeatAssignmentState::Starting {
            generation: lease.attempt_id,
            start_deadline: None,
            relaunches: MAX_SAME_PANE_STARTUP_RELAUNCHES,
        },
    );
    state.fail_startup_recovery(PANE, "the pane went away mid-attempt");

    let raw = std::fs::read_to_string(&state.ledger_path).unwrap();
    let naming: Vec<&str> = raw.lines().filter(|line| line.contains(TICKET)).collect();

    assert_eq!(
        naming.len(),
        2,
        "the attempt and its end, both named: {raw}"
    );
    assert!(naming[0].contains("\"record_type\":\"attempt-launch\""));
    assert!(naming[1].contains("\"outcome\":\"seat-lost\""));
    assert!(
        naming[1].contains("\"reason\":\"the pane went away mid-attempt\""),
        "a lost seat says why: {}",
        naming[1]
    );
}
