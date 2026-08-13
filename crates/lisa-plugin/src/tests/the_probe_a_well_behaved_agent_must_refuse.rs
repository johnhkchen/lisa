//! The refusal, reproduced: a live agent that will not write a lisa signal, and
//! a seat that survives it.
//!
//! On 2026-08-13 startup went unobserved on two `screen-design` panes whose
//! agents were twelve minutes into their attempts. Recovery typed a shell
//! readiness probe into each of them — a `printf … > .lisa/signals/pane-N.
//! shell-ready` for the pane's occupant to run. From the agent's own transcript:
//!
//! ```text
//! 19:45:36  user       command printf '%s' '{"ticket_id":"T-019-03","attempt_id":2}' \
//!                      > '.lisa/signals/pane-1.shell-ready.tmp.2-…' && command mv …
//!
//! 19:45:49  assistant  That's lisa's attempt-2 ready signal, not something I should write —
//!                      rail and its agents never write into `.lisa/signals/`; lisa owns that
//!                      file. Running it from here would tell the scheduler a pane is ready
//!                      when it isn't.
//! ```
//!
//! **The agent was right**, and every project that teaches its agents not to
//! write under `.lisa/` is a project where that recovery always failed. No
//! `.shell-ready` arrived, the window ran out, the seat was fenced and the pane
//! was closed with a working session inside it. The other outcome is no better:
//! a session that *runs* the probe forges a readiness proof for a pane that is
//! not ready.
//!
//! What these tests pin, in the terms `S-067-01` set:
//!
//! 1. Nothing is typed at a pane whose occupant is unknown, so there is no
//!    probe to refuse and no forged proof to admit.
//! 2. The refusal is *read*. Answering in prose fires the agent's own
//!    `UserPromptSubmit` and `Stop` hooks, so `.ack` and `.stopped` land in
//!    Lisa's signal directory whatever the agent decides to say — and those are
//!    what end the window, name themselves in the activity feed, and route the
//!    seat to the exit retry that treats the pane as occupied.
//! 3. The seat survives it. No fence, no closed pane, no silence.

use super::*;

use std::fs;

const TICKET: &str = "T-REFUSED";
const PANE: u32 = 1;

/// The field reading exactly: a seat whose acknowledgment deadline has passed
/// with a live agent in the pane that has proved nothing since Lisa typed.
///
/// Residency is retired by Lisa's own keystroke (`send_line_to_pane`), so a
/// session that received the launch line and has not yet run a hook is occupied
/// and unable to say so — which is indistinguishable, from here, from a pane
/// where nothing ever started.
fn unobserved_startup_on_an_unproven_pane() -> (State, AttemptLease, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let signal_dir = dir.path().join("signals");
    fs::create_dir_all(&signal_dir).unwrap();
    let (mut state, lease) = resident_pane_fixture(&signal_dir, TICKET);
    state.threads.insert(
        TICKET.to_string(),
        lisa_core::types::Thread::new(TICKET, PANE),
    );
    state
        .threads
        .get_mut(TICKET)
        .unwrap()
        .attempt_lease
        .replace(lease.clone());
    state.write_pane_lease_marker(PANE, &lease).unwrap();
    assert!(
        !state.provider_is_resident(PANE),
        "the pane has proved nothing, which is the reading that used to route it to the probe"
    );
    (state, lease, dir)
}

