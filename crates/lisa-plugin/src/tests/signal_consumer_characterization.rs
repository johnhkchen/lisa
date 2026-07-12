use super::*;

use lisa_core::types::{Thread, ThreadStatus};
use std::fs;
use std::time::{Duration, SystemTime};

const PANE_ID: u32 = 7;
const TICKET_ID: &str = "T-SIGNAL";

fn state_with_signal_dir() -> (State, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let signal_dir = dir.path().join("signals");
    fs::create_dir_all(&signal_dir).unwrap();
    (
        State {
            signal_dir,
            ..State::default()
        },
        dir,
    )
}

fn add_running_attempt(state: &mut State) -> AttemptLease {
    let mut slot = fresh_slot(PANE_ID, Some(AgentClient::Codex));
    slot.ticket_id = Some(TICKET_ID.to_string());
    state.agent_slots.push(slot);
    state
        .threads
        .insert(TICKET_ID.to_string(), Thread::new(TICKET_ID, PANE_ID));
    install_current_attempt(state, TICKET_ID)
}

#[test]
fn poll_tick_preserves_the_eight_consumer_order() {
    let source = include_str!("../lib.rs");
    let poll_tick = source
        .split_once("    fn poll_tick(&mut self) {")
        .expect("poll_tick must remain present")
        .1
        .split_once("    fn format_activity_event")
        .expect("poll_tick source boundary must remain present")
        .0;
    let expected = [
        "self.check_heartbeat_signals();",
        "self.check_awaiting_signals();",
        "self.check_process_start_signals();",
        "self.check_shell_ready_signals();",
        "self.check_codex_ack_signals();",
        "self.check_idle_signals();",
        "self.check_transition_signals();",
        "self.check_error_signals();",
    ];

    let mut remainder = poll_tick;
    for call in expected {
        let (_, after) = remainder
            .split_once(call)
            .unwrap_or_else(|| panic!("missing or reordered signal consumer: {call}"));
        remainder = after;
    }
}

#[test]
fn recognized_records_are_one_shot_before_payload_or_state_admission() {
    let cases = [
        ("heartbeat", "pane-7.heartbeat", "not-json"),
        ("process-start", "pane-7.started", "not-json"),
        ("shell-ready", "pane-7.shell-ready", "not-json"),
        ("codex-ack", "pane-7.ack", "not-json"),
        ("awaiting", "pane-7.awaiting", "body-is-ignored"),
        ("idle", "pane-7.idle", "body-is-ignored"),
        ("transition", "pane-seven.stopped", "body-is-ignored"),
        ("error", "pane-7.error", "body-is-ignored"),
    ];

    for (consumer, filename, body) in cases {
        let (mut state, _dir) = state_with_signal_dir();
        let path = state.signal_dir.join(filename);
        fs::write(&path, body).unwrap();

        match consumer {
            "heartbeat" => state.check_heartbeat_signals(),
            "process-start" => state.check_process_start_signals(),
            "shell-ready" => state.check_shell_ready_signals(),
            "codex-ack" => state.check_codex_ack_signals(),
            "awaiting" => state.check_awaiting_signals(),
            "idle" => state.check_idle_signals(),
            "transition" => state.check_transition_signals(),
            "error" => {
                state.check_error_signals();
            }
            _ => unreachable!(),
        }

        assert!(
            !path.exists(),
            "{consumer} must delete a recognized record before admission completes"
        );
    }
}

#[test]
fn idle_alone_admits_the_legacy_ticket_filename_family() {
    let cases = [
        ("heartbeat", "T-LEGACY.heartbeat"),
        ("process-start", "T-LEGACY.started"),
        ("shell-ready", "T-LEGACY.shell-ready"),
        ("codex-ack", "T-LEGACY.ack"),
        ("awaiting", "T-LEGACY.awaiting"),
        ("idle", "T-LEGACY.idle"),
        ("transition", "T-LEGACY.stopped"),
        ("error", "T-LEGACY.error"),
    ];

    for (consumer, filename) in cases {
        let (mut state, _dir) = state_with_signal_dir();
        let path = state.signal_dir.join(filename);
        fs::write(&path, "legacy body").unwrap();

        match consumer {
            "heartbeat" => state.check_heartbeat_signals(),
            "process-start" => state.check_process_start_signals(),
            "shell-ready" => state.check_shell_ready_signals(),
            "codex-ack" => state.check_codex_ack_signals(),
            "awaiting" => state.check_awaiting_signals(),
            "idle" => state.check_idle_signals(),
            "transition" => state.check_transition_signals(),
            "error" => {
                state.check_error_signals();
            }
            _ => unreachable!(),
        }

        assert_eq!(
            path.exists(),
            consumer != "idle",
            "unexpected legacy filename policy for {consumer}"
        );
    }
}

