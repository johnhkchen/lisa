# Structure: clock-injected deadline evaluator

## File inventory

### Create `crates/lisa-plugin/src/deadline.rs`

Own all clock and deadline-eligibility concepts for the plugin.

The module contains:

- `Clock` trait;
- `SystemClock` production implementation;
- `DeadlineEvaluator<C>`;
- one input view per policy;
- one typed action enum or struct per policy;
- policy evaluation methods;
- focused unit tests with a fixed clock.

The module remains private to `lisa-plugin`.

### Modify `crates/lisa-plugin/src/lib.rs`

- Declare `mod deadline`.
- Import evaluator input/action types used by `State`.
- Replace each scattered candidate traversal with an evaluator call.
- Preserve existing stateful action bodies and wrapper signatures.
- Retain T-039-04-01 characterization tests unchanged.
- Add integration-level injected-clock coverage only if pure module tests cannot
  prove all six wrappers route through the evaluator.

### Attempt-private artifacts

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`

These remain under `.lisa/attempts/T-039-04-02/1/work/` and are not included in
ticket source commits.

## Module boundary

`deadline` may depend on lightweight plugin/core domain types required to name
policies: `Phase`, `HealthStatus`, `ThreadStatus`, `TicketId`, and transition or
seat state if those types are available at module scope.

It must not depend on:

- `State`;
- Zellij pane APIs;
- adapters or follow-up payloads;
- lease mutation;
- provenance emission;
- activity logging;
- filesystem paths.

This keeps evaluation pure and makes the clock the only runtime dependency.

## Visibility

Types are `pub(crate)` only where `lib.rs` needs them.
Internal helpers remain private.
Fields are exposed through constructors or crate-visible structs when concise.
No type is re-exported from the crate.

## Clock types

```text
Clock
  now() -> SystemTime

SystemClock
  implements Clock using SystemTime::now()

DeadlineEvaluator<C>
  clock: C
  now() sampled once inside each evaluate_* call
```

The evaluator can also expose a sampled-time internal helper where the existing
acknowledgement `_at(now)` seam must be retained.

## Policy interfaces

### Acknowledgement

Input: iterator of absolute deadline candidates.
Output: expired candidates, preserving input order.

The state layer remains responsible for verifying the seat still equals the
captured state before applying its variant-specific action.

### Transition

Input: transition records plus wind-down and the three policy thresholds.
Output: `ExitReady`, `StopTimedOut`, or `ClearTimedOut` actions.

The action carries pane and optional ticket identity needed by existing effects.
Stop/clear awaiting-human suppression is evaluated here.

### Review

Input: thread eligibility records, timeout, and wind-down.
Output: finish-up candidates carrying ticket and pane identity.

Disabled duration yields no actions.

### Health

Input: thread observations, threshold, and prior cached health.
Output: current observations, with previous value available for detecting a
transition. The state layer logs only changes and inserts stable first values.

Removed-entry pruning remains in `State` because it owns the cache and thread map.

### Session

Input: thread budget records, global configuration, phase limits, and hard silence.
Output: `Warn` or `Reclaim` with ticket, pane, elapsed seconds, and phase.

The evaluator preserves global-before-phase precedence and exclusions.

### Stale

Input: thread activity records and hard timeout.
Output: reclaim candidates carrying ticket and pane identity.

## Application ordering

1. Sample the evaluator clock once.
2. Build immutable input records from state.
3. Evaluate into owned actions.
4. Drop immutable borrows.
5. Apply actions sequentially through existing state-machine code.

This matches the current collect-then-mutate shape and avoids borrow conflicts.

## Test organization

`deadline.rs` unit tests define `FixedClock(SystemTime)`.

Tests cover:

- acknowledgement before/equal boundary;
- transition strict thresholds and quietness;
- transition exit exemption asymmetry;
- review timeout, activity, prior-prompt, and human exemptions;
- health threshold and awaiting-human non-exemption by construction;
- session global and phase precedence;
- session active/human warning versus silent reclaim;
- stale active/human exclusion versus silent reclaim.

Existing inline tests in `lib.rs` remain the effect/action contract.

## Commit units

One meaningful source unit spans the new evaluator and its integration because
neither file is independently buildable. Commit both exact paths together:

- `crates/lisa-plugin/src/deadline.rs`
- `crates/lisa-plugin/src/lib.rs`

Formatting changes outside these files are not owned by the ticket and must not
be included.