/// The whole incident, end to end, with the outcome inverted.
#[test]
fn a_live_agent_that_refuses_the_probe_keeps_its_seat() {
    let (mut state, lease, dir) = unobserved_startup_on_an_unproven_pane();
    let now = std::time::SystemTime::now();

    // The deadline passes. Nothing is typed: no Ctrl-C, no probe, no keystroke
    // of any kind, so the agent is never asked to write into `.lisa/`.
    let enters_before = state.pending_enters.len();
    assert!(state.check_assignment_ack_timeouts_at(now).is_empty());
    assert_eq!(
        state.pending_enters.len(),
        enters_before,
        "a pane whose occupant is unknown is not typed at"
    );
    assert!(matches!(
        state.seat_assignment(PANE),
        Some(SeatAssignmentState::ObservingStartup { generation, .. })
            if generation == lease.attempt_id
    ));
    assert_eq!(
        state.current_leases[TICKET], lease,
        "the launched attempt is held open, so the process it named can still announce itself"
    );
    assert_eq!(
        state.agent_slots[0].transition_state,
        TransitionState::Idle,
        "the seat is not fenced and the pane is not closed"
    );
    assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some(TICKET));
    assert_eq!(state.error_alerts, Vec::<(String, u32)>::new());

    // The agent reads the launch line as a message and answers it in prose,
    // declining to write the signal. Refusing is still a turn: submitting the
    // prompt fires UserPromptSubmit and ending the turn fires Stop, so its own
    // hooks report it here whatever it decided to say.
    fs::write(
        state.signal_dir.join(format!("pane-{PANE}.ack")),
        "{\"prompt\":\"command printf '%s' '{}' > '.lisa/signals/pane-1.shell-ready.tmp.2-1'\"}",
    )
    .unwrap();
    fs::write(
        state.signal_dir.join(format!("pane-{PANE}.stopped")),
        "2026-08-13T19:45:49Z",
    )
    .unwrap();
    state.check_codex_ack_signals();
    state.check_transition_signals();
    assert!(state.pane_shows_an_occupant(PANE));

    let outcomes = state.check_assignment_ack_timeouts_at(now);

    // The seat is alive. It is not fenced, the pane is not closed, and the
    // thread still holds its ticket.
    assert!(outcomes.is_empty(), "no failure transition: {outcomes:?}");
    assert_ne!(
        state.seat_assignment(PANE),
        Some(SeatAssignmentState::StartupFailed),
        "the recovery must not fence a pane with a live agent in it"
    );
    assert_ne!(
        state.agent_slots[0].transition_state,
        TransitionState::Fenced
    );
    assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some(TICKET));
    assert_eq!(state.error_alerts, Vec::<(String, u32)>::new());
    assert_ne!(
        state.threads[TICKET].status,
        lisa_core::types::ThreadStatus::Failed
    );

    // And it took the treatment a pane with a provider in it gets: the exit is
    // re-sent, and the residency-aware exit policy waits for the session to
    // actually leave.
    assert_eq!(
        state.agent_slots[0].transition_state,
        TransitionState::WaitingForExit
    );
    let successor = state.current_leases[TICKET].clone();
    assert_eq!(successor.attempt_id, lease.attempt_id + 1);
    assert!(matches!(
        state.seat_assignment(PANE),
        Some(SeatAssignmentState::Starting {
            generation,
            relaunches: MAX_SAME_PANE_STARTUP_RELAUNCHES,
            ..
        }) if generation == successor.attempt_id
    ));

    // Not silently, either. The refusal is a fact Lisa read from its own signal
    // stream, and the line says which file carried it.
    assert!(
        state.activity_events().any(|event| matches!(
            event,
            ActivityEvent::Warning { message }
                if message.contains("answered instead of starting")
                    && message.contains(".stopped")
                    && message.contains(TICKET)
        )),
        "the pane's answer must be reported, naming the signal that carried it: {:?}",
        state.activity_events().collect::<Vec<_>>()
    );

    drop(dir);
}

/// The measured shape of the incident: an agent twelve minutes into its
/// attempt, producing work, whose `.started` this scheduler never saw.
///
/// A pane like that never reaches the observation window at all, because it is
/// never quiet: `on-heartbeat.sh` publishes `pane-<id>.heartbeat` on every tool
/// call, and since T-058-01-01 it publishes only when the calling process's own
/// immutable launch identity byte-matches `pane-<id>.lease`. So the heartbeat is
/// a second `.started` — the same claim, the same authentication — and it must
/// end the startup wait with the seat intact rather than re-exiting a session
/// that is doing the work.
#[test]
fn a_heartbeating_agent_keeps_the_seat_its_start_signal_never_claimed() {
    let (mut state, lease, _dir) = unobserved_startup_on_an_unproven_pane();
    let mut thread = lisa_core::types::Thread::new(TICKET, PANE);
    thread.attempt_lease = Some(lease.clone());
    state.threads.insert(TICKET.to_string(), thread);

    // Twelve minutes of tool calls, and not one `.started` among them.
    fs::write(
        state.signal_dir.join(format!("pane-{PANE}.heartbeat")),
        serde_json::to_string(&lease).unwrap(),
    )
    .unwrap();
    state.check_heartbeat_signals();

    assert_eq!(
        state.seat_assignment(PANE),
        Some(SeatAssignmentState::ReadyForAssignment {
            generation: lease.attempt_id
        }),
        "an authenticated heartbeat asserts everything `.started` asserts, and more"
    );
    assert!(state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Info { message }
            if message.contains("is working on attempt") && message.contains("heartbeat proves it")
    )));

    // The deadline that used to take the seat now finds nothing to recover.
    assert!(state
        .check_assignment_ack_timeouts_at(std::time::SystemTime::now())
        .is_empty());
    assert_eq!(
        state.current_leases[TICKET], lease,
        "no successor is minted over a process that is demonstrably running"
    );
    assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
    assert_ne!(
        state.seat_assignment(PANE),
        Some(SeatAssignmentState::StartupFailed)
    );
    assert_eq!(state.error_alerts, Vec::<(String, u32)>::new());
}

