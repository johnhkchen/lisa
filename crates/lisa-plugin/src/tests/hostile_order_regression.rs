use super::*;

use lisa_cli::commit_transaction::{complete_ticket, CompleteTicketRequest};
use lisa_core::provenance::{ProvenanceLedgerRecord, RunOutcome};
use lisa_core::types::{Phase, Thread, TicketStatus};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PRIMARY: &str = "T-ARCADE-PRIMARY";
const DEPENDENT: &str = "T-ARCADE-DEPENDENT";
const PRIMARY_PANE: u32 = 41;
const SPARE_PANE: u32 = 42;

struct NestedRepo {
    temp: tempfile::TempDir,
}

impl NestedRepo {
    fn new() -> Self {
        let repo = Self {
            temp: tempfile::tempdir().unwrap(),
        };
        repo.git(["init", "--quiet"]);
        repo.git(["config", "user.name", "Lisa Test"]);
        repo.git(["config", "user.email", "lisa@example.test"]);
        repo
    }

    fn root(&self) -> &Path {
        self.temp.path()
    }

    fn project_root(&self) -> PathBuf {
        self.root().join("games/midsummer")
    }

    fn write(&self, relative: impl AsRef<Path>, body: &str) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn git<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn git_string<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        String::from_utf8(self.git(args).stdout)
            .unwrap()
            .trim()
            .to_string()
    }

    fn commit_count(&self) -> u64 {
        self.git_string(["rev-list", "--count", "HEAD"])
            .parse()
            .unwrap()
    }
}

struct Scenario {
    repo: NestedRepo,
    state: State,
    lease: AttemptLease,
    baseline_head: String,
    baseline_count: u64,
}

impl Scenario {
    fn new(disposition: &str) -> Self {
        let repo = NestedRepo::new();
        let project_root = repo.project_root();
        let tickets_dir = project_root.join("docs/active/tickets");
        let work_dir = project_root.join("docs/active/work");
        repo.write(
            "games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md",
            &format!(
                "---\nid: {PRIMARY}\ntitle: hostile-order-primary\ntype: task\nstatus: open\npriority: high\nagent: codex\nphase: implement\n---\n\nArcade hostile-order fixture.\n"
            ),
        );
        repo.write(
            "games/midsummer/docs/active/tickets/T-ARCADE-DEPENDENT.md",
            &format!(
                "---\nid: {DEPENDENT}\ntitle: hostile-order-dependent\ntype: task\nstatus: open\npriority: high\nagent: codex\nphase: ready\ndepends_on: [{PRIMARY}]\n---\n\nDependent fixture.\n"
            ),
        );
        repo.git(["add", "games/midsummer/docs/active/tickets"]);
        repo.git(["commit", "--quiet", "-m", "fixture baseline"]);
        let baseline_head = repo.git_string(["rev-parse", "HEAD"]);
        let baseline_count = repo.commit_count();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let lisa_dir = project_root.join(".lisa");
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                lisa_bin: Some("lisa".to_string()),
                client: AgentClient::Codex,
                max_threads: 2,
                review_timeout_secs: 1,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            project_root: project_root.clone(),
            git_root: repo.root().to_path_buf(),
            attempt_dir: lisa_dir.join("attempts"),
            signal_dir: lisa_dir.join("signals"),
            ledger_path: lisa_dir.join("provenance.jsonl"),
            completion_journal_path: lisa_dir.join("completion-journal.jsonl"),
            completion_journal_healthy: true,
            codex_dir: lisa_dir.join("codex"),
            claude_dir: lisa_dir.join("claude"),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };

        let mut thread = Thread::new(PRIMARY, PRIMARY_PANE);
        thread.current_phase = Phase::Implement;
        thread.client = AgentClient::Codex;
        state.threads.insert(PRIMARY.to_string(), thread);

        let mut primary_slot = fresh_slot(PRIMARY_PANE, Some(AgentClient::Codex));
        primary_slot.ticket_id = Some(PRIMARY.to_string());
        primary_slot.transition_state = TransitionState::Idle;
        primary_slot.transition_started_at = Some(std::time::SystemTime::now());
        state.agent_slots.push(primary_slot);
        state
            .agent_slots
            .push(fresh_slot(SPARE_PANE, Some(AgentClient::Codex)));

