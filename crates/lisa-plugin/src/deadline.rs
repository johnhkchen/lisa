use std::time::{Duration, SystemTime};

use lisa_core::types::{HealthStatus, Phase, ThreadStatus, TicketId};

use crate::TransitionState;

pub(crate) trait Clock {
    fn now(&self) -> SystemTime;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl Clock for SystemTime {
    fn now(&self) -> SystemTime {
        *self
    }
}

pub(crate) struct DeadlineEvaluator {
    now: SystemTime,
}

impl DeadlineEvaluator {
    pub(crate) fn new(clock: impl Clock) -> Self {
        Self { now: clock.now() }
    }

    pub(crate) fn now(&self) -> SystemTime {
        self.now
    }

    pub(crate) fn acknowledgements<T: Copy>(
        &self,
        candidates: impl IntoIterator<Item = AcknowledgementInput<T>>,
    ) -> Vec<AcknowledgementAction<T>> {
        let now = self.now;
        candidates
            .into_iter()
            .filter(|candidate| now.duration_since(candidate.deadline).is_ok())
            .map(|candidate| AcknowledgementAction {
                pane_id: candidate.pane_id,
                state: candidate.state,
            })
            .collect()
    }

    pub(crate) fn transitions(
        &self,
        candidates: impl IntoIterator<Item = TransitionInput>,
        policy: TransitionPolicy,
    ) -> Vec<TransitionAction> {
        let now = self.now;
        candidates
            .into_iter()
            .filter_map(|candidate| {
                let started = candidate.started?;
                let elapsed_secs = elapsed(now, started).as_secs();
                let quiet = candidate
                    .last_activity
                    .is_none_or(|at| elapsed(now, at) >= policy.wind_down);
                match candidate.state {
                    TransitionState::WaitingForExit if elapsed_secs > policy.exit_grace_secs => {
                        Some(TransitionAction::ExitReady {
                            pane_id: candidate.pane_id,
                            ticket_id: candidate.ticket_id,
                        })
                    }
                    TransitionState::WaitingForStop
                        if elapsed_secs > policy.stop_timeout_secs
                            && quiet
                            && !candidate.awaiting_human =>
                    {
                        Some(TransitionAction::StopTimedOut {
                            pane_id: candidate.pane_id,
                        })
                    }
                    TransitionState::WaitingForClear
                        if elapsed_secs > policy.clear_timeout_secs
                            && quiet
                            && !candidate.awaiting_human =>
                    {
                        Some(TransitionAction::ClearTimedOut {
                            pane_id: candidate.pane_id,
                            ticket_id: candidate.ticket_id,
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    pub(crate) fn reviews(
        &self,
        candidates: impl IntoIterator<Item = ReviewInput>,
        timeout: Duration,
        wind_down: Duration,
    ) -> Vec<ReviewAction> {
        if timeout.is_zero() {
            return Vec::new();
        }
        let now = self.now;
        candidates
            .into_iter()
            .filter(|candidate| {
                candidate.status == ThreadStatus::Running
                    && candidate.phase == Phase::Review
                    && !candidate.already_prompted
                    && !candidate.awaiting_human
                    && elapsed(now, candidate.last_phase_change) >= timeout
                    && elapsed(now, candidate.last_activity) >= wind_down
            })
            .map(|candidate| ReviewAction {
                ticket_id: candidate.ticket_id,
                pane_id: candidate.pane_id,
            })
            .collect()
    }

    pub(crate) fn health(
        &self,
        candidates: impl IntoIterator<Item = HealthInput>,
        threshold: Duration,
    ) -> Vec<HealthObservation> {
        let now = self.now;
        candidates
            .into_iter()
            .map(|candidate| {
                let current = match candidate.status {
                    ThreadStatus::Failed => HealthStatus::Failed,
                    ThreadStatus::Running if elapsed(now, candidate.last_activity) >= threshold => {
                        HealthStatus::Stuck
                    }
                    _ => HealthStatus::Healthy,
                };
                HealthObservation {
                    ticket_id: candidate.ticket_id,
                    previous: candidate.previous,
                    current,
                }
            })
            .collect()
    }

    pub(crate) fn sessions(
        &self,
        candidates: impl IntoIterator<Item = SessionInput>,
        global_timeout: Duration,
        hard_silence: Duration,
    ) -> Vec<SessionAction> {
        let now = self.now;
        candidates
            .into_iter()
            .filter_map(|candidate| {
                if candidate.status != ThreadStatus::Running || candidate.pending_completion {
                    return None;
                }

                let global_elapsed = elapsed(now, candidate.started_at);
                let phase_elapsed = elapsed(now, candidate.last_phase_change);
                let exceeded = if !global_timeout.is_zero() && global_elapsed >= global_timeout {
                    Some(global_elapsed)
                } else if !candidate.phase_timeout.is_zero()
                    && phase_elapsed >= candidate.phase_timeout
                {
                    Some(phase_elapsed)
                } else {
                    None
                }?;

                let action = SessionDeadline {
                    ticket_id: candidate.ticket_id,
                    pane_id: candidate.pane_id,
                    elapsed_secs: exceeded.as_secs(),
                    phase: candidate.phase,
                };
                if elapsed(now, candidate.last_activity) >= hard_silence
                    && !candidate.awaiting_human
                {
                    Some(SessionAction::Reclaim(action))
                } else {
                    Some(SessionAction::Warn(action))
                }
            })
            .collect()
    }

    pub(crate) fn stale(
        &self,
        candidates: impl IntoIterator<Item = StaleInput>,
        hard_timeout: Duration,
    ) -> Vec<StaleAction> {
        let now = self.now;
        candidates
            .into_iter()
            .filter(|candidate| {
                candidate.status == ThreadStatus::Running
                    && !candidate.pending_completion
                    && !candidate.awaiting_human
                    && elapsed(now, candidate.last_activity) >= hard_timeout
            })
            .map(|candidate| StaleAction {
                ticket_id: candidate.ticket_id,
                pane_id: candidate.pane_id,
            })
            .collect()
    }
}

fn elapsed(now: SystemTime, since: SystemTime) -> Duration {
    now.duration_since(since).unwrap_or_default()
}

pub(crate) struct AcknowledgementInput<T> {
    pub(crate) pane_id: u32,
    pub(crate) state: T,
    pub(crate) deadline: SystemTime,
}

pub(crate) struct AcknowledgementAction<T> {
    pub(crate) pane_id: u32,
    pub(crate) state: T,
}

pub(crate) struct TransitionInput {
    pub(crate) pane_id: u32,
    pub(crate) ticket_id: Option<TicketId>,
    pub(crate) state: TransitionState,
    pub(crate) started: Option<SystemTime>,
    pub(crate) last_activity: Option<SystemTime>,
    pub(crate) awaiting_human: bool,
}

pub(crate) struct TransitionPolicy {
    pub(crate) wind_down: Duration,
    pub(crate) exit_grace_secs: u64,
    pub(crate) stop_timeout_secs: u64,
    pub(crate) clear_timeout_secs: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TransitionAction {
    ExitReady {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
    StopTimedOut {
        pane_id: u32,
    },
    ClearTimedOut {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
}

pub(crate) struct ReviewInput {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
    pub(crate) status: ThreadStatus,
    pub(crate) phase: Phase,
    pub(crate) already_prompted: bool,
    pub(crate) awaiting_human: bool,
    pub(crate) last_phase_change: SystemTime,
    pub(crate) last_activity: SystemTime,
}

pub(crate) struct ReviewAction {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
}

pub(crate) struct HealthInput {
    pub(crate) ticket_id: TicketId,
    pub(crate) status: ThreadStatus,
    pub(crate) last_activity: SystemTime,
    pub(crate) previous: Option<HealthStatus>,
}

pub(crate) struct HealthObservation {
    pub(crate) ticket_id: TicketId,
    pub(crate) previous: Option<HealthStatus>,
    pub(crate) current: HealthStatus,
}

pub(crate) struct SessionInput {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
    pub(crate) status: ThreadStatus,
    pub(crate) phase: Phase,
    pub(crate) pending_completion: bool,
    pub(crate) awaiting_human: bool,
    pub(crate) started_at: SystemTime,
    pub(crate) last_phase_change: SystemTime,
    pub(crate) last_activity: SystemTime,
    pub(crate) phase_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionDeadline {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
    pub(crate) elapsed_secs: u64,
    pub(crate) phase: Phase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionAction {
    Warn(SessionDeadline),
    Reclaim(SessionDeadline),
}

pub(crate) struct StaleInput {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
    pub(crate) status: ThreadStatus,
    pub(crate) pending_completion: bool,
    pub(crate) awaiting_human: bool,
    pub(crate) last_activity: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaleAction {
    pub(crate) ticket_id: TicketId,
    pub(crate) pane_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct FixedClock(SystemTime);

    impl Clock for FixedClock {
        fn now(&self) -> SystemTime {
            self.0
        }
    }

    fn evaluator(now_secs: u64) -> DeadlineEvaluator {
        DeadlineEvaluator::new(FixedClock(
            SystemTime::UNIX_EPOCH + Duration::from_secs(now_secs),
        ))
    }

    #[test]
    fn fixed_clock_drives_all_six_policies_at_their_boundaries() {
        let evaluator = evaluator(100);

        let acknowledgements = evaluator.acknowledgements([
            AcknowledgementInput {
                pane_id: 1,
                state: "equal",
                deadline: SystemTime::UNIX_EPOCH + Duration::from_secs(100),
            },
            AcknowledgementInput {
                pane_id: 2,
                state: "future",
                deadline: SystemTime::UNIX_EPOCH + Duration::from_secs(101),
            },
        ]);
        assert_eq!(acknowledgements.len(), 1);
        assert_eq!(acknowledgements[0].state, "equal");

        let transitions = evaluator.transitions(
            [TransitionInput {
                pane_id: 3,
                ticket_id: None,
                state: TransitionState::WaitingForExit,
                started: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(91)),
                last_activity: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(100)),
                awaiting_human: true,
            }],
            TransitionPolicy {
                wind_down: Duration::from_secs(20),
                exit_grace_secs: 8,
                stop_timeout_secs: 60,
                clear_timeout_secs: 90,
            },
        );
        assert_eq!(
            transitions,
            vec![TransitionAction::ExitReady {
                pane_id: 3,
                ticket_id: None
            }]
        );

        let reviews = evaluator.reviews(
            [ReviewInput {
                ticket_id: "T-R".into(),
                pane_id: 4,
                status: ThreadStatus::Running,
                phase: Phase::Review,
                already_prompted: false,
                awaiting_human: false,
                last_phase_change: SystemTime::UNIX_EPOCH + Duration::from_secs(90),
                last_activity: SystemTime::UNIX_EPOCH + Duration::from_secs(80),
            }],
            Duration::from_secs(10),
            Duration::from_secs(20),
        );
        assert_eq!(reviews.len(), 1);

        let health = evaluator.health(
            [HealthInput {
                ticket_id: "T-H".into(),
                status: ThreadStatus::Running,
                last_activity: SystemTime::UNIX_EPOCH + Duration::from_secs(90),
                previous: Some(HealthStatus::Healthy),
            }],
            Duration::from_secs(10),
        );
        assert_eq!(health[0].current, HealthStatus::Stuck);

        let sessions = evaluator.sessions(
            [SessionInput {
                ticket_id: "T-S".into(),
                pane_id: 5,
                status: ThreadStatus::Running,
                phase: Phase::Implement,
                pending_completion: false,
                awaiting_human: false,
                started_at: SystemTime::UNIX_EPOCH,
                last_phase_change: SystemTime::UNIX_EPOCH,
                last_activity: SystemTime::UNIX_EPOCH + Duration::from_secs(80),
                phase_timeout: Duration::ZERO,
            }],
            Duration::from_secs(100),
            Duration::from_secs(20),
        );
        assert!(matches!(sessions.as_slice(), [SessionAction::Reclaim(_)]));

        let stale = evaluator.stale(
            [StaleInput {
                ticket_id: "T-X".into(),
                pane_id: 6,
                status: ThreadStatus::Running,
                pending_completion: false,
                awaiting_human: false,
                last_activity: SystemTime::UNIX_EPOCH + Duration::from_secs(80),
            }],
            Duration::from_secs(20),
        );
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn policy_specific_exemptions_are_preserved() {
        let evaluator = evaluator(100);
        let policy = TransitionPolicy {
            wind_down: Duration::from_secs(10),
            exit_grace_secs: 8,
            stop_timeout_secs: 60,
            clear_timeout_secs: 90,
        };
        assert!(evaluator
            .transitions(
                [TransitionInput {
                    pane_id: 1,
                    ticket_id: None,
                    state: TransitionState::WaitingForStop,
                    started: Some(SystemTime::UNIX_EPOCH),
                    last_activity: Some(SystemTime::UNIX_EPOCH),
                    awaiting_human: true,
                }],
                policy,
            )
            .is_empty());

        let review = ReviewInput {
            ticket_id: "T-R".into(),
            pane_id: 2,
            status: ThreadStatus::Running,
            phase: Phase::Review,
            already_prompted: false,
            awaiting_human: true,
            last_phase_change: SystemTime::UNIX_EPOCH,
            last_activity: SystemTime::UNIX_EPOCH,
        };
        assert!(evaluator
            .reviews([review], Duration::from_secs(1), Duration::from_secs(1))
            .is_empty());

        let sessions = evaluator.sessions(
            [SessionInput {
                ticket_id: "T-S".into(),
                pane_id: 3,
                status: ThreadStatus::Running,
                phase: Phase::Implement,
                pending_completion: false,
                awaiting_human: true,
                started_at: SystemTime::UNIX_EPOCH,
                last_phase_change: SystemTime::UNIX_EPOCH,
                last_activity: SystemTime::UNIX_EPOCH,
                phase_timeout: Duration::ZERO,
            }],
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert!(matches!(sessions.as_slice(), [SessionAction::Warn(_)]));

        assert!(evaluator
            .stale(
                [StaleInput {
                    ticket_id: "T-X".into(),
                    pane_id: 4,
                    status: ThreadStatus::Running,
                    pending_completion: false,
                    awaiting_human: true,
                    last_activity: SystemTime::UNIX_EPOCH,
                }],
                Duration::from_secs(1),
            )
            .is_empty());
    }
}
