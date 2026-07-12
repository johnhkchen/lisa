# Research: characterize deadline paths

## Ticket boundary

- Ticket `T-039-04-01` is the first ticket in story `S-039-04`.
- The story will later centralize timeout traversal behind a clock-injected evaluator.
- This ticket precedes that refactor and is limited to characterization tests.
- The acceptance criterion names six paths: acknowledgement, transition, review,
  health, session, and stale-thread deadlines.
- It also explicitly requires active-session and awaiting-human exemptions.
- No production behavior, configured duration, or exemption is meant to change.
- The relevant implementation is concentrated in
  `crates/lisa-plugin/src/lib.rs`.
- Tests for this module are an inline `#[cfg(test)]` module in the same file.
- Inline tests can inspect private `State` fields and call private methods.

## State and configuration

- `State` owns the live `threads`, `agent_slots`, and seat-assignment maps.
- `Thread` carries `started_at`, `last_phase_change`, and `last_activity` clocks.
- `AgentSlot` carries `transition_started_at` and `last_activity_at` clocks.
- `SeatAssignmentState` variants carry absolute acknowledgement deadlines.
- `State::awaiting_human` is a set of pane IDs.
- `State::finish_up_sent` makes review prompting idempotent.
- `State::last_health` stores the prior observed health for transition logging.
- `State::over_budget_warned` makes active over-budget warnings idempotent.
- `PluginConfig::assignment_ack_timeout_secs` sets acknowledgement windows.
- `PluginConfig::wind_down_secs` defines the active-session quiet period.
- `PluginConfig::review_timeout_secs` sets the review phase budget.
- `PluginConfig::stuck_threshold_secs` is the health warning threshold.
- Twice `stuck_threshold_secs` is the hard-silence reclaim threshold.
- `PluginConfig::session_timeout_secs` is the global session budget.
- `PluginConfig::phase_timeouts` optionally adds per-phase budgets.

## Acknowledgement deadline

- `check_assignment_ack_timeouts_at(now)` already accepts an injected clock.
- It traverses `seat_assignments`, selecting variants with a deadline.
- Expiration uses `now.duration_since(deadline).is_ok()`.
- The comparison therefore fires exactly at the stored absolute deadline.
- Different seat states take different actions after expiration.
- `AssignedPendingAck` begins one fresh-session recovery.
- That recovery mints a successor attempt lease.
- It revokes the predecessor's current authority.
- It changes the seat to `Recovering` with no ack deadline yet.
- It sends the provider exit command and enters `WaitingForExit`.
- This path has no active-session or awaiting-human exemption.
- In fact, recovery deliberately removes awaiting-human markers because the old
  TUI is being abandoned.
- Existing acknowledgement tests cover many state-machine branches.
- The characterization seam can use `AssignedPendingAck` and assert the action
  immediately before and exactly at its deadline.

## Transition deadline

- `check_transition_timeouts()` reads `SystemTime::now()` internally.
- Its primary clock is each slot's `transition_started_at`.
- `WaitingForExit` compares against `AGENT_EXIT_GRACE_SECS`.
- `WaitingForStop` compares against `STOP_SIGNAL_TIMEOUT_SECS`.
- `WaitingForClear` compares against `CLEAR_SIGNAL_TIMEOUT_SECS`.
- Stop and clear fallbacks additionally require a quiet pane.
- Quietness is computed from `last_activity_at` and `wind_down_secs`.
- A recent `last_activity_at` is the active-session exemption.
- Awaiting-human is checked before applying stop or clear fallback actions.
- For a quiet, non-awaiting `WaitingForStop` slot, timeout sends `/clear` and
  advances the transition toward `WaitingForClear`.
- Native tests generally avoid branches that call Zellij host functions.
- `WaitingForClear` can be characterized through state and activity effects,
  but prompt delivery also reaches pane I/O.
- `WaitingForExit` can be characterized with a missing ticket; its action is
  purely local restoration of an idle empty shell.
- Existing tests separately cover active-session deferral and awaiting-human
  deferral for stop/clear transitions.

## Review deadline