        let lease = install_current_attempt(&mut state, PRIMARY);
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(
            staged.join("review.md"),
            "# Review\n\nThe hostile-order fixture is ready.\n",
        )
        .unwrap();
        write_review_disposition(&state, &lease, disposition);

        Self {
            repo,
            state,
            lease,
            baseline_head,
            baseline_count,
        }
    }

    fn age_review(&mut self) {
        let expired = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        let thread = self.state.threads.get_mut(PRIMARY).unwrap();
        thread.last_phase_change = expired;
        thread.last_activity = expired;
    }

    fn submit_done_key(&mut self) {
        assert!(self.state.handle_key(KeyWithModifier {
            bare_key: BareKey::Char('d'),
            key_modifiers: Default::default(),
        }));
        self.state.modal.cursor = self
            .state
            .modal
            .ticket_ids
            .iter()
            .position(|ticket_id| ticket_id == PRIMARY)
            .expect("primary Review is selectable from [d]one");
        assert!(self.state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));
    }

    fn assert_no_finish_up(&self) {
        assert!(!self.state.finish_up_sent.contains(PRIMARY));
        assert!(!self.state.activity_events().any(|event| matches!(
            event,
            ActivityEvent::FinishUpPromptSent { ticket_id, .. } if ticket_id == PRIMARY
        )));
    }

    fn restart(&self) -> State {
        let mut restarted = State {
            config: self.state.config.clone(),
            project_root: self.state.project_root.clone(),
            git_root: self.state.git_root.clone(),
            attempt_dir: self.state.attempt_dir.clone(),
            signal_dir: self.state.signal_dir.clone(),
            ledger_path: self.state.ledger_path.clone(),
            completion_journal_path: self.state.completion_journal_path.clone(),
            codex_dir: self.state.codex_dir.clone(),
            claude_dir: self.state.claude_dir.clone(),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        restarted.restore_completion_journal();
        restarted.rebuild_dag();

        let mut thread = Thread::new(PRIMARY, PRIMARY_PANE);
        thread.current_phase = Phase::Review;
        thread.client = AgentClient::Codex;
        thread.attempt_lease = Some(self.lease.clone());
        restarted.threads.insert(PRIMARY.to_string(), thread);
        restarted
            .lease_high_water
            .insert(PRIMARY.to_string(), self.lease.clone());
        restarted
            .current_leases
            .insert(PRIMARY.to_string(), self.lease.clone());

        let mut primary_slot = fresh_slot(PRIMARY_PANE, Some(AgentClient::Codex));
        primary_slot.ticket_id = Some(PRIMARY.to_string());
        primary_slot.attempt_lease = Some(self.lease.clone());
        restarted.agent_slots.push(primary_slot);
        restarted
            .agent_slots
            .push(fresh_slot(SPARE_PANE, Some(AgentClient::Codex)));
        restarted
    }
}

fn option(argv: &[String], name: &str) -> String {
    argv.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .unwrap_or_else(|| panic!("completion argv is missing {name}"))
}

fn transaction_request(state: &State, key: &CompletionGenerationId) -> CompleteTicketRequest {
    let ticket_file = state
        .dag
        .get_ticket(&PRIMARY.to_string())
        .unwrap()
        .file_path
        .clone();
    let (argv, context) = state.build_completion_command(key, &ticket_file).unwrap();
    assert_eq!(Path::new(&option(&argv, "--path")), state.git_root);
    assert_eq!(
        option(&argv, "--ticket-file"),
        "games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md"
    );
    assert_eq!(
        option(&argv, "--work-dir"),
        "games/midsummer/docs/active/work/T-ARCADE-PRIMARY"
    );
    assert_eq!(
        context.get("lisa_completion").map(String::as_str),
        Some(PRIMARY)
    );
    CompleteTicketRequest {
        repo_root: PathBuf::from(option(&argv, "--path")),
        ticket_id: option(&argv, "--ticket-id"),
        message: option(&argv, "--message"),
        ticket_file: PathBuf::from(option(&argv, "--ticket-file")),
        work_dir: PathBuf::from(option(&argv, "--work-dir")),
        completion_key: key.clone(),
    }
}

