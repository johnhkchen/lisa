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

/// The dispatcher half: a refusal is logged under its own name, correlated to
/// the operator's request, carrying a detail a person can act on.
///
/// Split from [`assert_named_rejection`] because one rejection kind no longer
/// reaches the modal at all: `[d]` routes a blocked disposition to the reason
/// step instead (T-053-01-02), so the guard's naming outlives its display.
fn assert_named_rejection_event(
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
    correlation_id.clone()
}

/// The same assertion for a fixture that is not [`TICKET_ID`], where the
/// correlation is whatever the dispatcher chose rather than a known constant.
fn assert_named_rejection_event_for(
    state: &State,
    ticket: &str,
    expected_kind: CompletionRejectionKind,
    detail_fragment: &str,
) {
    let found = state.activity_events().rev().any(|event| {
        matches!(
            event,
            ActivityEvent::CompletionRejected {
                ticket_id,
                kind,
                detail,
                correlation_id,
            } if ticket_id == ticket
                && *kind == expected_kind
                && !correlation_id.is_empty()
                && detail.contains(detail_fragment)
        )
    });
    assert!(
        found,
        "missing {expected_kind} rejection for {ticket} naming {detail_fragment:?}: {:?}",
        state.activity_log
    );
}

/// The dispatcher half plus its display: the refusal is also the thing the
/// operator is looking at.
fn assert_named_rejection(
    state: &State,
    expected_kind: CompletionRejectionKind,
    detail_fragment: &str,
) -> String {
    let correlation_id = assert_named_rejection_event(state, expected_kind, detail_fragment);
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
            && modal_correlation == &correlation_id
            && detail_fragment.is_empty() == modal_detail.is_empty()
            && modal_detail.contains(detail_fragment)
    ));
    correlation_id
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

/// The guard itself is unchanged: a completion carrying no operator-chosen
/// reason still refuses a blocked disposition, and says so by name.
///
/// This used to be driven through `[d]`. It is not any more — see
/// [`blocked_disposition_no_longer_dead_ends_on_the_done_key`] — so it exercises
/// the dispatcher directly, which is the surface the claim is actually about.
#[test]
fn blocked_disposition_rejects_operator_recovery_with_name_and_correlation() {
    let (mut state, _dir) = review_state();
    write_canonical_review_disposition(
        &state,
        TICKET_ID,
        r#"{"disposition":"block","reason":"resolve the blocked review"}"#,
    );

    state.mark_ticket_done(TICKET_ID);

    assert!(state.pending_completions.is_empty());
    assert!(state.launched_completion_effects.is_empty());
    assert_named_rejection_event(
        &state,
        CompletionRejectionKind::DispositionBlocked,
        "resolve the blocked review",
    );
}

/// The retirement of the dead end, pinned where the old expectation lived
/// (T-053-01-02). `[d]` on a parked ticket now leads to the reason step; it no
/// longer produces a rejection the operator can do nothing with.
#[test]
fn blocked_disposition_no_longer_dead_ends_on_the_done_key() {
    let (mut state, _dir) = review_state();
    write_canonical_review_disposition(
        &state,
        TICKET_ID,
        r#"{"disposition":"block","reason":"resolve the blocked review"}"#,
    );

    submit_from_done_key(&mut state);

    assert!(
        state.modal.reason_step.is_some(),
        "[d] must offer a signature rather than a rejection"
    );
    assert!(state.pending_completions.is_empty());
    assert!(state.launched_completion_effects.is_empty());
    assert!(
        !state.activity_events().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                ticket_id,
                kind: CompletionRejectionKind::DispositionBlocked,
                ..
            } if ticket_id == TICKET_ID
        )),
        "the [d] path must not reject a ticket it can offer a signature for: {:?}",
        state.activity_log
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