#[test]
fn heartbeat_requires_the_current_lease_then_refreshes_activity_and_gates() {
    let (mut state, _dir) = state_with_signal_dir();
    let lease = add_running_attempt(&mut state);
    let stale_clock = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    state.threads.get_mut(TICKET_ID).unwrap().last_activity = stale_clock;
    state.awaiting_human.insert(PANE_ID);
    state.notified_attention.insert(PANE_ID);

    let stale = AttemptLease {
        ticket_id: TICKET_ID.to_string(),
        attempt_id: lease.attempt_id + 1,
    };
    let path = state.signal_dir.join("pane-7.heartbeat");
    fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();
    state.check_heartbeat_signals();
    assert!(!path.exists());
    assert_eq!(state.threads[TICKET_ID].last_activity, stale_clock);
    assert!(state.awaiting_human.contains(&PANE_ID));
    assert!(state.notified_attention.contains(&PANE_ID));

    fs::write(&path, serde_json::to_string(&lease).unwrap()).unwrap();
    state.check_heartbeat_signals();
    assert!(!path.exists());
    assert!(state.threads[TICKET_ID].last_activity > stale_clock);
    assert!(state.agent_slots[0].last_activity_at.is_some());
    assert!(!state.awaiting_human.contains(&PANE_ID));
    assert!(!state.notified_attention.contains(&PANE_ID));
}

#[test]
fn process_start_requires_the_current_starting_lease_and_only_marks_ready() {
    let (mut state, _dir) = state_with_signal_dir();
    let lease = add_running_attempt(&mut state);
    state.seat_assignments.insert(
        PANE_ID,
        SeatAssignmentState::Starting {
            generation: lease.attempt_id,
            start_deadline: Some(SystemTime::now()),
            relaunches: 0,
        },
    );
    let path = state.signal_dir.join("pane-7.started");
    let stale = AttemptLease {
        ticket_id: TICKET_ID.to_string(),
        attempt_id: lease.attempt_id + 1,
    };

    fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();
    state.check_process_start_signals();
    assert!(!path.exists());
    assert!(matches!(
        state.seat_assignment(PANE_ID),
        Some(SeatAssignmentState::Starting { .. })
    ));

    fs::write(&path, serde_json::to_string(&lease).unwrap()).unwrap();
    state.check_process_start_signals();
    assert!(!path.exists());
    assert_eq!(
        state.seat_assignment(PANE_ID),
        Some(SeatAssignmentState::ReadyForAssignment {
            generation: lease.attempt_id
        })
    );
    assert!(!state.seat_is_owned(PANE_ID));
}

#[test]
fn shell_ready_requires_the_exact_reset_successor_before_relaunch() {
    let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
    state.config.assignment_ack_timeout_secs = 1;
    fs::create_dir_all(&state.signal_dir).unwrap();
    state.schedule_ready_tickets();
    let predecessor = state.current_leases["T-NAME"].clone();
    let deadline = match state.seat_assignment(10) {
        Some(SeatAssignmentState::Starting {
            start_deadline: Some(deadline),
            ..
        }) => deadline,
        other => panic!("expected Starting, got {other:?}"),
    };
    state.check_assignment_ack_timeouts_at(deadline);
    let successor = state.current_leases["T-NAME"].clone();
    assert!(matches!(
        state.seat_assignment(10),
        Some(SeatAssignmentState::ResettingStartup { generation, .. })
            if generation == successor.attempt_id
    ));

    let path = state.signal_dir.join("pane-10.shell-ready");
    fs::write(&path, serde_json::to_string(&predecessor).unwrap()).unwrap();
    state.check_shell_ready_signals();
    assert!(!path.exists());
    assert!(matches!(
        state.seat_assignment(10),
        Some(SeatAssignmentState::ResettingStartup { .. })
    ));

    fs::write(&path, serde_json::to_string(&successor).unwrap()).unwrap();
    state.check_shell_ready_signals();
    assert!(!path.exists());
    assert!(matches!(
        state.seat_assignment(10),
        Some(SeatAssignmentState::Starting {
            generation,
            relaunches: MAX_SAME_PANE_STARTUP_RELAUNCHES,
            ..
        }) if generation == successor.attempt_id
    ));
}