struct LostResultFixture {
    scenario: Scenario,
    original_pending: PendingCompletion,
    original_effect: EffectCommand,
    prior_commit_id: String,
}

impl LostResultFixture {
    fn new() -> Self {
        let mut scenario = Scenario::new(r#"{"disposition":"pass","reason":null}"#);
        scenario.state.check_artifact_advances();

        assert_eq!(scenario.state.threads[PRIMARY].current_phase, Phase::Review);
        assert_eq!(scenario.state.launched_completion_effects.len(), 1);
        let original_effect = scenario.state.launched_completion_effects[0].clone();
        let original_pending = scenario.state.pending_completions[PRIMARY].clone();
        let aggregate = &scenario.state.completion_aggregates[PRIMARY];
        assert_eq!(aggregate.completion_key(), &original_pending.completion_key);
        assert_eq!(
            aggregate.state(),
            &CompletionState::CommandInFlight {
                correlation: original_pending.correlation.clone(),
                deadline: original_pending.deadline,
            }
        );
        assert_eq!(
            fs::read_to_string(&scenario.state.completion_journal_path)
                .unwrap()
                .lines()
                .count(),
            2
        );

        let first = complete_ticket(transaction_request(
            &scenario.state,
            &original_pending.completion_key,
        ))
        .unwrap();
        assert_eq!(scenario.repo.commit_count(), scenario.baseline_count + 1);
        assert_eq!(
            scenario.repo.git_string(["rev-parse", "HEAD^"]),
            scenario.baseline_head
        );
        assert!(scenario
            .repo
            .git_string([
                "show",
                "HEAD:games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md"
            ])
            .contains("phase: done"));
        assert_eq!(
            fs::read_to_string(&scenario.state.completion_journal_path)
                .unwrap()
                .lines()
                .count(),
            2,
            "the fixture intentionally loses the first successful result"
        );
        assert!(!scenario.state.ledger_path.exists());

        Self {
            scenario,
            original_pending,
            original_effect,
            prior_commit_id: first.commit_id,
        }
    }

    fn restart_in_flight(&self) -> State {
        let restarted = self.scenario.restart();
        assert!(restarted.completion_journal_healthy);
        assert!(!restarted.pending_completions.contains_key(PRIMARY));
        let aggregate = &restarted.completion_aggregates[PRIMARY];
        assert_eq!(
            aggregate.completion_key(),
            &self.original_pending.completion_key
        );
        assert_eq!(
            aggregate.state(),
            &CompletionState::CommandInFlight {
                correlation: self.original_pending.correlation.clone(),
                deadline: self.original_pending.deadline,
            }
        );
        assert_eq!(
            restarted.reconciliation_state(PRIMARY),
            aggregate.state().clone()
        );
        let scanned = restarted.dag.get_ticket(&PRIMARY.to_string()).unwrap();
        assert_eq!(scanned.phase, self.original_pending.prior_phase);
        assert_eq!(scanned.status, self.original_pending.prior_status);
        restarted
    }

    fn replay_time(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(
                self.original_pending
                    .deadline
                    .unix_millis()
                    .saturating_sub(1),
            )
    }

    fn start_replay(&self, restarted: &mut State) {
        assert!(restarted.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: PRIMARY.to_string(),
                source_lease: self.scenario.lease.clone(),
            },
            self.replay_time(),
        ));
        assert_eq!(
            restarted.launched_completion_effects,
            vec![self.original_effect.clone()]
        );
        let replay = &restarted.pending_completions[PRIMARY];
        assert_eq!(replay.completion_key, self.original_pending.completion_key);
        assert_eq!(replay.correlation, self.original_pending.correlation);
        assert_eq!(replay.deadline, self.original_pending.deadline);
        assert!(replay.is_reconciliation_replay);
        assert_eq!(
            fs::read_to_string(&restarted.completion_journal_path)
                .unwrap()
                .lines()
                .count(),
            2,
            "reconstruction replay must retain the original journal intent"
        );
    }

    fn converge(&self, restarted: &mut State) {
        let replay = complete_ticket(transaction_request(
            restarted,
            &self.original_pending.completion_key,
        ))
        .unwrap();
        assert_eq!(replay.commit_id, self.prior_commit_id);
        assert!(replay.committed_paths.is_empty());
        assert_eq!(
            self.scenario.repo.commit_count(),
            self.scenario.baseline_count + 1,
            "same-generation replay must not create a second completion commit"
        );

        restarted.handle_completion_result(
            PRIMARY,
            Some(0),
            replay.commit_id.as_bytes().to_vec(),
            Vec::new(),
        );
        assert!(!restarted.pending_completions.contains_key(PRIMARY));
        let aggregate = &restarted.completion_aggregates[PRIMARY];
        assert_eq!(aggregate.state(), &CompletionState::Confirmed);
        assert_eq!(
            aggregate.confirmed_commit_id(),
            Some(self.prior_commit_id.as_str())
        );

        let journal = fs::read_to_string(&restarted.completion_journal_path).unwrap();
        assert_eq!(journal.lines().count(), 3);
        assert_eq!(journal.matches("\"state\":\"requested\"").count(), 1);
        assert_eq!(
            journal.matches("\"state\":\"command-in-flight\"").count(),
            1
        );
        assert_eq!(journal.matches("\"state\":\"confirmed\"").count(), 1);

        let records = read_mixed_ledger(&restarted.ledger_path);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0],
            ProvenanceLedgerRecord::Execution(record)
                if record.ticket_id == PRIMARY
                    && record.outcome == RunOutcome::Done
                    && record.authoritative
        ));
        assert_eq!(
            self.scenario.repo.commit_count(),
            self.scenario.baseline_count + 1
        );
    }
}

