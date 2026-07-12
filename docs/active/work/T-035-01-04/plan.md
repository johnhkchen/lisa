# T-035-01-04 Plan — bounded startup recovery

## Outcome

Deliver one atomic plugin change that bounds fresh process-start observation, retains a
failed assignment for operator action, exposes `startup-failed`, and proves through a
native regression that no missing signal can produce ownership or unbounded relaunch.

## Owned source paths

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/ui.rs
```

No other source path is expected. Phase artifacts remain attempt-private and are not
included in the source transaction.

## Step 1 — represent the bounded fresh-start lifecycle

Modify `SeatAssignmentState::Starting` to add:

```rust
start_deadline: Option<SystemTime>
```

Add fieldless `StartupFailed`.

Update comments so `None` means launch not submitted and `Some` means the positive
start wait is active.

Verification:

- compiler finds every construction and exhaustive match;
- `seat_is_owned` remains true only for `Owned`;
- `active_assignment_generation` remains limited to E-033 prompt states.

## Step 2 — expose terminal startup failure

Modify `ui::SeatAssignmentStatus` with `StartupFailed`.

Map:

- label to `startup-failed`;
- color to red;
- internal assignment state to the UI status in `to_ui_state`.

Verification:

- `cargo test -p lisa-plugin ui` or the relevant dashboard tests compile and pass;
- no existing assignment label changes.

## Step 3 — arm startup at actual fresh submission

Extend `start_assignment_ack_wait` to transform only:

```text
Starting(None) -> Starting(Some(deadline))
```

using the existing configured timeout plus delayed-Enter allowance.

Broaden the post-dispatch call guard to any `Idle` transport state. Let the helper's
state match decide whether a clock applies.

Preserve delayed launch behavior:

- cross-provider scheduling leaves `Starting(None)` during `WaitingForExit`;
- exit-grace fresh submission calls the existing arming helper;
- immediate empty-pane and `FreshExec` submission arm before scheduling returns;
- same-process Claude `Owned` remains a no-op;
- recycled Codex clear handshake remains unarmed until prompt delivery.

Verification:

- focused fresh dispatch test observes `Some(deadline)`;
- existing clear/exit timeout tests remain green;
- no deadline starts before cross-provider launch delivery.

## Step 4 — add terminal startup failure action

Add `fail_startup(pane_id, reason)` beside E-033's recovery failure helper.

Implement in this order:

1. require current `Starting`;
2. set `StartupFailed`;
3. resolve retained ticket from slot;
4. fail the thread if present;
5. deduplicate the ticket/pane error alert;
6. log an error with reset guidance.

Do not revoke lease, release slot, remove thread, send input, or schedule a retry.

Verification:

- direct regression observes the retained slot and attempt lease;
- thread status is failed;
- error alert exists once;
- activity contains provider-start reason and reset action.

## Step 5 — include startup deadlines in evaluation

Extend `check_assignment_ack_timeouts_at` deadline extraction to include armed
`Starting` states.

Extend its action match so an unchanged expired start calls `fail_startup`.

Keep:

- process-start scanner before timeout evaluation in `poll_tick`;
- collect-before-mutate behavior;
- exact current-state comparison before acting;
- E-033 pending and recovery actions unchanged.

Verification:

- exact signal at/before evaluation leaves `Owned` and cannot become failed;
- expired no-signal state leaves `Starting` once;
- unarmed `Starting(None)` is inert.

## Step 6 — update existing fresh-state assertions

Update test constructions and equality assertions for the new deadline field.

Use exact `Some` assertions when launch timing is relevant. Use `..` matching where a
test only cares that a route is pending fresh start.

Likely affected coverage:

- actual fallback route pane-title test;
- exact fresh process-start test;
- stale/split-brain state constructions;
- fresh native Codex route expectation.

Verification:

- `cargo test -p lisa-plugin --no-run` compiles all exhaustive patterns;
- changes do not weaken generation or lease assertions.

## Step 7 — add the acceptance regression

Add:

```text
test_missing_fresh_start_signal_fails_within_bound_without_relaunch
```

Fixture and setup:

- native one-ticket empty Claude pane;
- `assignment_ack_timeout_secs = 1`;
- no `.started` file at any point;
- schedule through the real dispatch method.

Before deadline assertions:

- current lease is installed;
- assignment is `Starting` with matching generation and stored deadline;
- dashboard is `starting`;
- seat is not owned;
- capture all launch evidence.

At deadline assertions:

- invoke injected-time timeout evaluation exactly at stored deadline;
- assignment is `StartupFailed`;
- dashboard is `startup-failed`;
- seat never became owned;
- thread is retained and failed;
- slot still holds ticket and exact lease;
- current lease remains installed;
- one error alert and actionable log exist;
- launch evidence is unchanged.

Repeated evaluation assertions:

- invoke evaluation beyond several additional timeout intervals;
- state remains `StartupFailed`;
- no new command, launch event, timer, lease, thread, or alert appears.

This directly proves the ticket acceptance criterion and N2 boundary.

## Step 8 — format and focused verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin test_missing_fresh_start_signal_fails_within_bound_without_relaunch
cargo test -p lisa-plugin test_fresh_dispatch_becomes_owned_only_after_exact_process_start
```

