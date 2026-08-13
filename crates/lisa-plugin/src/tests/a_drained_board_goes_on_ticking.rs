//! The board that finished, and the three tickets that sat on it.
//!
//! On 2026-08-12 a loop drained its board, an operator filed three more tickets
//! onto it, and nothing happened. The session was alive, the plugin was resident,
//! the tickets were `ready` — and the poll timer had been gone since the moment
//! the last ticket went Done, because `poll_tick()` returned without re-arming.
//! `.lisa/scheduler.alive` went stale for the same reason, which is how the same
//! board ended up with a second scheduler on it (`S-063-01`).
//!
//! What these tests pin:
//!
//! 1. A drained board keeps ticking, and keeps stamping while it does.
//! 2. A ticket filed onto it is picked up by the next tick — no keystroke, no
//!    quit modal, no restart.
//! 3. An idle tick costs a look at the directory, not a parse of every ticket
//!    on it.
//! 4. A ticket file that will not parse says so where an operator is looking,
//!    once, and says so again when it is fixed.

use super::*;

use std::fs;

const DONE_TICKET: &str = "T-DRAINED";
const NEW_TICKET: &str = "T-FILED-LATER";

fn ticket_file(phase: &str, status: &str, id: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {id}\ntype: task\nstatus: {status}\npriority: high\nphase: {phase}\ndepends_on: []\n---\n\n## Context\n\nOne ticket.\n"
    )
}

/// A scheduler whose board is one finished ticket, with one free seat, wired to
/// a project directory the way `load()` wires the real one.
fn drained_scheduler(root: &std::path::Path) -> State {
    let tickets_dir = root.join("docs/active/tickets");
    fs::create_dir_all(&tickets_dir).unwrap();
    fs::write(
        tickets_dir.join(format!("{DONE_TICKET}.md")),
        ticket_file("done", "done", DONE_TICKET),
    )
    .unwrap();
    let signal_dir = root.join(".lisa/signals");
    fs::create_dir_all(&signal_dir).unwrap();

    let tickets = ticket::scan_tickets(&tickets_dir).unwrap();
    let mut state = State {
        dag: Dag::from_tickets(tickets).unwrap(),
        config: PluginConfig {
            ticket_dir: tickets_dir,
            work_dir: root.join("docs/active/work"),
            max_threads: 1,
            wind_down_secs: 0,
            ..PluginConfig::new()
        },
        attempt_dir: root.join(".lisa/attempts"),
        signal_dir,
        scheduler_dir: root.join(lisa_core::schedulers::SCHEDULER_DIR),
        scheduler_id: "drained-desk".to_string(),
        ledger_path: root.join(".lisa/provenance.jsonl"),
        permissions_granted: true,
        slots_discovered: true,
        ..State::default()
    };
    state.scheduler_started_at = unix_secs(std::time::SystemTime::now()).unwrap();
    state.agent_slots.push(AgentSlot {
        pane_id: 7,
        ticket_id: None,
        attempt_lease: None,
        has_session: false,
        transition_state: TransitionState::Idle,
        transition_started_at: None,
        cooldown_until: None,
        last_activity_at: None,
        last_client: None,
    });
    state
}

fn dag_rebuilds(state: &State) -> usize {
    state
        .activity_events()
        .filter(|event| matches!(event, ActivityEvent::DagRecomputed { .. }))
        .count()
}

fn stamp_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(lisa_core::liveness::SCHEDULER_ALIVE_FILE)
}