/// N2: recovery is bounded and ends in a state that names its own way out.
///
/// The field shape, at the adapter: the completion fails on the condition its
/// own earlier success created, the operator signs it done, it fails again, and
/// before this ticket that could repeat forever. Two generations is the bound.
#[test]
fn repeated_done_key_stops_at_the_bound_and_names_the_command() {
    const TICKET: &str = "T-BOUNDED";
    const FIELD_STDERR: &str =
        "Error: ticket T-BOUNDED has no changes in the requested include paths";
    let (mut state, lease, _dir, _journal, _ledger) = completion_failure_fixture(TICKET);

    let disposition_ask = |state: &State| {
        let ReviewDisposition::Block { ask, .. } = parse_review_disposition(
            state
                .config
                .work_dir
                .join(TICKET)
                .join("review-disposition.json"),
        ) else {
            panic!("a failed completion must park with a block");
        };
        ask
    };

    // Generation 1: the scheduler's own attempt fails and parks.
    assert!(state.dispatch_completion(CompletionInput::Reconcile {
        ticket_id: TICKET.to_string(),
        source_lease: lease.clone(),
    }));
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    assert_eq!(state.launched_completion_effects.len(), 1);
    assert_eq!(
        state.completion_aggregates[TICKET].action_required_generations(),
        1
    );
    assert!(
        !disposition_ask(&state).contains("already-done"),
        "the first park still has an ordinary move"
    );

    // Generation 2: the operator signs it done, and it fails the same way.
    state.mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    assert_eq!(
        state.launched_completion_effects.len(),
        2,
        "the operator key must still work under the bound"
    );
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    assert_eq!(
        state.completion_aggregates[TICKET].action_required_generations(),
        2
    );

    // The bound. A third press launches nothing and says what does work.
    state.mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    assert_eq!(
        state.launched_completion_effects.len(),
        2,
        "past the bound the done key must not launch another completion"
    );
    assert_named_rejection_event_for(
        &state,
        TICKET,
        CompletionRejectionKind::LaunchFailed,
        "lisa already-done T-BOUNDED",
    );

    // It names the state it landed in, and the command that clears it.
    let ask = disposition_ask(&state);
    assert!(ask.contains("lisa already-done T-BOUNDED"), "{ask}");
    assert!(ask.contains("waiting"), "{ask}");
    assert_eq!(
        lisa_core::parking::validate_block_ask(&ask),
        Ok(()),
        "{ask}"
    );

    // And it is holding neither a seat nor a pane.
    let ticket = lisa_core::ticket::scan_tickets(&state.config.ticket_dir)
        .unwrap()
        .into_iter()
        .find(|listed| listed.id == TICKET)
        .unwrap();
    assert_eq!(ticket.status, TicketStatus::Blocked);
    assert!(!state.threads.contains_key(TICKET));
    assert!(state
        .agent_slots
        .iter()
        .all(|slot| slot.ticket_id.is_none()));
}

/// The loop stops re-attempting too — the "re-attempts on every loop start"
/// half of the field report, which lives in `reconciliation_state` rather than
/// in the done key.
#[test]
fn an_unpark_past_the_bound_does_not_re_arm_the_completion() {
    const TICKET: &str = "T-UNPARKED";
    const FIELD_STDERR: &str =
        "Error: ticket T-UNPARKED has no changes in the requested include paths";
    let (mut state, lease, _dir, _journal, _ledger) = completion_failure_fixture(TICKET);

    // `lisa unblock`'s flip, and the only part of it the scheduler sees:
    // blocked becomes open, the phase is left alone.
    let unpark = |state: &mut State| {
        lisa_core::ticket::update_ticket_status(
            state.config.ticket_dir.join(format!("{TICKET}.md")),
            TicketStatus::Open,
        )
        .unwrap();
        state.rebuild_dag();
    };

    // Generation 1 fails and parks.
    assert!(state.dispatch_completion(CompletionInput::Reconcile {
        ticket_id: TICKET.to_string(),
        source_lease: lease.clone(),
    }));
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    unpark(&mut state);

    // Under the bound, an unpark still re-arms — this is the behavior that made
    // the loop re-attempt on every start, kept deliberately for one more round.
    assert_eq!(
        state.completion_aggregates[TICKET].action_required_generations(),
        1
    );
    assert_eq!(
        state.reconciliation_state(TICKET),
        CompletionState::Eligible
    );

    // Generation 2: the operator signs it, it fails the same way, and parks.
    state.mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    unpark(&mut state);

    // Past the bound the same unpark no longer re-arms anything.
    assert_eq!(
        state.completion_aggregates[TICKET].action_required_generations(),
        2
    );
    assert!(matches!(
        state.reconciliation_state(TICKET),
        CompletionState::Rejected {
            retryability: Retryability::ActionRequired,
            ..
        }
    ));
    let launched = state.launched_completion_effects.len();
    assert_eq!(launched, 2);
    assert!(!state.dispatch_completion(CompletionInput::Reconcile {
        ticket_id: TICKET.to_string(),
        source_lease: lease,
    }));
    assert_eq!(state.launched_completion_effects.len(), launched);
}

