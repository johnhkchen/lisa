//! The failure of S-063-01, reproduced with two whole schedulers on one board.
//!
//! Not a unit test of the registry and not a test of `signal::ingest` — both of
//! those exist elsewhere. This is the shape of the incident itself: two
//! `State`s, one `.lisa/` directory, one `pane-1.started`, and a live scheduler
//! that never sees it because the other one got there first.
//!
//! The two facts it pins are the two the day turned on:
//!
//! 1. The board can say there are two of them. Before the registry, three
//!    schedulers and one produced byte-identical evidence.
//! 2. The scheduler that lost the race does not type its shell-readiness probe
//!    into the pane. That probe is a dead end against a live TUI — a
//!    `--dangerously-skip-permissions` session runs it and forges the readiness
//!    proof — and ten of twelve sessions on the day received it as their first
//!    and only prompt.

use super::*;

use std::fs;
use std::time::SystemTime;

const PANE_ID: u32 = 1;
const TICKET_ID: &str = "T-CONTESTED";

/// One scheduler, wired to a shared project directory exactly as `load()`
/// wires the real one.
fn scheduler(root: &std::path::Path, session: &str, id: &str) -> State {
    let signal_dir = root.join(".lisa/signals");
    fs::create_dir_all(&signal_dir).unwrap();
    let mut state = State {
        signal_dir,
        scheduler_dir: root.join(lisa_core::schedulers::SCHEDULER_DIR),
        scheduler_id: id.to_string(),
        zellij_pid: Some(9450),
        config: PluginConfig {
            session_name: Some(session.to_string()),
            ..PluginConfig::new()
        },
        ..State::default()
    };
    state.scheduler_started_at = unix_secs(SystemTime::now()).unwrap();
    state.stamp_scheduler_alive();
    state
}

/// Give this scheduler a seat on the shared pane, waiting for a startup it has
/// already stopped expecting.
fn seat_awaiting_startup(state: &mut State) -> AttemptLease {
    let lease = AttemptLease::mint(TICKET_ID.to_string(), None).unwrap();
    state.agent_slots.push(AgentSlot {
        pane_id: PANE_ID,
        ticket_id: Some(TICKET_ID.to_string()),
        attempt_lease: Some(lease.clone()),
        has_session: true,
        transition_state: TransitionState::Idle,
        transition_started_at: None,
        cooldown_until: None,
        last_activity_at: None,
        last_client: Some(AgentClient::Claude),
    });
    state
        .current_leases
        .insert(TICKET_ID.to_string(), lease.clone());
    state
        .lease_high_water
        .insert(TICKET_ID.to_string(), lease.clone());
    state.seat_assignments.insert(
        PANE_ID,
        SeatAssignmentState::Starting {
            generation: lease.attempt_id,
            start_deadline: Some(SystemTime::now() - std::time::Duration::from_secs(1)),
            relaunches: 0,
        },
    );
    lease
}

#[test]
fn a_started_signal_the_live_scheduler_never_sees_no_longer_ends_in_a_probe() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();

    // Yesterday evening's run, still stamping. Its client is long gone; the
    // Zellij server it lives in is not.
    let mut zombie = scheduler(root, "fascinating-drum", "drum-zombie");
    // This morning's run, the one the operator is actually looking at.
    let mut live = scheduler(root, "blossoming-cymbal", "cymbal-live");

    // 1. The board can be counted, and each entry says how to stop it.
    let roster = lisa_core::schedulers::read_roster(root);
    assert_eq!(roster.len(), 2, "two schedulers, two records");
    assert_eq!(
        roster
            .iter()
            .filter_map(|record| record.stop_command())
            .collect::<Vec<_>>()
            .len(),
        2,
        "each record carries the command that ends it"
    );
    let now = unix_secs(SystemTime::now()).unwrap();
    live.note_other_schedulers(now);
    assert_eq!(live.other_schedulers.len(), 1);
    assert!(live.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Warning { message }
            if message.contains("another scheduler")
                && message.contains("zellij kill-session fascinating-drum")
    )));

    // 2. Both schedulers have a seat on the same pane, and the agent that
    //    really did start publishes exactly one `.started`.
    let live_lease = seat_awaiting_startup(&mut live);
    seat_awaiting_startup(&mut zombie);
    fs::write(
        root.join(".lisa/signals").join("pane-1.started"),
        serde_json::to_string(&live_lease).unwrap(),
    )
    .unwrap();

    // 3. The zombie polls first and consumes it. This is the single-consumer
    //    contract working exactly as designed, for the wrong reader.
    zombie.check_process_start_signals();
    assert!(
        !root.join(".lisa/signals/pane-1.started").exists(),
        "the signal is gone, and the scheduler that needed it never saw it"
    );
    live.check_process_start_signals();

    // 4. The live scheduler's acknowledgment window expires. Before the
    //    receipt existed, this is where it interrupted a running Claude and
    //    typed a probe the session answered on its behalf. The probe is gone,
    //    and so is the silent window that replaced it: this seat is fenced with
    //    the real diagnosis instead.
    live.check_assignment_ack_timeouts_at(SystemTime::now());

    assert!(
        !live.attempt_lifecycle.iter().any(|event| matches!(
            event,
            AttemptLifecycleEvent::StartupObservationOpened { .. }
        )),
        "no window is opened on a pane whose start signal went to another scheduler; \
         waiting for evidence a second scheduler is eating is waiting forever"
    );
    assert!(matches!(
        live.seat_assignment(PANE_ID),
        Some(SeatAssignmentState::StartupFailed)
    ));
    assert!(
        live.activity_events().any(|event| matches!(
            event,
            ActivityEvent::Error { message } | ActivityEvent::Warning { message }
                if message.contains("consumed by another scheduler")
                    && message.contains("fascinating-drum")
        )),
        "the failure names the scheduler that took the signal, not a mystery"
    );
}

/// The same board with the other scheduler stopped: nothing about this changes
/// the ordinary single-scheduler run.
#[test]
fn one_scheduler_alone_keeps_every_startup_path_it_had() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let mut live = scheduler(root, "blossoming-cymbal", "cymbal-live");
    let lease = seat_awaiting_startup(&mut live);
    live.write_pane_lease_marker(PANE_ID, &lease).unwrap();
    fs::write(
        root.join(".lisa/signals").join("pane-1.started"),
        serde_json::to_string(&lease).unwrap(),
    )
    .unwrap();

    // It consumes its own start signal, leaves its own receipt, and the
    // receipt is not evidence against itself.
    live.check_process_start_signals();
    assert!(!root.join(".lisa/signals/pane-1.started").exists());
    let receipts = lisa_core::schedulers::read_receipts(&live.scheduler_dir);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].scheduler_id, "cymbal-live");
    assert_eq!(receipts[0].signal, "pane-1.started");
    assert!(live
        .started_signal_taken_by_another(PANE_ID, SystemTime::now())
        .is_none());
}
