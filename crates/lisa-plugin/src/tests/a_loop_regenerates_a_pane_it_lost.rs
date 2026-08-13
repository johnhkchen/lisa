//! A loop that loses a pane puts it back, and does not resume the attempt that
//! died with it.
//!
//! `.lisa-layout.kdl` declares the coding panes and Zellij creates them once, at
//! `lisa loop`. The plugin had never created one since, so a lost pane was gone
//! for the session's lifetime. Measured on `screen-design` on 2026-08-13, after
//! two panes were lost to the probe `S-067-01` describes:
//!
//! ```text
//! lisa-5            → 4 children    healthy
//! screen-design-4   → 2 children    running on half its panes
//! ```
//!
//! `pane_count = max_threads * 2` — *"extra idle panes absorb new tickets while
//! finishing panes wind down"* — had silently become `max_threads * 1` with no
//! spare, and the only recovery was restarting the loop, which on that board
//! would have cost two agents mid-ticket.
//!
//! What these tests pin:
//!
//! 1. The loss is **noticed**, off the `PaneUpdate` Zellij already sends when
//!    the pane set changes, against the count the layout declared.
//! 2. The pane comes back **into the stack**, at the arrangement launch made,
//!    with the operator's focus handed back.
//! 3. Nothing in flight moves: surviving seats keep their panes, their tickets
//!    and their leases.
//! 4. A regenerated pane is a **fresh seat**. The attempt that died with the old
//!    pane is over and recorded as a lost seat (`T-067-01-02`), never quietly
//!    resumed in the new pane.
//! 5. Regeneration is **bounded**: three asks in ten minutes, then the loop says
//!    it has stopped and carries on with the panes it has.

use super::*;

use lisa_core::pane_heal::{
    publish_request, read_receipt, PaneHealAnswer, PaneHealRequest, PANE_HEAL_ANSWER_FILE,
};
use lisa_core::provenance::{ProvenanceLedgerRecord, ProvenanceRecord, RunOutcome};

/// The four coding panes `max_threads = 2` makes, plus the dashboard, as a
/// manifest.
///
/// `pane_y` is the stack's order, exactly as Zellij reports it: collapsed
/// members are one row of title bar and the expanded one holds the rest. The
/// dashboard is the plugin pane the census finds its tab by.
fn manifest(coding: &[u32], focused: Option<u32>, plugin_pane: u32) -> PaneManifest {
    let mut panes: Vec<PaneInfo> = coding
        .iter()
        .enumerate()
        .map(|(row, id)| pane_info(*id, false, row, focused == Some(*id)))
        .collect();
    panes.push(pane_info(plugin_pane, true, coding.len(), false));
    PaneManifest {
        panes: HashMap::from([(0, panes)]),
    }
}

fn pane_info(id: u32, is_plugin: bool, row: usize, is_focused: bool) -> PaneInfo {
    PaneInfo {
        id,
        is_plugin,
        is_focused,
        pane_y: row,
        pane_rows: 1,
        pane_columns: 80,
        ..PaneInfo::default()
    }
}

