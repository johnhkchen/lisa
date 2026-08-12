use super::*;

use std::fs;
use std::time::{Duration, SystemTime};

const PANE_ID: u32 = 23;
const TICKET_ID: &str = "T-INGESTION-REGRESSION";

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

fn sorted(mut records: Vec<SignalRecord>) -> Vec<SignalRecord> {
    records.sort_by_key(|record| format!("{record:?}"));
    records
}

#[test]
fn every_request_produces_its_exact_typed_record_contract() {
    let dir = tempfile::tempdir().unwrap();
    let lease = AttemptLease {
        ticket_id: TICKET_ID.to_string(),
        attempt_id: 41,
    };
    let lease_json = serde_json::to_string(&lease).unwrap();

    let heartbeat = dir.path().join("pane-1.heartbeat");
    fs::write(&heartbeat, &lease_json).unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::Heartbeats, None),
        vec![SignalRecord::Heartbeat {
            pane_id: 1,
            lease: lease.clone(),
        }]
    );
    assert!(!heartbeat.exists());

    let started = dir.path().join("pane-2.started");
    fs::write(&started, &lease_json).unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::ProcessStarts, None),
        vec![SignalRecord::ProcessStarted {
            pane_id: 2,
            lease: lease.clone(),
        }]
    );
    assert!(!started.exists());

    let shell_ready = dir.path().join("pane-3.shell-ready");
    fs::write(&shell_ready, &lease_json).unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::ShellReady, None),
        vec![SignalRecord::ShellReady {
            pane_id: 3,
            lease: lease.clone(),
        }]
    );
    assert!(!shell_ready.exists());

    let claim_path = dir.path().join("pane-4.claim");
    let claim = lisa_core::claim::AssignmentClaim {
        ticket_id: TICKET_ID.to_string(),
        attempt_id: 41,
        nonce: u128::from(u64::MAX) + 42,
    };
    fs::write(&claim_path, serde_json::to_string(&claim).unwrap()).unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::Claims, None),
        vec![SignalRecord::Claim { pane_id: 4, claim }]
    );
    assert!(!claim_path.exists());

    let ack = dir.path().join("pane-5.ack");
    let provider_payload = r#"{"hook_event_name":"UserPromptSubmit","provider":"codex"}"#;
    fs::write(&ack, provider_payload).unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::CodexAcknowledgements, None),
        vec![SignalRecord::CodexAcknowledgement {
            pane_id: 5,
            payload: provider_payload.to_string(),
        }]
    );
    assert!(!ack.exists());

    let awaiting = dir.path().join("pane-5.awaiting");
    fs::write(&awaiting, "presence body must not enter the record").unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::Awaiting, None),
        vec![SignalRecord::Awaiting { pane_id: 5 }]
    );
    assert!(!awaiting.exists());

    let pane_idle = dir.path().join("pane-6.idle");
    let legacy_idle = dir.path().join("T-LEGACY.idle");
    fs::write(&pane_idle, "ignored").unwrap();
    fs::write(&legacy_idle, "ignored").unwrap();
    assert_eq!(
        sorted(signal::ingest(dir.path(), SignalRequest::Idle, None)),
        sorted(vec![
            SignalRecord::Idle {
                target: IdleTarget::Pane(6),
            },
            SignalRecord::Idle {
                target: IdleTarget::LegacyTicket("T-LEGACY".to_string()),
            },
        ])
    );
    assert!(!pane_idle.exists());
    assert!(!legacy_idle.exists());

    let stopped = dir.path().join("pane-7.stopped");
    let cleared = dir.path().join("pane-8.cleared");
    fs::write(&stopped, "presence only").unwrap();
    fs::write(&cleared, "presence only").unwrap();
    assert_eq!(
        sorted(signal::ingest(dir.path(), SignalRequest::Transitions, None)),
        sorted(vec![
            SignalRecord::Stopped { pane_id: 7 },
            SignalRecord::Cleared { pane_id: 8 },
        ])
    );
    assert!(!stopped.exists());
    assert!(!cleared.exists());

    let error = dir.path().join("pane-9.error");
    fs::write(&error, "provider failure detail must not enter the record").unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::Errors, None),
        vec![SignalRecord::Error { pane_id: 9 }]
    );
    assert!(!error.exists());
}