#[test]
fn plugin_restart_reconstruction_fixture_converges_on_single_prior_commit() {
    let fixture = LostResultFixture::new();
    let mut restarted = fixture.restart_in_flight();

    fixture.start_replay(&mut restarted);
    fixture.converge(&mut restarted);

    let confirmed_restart = fixture.scenario.restart();
    let aggregate = &confirmed_restart.completion_aggregates[PRIMARY];
    assert_eq!(aggregate.state(), &CompletionState::Confirmed);
    assert_eq!(
        aggregate.confirmed_commit_id(),
        Some(fixture.prior_commit_id.as_str())
    );
    assert_eq!(
        confirmed_restart.reconciliation_state(PRIMARY),
        CompletionState::Confirmed
    );
}

#[test]
fn lost_result_duplicate_stop_fixture_converges_on_single_prior_commit() {
    let fixture = LostResultFixture::new();
    let mut restarted = fixture.restart_in_flight();
    let original_journal = fs::read_to_string(&restarted.completion_journal_path).unwrap();

    restarted.handle_stopped_signal(PRIMARY_PANE);
    restarted.handle_stopped_signal(PRIMARY_PANE);
    assert!(restarted.pending_completions.is_empty());
    assert!(restarted.launched_completion_effects.is_empty());
    assert_eq!(
        fs::read_to_string(&restarted.completion_journal_path).unwrap(),
        original_journal
    );

    fixture.start_replay(&mut restarted);
    restarted.handle_stopped_signal(PRIMARY_PANE);
    restarted.handle_stopped_signal(PRIMARY_PANE);
    assert!(!restarted.dispatch_completion_at(
        CompletionInput::Reconcile {
            ticket_id: PRIMARY.to_string(),
            source_lease: fixture.scenario.lease.clone(),
        },
        fixture.replay_time(),
    ));
    assert_eq!(restarted.launched_completion_effects.len(), 1);
    assert_eq!(
        fs::read_to_string(&restarted.completion_journal_path).unwrap(),
        original_journal
    );

    fixture.converge(&mut restarted);
    let final_journal = fs::read_to_string(&restarted.completion_journal_path).unwrap();
    let final_ledger = fs::read_to_string(&restarted.ledger_path).unwrap();
    restarted.handle_completion_result(
        PRIMARY,
        Some(0),
        fixture.prior_commit_id.as_bytes().to_vec(),
        Vec::new(),
    );
    assert_eq!(
        fs::read_to_string(&restarted.completion_journal_path).unwrap(),
        final_journal
    );
    assert_eq!(
        fs::read_to_string(&restarted.ledger_path).unwrap(),
        final_ledger
    );
    assert_eq!(
        fixture.scenario.repo.commit_count(),
        fixture.scenario.baseline_count + 1
    );
}

