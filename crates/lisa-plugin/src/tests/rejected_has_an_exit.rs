//! The 0.4.4 field sequence, end to end, against a real repository.
//!
//! One completion loses its race and is rejected `action-required` while the
//! work it was sealing is sitting in history under an earlier key. The board
//! then reads blocked at Review, the journal reads rejected, and before this
//! ticket no documented command could make them agree — the operator's actual
//! escape was to move the ticket out of `docs/active/tickets/` so Lisa would
//! stop looking at it.
//!
//! This drives the whole thing: the lost race, the rejection, the second
//! rejection that reaches the bound, and the recovery.

use super::*;

use lisa_cli::already_done::{
    run_already_done, AlreadyDoneOutcome, AlreadyDoneRequest, SealSource,
};
use lisa_cli::commit_transaction::{complete_ticket, CompleteTicketRequest};
use lisa_core::completion::{completion_key_marker, COMPLETION_KEY_PREFIX};
use lisa_core::completion_journal::MAX_ACTION_REQUIRED_GENERATIONS;
use lisa_core::types::{Phase, Thread, TicketStatus};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TICKET: &str = "T-LOST-RACE";
const PANE: u32 = 61;
/// The exact refusal the field run hit, over and over.
const EMPTY_INCLUDE_DIFF: &str =
    "Error: ticket T-LOST-RACE has no changes in the requested include paths";

struct Field {
    _temp: tempfile::TempDir,
    root: PathBuf,
    state: State,
    lease: AttemptLease,
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn head_message(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([OsStr::new("log"), OsStr::new("--format=%B")])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

impl Field {
    /// A board whose one ticket is in Review, in a repository with history.
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("field run");
        let tickets_dir = root.join("docs/active/tickets");
        let work_dir = root.join("docs/active/work");
        fs::create_dir_all(work_dir.join(TICKET)).unwrap();
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{TICKET}.md")),
            format!(
                "---\nid: {TICKET}\ntitle: lost-race\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n\nField fixture.\n"
            ),
        )
        .unwrap();
        fs::write(
            work_dir.join(TICKET).join("review.md"),
            "# Review\n\nTwelve recipes, all fine.\n",
        )
        .unwrap();

        git(&root, &["init", "--quiet", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Lisa Test"]);
        git(&root, &["config", "user.email", "lisa@example.test"]);
        git(&root, &["add", "docs"]);
        git(&root, &["commit", "--quiet", "-m", "fixture baseline"]);

        let lisa_dir = root.join(".lisa");
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                lisa_bin: Some("lisa".to_string()),
                ..PluginConfig::new()
            },
            project_root: root.clone(),
            git_root: root.clone(),
            attempt_dir: lisa_dir.join("attempts"),
            ledger_path: lisa_dir.join("provenance.jsonl"),
            completion_journal_path: root
                .join(lisa_core::completion_journal::COMPLETION_JOURNAL_RELATIVE_PATH),
            completion_journal_healthy: true,
            ..State::default()
        };
        fs::create_dir_all(&lisa_dir).unwrap();

        let mut thread = Thread::new(TICKET, PANE);
        thread.current_phase = Phase::Review;
        state.threads.insert(TICKET.to_string(), thread);
        let mut slot = fresh_slot(PANE, Some(AgentClient::Codex));
        slot.ticket_id = Some(TICKET.to_string());
        state.agent_slots.push(slot);
        let lease = install_current_attempt(&mut state, TICKET);
        fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        write_private_review(&state, &lease);

        Self {
            _temp: temp,
            root,
            state,
            lease,
        }
    }

    fn journal_bytes(&self) -> String {
        fs::read_to_string(&self.state.completion_journal_path).unwrap_or_default()
    }

    fn board_ticket(&self) -> lisa_core::types::Ticket {
        lisa_core::ticket::scan_tickets(&self.state.config.ticket_dir)
            .unwrap()
            .into_iter()
            .find(|listed| listed.id == TICKET)
            .unwrap()
    }

    /// What a fresh plugin process would read from the journal alone.
    fn after_restart(&self) -> CompletionJournalAggregate {
        completion_journal::load(&self.state.completion_journal_path)
            .unwrap()
            .remove(TICKET)
            .expect("the journal must retain this completion")
    }

    fn recover(&self) -> Result<AlreadyDoneOutcome, String> {
        run_already_done(
            AlreadyDoneRequest {
                project_root: &self.root,
                ticket_dir: &self.state.config.ticket_dir,
                work_dir: &self.state.config.work_dir,
                journal_path: &self.state.completion_journal_path,
            },
            TICKET,
        )
    }
}

