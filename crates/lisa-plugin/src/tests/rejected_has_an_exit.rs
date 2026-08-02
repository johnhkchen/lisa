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

use lisa_cli::already_done::{run_already_done, AlreadyDoneOutcome, AlreadyDoneRequest};
use lisa_cli::commit_transaction::{complete_ticket, CompleteTicketRequest};
use lisa_core::completion::completion_key_marker;
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