/// Send-back is for disagreeing with a review. Past the bound the review is not
/// what is holding the ticket, so `[s]` declines rather than handing it a seat
/// and a pane to fail in again.
#[test]
fn send_back_declines_past_the_bound_and_points_at_the_command() {
    const TICKET: &str = "T-SENTBACK";
    const FIELD_STDERR: &str =
        "Error: ticket T-SENTBACK has no changes in the requested include paths";
    let (mut state, lease, _dir, _journal, _ledger) = completion_failure_fixture(TICKET);

    assert!(state.dispatch_completion(CompletionInput::Reconcile {
        ticket_id: TICKET.to_string(),
        source_lease: lease,
    }));
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    state.mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );
    assert!(state.recovery_generations_exhausted(TICKET));

    state.send_back_for_review(TICKET);

    let ticket = lisa_core::ticket::scan_tickets(&state.config.ticket_dir)
        .unwrap()
        .into_iter()
        .find(|listed| listed.id == TICKET)
        .unwrap();
    assert_eq!(
        ticket.status,
        TicketStatus::Blocked,
        "send-back must not reopen a ticket Lisa has stopped trying to record"
    );
    assert!(
        state.activity_events().any(|event| matches!(
            event,
            ActivityEvent::Warning { message }
                if message.contains("lisa already-done T-SENTBACK")
        )),
        "{:?}",
        state.activity_log
    );
}

/// A transport failure and a review judgement are different things, and the
/// record says which one it is in a field.
///
/// The field artifact this pins is verbatim from the 0.4.4 run:
/// `{"disposition":"block","reason":"… Error: ticket T-002-05 has no changes in
/// the requested include paths"}`. The operator read `block` as a verdict and
/// went looking for what was wrong with twelve recipes. Nothing was.
#[test]
fn a_recording_failure_is_not_a_reviewers_block() {
    const FIELD_STDERR: &str =
        "Error: ticket T-RECORDING has no changes in the requested include paths";
    let (mut state, lease, _dir, journal, _ledger) = completion_failure_fixture("T-RECORDING");
    assert!(state.dispatch_completion(CompletionInput::Reconcile {
        ticket_id: "T-RECORDING".to_string(),
        source_lease: lease,
    }));

    state.handle_completion_result(
        "T-RECORDING",
        Some(1),
        Vec::new(),
        FIELD_STDERR.as_bytes().to_vec(),
    );

    let published = state
        .config
        .work_dir
        .join("T-RECORDING")
        .join("review-disposition.json");
    let ReviewDisposition::Block {
        reason,
        ask,
        origin,
        remedy_owner,
        ..
    } = parse_review_disposition(&published)
    else {
        panic!("a failed completion must park with a block");
    };

    // (a) Separable by field, without reading a word of the prose.
    assert_eq!(origin, DispositionOrigin::InternalCommand);
    assert_eq!(remedy_owner, RemedyOwner::Operator);
    // (b) The reason is about the boundary, not about the work.
    assert!(!reason.contains(FIELD_STDERR), "{reason}");
    assert!(!reason.contains("include paths"), "{reason}");
    assert!(
        reason.contains("not a judgement about the work"),
        "{reason}"
    );
    // (c) The ask is a move a person can make.
    assert_eq!(
        lisa_core::parking::validate_block_ask(&ask),
        Ok(()),
        "{ask}"
    );
    // (d) The command's own text is moved, not lost.
    assert!(std::fs::read_to_string(&journal)
        .unwrap()
        .contains(FIELD_STDERR));

    // A reviewer's block on the same ticket stays a reviewer's block.
    write_canonical_review_disposition(
        &state,
        "T-RECORDING",
        r#"{"disposition":"block","reason":"the checkout test fails","remedy_owner":"agent","ask":"Fix the failing checkout test."}"#,
    );
    assert!(matches!(
        parse_review_disposition(&published),
        ReviewDisposition::Block {
            origin: DispositionOrigin::Review,
            ..
        }
    ));
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
