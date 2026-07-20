use super::*;

use lisa_core::types::{Phase, Thread, TicketStatus};
use std::fs;

const TICKET_ID: &str = "T-OPERATOR";
const PANE_ID: u32 = 17;

fn review_state() -> (State, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let tickets_dir = dir.path().join("tickets");
    let work_dir = dir.path().join("work");
    fs::create_dir_all(&tickets_dir).unwrap();
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        tickets_dir.join(format!("{TICKET_ID}.md")),
        format!(
            "---\nid: {TICKET_ID}\ntitle: operator-recovery\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n\nOperator recovery fixture.\n"
        ),
    )
    .unwrap();

    let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
    let mut state = State {
        dag: Dag::from_tickets(tickets).unwrap(),
        config: PluginConfig {
            ticket_dir: tickets_dir.clone(),
            work_dir,
            ..PluginConfig::new()
        },
        ..State::default()
    };
    write_canonical_review_disposition(
        &state,
        TICKET_ID,
        r#"{"disposition":"pass","reason":null}"#,
    );
    state.project_root = dir.path().to_path_buf();
    state.git_root = dir.path().to_path_buf();
    (state, dir)
}

fn add_active_review_attempt(state: &mut State) -> AttemptLease {
    let mut thread = Thread::new(TICKET_ID, PANE_ID);
    thread.current_phase = Phase::Review;
    thread.client = AgentClient::Codex;
    state.threads.insert(TICKET_ID.to_string(), thread);

    let mut slot = fresh_slot(PANE_ID, Some(AgentClient::Codex));
    slot.ticket_id = Some(TICKET_ID.to_string());
    state.agent_slots.push(slot);

    install_current_attempt(state, TICKET_ID)
}

fn submit_from_done_key(state: &mut State) {
    assert!(state.handle_key(KeyWithModifier {
        bare_key: BareKey::Char('d'),
        key_modifiers: Default::default(),
    }));
    assert!(state.modal.open, "[d] must open the MarkDone modal");
    assert_eq!(state.modal.mode, ModalMode::MarkDone);
    assert_eq!(
        state
            .modal
            .ticket_ids
            .get(state.modal.cursor)
            .map(String::as_str),
        Some(TICKET_ID)
    );
    assert!(state.handle_key(KeyWithModifier {
        bare_key: BareKey::Enter,
        key_modifiers: Default::default(),
    }));
}

fn operator_correlation() -> String {
    State::completion_correlation(CompletionId::new(TICKET_ID), AttemptId::new("operator"))
        .to_string()
}

fn assert_operator_pending(state: &State) -> String {
    let pending = state
        .pending_completions
        .get(TICKET_ID)
        .expect("operator request must be pending");
    assert_eq!(pending.authority, CompletionAuthority::Operator);
    assert_eq!(
        pending.source,
        CompletionSource::OperatorRequested(OperatorRequestSource::MarkDoneKey)
    );
    assert_eq!(pending.completion_key.attempt_id().as_str(), "operator");
    assert_eq!(
        state.launched_completion_effects,
        vec![EffectCommand::LaunchCompletion {
            attempt_id: AttemptId::new("operator"),
            completion_id: CompletionId::new(TICKET_ID),
        }]
    );

    let correlation_id = pending.completion_key.to_string();
    assert_eq!(correlation_id, operator_correlation());
    assert!(state.modal.open, "Pending must remain visible");
    assert_eq!(
        state.modal.operator_outcome,
        Some(OperatorModalOutcome::Pending {
            ticket_id: TICKET_ID.to_string(),
            correlation_id: correlation_id.clone(),
        })
    );
    correlation_id
}

fn assert_named_rejection(
    state: &State,
    expected_kind: CompletionRejectionKind,
    detail_fragment: &str,
) -> String {
    let event = state
        .activity_events()
        .rev()
        .find(|event| {
            matches!(
                event,
                ActivityEvent::CompletionRejected {
                    ticket_id,
                    kind,
                    ..
                } if ticket_id == TICKET_ID && *kind == expected_kind
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "missing {expected_kind} rejection for {TICKET_ID}: {:?}",
                state.activity_log
            )
        });
    let ActivityEvent::CompletionRejected {
        correlation_id,
        detail,
        ..
    } = event
    else {
        unreachable!()
    };
    assert_eq!(correlation_id, &operator_correlation());
    assert!(!correlation_id.is_empty());
    assert!(
        detail.contains(detail_fragment),
        "expected {detail:?} to contain {detail_fragment:?}"
    );
    assert!(state.modal.open, "Rejection must remain visible");
    assert!(matches!(
        state.modal.operator_outcome.as_ref(),
        Some(OperatorModalOutcome::Rejected {
            ticket_id,
            kind,
            correlation_id: modal_correlation,
            detail: modal_detail,
        }) if ticket_id == TICKET_ID
            && *kind == expected_kind
            && modal_correlation == correlation_id
            && modal_detail == detail
    ));
    correlation_id.clone()
}

