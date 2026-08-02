use lisa_core::completion::{
    reconcile, reduce, AttemptId, CompletionDeadline, CompletionEvent, CompletionId,
    CompletionState, CorrelationId, CurrentLeaseArtifactAdmission, DurableCompletionInputs,
    EffectCommand, Reconciliation,
};
use lisa_core::disposition::{RemedyOwner, ReviewDisposition};
use proptest::prelude::*;
use proptest_state_machine::{prop_state_machine, ReferenceStateMachine, StateMachineTest};

const ATTEMPT: &str = "generated-attempt";
const COMPLETION: &str = "generated-completion";
const CORRELATION: &str = "generated-command";
const NOW_MILLIS: u64 = 100;
const DEADLINE_MILLIS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScenarioEvent {
    ObservePassingReview,
    ObserveBlockedReview,
    EnterReviewPhase,
    StopBeforePoll,
    Poll,
    DuplicateResult,
    Reload,
    Timeout,
    ManualRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelDisposition {
    Pass,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelState {
    disposition: Option<ModelDisposition>,
    review_phase: bool,
    live_effects: usize,
    effects_issued: usize,
    authoritative_done: usize,
}

impl ModelState {
    fn clean() -> Self {
        Self {
            disposition: None,
            review_phase: false,
            live_effects: 0,
            effects_issued: 0,
            authoritative_done: 0,
        }
    }

    fn artifact_admitted(&self) -> bool {
        self.review_phase && self.disposition.is_some()
    }

    fn reconcile(&mut self) {
        if self.artifact_admitted()
            && self.disposition == Some(ModelDisposition::Pass)
            && self.live_effects == 0
            && self.authoritative_done == 0
        {
            self.live_effects = 1;
            self.effects_issued += 1;
        }
    }

    fn accept_result(&mut self) {
        if self.live_effects == 1 {
            self.live_effects = 0;
            self.authoritative_done += 1;
        }
    }

    fn apply_observation(&mut self, event: ScenarioEvent) {
        match event {
            ScenarioEvent::ObservePassingReview if self.disposition.is_none() => {
                self.disposition = Some(ModelDisposition::Pass);
            }
            ScenarioEvent::ObserveBlockedReview if self.disposition.is_none() => {
                self.disposition = Some(ModelDisposition::Block);
            }
            ScenarioEvent::EnterReviewPhase => self.review_phase = true,
            ScenarioEvent::DuplicateResult | ScenarioEvent::ManualRecovery => {
                self.accept_result();
            }
            ScenarioEvent::StopBeforePoll
            | ScenarioEvent::Poll
            | ScenarioEvent::Reload
            | ScenarioEvent::Timeout
            | ScenarioEvent::ObservePassingReview
            | ScenarioEvent::ObserveBlockedReview => {}
        }
        self.reconcile();
    }
}

struct CompletionReferenceMachine;

impl ReferenceStateMachine for CompletionReferenceMachine {
    type State = ModelState;
    type Transition = ScenarioEvent;

    fn init_state() -> BoxedStrategy<Self::State> {
        Just(ModelState::clean()).boxed()
    }

    fn transitions(_state: &Self::State) -> BoxedStrategy<Self::Transition> {
        prop_oneof![
            Just(ScenarioEvent::ObservePassingReview),
            Just(ScenarioEvent::ObserveBlockedReview),
            Just(ScenarioEvent::EnterReviewPhase),
            Just(ScenarioEvent::StopBeforePoll),
            Just(ScenarioEvent::Poll),
            Just(ScenarioEvent::DuplicateResult),
            Just(ScenarioEvent::Reload),
            Just(ScenarioEvent::Timeout),
            Just(ScenarioEvent::ManualRecovery),
        ]
        .boxed()
    }

    fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
        state.apply_observation(*transition);
        state
    }
}

#[derive(Debug)]
struct CompletionHarness {
    disposition: Option<ModelDisposition>,
    review_phase: bool,
    state: CompletionState,
    live_effects: usize,
    effects_issued: usize,
    authoritative_done: usize,
}

impl CompletionHarness {
    fn clean() -> Self {
        Self {
            disposition: None,
            review_phase: false,
            state: CompletionState::Eligible,
            live_effects: 0,
            effects_issued: 0,
            authoritative_done: 0,
        }
    }

    fn artifact_admitted(&self) -> bool {
        self.review_phase && self.disposition.is_some()
    }

    fn durable_inputs(&self) -> DurableCompletionInputs {
        DurableCompletionInputs {
            artifact_admission: self
                .artifact_admitted()
                .then(|| CurrentLeaseArtifactAdmission {
                    attempt_id: AttemptId::new(ATTEMPT),
                    completion_id: CompletionId::new(COMPLETION),
                }),
            disposition: match self.disposition {
                Some(ModelDisposition::Pass) => ReviewDisposition::Pass,
                Some(ModelDisposition::Block) => ReviewDisposition::Block {
                    reason: "generated blocked Review".into(),
                    remedy_owner: RemedyOwner::Operator,
                    ask: "generated blocked Review".into(),
                    steps: None,
                    check: None,
                    unstructured: true,
                    origin: lisa_core::disposition::DispositionOrigin::Review,
                },
                None => ReviewDisposition::Invalid {
                    reason: "Review not observed".into(),
                },
            },
        }
    }

    fn reconcile(&mut self) {
        match reconcile(
            &self.durable_inputs(),
            &self.state,
            CompletionDeadline::from_unix_millis(NOW_MILLIS),
        ) {
            Reconciliation::Effect(effect) => {
                assert_eq!(
                    effect,
                    EffectCommand::LaunchCompletion {
                        attempt_id: AttemptId::new(ATTEMPT),
                        completion_id: CompletionId::new(COMPLETION),
                    }
                );
                let transition = reduce(
                    self.state.clone(),
                    CompletionEvent::Request {
                        attempt_id: AttemptId::new(ATTEMPT),
                        completion_id: CompletionId::new(COMPLETION),
                    },
                )
                .expect("reconciled completion request must be accepted");
                assert_eq!(transition.effect, Some(effect));
                self.state = transition.state;
                self.live_effects += 1;
                self.effects_issued += 1;

                let launched = reduce(
                    self.state.clone(),
                    CompletionEvent::CommandLaunched {
                        correlation: CorrelationId::new(CORRELATION),
                        deadline: CompletionDeadline::from_unix_millis(DEADLINE_MILLIS),
                    },
                )
                .expect("new completion effect must accept its launch correlation");
                assert_eq!(launched.effect, None);
                self.state = launched.state;
            }
            Reconciliation::ReplayCommandInFlight {
                correlation,
                deadline,
            } => {
                assert_eq!(correlation, CorrelationId::new(CORRELATION));
                assert_eq!(deadline.unix_millis(), DEADLINE_MILLIS);
                assert_eq!(self.live_effects, 1);
            }
            Reconciliation::CommandInFlightDeadlineExceeded { .. } => {
                panic!("generated reconciliation time must remain before the deadline")
            }
            Reconciliation::None => {}
        }
    }

    fn present_result(&mut self) {
        let prior = self.state.clone();
        match reduce(
            prior.clone(),
            CompletionEvent::CommandSucceeded {
                correlation: CorrelationId::new(CORRELATION),
            },
        ) {
            Ok(transition) => {
                assert_eq!(transition.state, CompletionState::Confirmed);
                assert_eq!(transition.effect, None);
                assert_eq!(self.live_effects, 1);
                self.live_effects = 0;
                self.authoritative_done += 1;
                self.state = transition.state;
            }
            Err(_) => self.state = prior,
        }
    }

    fn reload(&mut self) {
        self.state = if self.authoritative_done == 1 {
            CompletionState::Confirmed
        } else if self.live_effects == 1 {
            CompletionState::CommandInFlight {
                correlation: CorrelationId::new(CORRELATION),
                deadline: CompletionDeadline::from_unix_millis(DEADLINE_MILLIS),
            }
        } else {
            CompletionState::Eligible
        };
    }

    fn apply_observation(&mut self, event: ScenarioEvent) {
        match event {
            ScenarioEvent::ObservePassingReview if self.disposition.is_none() => {
                self.disposition = Some(ModelDisposition::Pass);
            }
            ScenarioEvent::ObserveBlockedReview if self.disposition.is_none() => {
                self.disposition = Some(ModelDisposition::Block);
            }
            ScenarioEvent::EnterReviewPhase => self.review_phase = true,
            ScenarioEvent::DuplicateResult | ScenarioEvent::ManualRecovery => {
                self.present_result();
            }
            ScenarioEvent::Reload => self.reload(),
            ScenarioEvent::StopBeforePoll
            | ScenarioEvent::Poll
            | ScenarioEvent::Timeout
            | ScenarioEvent::ObservePassingReview
            | ScenarioEvent::ObserveBlockedReview => {}
        }
        self.reconcile();
    }
}

struct CompletionStateMachineTest;

impl StateMachineTest for CompletionStateMachineTest {
    type SystemUnderTest = CompletionHarness;
    type Reference = CompletionReferenceMachine;

    fn init_test(_ref_state: &ModelState) -> Self::SystemUnderTest {
        CompletionHarness::clean()
    }

    fn apply(
        mut state: Self::SystemUnderTest,
        ref_state: &ModelState,
        transition: ScenarioEvent,
    ) -> Self::SystemUnderTest {
        state.apply_observation(transition);
        assert_eq!(state.disposition, ref_state.disposition);
        assert_eq!(state.review_phase, ref_state.review_phase);
        state
    }

    fn check_invariants(state: &Self::SystemUnderTest, ref_state: &ModelState) {
        assert_eq!(state.live_effects, ref_state.live_effects);
        assert_eq!(state.effects_issued, ref_state.effects_issued);
        assert_eq!(state.authoritative_done, ref_state.authoritative_done);

        assert!(
            state.live_effects <= 1,
            "at most one completion effect is live"
        );
        assert!(
            state.authoritative_done <= 1,
            "at most one authoritative Done is accepted"
        );

        if ref_state.artifact_admitted() && ref_state.disposition == Some(ModelDisposition::Pass) {
            assert_eq!(
                state.live_effects + state.authoritative_done,
                1,
                "an admitted passing Review must be live or authoritatively Done"
            );
        }
        if ref_state.artifact_admitted() && ref_state.disposition == Some(ModelDisposition::Block) {
            assert_eq!(
                state.authoritative_done, 0,
                "a blocked Review must never complete"
            );
        }

        assert_eq!(
            matches!(state.state, CompletionState::Confirmed),
            state.authoritative_done == 1
        );
        assert_eq!(
            matches!(state.state, CompletionState::CommandInFlight { .. }),
            state.live_effects == 1
        );
    }
}

prop_state_machine! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn generated_event_orderings_preserve_completion_invariants(
        sequential 1..64 => CompletionStateMachineTest
    );
}
