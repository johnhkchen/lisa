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
                match candidate.state {
                    TransitionState::WaitingForExit if elapsed_secs > policy.exit_grace_secs => {
                        Some(TransitionAction::ExitReady {
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
}

/// The exit grace is the whole transition policy: a session that has been told
/// to `/exit` is given a bounded window to tear down, and nothing exempts it —
/// not pane activity, not a pending question. The pane is already leaving.
pub(crate) struct TransitionPolicy {
    pub(crate) exit_grace_secs: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TransitionAction {
    ExitReady {
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
            }],
            TransitionPolicy { exit_grace_secs: 8 },
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
        // The transition policy is deliberately absent here: it has no
        // exemptions to preserve. A pane told to `/exit` leaves on the grace
        // clock alone — pending questions and pane activity do not hold it
        // back, unlike the review/session/stale policies below.
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

    #[test]
    fn cross_policy_deadline_actions_remain_distinct() {
        let evaluator = evaluator(100);

        let acknowledgements = evaluator.acknowledgements([AcknowledgementInput {
            pane_id: 11,
            state: "expired",
            deadline: SystemTime::UNIX_EPOCH + Duration::from_secs(99),
        }]);
        assert!(matches!(
            acknowledgements.as_slice(),
            [AcknowledgementAction {
                pane_id: 11,
                state: "expired"
            }]
        ));

        // The transition policy emits exactly one action, and only for the one
        // state that can stall. Idle and Fenced panes are past their deadline by
        // the same clock and still yield nothing.
        let transitions = evaluator.transitions(
            [
                TransitionInput {
                    pane_id: 21,
                    ticket_id: Some("T-EXIT".into()),
                    state: TransitionState::WaitingForExit,
                    started: Some(SystemTime::UNIX_EPOCH),
                },
                TransitionInput {
                    pane_id: 22,
                    ticket_id: Some("T-IDLE".into()),
                    state: TransitionState::Idle,
                    started: Some(SystemTime::UNIX_EPOCH),
                },
                TransitionInput {
                    pane_id: 23,
                    ticket_id: Some("T-FENCED".into()),
                    state: TransitionState::Fenced,
                    started: Some(SystemTime::UNIX_EPOCH),
                },
            ],
            TransitionPolicy { exit_grace_secs: 8 },
        );
        assert_eq!(
            transitions,
            vec![TransitionAction::ExitReady {
                pane_id: 21,
                ticket_id: Some("T-EXIT".into()),
            }]
        );

        let reviews = evaluator.reviews(
            [ReviewInput {
                ticket_id: "T-REVIEW".into(),
                pane_id: 31,
                status: ThreadStatus::Running,
                phase: Phase::Review,
                already_prompted: false,
                awaiting_human: false,
                last_phase_change: SystemTime::UNIX_EPOCH,
                last_activity: SystemTime::UNIX_EPOCH,
            }],
            Duration::from_secs(10),
            Duration::from_secs(10),
        );
        assert!(matches!(
            reviews.as_slice(),
            [ReviewAction {
                ticket_id,
                pane_id: 31
            }] if ticket_id == "T-REVIEW"
        ));

        let health = evaluator.health(
            [HealthInput {
                ticket_id: "T-HEALTH".into(),
                status: ThreadStatus::Running,
                last_activity: SystemTime::UNIX_EPOCH,
                previous: Some(HealthStatus::Healthy),
            }],
            Duration::from_secs(10),
        );
        assert!(matches!(
            health.as_slice(),
            [HealthObservation {
                ticket_id,
                previous: Some(HealthStatus::Healthy),
                current: HealthStatus::Stuck,
            }] if ticket_id == "T-HEALTH"
        ));

        let sessions = evaluator.sessions(
            [SessionInput {
                ticket_id: "T-SESSION".into(),
                pane_id: 41,
                status: ThreadStatus::Running,
                phase: Phase::Implement,
                pending_completion: false,
                awaiting_human: false,
                started_at: SystemTime::UNIX_EPOCH,
                last_phase_change: SystemTime::UNIX_EPOCH + Duration::from_secs(90),
                last_activity: SystemTime::UNIX_EPOCH,
                phase_timeout: Duration::from_secs(20),
            }],
            Duration::from_secs(50),
            Duration::from_secs(10),
        );
        assert_eq!(
            sessions,
            vec![SessionAction::Reclaim(SessionDeadline {
                ticket_id: "T-SESSION".into(),
                pane_id: 41,
                elapsed_secs: 100,
                phase: Phase::Implement,
            })]
        );

        let stale = evaluator.stale(
            [StaleInput {
                ticket_id: "T-STALE".into(),
                pane_id: 51,
                status: ThreadStatus::Running,
                pending_completion: false,
                awaiting_human: false,
                last_activity: SystemTime::UNIX_EPOCH,
            }],
            Duration::from_secs(10),
        );
        assert_eq!(
            stale,
            vec![StaleAction {
                ticket_id: "T-STALE".into(),
                pane_id: 51,
            }]
        );
    }

    #[test]
    fn cross_policy_activity_and_human_exemptions_remain_distinct() {
        let evaluator = evaluator(100);
        let recent = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        // Acknowledgement evaluation intentionally has no activity or
        // awaiting-human exemption input.
        let acknowledgements = evaluator.acknowledgements([AcknowledgementInput {
            pane_id: 10,
            state: "expired-without-exemptions",
            deadline: recent,
        }]);
        assert!(matches!(
            acknowledgements.as_slice(),
            [AcknowledgementAction {
                pane_id: 10,
                state: "expired-without-exemptions"
            }]
        ));

        // Transition evaluation, like acknowledgement above, now carries no
        // activity or awaiting-human exemption input at all: a pane told to
        // `/exit` leaves on the grace clock. The exemptions below belong to the
        // review, session, and stale policies and must stay distinct from it.

        let reviews = evaluator.reviews(
            [
                ReviewInput {
                    ticket_id: "T-REVIEW-ACTIVE".into(),
                    pane_id: 31,
                    status: ThreadStatus::Running,
                    phase: Phase::Review,
                    already_prompted: false,
                    awaiting_human: false,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: recent,
                },
                ReviewInput {
                    ticket_id: "T-REVIEW-HUMAN".into(),
                    pane_id: 32,
                    status: ThreadStatus::Running,
                    phase: Phase::Review,
                    already_prompted: false,
                    awaiting_human: true,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: SystemTime::UNIX_EPOCH,
                },
                ReviewInput {
                    ticket_id: "T-REVIEW-FIRE".into(),
                    pane_id: 33,
                    status: ThreadStatus::Running,
                    phase: Phase::Review,
                    already_prompted: false,
                    awaiting_human: false,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: SystemTime::UNIX_EPOCH,
                },
            ],
            Duration::from_secs(10),
            Duration::from_secs(10),
        );
        assert!(matches!(
            reviews.as_slice(),
            [ReviewAction {
                ticket_id,
                pane_id: 33
            }] if ticket_id == "T-REVIEW-FIRE"
        ));

        // Health is observational: recent activity is Healthy, while a quiet
        // awaiting-human pane remains Stuck because that marker is not an input.
        let health = evaluator.health(
            [
                HealthInput {
                    ticket_id: "T-HEALTH-ACTIVE".into(),
                    status: ThreadStatus::Running,
                    last_activity: recent,
                    previous: None,
                },
                HealthInput {
                    ticket_id: "T-HEALTH-HUMAN".into(),
                    status: ThreadStatus::Running,
                    last_activity: SystemTime::UNIX_EPOCH,
                    previous: None,
                },
            ],
            Duration::from_secs(10),
        );
        assert!(matches!(
            health.as_slice(),
            [
                HealthObservation {
                    ticket_id: active,
                    current: HealthStatus::Healthy,
                    ..
                },
                HealthObservation {
                    ticket_id: human,
                    current: HealthStatus::Stuck,
                    ..
                }
            ] if active == "T-HEALTH-ACTIVE" && human == "T-HEALTH-HUMAN"
        ));

        let sessions = evaluator.sessions(
            [
                SessionInput {
                    ticket_id: "T-SESSION-ACTIVE".into(),
                    pane_id: 41,
                    status: ThreadStatus::Running,
                    phase: Phase::Implement,
                    pending_completion: false,
                    awaiting_human: false,
                    started_at: SystemTime::UNIX_EPOCH,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: recent,
                    phase_timeout: Duration::ZERO,
                },
                SessionInput {
                    ticket_id: "T-SESSION-HUMAN".into(),
                    pane_id: 42,
                    status: ThreadStatus::Running,
                    phase: Phase::Implement,
                    pending_completion: false,
                    awaiting_human: true,
                    started_at: SystemTime::UNIX_EPOCH,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: SystemTime::UNIX_EPOCH,
                    phase_timeout: Duration::ZERO,
                },
                SessionInput {
                    ticket_id: "T-SESSION-FIRE".into(),
                    pane_id: 43,
                    status: ThreadStatus::Running,
                    phase: Phase::Implement,
                    pending_completion: false,
                    awaiting_human: false,
                    started_at: SystemTime::UNIX_EPOCH,
                    last_phase_change: SystemTime::UNIX_EPOCH,
                    last_activity: SystemTime::UNIX_EPOCH,
                    phase_timeout: Duration::ZERO,
                },
            ],
            Duration::from_secs(50),
            Duration::from_secs(10),
        );
        assert_eq!(
            sessions,
            vec![
                SessionAction::Warn(SessionDeadline {
                    ticket_id: "T-SESSION-ACTIVE".into(),
                    pane_id: 41,
                    elapsed_secs: 100,
                    phase: Phase::Implement,
                }),
                SessionAction::Warn(SessionDeadline {
                    ticket_id: "T-SESSION-HUMAN".into(),
                    pane_id: 42,
                    elapsed_secs: 100,
                    phase: Phase::Implement,
                }),
                SessionAction::Reclaim(SessionDeadline {
                    ticket_id: "T-SESSION-FIRE".into(),
                    pane_id: 43,
                    elapsed_secs: 100,
                    phase: Phase::Implement,
                }),
            ]
        );

        let stale = evaluator.stale(
            [
                StaleInput {
                    ticket_id: "T-STALE-ACTIVE".into(),
                    pane_id: 51,
                    status: ThreadStatus::Running,
                    pending_completion: false,
                    awaiting_human: false,
                    last_activity: recent,
                },
                StaleInput {
                    ticket_id: "T-STALE-HUMAN".into(),
                    pane_id: 52,
                    status: ThreadStatus::Running,
                    pending_completion: false,
                    awaiting_human: true,
                    last_activity: SystemTime::UNIX_EPOCH,
                },
                StaleInput {
                    ticket_id: "T-STALE-FIRE".into(),
                    pane_id: 53,
                    status: ThreadStatus::Running,
                    pending_completion: false,
                    awaiting_human: false,
                    last_activity: SystemTime::UNIX_EPOCH,
                },
            ],
            Duration::from_secs(10),
        );
        assert_eq!(
            stale,
            vec![StaleAction {
                ticket_id: "T-STALE-FIRE".into(),
                pane_id: 53,
            }]
        );
    }
}