#[test]
fn the_lost_race_that_could_not_be_recovered_now_can_be() {
    let mut field = Field::new();
    let sealing_key = CompletionGenerationId::new(
        CompletionId::new(TICKET),
        AttemptId::new(field.lease.attempt_id.to_string()),
        1,
    );
    let correlation = CorrelationId::new(sealing_key.to_string());

    // 1. The completion launches and goes in-flight.
    for transition in [
        CompletionJournalTransition::Requested {
            key: sealing_key.clone(),
            prior_phase: Phase::Review,
            prior_status: TicketStatus::Open,
            note: None,
        },
        CompletionJournalTransition::CommandInFlight {
            key: sealing_key.clone(),
            correlation: correlation.clone(),
            deadline: CompletionDeadline::from_unix_millis(1),
        },
    ] {
        completion_journal::append_with_seal(
            &field.state.completion_journal_path,
            CompletionSeal::Commit,
            transition,
        )
        .unwrap();
    }
    field.state.restore_completion_journal();

    // 2. The command runs and the commit lands — this is the work that is
    //    verifiably at HEAD for the rest of the sequence. Invoking the
    //    plumbing directly still works as plumbing, and still writes no
    //    journal transition, which is the third field gap: an operator could
    //    seal the work and still not convince Lisa of it.
    let journal_before_plumbing = field.journal_bytes();
    let sealed = complete_ticket(CompleteTicketRequest {
        repo_root: field.root.clone(),
        ticket_id: TICKET.to_string(),
        message: format!("Complete {TICKET}"),
        ticket_file: PathBuf::from("docs/active/tickets").join(format!("{TICKET}.md")),
        work_dir: PathBuf::from("docs/active/work").join(TICKET),
        completion_key: sealing_key.clone(),
    })
    .unwrap();
    assert!(head_message(&field.root).contains(&completion_key_marker(&sealing_key)));
    assert_eq!(
        field.journal_bytes(),
        journal_before_plumbing,
        "the plumbing must not write the journal — that is the adapter's job"
    );

    // 3. But the result never gets back before the reconciliation deadline.
    //    The completion is rejected action-required with the work committed.
    assert!(field.state.expire_in_flight_completion(
        TICKET,
        correlation,
        CompletionDeadline::from_unix_millis(1),
    ));
    assert!(matches!(
        field.after_restart().state(),
        CompletionState::Rejected {
            retryability: Retryability::ActionRequired,
            ..
        }
    ));
    // The exact field disagreement: history says done, the board says waiting.
    assert_eq!(field.board_ticket().status, TicketStatus::Blocked);
    assert_eq!(field.board_ticket().phase, Phase::Review);
    assert!(field.after_restart().masks_durable_done());

    // 4. The operator signs it done. It fails on the condition its own earlier
    //    success created, and that spends the last generation.
    field
        .state
        .mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    field.state.handle_completion_result(
        TICKET,
        Some(1),
        Vec::new(),
        EMPTY_INCLUDE_DIFF.as_bytes().to_vec(),
    );
    assert_eq!(
        field.after_restart().action_required_generations(),
        MAX_ACTION_REQUIRED_GENERATIONS
    );
    // The generation that spends the last one is the one whose row stops
    // saying "let Lisa try again" and names the command that ends it.
    assert!(
        field
            .journal_bytes()
            .contains(&format!("`{ALREADY_DONE_COMMAND} {TICKET}`")),
        "the rejection at the bound must name the action: {}",
        field.journal_bytes()
    );

    // 5. The old escape is closed off, and says what to run instead.
    let launched = field.state.launched_completion_effects.len();
    field
        .state
        .mark_ticket_done_with_override(TICKET, OverrideReason::EvidenceSatisfies);
    assert_eq!(
        field.state.launched_completion_effects.len(),
        launched,
        "past the bound nothing may be re-attempted"
    );

    // 6. The recovery. One command, and it reports the commit that was already
    //    there rather than making a new one.
    let commits_before = commit_count(&field.root);
    let outcome = field.recover().unwrap();
    assert_eq!(
        outcome,
        AlreadyDoneOutcome::Recovered {
            ticket_id: TICKET.to_string(),
            commit_id: sealed.commit_id.clone(),
            ticket_file_rewritten: true,
            seal: SealSource::Adopted,
        }
    );
    assert_eq!(commit_count(&field.root), commits_before);

    // 7. Terminal in the journal, and the board agrees with it.
    let recovered = field.after_restart();
    assert_eq!(recovered.state(), &CompletionState::Confirmed);
    assert_eq!(
        recovered.confirmed_commit_id(),
        Some(sealed.commit_id.as_str())
    );
    assert!(
        !recovered.masks_durable_done(),
        "a confirmed completion must stop masking the board's Done"
    );
    assert_eq!(recovered.completion_key().attempt_id().as_str(), "operator");
    assert_eq!(field.board_ticket().phase, Phase::Done);
    assert_eq!(field.board_ticket().status, TicketStatus::Done);

    // 8. And a plugin that restarts onto this journal reads it the same way,
    //    rather than fencing on a record it cannot replay.
    let mut restarted = State {
        completion_journal_path: field.state.completion_journal_path.clone(),
        ..State::default()
    };
    restarted.restore_completion_journal();
    assert!(restarted.completion_journal_healthy);
    assert_eq!(
        restarted.completion_aggregates[TICKET].state(),
        &CompletionState::Confirmed
    );

    // 9. Running it again is a no-op that says so.
    assert!(matches!(
        field.recover().unwrap(),
        AlreadyDoneOutcome::Declined(message) if message.contains("already finished")
    ));
}