#[test]
fn active_review_accepts_explicit_operator_recovery_with_correlation() {
    let (mut state, _dir) = review_state();
    let lease = add_active_review_attempt(&mut state);

    submit_from_done_key(&mut state);

    let correlation_id = assert_operator_pending(&state);
    assert_eq!(state.current_leases.get(TICKET_ID), Some(&lease));
    assert_eq!(
        state.threads[TICKET_ID].attempt_lease.as_ref(),
        Some(&lease)
    );
    assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&lease));
    assert_ne!(
        correlation_id,
        State::completion_correlation(
            CompletionId::new(TICKET_ID),
            AttemptId::new(lease.attempt_id.to_string()),
        )
        .to_string(),
        "operator input must not borrow the active attempt"
    );
}

#[test]
fn orphaned_review_accepts_operator_recovery_without_attempt_authority() {
    let (mut state, _dir) = review_state();
    assert!(!state.threads.contains_key(TICKET_ID));
    assert!(!state.current_leases.contains_key(TICKET_ID));

    submit_from_done_key(&mut state);

    assert_operator_pending(&state);
    assert!(!state.threads.contains_key(TICKET_ID));
    assert!(!state.current_leases.contains_key(TICKET_ID));
}

#[test]
fn blocked_disposition_rejects_operator_recovery_with_name_and_correlation() {
    let (mut state, _dir) = review_state();
    write_canonical_review_disposition(
        &state,
        TICKET_ID,
        r#"{"disposition":"block","reason":"resolve the blocked review"}"#,
    );

    submit_from_done_key(&mut state);

    assert!(state.pending_completions.is_empty());
    assert!(state.launched_completion_effects.is_empty());
    assert_named_rejection(
        &state,
        CompletionRejectionKind::DispositionBlocked,
        "resolve the blocked review",
    );
}

#[test]
fn stale_attempt_records_do_not_override_explicit_operator_authority() {
    let (mut state, _dir) = review_state();
    let stale = add_active_review_attempt(&mut state);
    let current = AttemptLease::mint(TICKET_ID, Some(&stale)).unwrap();
    state
        .lease_high_water
        .insert(TICKET_ID.to_string(), current.clone());
    state
        .current_leases
        .insert(TICKET_ID.to_string(), current.clone());

    submit_from_done_key(&mut state);

    assert_operator_pending(&state);
    assert_eq!(state.current_leases.get(TICKET_ID), Some(&current));
    assert_eq!(
        state.threads[TICKET_ID].attempt_lease.as_ref(),
        Some(&stale)
    );
    assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&stale));
    assert!(!state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::CompletionRejected {
            ticket_id,
            kind: CompletionRejectionKind::StaleLease,
            ..
        } if ticket_id == TICKET_ID
    )));
}

#[test]
fn already_pending_operator_recovery_rejects_with_name_and_same_correlation() {
    let (mut state, _dir) = review_state();
    submit_from_done_key(&mut state);
    let original_correlation = assert_operator_pending(&state);

    state.open_mark_done_modal();
    assert!(state.handle_key(KeyWithModifier {
        bare_key: BareKey::Enter,
        key_modifiers: Default::default(),
    }));

    let rejected_correlation = assert_named_rejection(
        &state,
        CompletionRejectionKind::AlreadyPending,
        "already pending",
    );
    assert_eq!(rejected_correlation, original_correlation);
    assert_eq!(
        state.launched_completion_effects.len(),
        1,
        "a duplicate operator request must not launch a second effect"
    );
}

#[test]
fn launch_failure_rejects_operator_recovery_with_name_and_correlation() {
    let (mut state, dir) = review_state();
    state.completion_journal_path = dir.path().join("completion-journal.jsonl");
    state.completion_journal_healthy = true;
    assert!(state.config.lisa_bin.is_none());

    submit_from_done_key(&mut state);

    assert!(state.pending_completions.is_empty());
    assert!(state.launched_completion_effects.is_empty());
    assert_named_rejection(
        &state,
        CompletionRejectionKind::LaunchFailed,
        "lisa_bin is not configured",
    );
}

#[test]
fn successful_operator_recovery_accepts_the_original_correlation_and_releases() {
    let (mut state, _dir) = review_state();
    add_active_review_attempt(&mut state);
    submit_from_done_key(&mut state);
    let correlation_id = assert_operator_pending(&state);

    let ticket_path = state.config.ticket_dir.join(format!("{TICKET_ID}.md"));
    lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
    state.handle_completion_result(TICKET_ID, Some(0), vec![b'a'; 40], Vec::new());

    assert!(!state.pending_completions.contains_key(TICKET_ID));
    assert_eq!(
        state.modal.operator_outcome,
        Some(OperatorModalOutcome::Accepted {
            ticket_id: TICKET_ID.to_string(),
            correlation_id,
        })
    );
    assert!(state.modal.open, "Accepted must remain visible");
    assert!(!state.threads.contains_key(TICKET_ID));
    assert!(state.agent_slots[0].ticket_id.is_none());
    let ticket = state.dag.get_ticket(&TICKET_ID.to_string()).unwrap();
    assert_eq!(ticket.phase, Phase::Done);
    assert_eq!(ticket.status, TicketStatus::Done);
}