#[test]
fn passing_review_hostile_order_converges_once_and_schedules_dependent() {
    let mut scenario = Scenario::new(r#"{"disposition":"pass","reason":null}"#);

    scenario.state.check_artifact_advances();
    assert_eq!(scenario.state.threads[PRIMARY].current_phase, Phase::Review);
    let ticket =
        fs::read_to_string(scenario.state.config.ticket_dir.join("T-ARCADE-PRIMARY.md")).unwrap();
    assert!(ticket.contains("phase: review"));
    assert!(!ticket.contains("phase: done"));
    assert_eq!(scenario.state.launched_completion_effects.len(), 1);
    let initial_effect = scenario.state.launched_completion_effects[0].clone();
    let pending = scenario.state.pending_completions[PRIMARY].clone();
    assert_eq!(
        fs::read_to_string(&scenario.state.completion_journal_path)
            .unwrap()
            .lines()
            .count(),
        2
    );

    // A second `.stopped` while the completion is already in flight must not
    // launch another one, and drives no transition of its own.
    scenario.state.handle_stopped_signal(PRIMARY_PANE);
    assert_eq!(
        scenario.state.agent_slots[0].transition_state,
        TransitionState::Idle
    );
    assert_eq!(scenario.state.launched_completion_effects.len(), 1);

    scenario.age_review();
    scenario.state.check_review_timeouts();
    scenario.assert_no_finish_up();

    scenario.submit_done_key();
    assert_eq!(scenario.state.launched_completion_effects.len(), 1);
    assert!(matches!(
        scenario.state.modal.operator_outcome,
        Some(OperatorModalOutcome::Rejected {
            kind: CompletionRejectionKind::AlreadyPending,
            ..
        })
    ));

    let request = transaction_request(&scenario.state, &pending.completion_key);
    let first = complete_ticket(request).unwrap();
    assert_eq!(scenario.repo.commit_count(), scenario.baseline_count + 1);
    assert_eq!(
        scenario.repo.git_string(["rev-parse", "HEAD^"]),
        scenario.baseline_head
    );
    assert!(scenario
        .repo
        .git_string([
            "show",
            "HEAD:games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md"
        ])
        .contains("phase: done"));
    assert_eq!(
        fs::read_to_string(&scenario.state.completion_journal_path)
            .unwrap()
            .lines()
            .count(),
        2,
        "the first successful result is intentionally delayed across reload"
    );
    assert!(!scenario.state.ledger_path.exists());

    let mut restarted = scenario.restart();
    let scanned = restarted.dag.get_ticket(&PRIMARY.to_string()).unwrap();
    assert_eq!(scanned.phase, Phase::Review);
    assert_eq!(scanned.status, TicketStatus::Open);

    restarted.handle_stopped_signal(PRIMARY_PANE);
    assert!(restarted.launched_completion_effects.is_empty());
    let replay_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_001);
    assert!(restarted.dispatch_completion_at(
        CompletionInput::Reconcile {
            ticket_id: PRIMARY.to_string(),
            source_lease: scenario.lease.clone(),
        },
        replay_time,
    ));
    assert_eq!(restarted.launched_completion_effects, vec![initial_effect]);
    assert_eq!(
        restarted.pending_completions[PRIMARY].completion_key,
        pending.completion_key
    );
    assert!(restarted.pending_completions[PRIMARY].is_reconciliation_replay);

    restarted.handle_stopped_signal(PRIMARY_PANE);
    assert!(!restarted.dispatch_completion_at(
        CompletionInput::Reconcile {
            ticket_id: PRIMARY.to_string(),
            source_lease: scenario.lease.clone(),
        },
        replay_time + std::time::Duration::from_secs(1),
    ));
    assert_eq!(restarted.launched_completion_effects.len(), 1);
    assert_eq!(
        fs::read_to_string(&restarted.completion_journal_path)
            .unwrap()
            .lines()
            .count(),
        2
    );

    let replay = complete_ticket(transaction_request(&restarted, &pending.completion_key)).unwrap();
    assert_eq!(replay.commit_id, first.commit_id);
    assert!(replay.committed_paths.is_empty());
    assert_eq!(scenario.repo.commit_count(), scenario.baseline_count + 1);

    restarted.handle_completion_result(
        PRIMARY,
        Some(0),
        replay.commit_id.as_bytes().to_vec(),
        Vec::new(),
    );
    restarted.handle_completion_result(
        PRIMARY,
        Some(0),
        replay.commit_id.as_bytes().to_vec(),
        Vec::new(),
    );

    let journal = fs::read_to_string(&restarted.completion_journal_path).unwrap();
    assert_eq!(journal.lines().count(), 3);
    assert_eq!(journal.matches("\"state\":\"confirmed\"").count(), 1);
    let records = read_mixed_ledger(&restarted.ledger_path);
    assert_eq!(records.len(), 1);
    assert!(matches!(
        &records[0],
        ProvenanceLedgerRecord::Execution(record)
            if record.ticket_id == PRIMARY
                && record.outcome == RunOutcome::Done
                && record.authoritative
    ));
    assert!(!restarted.threads.contains_key(PRIMARY));
    assert!(!restarted.current_leases.contains_key(PRIMARY));
    assert!(restarted
        .agent_slots
        .iter()
        .all(|slot| slot.ticket_id.as_deref() != Some(PRIMARY)));
    assert!(restarted.threads.contains_key(DEPENDENT));
    assert!(restarted
        .agent_slots
        .iter()
        .any(|slot| slot.ticket_id.as_deref() == Some(DEPENDENT)));
    assert!(!restarted.finish_up_sent.contains(PRIMARY));
    assert!(!restarted.activity_events().any(|event| matches!(
        event,
        ActivityEvent::FinishUpPromptSent { ticket_id, .. } if ticket_id == PRIMARY
    )));
}