/// A killed `already-done` must not leave a fresh dead end behind it.
///
/// The command writes three records, so there are two windows where a killed
/// process leaves a half-written recovery generation. That half-written state
/// reads as `Requested`, which masks the board's Done — exactly the shape of
/// the problem this command exists to remove. Running it again finishes the
/// generation it started rather than refusing it.
#[test]
fn an_interrupted_recovery_is_finished_by_running_it_again() {
    for stop_after in [1, 2] {
        let mut field = Field::new();
        let sealing_key = CompletionGenerationId::new(
            CompletionId::new(TICKET),
            AttemptId::new(field.lease.attempt_id.to_string()),
            1,
        );
        let correlation = CorrelationId::new(sealing_key.to_string());
        for transition in [
            CompletionJournalTransition::Requested {
                key: sealing_key.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
            CompletionJournalTransition::CommandInFlight {
                key: sealing_key.clone(),
                correlation: correlation.clone(),
                deadline: CompletionDeadline::from_unix_millis(1),
            },
        ] {
            completion_journal::append_with_seal(
                &field.state.completion_journal_path,
                CompletionSeal::Commit,
                transition,
            )
            .unwrap();
        }
        field.state.restore_completion_journal();
        let sealed = complete_ticket(CompleteTicketRequest {
            repo_root: field.root.clone(),
            ticket_id: TICKET.to_string(),
            message: format!("Complete {TICKET}"),
            ticket_file: PathBuf::from("docs/active/tickets").join(format!("{TICKET}.md")),
            work_dir: PathBuf::from("docs/active/work").join(TICKET),
            completion_key: sealing_key,
        })
        .unwrap();
        assert!(field.state.expire_in_flight_completion(
            TICKET,
            correlation,
            CompletionDeadline::from_unix_millis(1),
        ));

        // Simulate the interruption by writing only the first `stop_after`
        // records of the recovery generation.
        let recovery_key = CompletionGenerationId::new(
            CompletionId::new(TICKET),
            AttemptId::new("operator"),
            field.after_restart().completion_key().generation() + 1,
        );
        let recovery_correlation = CorrelationId::new(recovery_key.to_string());
        let partial = [
            CompletionJournalTransition::Requested {
                key: recovery_key.clone(),
                prior_phase: field.after_restart().prior_phase(),
                prior_status: field.after_restart().prior_status(),
                note: None,
            },
            CompletionJournalTransition::CommandInFlight {
                key: recovery_key,
                correlation: recovery_correlation,
                deadline: CompletionDeadline::from_unix_millis(0),
            },
        ];
        for transition in partial.into_iter().take(stop_after) {
            completion_journal::append_with_seal(
                &field.state.completion_journal_path,
                CompletionSeal::Commit,
                transition,
            )
            .unwrap();
        }
        assert!(
            field.after_restart().masks_durable_done(),
            "a half-written recovery is exactly the state that hides Done"
        );

        let outcome = field.recover().unwrap();

        assert!(
            matches!(outcome, AlreadyDoneOutcome::Recovered { ref commit_id, .. }
                if commit_id == &sealed.commit_id),
            "stopped after {stop_after}: {outcome:?}"
        );
        assert_eq!(field.after_restart().state(), &CompletionState::Confirmed);
        assert_eq!(field.board_ticket().status, TicketStatus::Done);
    }
}

/// The 2026-08-13 `renderer` sequence: the completion **commit itself** fails.
///
/// The lost race above at least left a sealed commit behind to be adopted.
/// Here nothing carries the key, because the thing that failed is the thing
/// that would have written it — and the ticket then sat on a board reporting
/// it ready, beside a run that would not take it, for two days. Five
/// documented recoveries were run against it and none applied.
///
/// The escape is one command, and it changes no file by hand.
#[test]
fn a_completion_whose_commit_never_landed_is_finished_by_one_command() {
    let mut field = Field::new();
    let sealing_key = CompletionGenerationId::new(
        CompletionId::new(TICKET),
        AttemptId::new(field.lease.attempt_id.to_string()),
        1,
    );
    let correlation = CorrelationId::new(sealing_key.to_string());

    // 1. The completion launches and goes in-flight, and its commit never
    //    lands. No `complete_ticket` here: that is the whole difference.
    for transition in [
        CompletionJournalTransition::Requested {
            key: sealing_key.clone(),
            prior_phase: Phase::Review,
            prior_status: TicketStatus::Open,
            note: None,
        },
        CompletionJournalTransition::CommandInFlight {
            key: sealing_key.clone(),
            correlation: correlation.clone(),
            deadline: CompletionDeadline::from_unix_millis(1),
        },
    ] {
        completion_journal::append_with_seal(
            &field.state.completion_journal_path,
            CompletionSeal::Commit,
            transition,
        )
        .unwrap();
    }
    field.state.restore_completion_journal();

    // 2. The reconciliation deadline passes and the completion parks
    //    action-required, with nothing in history to show for it.
    assert!(field.state.expire_in_flight_completion(
        TICKET,
        correlation,
        CompletionDeadline::from_unix_millis(1),
    ));
    assert!(matches!(
        field.after_restart().state(),
        CompletionState::Rejected {
            retryability: Retryability::ActionRequired,
            ..
        }
    ));
    assert!(
        !head_message(&field.root).contains(COMPLETION_KEY_PREFIX),
        "the premise is that no commit carries this ticket's key"
    );
    // The rejection Lisa wrote down names the command that clears it.
    // `action-required` is the field that says a person must act, and the row
    // carrying it is where that person is standing.
    assert!(
        field
            .journal_bytes()
            .contains(&format!("run: `lisa unblock {TICKET}`")),
        "an action-required rejection must name the action: {}",
        field.journal_bytes()
    );

    // 3. One command. It writes the seal that failed rather than refusing for
    //    want of one.
    let commits_before = commit_count(&field.root);
    let outcome = field.recover().unwrap();
    let AlreadyDoneOutcome::Recovered {
        commit_id,
        ticket_file_rewritten,
        seal,
        ..
    } = outcome
    else {
        panic!("the field case must be recoverable: {outcome:?}");
    };
    assert_eq!(seal, SealSource::Written);
    assert_eq!(commit_count(&field.root), commits_before + 1);

    // 4. The seal is a real completion commit, under the operator's own
    //    generation, and it carries the Done bytes — so no file is left for a
    //    person to commit by hand.
    let recovery_key = CompletionGenerationId::new(
        CompletionId::new(TICKET),
        AttemptId::new("operator"),
        sealing_key.generation() + 1,
    );
    assert!(head_message(&field.root).contains(&completion_key_marker(&recovery_key)));
    assert!(!ticket_file_rewritten);
    assert_eq!(field.board_ticket().phase, Phase::Done);
    assert_eq!(field.board_ticket().status, TicketStatus::Done);
    assert_eq!(
        porcelain(&field.root, "docs/active/tickets"),
        "",
        "the seal must commit the ticket file it rewrote"
    );

    // 5. Terminal in the journal, replayable by a fresh plugin, and no longer
    //    masking the board's Done — so nothing declines to schedule it.
    let recovered = field.after_restart();
    assert_eq!(recovered.state(), &CompletionState::Confirmed);
    assert_eq!(recovered.confirmed_commit_id(), Some(commit_id.as_str()));
    assert!(!recovered.masks_durable_done());
    let mut restarted = State {
        completion_journal_path: field.state.completion_journal_path.clone(),
        ..State::default()
    };
    restarted.restore_completion_journal();
    assert!(restarted.completion_journal_healthy);

    // 6. And it is done: a second run has nothing left to settle.
    assert!(matches!(
        field.recover().unwrap(),
        AlreadyDoneOutcome::Declined(message) if message.contains("already finished")
    ));
}

fn porcelain(root: &Path, pathspec: &str) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "--", pathspec])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn commit_count(root: &Path) -> usize {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