#[test]
fn a_ticket_filed_onto_a_drained_board_is_picked_up_without_a_keystroke() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let mut state = drained_scheduler(root);

    // 1. The board drains. Before this change the tick returned here and the
    //    loop never ticked again.
    state.poll_tick();
    assert!(state.terminated, "the board is finished");
    assert!(
        state.pending_timer_count > 0,
        "a finished board is still on the clock"
    );
    assert!(stamp_path(root).exists());
    assert_eq!(
        state
            .activity_events()
            .filter(|event| matches!(event, ActivityEvent::AllTicketsDone))
            .count(),
        1
    );

    // 2. Nothing on the board has moved, so the tick looks and stops. It costs
    //    a read_dir, not a parse of every ticket — and it still stamps.
    let rebuilds_after_drain = dag_rebuilds(&state);
    fs::remove_file(stamp_path(root)).unwrap();
    state.pending_timer_count = 0;
    state.poll_tick();
    assert!(
        stamp_path(root).exists(),
        "a scheduler with nothing to do is still a scheduler"
    );
    assert_eq!(state.pending_timer_count, 1, "and is still on the clock");
    assert_eq!(
        dag_rebuilds(&state),
        rebuilds_after_drain,
        "an idle tick does not re-parse the board"
    );
    assert_eq!(
        state
            .activity_events()
            .filter(|event| matches!(event, ActivityEvent::AllTicketsDone))
            .count(),
        1,
        "and does not announce the same drain twice"
    );

    // 3. An operator files a ticket. Nobody presses anything.
    fs::write(
        state.config.ticket_dir.join(format!("{NEW_TICKET}.md")),
        ticket_file("ready", "open", NEW_TICKET),
    )
    .unwrap();

    state.poll_tick();

    assert!(!state.terminated, "the board woke up");
    assert!(
        state.dag.get_ticket(&NEW_TICKET.to_string()).is_some(),
        "the ticket filed onto a finished board is on the board"
    );
    assert!(
        state.threads.contains_key(NEW_TICKET),
        "and it has been assigned"
    );
    assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some(NEW_TICKET));
    assert!(state.pending_timer_count > 0);
    assert!(dag_rebuilds(&state) > rebuilds_after_drain);
}

#[test]
fn the_quit_modal_still_resumes_a_drained_board() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let mut state = drained_scheduler(root);
    state.poll_tick();
    assert!(state.terminated);

    fs::write(
        state.config.ticket_dir.join(format!("{NEW_TICKET}.md")),
        ticket_file("ready", "open", NEW_TICKET),
    )
    .unwrap();

    // The path that used to be the only way back is unchanged, and does not
    // need the timer to have stopped to work.
    state.keep_working();

    assert!(!state.terminated);
    assert!(state.threads.contains_key(NEW_TICKET));
    assert!(state.pending_timer_count > 0);
}

#[test]
fn a_ticket_file_that_will_not_parse_is_reported_once_and_its_repair_too() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let mut state = drained_scheduler(root);
    let broken = state.config.ticket_dir.join("T-UNREADABLE.md");
    fs::write(
        &broken,
        "---\nid: T-UNREADABLE\ntitle: half a ticket\n---\n",
    )
    .unwrap();

    state.rebuild_dag();

    let reported = |state: &State| {
        state
            .activity_events()
            .filter(|event| {
                matches!(
                    event,
                    ActivityEvent::Error { message }
                        if message.contains("T-UNREADABLE.md") && message.contains("Skipped")
                )
            })
            .count()
    };
    assert_eq!(reported(&state), 1, "the operator is told, in the feed");
    assert!(
        state.dag.get_ticket(&DONE_TICKET.to_string()).is_some(),
        "and the rest of the board is unaffected"
    );

    // Every five seconds forever is not reporting, it is noise.
    state.rebuild_dag();
    state.rebuild_dag();
    assert_eq!(reported(&state), 1, "reported on the edge, not the level");

    fs::write(&broken, ticket_file("ready", "open", "T-UNREADABLE")).unwrap();
    state.rebuild_dag();
    assert!(
        state.activity_events().any(|event| matches!(
            event,
            ActivityEvent::Info { message } if message.contains("reads cleanly again")
        )),
        "the repair is worth a line too"
    );
    assert!(state.dag.get_ticket(&"T-UNREADABLE".to_string()).is_some());
}