If formatting check fails because the new source is not formatted, run `cargo fmt --all`
and then repeat the check. Formatting may mechanically touch only the owned source paths;
inspect before committing.

Expected focused results:

- missing signal reaches startup failure deterministically;
- matching signal still establishes ownership exactly once.

## Step 9 — regression verification

Run E-033/E-034-sensitive focused coverage by name where practical:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin split_brain
cargo test -p lisa-plugin recycled_codex
```

Then run:

```text
cargo test -p lisa-plugin
cargo test --workspace
```

Acceptance requires the native test; workspace verification protects the shared config,
CLI, scheduler, and dashboard integration against exhaustive-match or behavior regressions.

## Step 10 — inspect source ownership

Before committing:

```text
git diff -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
git diff --check -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
git status --short
```

Confirm:

- only ticket-intended hunks exist in the two owned source files;
- unrelated Lisa-managed ticket/story/provenance changes remain excluded;
- no ordinary index entry exists for either owned source file;
- no extra source file became modified or untracked.

## Step 11 — commit the atomic source unit

Use Lisa's isolated transaction only:

```text
lisa commit-ticket \
  --ticket-id T-035-01-04 \
  --message "fix(plugin): bound fresh startup recovery" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

Do not run `git add`, `git add -A`, or ordinary `git commit`.

If the installed Lisa command is unavailable or lacks `commit-ticket`, inspect repository
CLI help and use the project-prescribed equivalent invocation without touching the
ordinary index.

## Step 12 — post-commit hygiene

Run:

```text
git status --short
git diff -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
git diff --cached -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
git log -1 --oneline
```

Require:

- both ticket-owned source paths clean;
- neither path staged in the ordinary index;
- commit contains exactly the intended paths;
- concurrent non-ticket changes remain present and untouched.

## Step 13 — progress and review artifacts

Write `progress.md` throughout implementation with:

- completed plan steps;
- exact source commit hash/message;
- tests and results;
- deviations and rationale;
- final repository hygiene.

After code, commit, and verification are complete, inspect the committed diff and write
`review.md` summarizing:

- internal and UI state changes;
- arming and timeout behavior;
- acceptance criteria proof;
- regression coverage;
- open concerns and honest test boundary;
- commit and ownership hygiene.

Do not update ticket phase/status. After `review.md`, remain assigned to this ticket and
stop so Lisa can publish artifacts and prepare the completion commit.

## Atomicity rationale

The internal startup state, timeout behavior, dashboard label, and regression form one
meaningful compiling unit. Splitting `lib.rs` and `ui.rs` would temporarily break
exhaustive matching or expose an unmapped state. Commit them together with exact includes.

## Rollback boundary

If verification reveals a design flaw before commit, edit only the two owned source
paths and document the deviation in `progress.md`. Do not revert or overwrite unrelated
working-tree changes.

If the isolated commit fails, leave source changes intact, diagnose the Lisa command,
and retry the isolated transaction. Never fall back to the ordinary Git index.