/// A board on four coding panes whose layout says so, with a real ledger.
fn four_pane_board() -> (State, tempfile::TempDir) {
    let (mut state, dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
    state.config.agent_panes = Some(4);
    state.config.max_threads = 2;
    state.ledger_path = dir.path().join(".lisa/provenance.jsonl");
    state.plugin_pane_id = Some(99);
    state.agent_slots.clear();
    for pane_id in [10, 11, 12, 13] {
        state.agent_slots.push(fresh_slot(pane_id, None));
    }
    (state, dir)
}

fn slot_panes(state: &State) -> Vec<u32> {
    state.agent_slots.iter().map(|slot| slot.pane_id).collect()
}

fn ledger_rows(state: &State) -> Vec<ProvenanceLedgerRecord> {
    std::fs::read_to_string(&state.ledger_path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("readable ledger row"))
        .collect()
}

/// The rows that end an attempt, which is the only kind healing writes.
fn terminal_rows(state: &State) -> Vec<ProvenanceRecord> {
    ledger_rows(state)
        .into_iter()
        .filter_map(|row| match row {
            ProvenanceLedgerRecord::Execution(exec) => Some(exec),
            _ => None,
        })
        .collect()
}

/// The reported board, healed: four panes declared, two present, and the loop
/// asks for one back without being polled.
#[test]
fn a_board_running_on_half_its_panes_asks_for_one_back() {
    let (mut state, _dir) = four_pane_board();

    // Two panes lost, exactly as measured. Focus is on the dashboard, where an
    // operator watching the run leaves it.
    state.census_panes(&manifest(&[10, 11], None, 99));

    assert_eq!(
        state.heal_census,
        heal::Census {
            declared: Some(4),
            present: 2
        }
    );
    assert_eq!(
        state.heal_calls,
        vec![HealCall::OpenedPane],
        "one ask per observation: the next decision is made from the manifest the pane arrives in"
    );
    assert_eq!(
        slot_panes(&state),
        vec![10, 11],
        "the slots for panes that no longer exist are gone, so nothing is launched into them"
    );
    assert!(state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Warning { message }
            if message.contains("2 of the 4 coding panes") && message.contains("attempt 1 of 3")
    )));
}

/// The pane arrives, joins the stack at the arrangement launch made, and the
/// operator gets their focus back.
#[test]
fn the_replacement_pane_joins_the_stack_and_gives_focus_back() {
    let (mut state, _dir) = four_pane_board();
    state.permissions_granted = true;

    // Pane 12 dies while the operator is reading pane 10.
    state.census_panes(&manifest(&[10, 11, 13], Some(10), 99));
    assert_eq!(state.heal_calls, vec![HealCall::OpenedPane]);
    assert_eq!(slot_panes(&state), vec![10, 11, 13]);

    // Zellij opens pane 20 wherever it likes — here at the bottom, outside the
    // stack's order — and focuses it, because that is what opening a pane does.
    state.census_panes(&manifest(&[10, 11, 13, 20], Some(20), 99));

    assert_eq!(
        state.heal_calls,
        vec![
            HealCall::OpenedPane,
            HealCall::Restacked(vec![10, 11, 13, 20]),
            HealCall::Refocused(10),
        ],
        "a replacement pane outside the stack is a new bug wearing the fix's clothes"
    );
    assert_eq!(slot_panes(&state), vec![10, 11, 13, 20]);
    assert!(state.heal_outstanding.is_none());
    assert!(state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Info { message }
            if message.contains("Put pane 20 back into the agent stack")
    )));

    // And it settles: a whole board asks for nothing more, however many
    // manifests arrive.
    let calls = state.heal_calls.len();
    for _ in 0..5 {
        state.census_panes(&manifest(&[10, 11, 13, 20], Some(10), 99));
    }
    assert_eq!(state.heal_calls.len(), calls);
}

/// The whole reason to heal rather than restart: the agents that survived keep
/// everything.
#[test]
fn the_seats_that_survived_keep_their_tickets_and_their_leases() {
    let (mut state, _dir) = four_pane_board();
    let working = AttemptLease::mint("T-ALIVE".to_string(), None).unwrap();
    state.agent_slots[0].ticket_id = Some("T-ALIVE".to_string());
    state.agent_slots[0].attempt_lease = Some(working.clone());
    state.agent_slots[0].has_session = true;
    state
        .current_leases
        .insert("T-ALIVE".to_string(), working.clone());
    state
        .threads
        .insert("T-ALIVE".to_string(), Thread::new("T-ALIVE", 10));

    state.census_panes(&manifest(&[10, 11, 12], None, 99));
    state.census_panes(&manifest(&[10, 11, 12, 20], None, 99));

    let survivor = &state.agent_slots[0];
    assert_eq!(survivor.pane_id, 10);
    assert_eq!(survivor.ticket_id.as_deref(), Some("T-ALIVE"));
    assert_eq!(survivor.attempt_lease.as_ref(), Some(&working));
    assert_eq!(state.current_leases["T-ALIVE"], working);
    assert_ne!(
        state.threads["T-ALIVE"].status,
        lisa_core::types::ThreadStatus::Failed,
        "healing a different pane must not touch an attempt in flight"
    );
    assert!(
        terminal_rows(&state).is_empty(),
        "nothing ended, nothing filed"
    );
}