- `check_review_timeouts()` reads `SystemTime::now()` internally.
- The deadline clock is `Thread::last_phase_change`.
- Eligibility requires a running thread in `Phase::Review`.
- Elapsed review time must meet or exceed `review_timeout_secs`.
- The thread must also be quiet for at least `wind_down_secs` according to
  `Thread::last_activity`.
- Recent activity is therefore the active-session exemption.
- An awaiting-human pane is skipped after candidate collection.
- Skipping awaiting-human does not insert into `finish_up_sent`.
- A qualifying thread receives an adapter-specific finish-up follow-up.
- The action bumps pane activity and resets the phase-change clock.
- It records the ticket in `finish_up_sent` and logs
  `FinishUpPromptSent`.
- Existing tests cover firing, idempotence, disablement, phase/status filters,
  active-session deferral, and awaiting-human deferral.

## Health deadline

- `evaluate_health()` reads `SystemTime::now()` internally.
- It passes `stuck_threshold_secs` to `Thread::health`.
- `Thread::health` derives running-thread health from `last_activity`.
- At or beyond the threshold a running thread becomes `HealthStatus::Stuck`.
- This is an observational path, not a reclamation path.
- The action is updating `last_health` and logging `HealthStateChanged` when the
  value differs from the previous observation.
- A fresh thread is recorded as healthy without a transition event.
- Failed threads are represented as `HealthStatus::Failed`.
- Removed threads are removed from the health cache.
- There is no active-session special case beyond the activity clock itself.
- There is no awaiting-human exemption in health evaluation.
- Awaiting-human sessions may therefore be displayed as stuck while remaining
  protected from destructive reclaim actions.

## Session deadline

- `check_session_timeouts()` reads `SystemTime::now()` internally.
- It first checks total elapsed time from `Thread::started_at` against the
  global `session_timeout_secs`.
- If the global budget has not fired, it checks `last_phase_change` against an
  optional per-phase timeout.
- A budget overrun alone is advisory.
- Destructive action also requires silence from `last_activity` for at least
  twice `stuck_threshold_secs`.
- Recent activity is the active-session exemption.
- Awaiting-human is an explicit exemption from destructive reclamation even
  after both budget and hard-silence thresholds are exceeded.
- Active or awaiting over-budget threads receive a one-time warning.
- A qualifying timeout fails the thread, fences its attempt, emits provenance,
  releases its slot, removes it, records a timeout alert, and logs an event.
- Pending completion transactions are excluded.
- Existing tests exercise global and phase budgets, active deferral, hard
  silence, awaiting-human protection, disabled configuration, and outcomes.

## Stale-thread deadline

- `detect_stale_threads()` reads `SystemTime::now()` internally.
- It uses `Thread::health(now, 2 * stuck_threshold_secs)`.
- Its clock is therefore `last_activity`, via the health calculation.
- Only running threads are candidates.
- Pending completions are excluded.
- Awaiting-human panes are explicitly excluded.
- A recent heartbeat or other activity is the active-session exemption.
- A stale thread is failed, fenced, recorded in provenance, released, removed,
  and reported through an error activity event.
- The returned action is `StaleThreadReclaimed`.
- Existing tests cover normal reclaim, not-yet-stale behavior, active activity,
  awaiting-human protection, and fencing outcomes.

## Test constraints and existing patterns

- Native plugin tests cannot safely execute arbitrary Zellij host calls.
- Characterization fixtures should select branches whose observable action is
  local, or rely on the test-mode pane-I/O capture already used by nearby tests.
- Absolute timestamps should be derived from one fixture-local `now` to prevent
  inconsistent independently sampled clocks.
- Existing non-injected methods still sample the wall clock during the call.
- Tests for those methods need wide margins around boundaries rather than
  sleep-based exact-boundary assertions.
- The acknowledgement method is the exception: its injected `now` permits an
  exact boundary assertion today.
- Tests should state which field is the policy clock, which exemption applies,
  and which action proves firing.
- The future evaluator ticket must be able to retain these tests unchanged.
- Therefore tests should call the current public-in-module policy methods, not
  hypothetical evaluator interfaces.
- Ticket-owned source scope is one file:
  `crates/lisa-plugin/src/lib.rs`.
- Phase artifacts belong only in the attempt-private work directory.
- Source changes must be committed with `lisa commit-ticket` and the exact path.