#[test]
fn blocked_review_hostile_order_has_no_completion_side_effects() {
    let reason = "resolve the blocked Arcade review";
    let mut scenario = Scenario::new(&format!(r#"{{"disposition":"block","reason":"{reason}"}}"#));

    scenario.state.check_artifact_advances();
    assert_eq!(scenario.state.threads[PRIMARY].current_phase, Phase::Review);
    assert!(scenario.state.pending_completions.is_empty());
    assert!(scenario.state.launched_completion_effects.is_empty());
    assert!(!scenario.state.completion_journal_path.exists());

    scenario.state.handle_stopped_signal(PRIMARY_PANE);
    assert!(!scenario
        .state
        .dispatch_completion(CompletionInput::Reconcile {
            ticket_id: PRIMARY.to_string(),
            source_lease: scenario.lease.clone(),
        }));
    scenario.age_review();
    scenario.state.check_review_timeouts();
    scenario.submit_done_key();

    assert!(scenario.state.pending_completions.is_empty());
    assert!(scenario.state.launched_completion_effects.is_empty());
    assert!(!scenario.state.completion_journal_path.exists());
    assert!(!scenario.state.ledger_path.exists());
    assert_eq!(
        scenario.repo.git_string(["rev-parse", "HEAD"]),
        scenario.baseline_head
    );
    assert_eq!(scenario.repo.commit_count(), scenario.baseline_count);
    let ticket =
        fs::read_to_string(scenario.state.config.ticket_dir.join("T-ARCADE-PRIMARY.md")).unwrap();
    assert!(ticket.contains("phase: review"));
    assert!(!ticket.contains("phase: done"));
    assert_eq!(
        scenario.state.agent_slots[0].ticket_id.as_deref(),
        Some(PRIMARY)
    );
    assert_eq!(
        scenario.state.current_leases.get(PRIMARY),
        Some(&scenario.lease)
    );
    assert!(scenario.state.threads.contains_key(PRIMARY));
    assert!(!scenario.state.threads.contains_key(DEPENDENT));
    assert!(scenario.state.activity_events().any(|event| matches!(
        event,
        ActivityEvent::CompletionRejected {
            kind: CompletionRejectionKind::DispositionBlocked,
            detail,
            ..
        } if detail.contains(reason)
    )));
    scenario.assert_no_finish_up();
}