#[test]
fn codex_ack_requires_the_exact_tag_then_promotes_and_bumps_activity() {
    let (mut state, _dir) = state_with_signal_dir();
    let lease = add_running_attempt(&mut state);
    state.seat_assignments.insert(
        PANE_ID,
        SeatAssignmentState::AssignedPendingAck {
            generation: lease.attempt_id,
            ack_deadline: None,
        },
    );
    let path = state.signal_dir.join("pane-7.ack");
    let stale_payload = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": codex_ack::tag_codex_assignment(
            "stale",
            codex_ack::CodexAssignmentRef {
                ticket_id: TICKET_ID,
                generation: lease.attempt_id + 1,
            },
        ),
    });
    fs::write(&path, stale_payload.to_string()).unwrap();
    state.check_codex_ack_signals();
    assert!(!path.exists());
    assert!(!state.seat_is_owned(PANE_ID));
    assert!(state.agent_slots[0].last_activity_at.is_none());

    let exact_payload = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": codex_ack::tag_codex_assignment(
            "exact",
            codex_ack::CodexAssignmentRef {
                ticket_id: TICKET_ID,
                generation: lease.attempt_id,
            },
        ),
    });
    fs::write(&path, exact_payload.to_string()).unwrap();
    state.check_codex_ack_signals();
    assert!(!path.exists());
    assert_eq!(
        state.seat_assignment(PANE_ID),
        Some(SeatAssignmentState::Owned)
    );
    assert!(state.agent_slots[0].last_activity_at.is_some());
    assert!(state.activity_log.iter().any(|event| matches!(
        event,
        ActivityEvent::Info { message } if message.contains("acknowledged its assignment")
    )));
}

#[test]
fn awaiting_treats_the_body_as_presence_without_refreshing_activity() {
    let (mut state, _dir) = state_with_signal_dir();
    state
        .agent_slots
        .push(fresh_slot(PANE_ID, Some(AgentClient::Claude)));
    let path = state.signal_dir.join("pane-7.awaiting");
    fs::write(&path, "arbitrary non-json provider text").unwrap();

    state.check_awaiting_signals();

    assert!(!path.exists());
    assert!(state.awaiting_human.contains(&PANE_ID));
    assert!(state.agent_slots[0].last_activity_at.is_none());
}

#[test]
fn idle_legacy_name_ignores_the_body_and_reports_its_phase_effect() {
    let dir = tempfile::tempdir().unwrap();
    let tickets_dir = dir.path().join("tickets");
    let signal_dir = dir.path().join("signals");
    fs::create_dir_all(&tickets_dir).unwrap();
    fs::create_dir_all(&signal_dir).unwrap();
    fs::write(
        tickets_dir.join("T-LEGACY.md"),
        "---\nid: T-LEGACY\ntitle: legacy signal\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n",
    )
    .unwrap();
    let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
    let mut state = State {
        dag: Dag::from_tickets(tickets).unwrap(),
        config: PluginConfig {
            ticket_dir: tickets_dir.clone(),
            work_dir: dir.path().join("work"),
            ..PluginConfig::new()
        },
        signal_dir: signal_dir.clone(),
        ..State::default()
    };
    let mut thread = Thread::new("T-LEGACY", PANE_ID);
    thread.current_phase = Phase::Research;
    state.threads.insert("T-LEGACY".to_string(), thread);
    let path = signal_dir.join("T-LEGACY.idle");
    fs::write(&path, "arbitrary legacy body").unwrap();

    state.check_idle_signals();

    assert!(!path.exists());
    assert_eq!(state.threads["T-LEGACY"].current_phase, Phase::Research);
    assert_eq!(state.idle_alerts.len(), 1);
    assert_eq!(state.idle_alerts[0].0, "T-LEGACY");
    assert!(state.idle_alerts[0].1.contains("research.md not found"));
}

#[test]
fn transition_treats_bodies_as_presence_and_refreshes_valid_panes() {
    let (mut state, _dir) = state_with_signal_dir();
    state
        .agent_slots
        .push(fresh_slot(PANE_ID, Some(AgentClient::Claude)));
    let path = state.signal_dir.join("pane-7.stopped");
    fs::write(&path, "arbitrary stop body").unwrap();

    state.check_transition_signals();

    assert!(!path.exists());
    assert!(state.agent_slots[0].last_activity_at.is_some());
    assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);

    let unrelated = state.signal_dir.join("pane-7.idle");
    fs::write(&unrelated, "owned by the idle consumer").unwrap();
    state.check_transition_signals();
    assert!(unrelated.exists());
}

#[test]
fn error_treats_the_body_as_presence_and_reclaims_the_running_thread() {
    let (mut state, _dir) = state_with_signal_dir();
    add_running_attempt(&mut state);
    let path = state.signal_dir.join("pane-7.error");
    fs::write(&path, "arbitrary non-json failure detail").unwrap();

    state.check_error_signals();

    assert!(!path.exists());
    assert!(!state.threads.contains_key(TICKET_ID));
    assert!(state.agent_slots[0].ticket_id.is_none());
    assert!(state.agent_slots[0].has_session);
    assert_eq!(state.error_alerts, vec![(TICKET_ID.to_string(), PANE_ID)]);
    assert!(state.activity_log.iter().any(|event| matches!(
        event,
        ActivityEvent::Error { message } if message.contains(TICKET_ID)
    )));
    assert!(!state
        .threads
        .values()
        .any(|thread| thread.status == ThreadStatus::Running));
}
