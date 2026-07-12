# T-035-01-04 Progress — bounded startup recovery

## Status

Implementation is complete, verified, and committed through Lisa's isolated transaction.

## Completed phase work

- Read `CLAUDE.md`, `AGENTS.md`, the ticket, and RDSPI workflow.
- Mapped fresh launch, exact start-signal admission, E-033 recovery, timeout, UI, and
  native test boundaries in `research.md`.
- Evaluated five approaches and selected a provider-neutral retained startup failure in
  `design.md`.
- Defined the two-file source boundary and state/test interfaces in `structure.md`.
- Sequenced implementation, focused regression, full verification, and isolated commit
  hygiene in `plan.md`.

Lisa detected the attempt-private phase artifacts and advanced ticket phase automatically.
The ticket frontmatter was not manually edited.

## Implemented source changes

### `crates/lisa-plugin/src/lib.rs`

Extended `SeatAssignmentState::Starting` with:

```rust
start_deadline: Option<SystemTime>
```

The field distinguishes a reserved delayed fresh route from a launch that has actually
been submitted and is awaiting its exact `.started` lease signal.

Added `SeatAssignmentState::StartupFailed` as a provider-neutral terminal assignment
state. It remains not-owned and retains the physical seat reservation.

Extended `start_assignment_ack_wait` to arm unarmed startup waits using the existing
positive E-033 `assignment_ack_timeout_secs` plus delayed-Enter allowance.

Broadened immediate post-dispatch arming from generation-tagged recycled Codex only to
any applicable state whose transport is already `Idle`. The helper remains a no-op for
same-process Claude `Owned` assignments.

Cross-provider routes remain unarmed while `WaitingForExit`; the existing exit-grace
launch path arms the startup deadline only after submitting the prepared fresh launcher.

Added `fail_startup`, which:

- accepts only an active `Starting` state;
- writes `StartupFailed` first;
- fails the retained logical thread;
- keeps slot ticket, attempt lease, and current lease installed;
- deduplicates the ticket/pane error alert;
- logs an explicit reset-to-retry instruction;
- sends no input and performs no scheduler release or relaunch.

Extended the injected-time assignment deadline evaluator to recognize armed starting
states and route expiry into `fail_startup`. Unarmed starting, owned, recovery-failed,
and startup-failed states remain inert.

Preserved poll order: process-start signal consumption runs before transition delivery
and timeout evaluation, so positive start evidence visible at the boundary wins.

Updated existing fresh-state expectations to distinguish immediate `Some(deadline)`
from delayed cross-provider `None`.

Added native regression:

```text
test_missing_fresh_start_signal_fails_within_bound_without_relaunch
```

The test withholds `.started`, extracts the real stored deadline, evaluates it without
sleeping, and proves:

- exact generation is pending before expiry;
- the seat is never owned;
- deadline expiry produces `StartupFailed`;
- thread, slot, and current lease are retained;
- thread status and operator alert are failed/actionable;
- UI conversion exposes `StartupFailed`;
- repeated later evaluations leave one launch event and one alert unchanged.

### `crates/lisa-plugin/src/ui.rs`

Added `SeatAssignmentStatus::StartupFailed` with:

- stable label `startup-failed`;
- terminal red color;
- exact internal-to-UI mapping from the plugin state.

This is intentionally distinct from E-033's `recovery-failed` status.

## Deviations from plan

The planned assertion initially attempted to render the failed assignment in the active
thread table. Existing UI architecture excludes failed threads from `active_threads`,
including E-033 recovery failures, and surfaces them through the attention alert path.

The test was corrected to assert the explicit `seat_assignment_statuses` conversion plus
the failed-thread alert and actionable activity message. No production behavior was
changed to broaden dashboard table rendering because that would be unrelated UI scope.

All other implementation steps followed the plan.

## Focused verification

Passed:

```text
cargo test -p lisa-plugin test_missing_fresh_start_signal_fails_within_bound_without_relaunch
1 passed

cargo test -p lisa-plugin test_fresh_dispatch_becomes_owned_only_after_exact_process_start
1 passed
```

E-033/E-034 regression filters passed:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
1 passed

cargo test -p lisa-plugin split_brain
1 passed

cargo test -p lisa-plugin recycled_codex
2 passed
```

## Full verification

Passed:

```text
cargo fmt --all -- --check

cargo test -p lisa-plugin
278 passed; 0 failed

cargo test --workspace
lisa-cli:    274 passed; 0 failed
lisa-core:   155 passed; 0 failed
lisa-plugin: 278 passed; 0 failed
doc tests:   passed
```

`git diff --check` reports no whitespace errors for the two owned source paths.

## Working-tree isolation

Pre-existing/concurrent changes remain outside ticket ownership:

- `.lisa/provenance.jsonl`;
- `docs/active/epic/E-035.md`;
- `docs/active/stories/S-035-02.md`;
- `docs/active/tickets/T-035-01-04.md` (Lisa phase transitions);
- `docs/active/tickets/T-035-02-01.md`;
- concurrent untracked story/ticket files.

Lisa also published admitted artifacts to `docs/active/work/T-035-01-04/`; this attempt
did not write that shared path directly.

The ordinary Git index is empty. Only these source paths will be included in the isolated
ticket commit:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/ui.rs
```

## Isolated source commit

Created with:

```text
lisa commit-ticket \
  --ticket-id T-035-01-04 \
  --message "fix(plugin): bound fresh startup recovery" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

Result:

```text
ae2fd95e72cea4e86584292e3ecf33424b3c132e
fix(plugin): bound fresh startup recovery
```

`git show --name-only` confirms the commit contains exactly the two ticket-owned source
paths. Post-commit `git diff` and `git diff --cached` for both paths are empty. No
ticket-owned source file is staged, modified, or untracked.

## Remaining work

1. Inspect the committed diff and write `review.md`.
2. Remain on this ticket for Lisa's completion publication.
