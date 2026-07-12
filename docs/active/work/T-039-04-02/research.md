# Research: clock-injected deadline evaluator

## Ticket boundary

- Ticket `T-039-04-02` follows the deadline characterization ticket.
- Its acceptance criterion names the same six deadline paths.
- The paths are acknowledgement, transition, review, health, session, and stale thread.
- The existing characterization tests must pass without modification.
- The requested production change is central evaluation with an injectable clock.
- Existing configured durations and exemptions are part of the preserved contract.
- The implementation is concentrated in `crates/lisa-plugin/src/lib.rs`.
- No deadline abstraction currently exists in another crate or module.

## Periodic execution

- Plugin polling invokes each deadline method from the state update loop.
- Each method currently performs both eligibility traversal and its stateful effects.
- Five methods read `SystemTime::now()` directly.
- Acknowledgement is the exception and already has an `_at(now)` seam.
- The wrappers have private visibility inside the plugin crate.
- Inline tests can call private methods and inspect private state.
- Native tests avoid arbitrary Zellij host calls unless captured by test support.

## Shared time representation

- Core thread and slot timestamps use `std::time::SystemTime`.
- Elapsed calculations use `duration_since(...).unwrap_or_default()`.
- This makes future timestamps behave as zero elapsed time.
- Acknowledgement stores absolute deadlines and uses `duration_since(deadline).is_ok()`.
- The acknowledgement comparison is inclusive at equality.
- Review, health, session, and stale comparisons are inclusive.
- Transition comparisons use whole seconds and strict `>` comparisons.
- Preserving that transition boundary is required by characterization.

## Acknowledgement path

- `check_assignment_ack_timeouts_at(now)` traverses `seat_assignments`.
- Five seat variants can expose an absolute deadline.
- Deadline-bearing states are copied into a candidate list with their pane IDs.
- The state is rechecked before an action is applied.
- This guards against an earlier action changing the same seat.
- Each seat variant has a distinct state-machine action.
- Actions include delivery retry, startup recovery, terminal failure, and fresh-session recovery.
- The path returns failure transition outcomes for terminal branches.
- There is no active-session exemption.
- There is no awaiting-human exemption.
- Fresh-session recovery intentionally clears awaiting-human state.

## Transition path

- `check_transition_timeouts()` traverses `agent_slots`.
- Its policy clock is `transition_started_at`.
- `WaitingForExit` uses `AGENT_EXIT_GRACE_SECS`.
- `WaitingForStop` uses `STOP_SIGNAL_TIMEOUT_SECS`.
- `WaitingForClear` uses `CLEAR_SIGNAL_TIMEOUT_SECS`.
- Exit expiry depends only on transition age.
- Stop and clear also require quietness from `last_activity_at`.
- Quietness uses `wind_down_secs` with an inclusive duration comparison.
- Awaiting-human is checked only when actions are applied.
- Exit actions may restore an empty shell or launch the pending provider.
- Stop actions send `/clear` and advance the state.
- Clear actions send the next prompt and return the slot to idle.

## Review path

- `check_review_timeouts()` is disabled when its configured duration is zero.
- It traverses running Review threads.
- It excludes tickets already recorded in `finish_up_sent`.
- Budget age is measured from `last_phase_change`.
- Quietness is measured from `last_activity` against `wind_down_secs`.
- Awaiting-human is checked before the action.
- The action sends an adapter-specific follow-up.
- It bumps pane activity and resets the phase-change clock.
- The reset currently samples wall time separately from initial evaluation.
- It inserts the ticket into `finish_up_sent` and logs an activity event.

## Health path

- `evaluate_health()` traverses running and failed threads.
- It calls `Thread::health(now, stuck_threshold)`.
- Running health depends on `last_activity`.
- Failed threads report failed independently of time.
- Changed health values are written to `last_health` and logged.
- Previously unseen stable values are inserted without a transition event.
- Entries for removed threads are pruned.
- Awaiting-human is deliberately not exempt from observational health.

## Session path

- `check_session_timeouts()` supports a global and per-phase budget.
- It exits early only when both mechanisms are disabled.
- It traverses running threads and excludes pending completions.
- Global age is measured from `started_at`.
- Phase age is measured from `last_phase_change` only if global did not fire.
- Global therefore has precedence when both budgets have elapsed.
- An exceeded budget becomes destructive only after hard silence.
- Hard silence is twice `stuck_threshold_secs` from `last_activity`.
- Awaiting-human suppresses destructive action.
- Active and awaiting-human over-budget threads receive one warning.
- Destructive action fails, fences, records, releases, removes, and reports the thread.
- The method returns typed `FailureTransitionOutcome` values.

## Stale-thread path

- `detect_stale_threads()` uses the same hard-silence duration as session reclaim.
- It traverses running threads and excludes pending completions.
- It calls `Thread::health(now, hard_timeout)` to identify stuck threads.
- Awaiting-human excludes a thread from reclaim.
- Reclaim fails, fences, records, releases, removes, and reports the thread.
- The method returns typed stale-reclaim outcomes.

## Existing test boundary

- T-039-04-01 added one named characterization test per path.
- Those tests call the six existing `State` methods.
- Acknowledgement asserts the exact inclusive boundary through `_at(now)`.
- The other five use wide wall-clock margins.
- They assert active-session and awaiting-human behavior where applicable.
- They also assert observable per-policy actions.
- Numerous older tests cover individual state-machine branches.

## Constraints

- Centralizing effects would require broad access to `State` internals and host I/O.
- Eligibility can be represented without moving those effects.
- Candidate actions need enough identity to let `State` apply the correct branch.
- Borrowing requires candidate collection before mutable effects.
- The evaluator must not erase transition strictness or policy-specific exemptions.
- A test clock should drive all six paths deterministically without sleeping.
- Ticket-owned source files must be committed with exact Lisa includes.
