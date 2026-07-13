use lisa_core::{
    completion::{
        reconcile, reduce, AttemptId, CompletionDeadline, CompletionEvent, CompletionId,
        CompletionState, CorrelationId, CurrentLeaseArtifactAdmission, DurableCompletionInputs,
        EffectCommand, Reconciliation, Transition,
    },
    disposition::ReviewDisposition,
};

const ATTEMPT: &str = "T-009-01-01/attempt-1";
const COMPLETION: &str = "T-009-01-01/completion-1";
const CORRELATION: &str = "T-009-01-01/manual-result";
const RECORDED_REVIEW_TIMEOUT_SECS: u64 = 10 * 60;
const RECONCILIATION_NOW_MILLIS: u64 = 100;
const RECONCILIATION_DEADLINE_MILLIS: u64 = 200;

#[derive(Debug, Clone, Copy)]
enum RecordedEvent {
    ReviewArtifactWritten,
    PhaseAdvancedToReview,
    StopObserved,
    ReviewTimeoutElapsed { elapsed_secs: u64 },
    Reloaded,
    ManualCompletionConfirmed,
}

fn recorded_t_009_01_01_trace() -> [RecordedEvent; 6] {
    [
        RecordedEvent::ReviewArtifactWritten,
        RecordedEvent::PhaseAdvancedToReview,
        RecordedEvent::StopObserved,
        RecordedEvent::ReviewTimeoutElapsed {
            elapsed_secs: RECORDED_REVIEW_TIMEOUT_SECS,
        },
        RecordedEvent::Reloaded,
        RecordedEvent::ManualCompletionConfirmed,
    ]
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Observation {
    requests: usize,
    confirmations: usize,
    finish_up_prompts: usize,
    re_requests: usize,
}

struct AggregateReplay {
    phase_is_review: bool,
    review_artifact_present: bool,
    state: CompletionState,
    observation: Observation,
}

impl AggregateReplay {
    fn new() -> Self {
        Self {
            phase_is_review: false,
            review_artifact_present: false,
            state: CompletionState::Eligible,
            observation: Observation::default(),
        }
    }

    fn replay(mut self, trace: &[RecordedEvent]) -> (Observation, CompletionState) {
        for event in trace {
            self.apply(*event);
        }
        (self.observation, self.state)
    }

    fn apply(&mut self, event: RecordedEvent) {
        match event {
            RecordedEvent::ReviewArtifactWritten => {
                self.review_artifact_present = true;
            }
            RecordedEvent::PhaseAdvancedToReview => {
                self.phase_is_review = true;
                self.reconcile_obligation();
            }
            RecordedEvent::StopObserved | RecordedEvent::Reloaded => {
                self.reconcile_obligation();
            }
            RecordedEvent::ReviewTimeoutElapsed { elapsed_secs } => {
                assert_eq!(elapsed_secs, RECORDED_REVIEW_TIMEOUT_SECS);
                if !self.review_artifact_present {
                    self.observation.finish_up_prompts += 1;
                }
                self.reconcile_obligation();
            }
            RecordedEvent::ManualCompletionConfirmed => {
                let transition = self.reduce(CompletionEvent::CommandSucceeded {
                    correlation: CorrelationId::new(CORRELATION),
                });
                assert_eq!(transition.effect, None);
                assert_eq!(transition.state, CompletionState::Confirmed);
                self.state = transition.state;
                self.observation.confirmations += 1;
                self.reconcile_obligation();
            }
        }
    }

    fn reconcile_obligation(&mut self) {
        if !self.phase_is_review {
            return;
        }

        let durable_inputs = DurableCompletionInputs {
            artifact_admission: self.review_artifact_present.then(|| {
                CurrentLeaseArtifactAdmission {
                    attempt_id: AttemptId::new(ATTEMPT),
                    completion_id: CompletionId::new(COMPLETION),
                }
            }),
            disposition: ReviewDisposition::Pass,
        };

        match reconcile(
            &durable_inputs,
            &self.state,
            CompletionDeadline::from_unix_millis(RECONCILIATION_NOW_MILLIS),
        ) {
            Reconciliation::Effect(effect) => self.apply_request(effect),
            Reconciliation::None => {}
            Reconciliation::ReplayCommandInFlight {
                correlation,
                deadline,
            } => {
                assert_eq!(correlation, CorrelationId::new(CORRELATION));
                assert_eq!(deadline.unix_millis(), RECONCILIATION_DEADLINE_MILLIS);
            }
            Reconciliation::CommandInFlightDeadlineExceeded { .. } => {
                panic!("recorded reconciliation time must remain before the deadline")
            }
        }
    }

    fn apply_request(&mut self, reconciled_effect: EffectCommand) {
        if self.observation.requests > 0 {
            self.observation.re_requests += 1;
            return;
        }

        let transition = self.reduce(CompletionEvent::Request {
            attempt_id: AttemptId::new(ATTEMPT),
            completion_id: CompletionId::new(COMPLETION),
        });
        assert_eq!(transition.state, CompletionState::Requested);
        assert_eq!(transition.effect, Some(reconciled_effect));
        self.state = transition.state;
        self.observation.requests += 1;

        let transition = self.reduce(CompletionEvent::CommandLaunched {
            correlation: CorrelationId::new(CORRELATION),
            deadline: CompletionDeadline::from_unix_millis(RECONCILIATION_DEADLINE_MILLIS),
        });
        assert_eq!(transition.effect, None);
        assert_eq!(
            transition.state,
            CompletionState::CommandInFlight {
                correlation: CorrelationId::new(CORRELATION),
                deadline: CompletionDeadline::from_unix_millis(RECONCILIATION_DEADLINE_MILLIS,),
            }
        );
        self.state = transition.state;
    }

    fn reduce(&mut self, event: CompletionEvent) -> Transition {
        reduce(
            std::mem::replace(&mut self.state, CompletionState::Eligible),
            event,
        )
        .expect("the recorded trace must contain only legal aggregate transitions")
    }
}

fn replay_naive_edge_triggered_stub(trace: &[RecordedEvent]) -> Observation {
    let mut phase_is_review = false;
    let mut request_pending = false;
    let mut observation = Observation::default();

    for event in trace {
        match event {
            RecordedEvent::ReviewArtifactWritten if phase_is_review => {
                request_pending = true;
                observation.requests += 1;
            }
            RecordedEvent::PhaseAdvancedToReview => phase_is_review = true,
            RecordedEvent::ReviewTimeoutElapsed { elapsed_secs } => {
                assert_eq!(*elapsed_secs, RECORDED_REVIEW_TIMEOUT_SECS);
                if !request_pending {
                    observation.finish_up_prompts += 1;
                }
            }
            RecordedEvent::ManualCompletionConfirmed => observation.confirmations += 1,
            RecordedEvent::ReviewArtifactWritten
            | RecordedEvent::StopObserved
            | RecordedEvent::Reloaded => {}
        }
    }

    observation
}

#[test]
fn recorded_t_009_01_01_trace_converges_once_without_finish_up_or_rerequest() {
    let trace = recorded_t_009_01_01_trace();
    let expected = Observation {
        requests: 1,
        confirmations: 1,
        finish_up_prompts: 0,
        re_requests: 0,
    };

    let naive = replay_naive_edge_triggered_stub(&trace);
    assert_ne!(
        naive, expected,
        "the fixture must catch edge-triggered logic"
    );
    assert_eq!(
        naive,
        Observation {
            requests: 0,
            confirmations: 1,
            finish_up_prompts: 1,
            re_requests: 0,
        },
        "artifact-before-phase must reproduce the missed request and stale finish-up"
    );

    let (actual, final_state) = AggregateReplay::new().replay(&trace);
    assert_eq!(actual, expected);
    assert_eq!(final_state, CompletionState::Confirmed);
}