#[test]
fn recognition_keeps_strict_and_broad_deletion_policies_distinct() {
    let dir = tempfile::tempdir().unwrap();

    let invalid_strict_name = dir.path().join("pane-seven.heartbeat");
    fs::write(&invalid_strict_name, "not a lease").unwrap();
    assert!(signal::ingest(dir.path(), SignalRequest::Heartbeats, None).is_empty());
    assert!(invalid_strict_name.exists());

    let malformed_strict_payload = dir.path().join("pane-7.heartbeat");
    fs::write(&malformed_strict_payload, "not a lease").unwrap();
    assert!(signal::ingest(dir.path(), SignalRequest::Heartbeats, None).is_empty());
    assert!(!malformed_strict_payload.exists());

    let raw_ack = dir.path().join("pane-8.ack");
    fs::write(&raw_ack, "not-json-but-still-provider-payload").unwrap();
    assert_eq!(
        signal::ingest(dir.path(), SignalRequest::CodexAcknowledgements, None),
        vec![SignalRecord::CodexAcknowledgement {
            pane_id: 8,
            payload: "not-json-but-still-provider-payload".to_string(),
        }]
    );
    assert!(!raw_ack.exists());

    let legacy_ack = dir.path().join("T-LEGACY.ack");
    fs::write(&legacy_ack, "legacy naming is idle-only").unwrap();
    assert!(signal::ingest(dir.path(), SignalRequest::CodexAcknowledgements, None).is_empty());
    assert!(legacy_ack.exists());

    let invalid_broad_idle = dir.path().join("pane-seven.idle");
    fs::write(&invalid_broad_idle, "ignored").unwrap();
    assert!(signal::ingest(dir.path(), SignalRequest::Idle, None).is_empty());
    assert!(!invalid_broad_idle.exists());

    let invalid_broad_transition = dir.path().join("pane-seven.stopped");
    let unrelated_idle = dir.path().join("pane-10.idle");
    fs::write(&invalid_broad_transition, "ignored").unwrap();
    fs::write(&unrelated_idle, "owned by idle").unwrap();
    assert!(signal::ingest(dir.path(), SignalRequest::Transitions, None).is_empty());
    assert!(!invalid_broad_transition.exists());
    assert!(unrelated_idle.exists());
}

#[test]
fn typed_lease_ingestion_stays_separate_from_current_attempt_admission() {
    let (mut state, _dir) = state_with_signal_dir();
    let current = add_running_attempt(&mut state);
    let baseline = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    state.threads.get_mut(TICKET_ID).unwrap().last_activity = baseline;
    state.awaiting_human.insert(PANE_ID);
    state.notified_attention.insert(PANE_ID);

    let stale = AttemptLease {
        ticket_id: TICKET_ID.to_string(),
        attempt_id: current.attempt_id + 1,
    };
    let path = state.signal_dir.join(format!("pane-{PANE_ID}.heartbeat"));
    fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();

    assert_eq!(
        signal::ingest(&state.signal_dir, SignalRequest::Heartbeats, None),
        vec![SignalRecord::Heartbeat {
            pane_id: PANE_ID,
            lease: stale.clone(),
        }],
        "ingestion validates lease shape but must not decide lease currency"
    );
    assert!(!path.exists());

    fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();
    state.check_heartbeat_signals();
    assert!(!path.exists());
    assert_eq!(state.threads[TICKET_ID].last_activity, baseline);
    assert!(state.agent_slots[0].last_activity_at.is_none());
    assert!(state.awaiting_human.contains(&PANE_ID));
    assert!(state.notified_attention.contains(&PANE_ID));

    fs::write(&path, serde_json::to_string(&current).unwrap()).unwrap();
    state.check_heartbeat_signals();
    assert!(!path.exists());
    assert!(state.threads[TICKET_ID].last_activity > baseline);
    assert!(state.agent_slots[0].last_activity_at.is_some());
    assert!(!state.awaiting_human.contains(&PANE_ID));
    assert!(!state.notified_attention.contains(&PANE_ID));
}

#[test]
fn poll_tick_preserves_signal_admission_and_timeout_interleaving() {
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
        "self.deliver_ready_assignments();",
        "self.check_process_start_signals();",
        "self.check_shell_ready_signals();",
        "self.check_claim_signals();",
        "self.check_codex_ack_signals();",
        "self.check_artifact_advances();",
        "self.check_idle_signals();",
        "self.check_transition_signals();",
        "self.check_error_signals();",
        "self.check_transition_timeouts();",
        "self.check_assignment_ack_timeouts();",
    ];

    let mut remainder = poll_tick;
    for call in expected {
        let (_, after) = remainder
            .split_once(call)
            .unwrap_or_else(|| panic!("missing or reordered poll operation: {call}"));
        remainder = after;
    }
}