/// A pane that dies with an agent in it ends that attempt, and the pane that
/// replaces it is a fresh seat rather than the dead one resumed.
#[test]
fn a_regenerated_pane_is_a_fresh_seat_and_the_lost_attempt_is_recorded() {
    let (mut state, _dir) = four_pane_board();
    let lost = AttemptLease::mint("T-LOST".to_string(), None).unwrap();
    state.agent_slots[2].ticket_id = Some("T-LOST".to_string());
    state.agent_slots[2].attempt_lease = Some(lost.clone());
    state.agent_slots[2].has_session = true;
    state.agent_slots[2].last_client = Some(AgentClient::Claude);
    state
        .current_leases
        .insert("T-LOST".to_string(), lost.clone());
    let mut thread = Thread::new("T-LOST", 12);
    thread.attempt_lease = Some(lost.clone());
    state.threads.insert("T-LOST".to_string(), thread);

    // Pane 12 goes away under the agent working T-LOST.
    state.census_panes(&manifest(&[10, 11, 13], None, 99));

    let rows = terminal_rows(&state);
    assert_eq!(rows.len(), 1, "one row, for one lost seat: {rows:?}");
    assert_eq!(rows[0].ticket_id, "T-LOST");
    assert_eq!(rows[0].outcome, RunOutcome::SeatLost);
    assert_eq!(rows[0].attempt_lease, lost);
    assert_eq!(rows[0].pane_id, 12);
    assert!(rows[0]
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("pane 12 closed")));
    assert_eq!(
        state.current_leases.get("T-LOST"),
        None,
        "the attempt that died with the pane is over"
    );
    assert_eq!(
        state.threads["T-LOST"].status,
        lisa_core::types::ThreadStatus::Failed
    );

    // The replacement arrives idle: no ticket, no lease, no session. Healing
    // must not paper over a lost attempt by handing its seat on.
    state.census_panes(&manifest(&[10, 11, 13, 20], None, 99));
    let adopted = state
        .agent_slots
        .iter()
        .find(|slot| slot.pane_id == 20)
        .expect("the replacement pane became a seat");
    assert_eq!(adopted.ticket_id, None);
    assert_eq!(adopted.attempt_lease, None);
    assert!(!adopted.has_session);
    assert_eq!(adopted.last_client, None);
    assert_eq!(adopted.transition_state, TransitionState::Idle);
    assert_eq!(terminal_rows(&state).len(), 1, "adoption files nothing");
}

/// An ending Lisa already recorded is not recorded twice.
///
/// `fail_startup_recovery` and the hard-silence fence both write their row and
/// *then* close the pane, so the pane's disappearance arrives here after the
/// attempt is already over. Matching the slot's lease against `current_leases`
/// is what tells the two apart.
#[test]
fn a_pane_lisa_closed_itself_does_not_file_a_second_lost_seat() {
    let (mut state, _dir) = four_pane_board();
    let fenced = AttemptLease::mint("T-FENCED".to_string(), None).unwrap();
    state.agent_slots[1].ticket_id = Some("T-FENCED".to_string());
    state.agent_slots[1].attempt_lease = Some(fenced);
    state.agent_slots[1].transition_state = TransitionState::Fenced;
    state
        .threads
        .insert("T-FENCED".to_string(), Thread::new("T-FENCED", 11));
    // The lease was revoked before the pane was closed, which every terminal
    // path does and this one relies on.
    assert!(state.current_leases.is_empty());

    state.census_panes(&manifest(&[10, 12, 13], None, 99));

    assert!(
        terminal_rows(&state).is_empty(),
        "the row for this ending was written before the fence destroyed its inputs"
    );
    assert_eq!(slot_panes(&state), vec![10, 12, 13]);
    // And the capacity the fence took away is asked for back, which is the
    // dominant way a `lisa` board loses panes at all.
    assert_eq!(state.heal_calls, vec![HealCall::OpenedPane]);
}