/// The counterfactual, so the fix cannot silently regress into the old shape:
/// the seat is never resettable on unobserved startup alone.
///
/// A window that is merely open is not a window that has answered. Ticking it
/// repeatedly before its deadline must mint nothing, type nothing and fence
/// nothing — `refuse-rather-than-guess`, in the direction of waiting.
#[test]
fn an_open_window_decides_nothing_however_often_it_is_ticked() {
    let (mut state, lease, _dir) = unobserved_startup_on_an_unproven_pane();
    let now = std::time::SystemTime::now();
    state.check_assignment_ack_timeouts_at(now);

    let observe_deadline = match state.seat_assignment(PANE) {
        Some(SeatAssignmentState::ObservingStartup {
            observe_deadline, ..
        }) => observe_deadline,
        other => panic!("expected a startup observation window, got {other:?}"),
    };
    let enters = state.pending_enters.len();

    for tick in 0..5 {
        assert!(state
            .check_assignment_ack_timeouts_at(
                observe_deadline - std::time::Duration::from_secs(5 - tick),
            )
            .is_empty());
        assert!(matches!(
            state.seat_assignment(PANE),
            Some(SeatAssignmentState::ObservingStartup { .. })
        ));
        assert_eq!(state.current_leases[TICKET], lease, "tick {tick} minted");
        assert_eq!(state.pending_enters.len(), enters, "tick {tick} typed");
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Idle,
            "tick {tick} moved the pane"
        );
    }

    // The marker still names the launched attempt the whole way through, which
    // is what keeps the window answerable by the process it was opened for.
    let marker: AttemptLease = serde_json::from_str(
        &fs::read_to_string(state.signal_dir.join(format!("pane-{PANE}.lease"))).unwrap(),
    )
    .unwrap();
    assert_eq!(marker, lease);
}

/// The `.awaiting` leg of the same failure, which residency alone cannot see.
///
/// An agent that reads the launch line and asks the human about it is blocked
/// on an `AskUserQuestion` — alive, and silent until somebody answers. The flag
/// that records it survives a keystroke that retires residency, so this is a
/// pane that is occupied and cannot prove it by the residency map. It must
/// still be treated as occupied.
#[test]
fn a_pane_blocked_on_a_question_is_occupied_even_with_no_residency_evidence() {
    let (mut state, lease, _dir) = unobserved_startup_on_an_unproven_pane();
    state.awaiting_human.insert(PANE);
    assert!(!state.provider_is_resident(PANE));
    assert!(state.pane_shows_an_occupant(PANE));

    assert!(state
        .check_assignment_ack_timeouts_at(std::time::SystemTime::now())
        .is_empty());

    assert_eq!(
        state.agent_slots[0].transition_state,
        TransitionState::WaitingForExit,
        "a pane blocked on a question is exited, never probed or relaunched into"
    );
    assert_eq!(
        state.current_leases[TICKET].attempt_id,
        lease.attempt_id + 1
    );
    assert!(
        !state.awaiting_human.contains(&PANE),
        "the question flag must be cleared before the exit is typed, or the write is dropped"
    );
    assert!(!state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Info { message } if message.contains("Suppressed injection")
    )));
}