/// A pane that dies the instant it is created must not spin.
#[test]
fn a_pane_that_dies_on_arrival_stops_after_three_asks() {
    let (mut state, _dir) = four_pane_board();

    for _ in 0..8 {
        // Each round: the board is short, Lisa asks, and the pane never appears.
        state.census_panes(&manifest(&[10, 11, 12], None, 99));
        // The ask ages out, so the next observation is free to decide again.
        if let Some(ask) = state.heal_outstanding.as_mut() {
            ask.asked_at = ask.asked_at.saturating_sub(heal::ADOPTION_TIMEOUT_SECS);
        }
    }

    assert_eq!(
        state
            .heal_calls
            .iter()
            .filter(|call| **call == HealCall::OpenedPane)
            .count(),
        heal::MAX_REGENERATIONS,
        "the bound is three asks, not three asks per observation"
    );
    assert!(state.heal_budget.has_given_up());
    let give_ups = state
        .activity_events()
        .filter(|event| {
            matches!(event, ActivityEvent::Error { message }
                if message.contains("not asking again"))
        })
        .count();
    assert_eq!(give_ups, 1, "said once, not once per manifest");
    assert!(state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::Error { message }
            if message.contains("3 times in 10 minutes") && message.contains("restart the")
    )));
}

/// A run launched from a layout that never said how many panes it made invents
/// none.
#[test]
fn a_layout_that_never_declared_its_panes_heals_nothing() {
    let (mut state, _dir) = four_pane_board();
    state.config.agent_panes = None;

    state.census_panes(&manifest(&[10, 11], None, 99));

    assert!(state.heal_calls.is_empty());
    assert!(state.heal_census.is_whole());
    // The slots for the vanished panes still go: an idle seat pointing at a
    // pane that does not exist is wrong whether or not it can be replaced.
    assert_eq!(slot_panes(&state), vec![10, 11]);
}

/// A manifest that does not describe the tab Lisa knows decides nothing.
#[test]
fn an_unrecognised_manifest_retires_nothing() {
    let (mut state, _dir) = four_pane_board();

    // No plugin pane in it at all: a manifest that arrived before the plugin
    // knew its own id, or one caught mid-relayout.
    let mut orphan = manifest(&[10, 11], None, 99);
    orphan.panes.insert(0, Vec::new());
    state.census_panes(&orphan);
    assert_eq!(slot_panes(&state), vec![10, 11, 12, 13]);
    assert!(state.heal_calls.is_empty());

    // Every coding pane gone at once is not four deaths.
    state.census_panes(&manifest(&[], None, 99));
    assert_eq!(slot_panes(&state), vec![10, 11, 12, 13]);
    assert!(state.heal_calls.is_empty());

    // An operator's own tab is not Lisa's panes.
    let mut other_tab = manifest(&[10, 11, 12, 13], None, 99);
    other_tab
        .panes
        .insert(1, vec![pane_info(77, false, 0, true)]);
    state.census_panes(&other_tab);
    assert_eq!(slot_panes(&state), vec![10, 11, 12, 13]);
    assert!(state.heal_calls.is_empty());
}

/// `rail` asks, and gets one of exactly three answers.
mod the_ask {
    use super::*;

    fn asked(state: &mut State, root: &std::path::Path, nonce: &str) {
        state.heal_root = root.to_path_buf();
        publish_request(root, &PaneHealRequest::new(nonce, 0, "rail")).unwrap();
        state.check_pane_heal_requests();
    }

    #[test]
    fn asked_and_already_fine() {
        let (mut state, dir) = four_pane_board();
        state.census_panes(&manifest(&[10, 11, 12, 13], None, 99));

        asked(&mut state, dir.path(), "n-fine");

        let receipt = read_receipt(dir.path(), "n-fine").expect("an ask is always answered");
        assert_eq!(receipt.answer, PaneHealAnswer::AlreadyFine);
        assert_eq!(receipt.declared, Some(4));
        assert_eq!(receipt.present, 4);
        assert!(receipt.detail.contains("Nothing to do"));
        assert!(
            state.heal_calls.is_empty(),
            "asking a whole board creates nothing"
        );
    }

    #[test]
    fn asked_and_healed_once_the_pane_exists() {
        let (mut state, dir) = four_pane_board();
        state.census_panes(&manifest(&[10, 11, 12], None, 99));
        // The automatic ask already went out; the request arrives while it is
        // outstanding, which is the ordinary race.
        assert_eq!(state.heal_calls, vec![HealCall::OpenedPane]);

        asked(&mut state, dir.path(), "n-heal");
        assert!(
            read_receipt(dir.path(), "n-heal").is_none(),
            "'asked and healed' is a claim about a pane that exists, so the receipt waits"
        );

        state.census_panes(&manifest(&[10, 11, 12, 20], None, 99));

        let receipt = read_receipt(dir.path(), "n-heal").expect("the arrival answers the ask");
        assert_eq!(receipt.answer, PaneHealAnswer::Healed);
        assert_eq!(receipt.present, 4);
        assert!(receipt.detail.contains("Put a pane back"));
    }

    #[test]
    fn asked_and_refused_when_the_loop_has_stopped_trying() {
        let (mut state, dir) = four_pane_board();
        for _ in 0..heal::MAX_REGENERATIONS {
            state.census_panes(&manifest(&[10, 11, 12], None, 99));
            if let Some(ask) = state.heal_outstanding.as_mut() {
                ask.asked_at = ask.asked_at.saturating_sub(heal::ADOPTION_TIMEOUT_SECS);
            }
        }
        assert!(state.heal_budget.has_given_up());
        let asks_before = state.heal_calls.len();

        asked(&mut state, dir.path(), "n-refused");

        let receipt = read_receipt(dir.path(), "n-refused").expect("a refusal is still an answer");
        assert_eq!(receipt.answer, PaneHealAnswer::Refused);
        assert!(
            receipt.detail.contains("Restart the loop"),
            "a refusal names the way through: {}",
            receipt.detail
        );
        assert_eq!(
            state.heal_calls.len(),
            asks_before,
            "a refusal creates nothing"
        );
    }

    #[test]
    fn asked_and_refused_when_the_layout_never_said() {
        let (mut state, dir) = four_pane_board();
        state.config.agent_panes = None;
        state.census_panes(&manifest(&[10, 11], None, 99));

        asked(&mut state, dir.path(), "n-undeclared");

        let receipt = read_receipt(dir.path(), "n-undeclared").expect("an ask is always answered");
        assert_eq!(receipt.answer, PaneHealAnswer::Refused);
        assert!(receipt
            .detail
            .contains("does not say how many coding panes"));
    }

    #[test]
    fn an_ask_that_finds_a_short_board_starts_the_regeneration_itself() {
        // The point of the ask: whoever noticed first does not have to wait for
        // the next PaneUpdate. Here the board went short before this scheduler
        // was watching, so nothing has asked yet.
        let (mut state, dir) = four_pane_board();
        state.heal_census = heal::Census {
            declared: Some(4),
            present: 3,
        };
        state.agent_slots.pop();

        asked(&mut state, dir.path(), "n-start");

        assert_eq!(state.heal_calls, vec![HealCall::OpenedPane]);
        assert!(read_receipt(dir.path(), "n-start").is_none());
        state.census_panes(&manifest(&[10, 11, 12, 20], None, 99));
        assert_eq!(
            read_receipt(dir.path(), "n-start").map(|receipt| receipt.answer),
            Some(PaneHealAnswer::Healed)
        );
    }

    #[test]
    fn an_ask_is_consumed_once_so_a_second_scheduler_does_not_answer_it() {
        let (mut state, dir) = four_pane_board();
        state.census_panes(&manifest(&[10, 11, 12, 13], None, 99));
        asked(&mut state, dir.path(), "n-once");
        std::fs::remove_file(dir.path().join(PANE_HEAL_ANSWER_FILE)).unwrap();

        state.check_pane_heal_requests();

        assert!(
            read_receipt(dir.path(), "n-once").is_none(),
            "the request was taken by the first reader and is not there to answer twice"
        );
    }
}
