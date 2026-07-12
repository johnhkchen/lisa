# Plan: recycled-seat assignment state model

## Implementation strategy

Implement the new assignment dimension in one cohesive scheduler source unit. Keep
the source change narrowly scoped to state vocabulary, lifecycle bookkeeping, and
unit tests. Commit the source through Lisa’s isolated ticket transaction after all
verification passes. Record results and any deviations in `progress.md`.

## Step 1: establish a clean ownership baseline

1. Inspect `git status --short` before editing.
2. Record that unrelated modified and untracked paths already exist.
3. Treat only `crates/lisa-plugin/src/lib.rs` as ticket-owned source.
4. Treat `docs/active/work/T-033-01-01/` as ticket-owned work artifacts.
5. Do not modify the ticket frontmatter.
6. Do not use ordinary `git add` or `git commit`.

Verification:

- The planned source path has no preexisting worktree modification.
- Unrelated dirty paths are left untouched.

## Step 2: add assignment-state vocabulary

1. Add private `SeatAssignmentState` beside `TransitionState`.
2. Add `AssignedPendingAck`.
3. Add `Owned`.
4. Add `Recovering`.
5. Derive debug, copy, clone, equality traits.
6. Narrowly allow dead code on `Recovering` if production code does not use it yet.
7. Document that the recovery ticket owns the future transition.

Verification:

- The enum compiles.
- The named states match the ticket language.
- No serialization or public API is added.

Atomic outcome: the scheduler can represent all story assignment states.

## Step 3: add scheduler-owned storage and queries

1. Add `seat_assignments: HashMap<u32, SeatAssignmentState>` to `State`.
2. Document absence as unassigned.
3. Add `seat_assignment(pane_id)`.
4. Add `seat_is_owned(pane_id)`.
5. Make ownership true only for `Owned`.
6. Keep helpers private until a later projection boundary requires visibility.

Verification:

- `State::default()` creates an empty assignment map.
- No assignment reports owned.
- Pending and recovering do not report owned.

Atomic outcome: callers have one authoritative ownership predicate.

## Step 4: classify schedule-time assignments

1. Capture `has_session` immediately after choosing a slot.
2. Preserve the captured value before recycle mutates residency.
3. Leave launch/reset/recycle command behavior unchanged.
4. After slot reservation, classify incoming Codex plus existing session as pending.
5. Classify all other assignments as owned.
6. Insert the state by physical pane ID.
7. Leave thread creation and capacity accounting unchanged.

Verification:

- Fresh Codex maps to owned.
- Reused Codex maps to pending.
- Recycled cross-provider Codex maps to pending.
- Fresh and reused Claude map to owned.
- Existing transport states remain unchanged.

Atomic outcome: every new assignment has explicit scheduler truth.

## Step 5: integrate cleanup

1. Update `release_slot_for_ticket` to remove assignment state for its pane.
2. Structure the removal outside the active slot borrow if needed.
3. Preserve cooldown behavior.
4. Preserve `has_session` and `last_client` behavior.
5. Preserve existing pane rename and activity log behavior.
6. Clear assignment state in the missing-ticket exit-timeout abandonment branch.

Verification:

- Released slots have no assignment state.
- Released slots report not owned.
- Resident sessions still remain available after cooldown.
- Abandoned recycle transitions cannot leave stale state.

Atomic outcome: assignment metadata follows slot reservation lifetime.

## Step 6: add primary Codex regression

1. Build a schedulable Codex ticket with a resident Codex session.
2. Call `schedule_ready_tickets`.
3. Assert the same-provider reuse transport remains `WaitingForClear`.
4. Assert ticket reservation remains present.
5. Assert assignment is `AssignedPendingAck`.
6. Assert `seat_is_owned` is false.

Verification:

- The test fails on the pre-change scheduler because no explicit pending state exists.
- The test passes with the new model.
- The assertion directly matches the ticket acceptance criterion.

Atomic outcome: recycled Codex no longer reports ownership at handoff time.

## Step 7: add fresh and Claude controls

1. Schedule a fresh Codex ticket into an empty shell.
2. Assert it becomes `Owned`.
3. Assert the ownership predicate is true.
4. Schedule a Claude ticket into a resident Claude session.
5. Assert it follows `WaitingForClear`.
6. Assert it becomes `Owned`.
7. Assert its ownership predicate is true.

Verification:

- The control tests distinguish reassignment from initial launch.
- Claude’s current scheduling and reset semantics remain intact.

Atomic outcome: the new state model is provider-scoped at the intended boundary.

## Step 8: add timeout-threading coverage

1. Seed a `WaitingForClear` Codex slot past clear timeout.
2. Seed its assignment as `AssignedPendingAck`.
3. Run `check_transition_timeouts`.
4. Assert transport returns to `Idle`.
5. Assert assignment remains pending and not owned.
6. Seed the existing `WaitingForExit` launch case with pending assignment.
7. Run its timeout.
8. Assert session launch bookkeeping completes.
9. Assert assignment remains pending and not owned.

Verification:

- Current transport fallbacks cannot silently promote ownership.
- Both reuse and cross-provider timeout paths carry the state.

Atomic outcome: assignment truth survives existing transport timers.

## Step 9: add release coverage

1. Seed a slot with a ticket and owned or pending assignment state.
2. Call `release_slot_for_ticket`.
3. Assert `ticket_id` is cleared.
4. Assert assignment state is absent.
5. Assert ownership is false.
6. Assert the resident session remains as before.

Verification:

- The assignment map cannot retain normal stale ownership.

Atomic outcome: assignment state has a defined lifecycle endpoint.

## Step 10: format and run focused tests

1. Run `cargo fmt --all -- --check`.
2. If formatting is required, run `cargo fmt --all` as a mechanical rewrite.
3. Run focused tests for the new assignment test names.
4. Run existing recycle and transition-timeout tests.
5. Run all `lisa-plugin` library tests.

Expected commands:

```text
cargo test -p lisa-plugin recycled_codex
cargo test -p lisa-plugin transition_timeouts
cargo test -p lisa-plugin --lib
```

Verification:

- Every new test passes.
- Existing recycle and timeout tests pass.
- Formatting check is clean.

## Step 11: run broader verification

1. Run `cargo test --workspace`.
2. Run `cargo clippy -p lisa-plugin --all-targets -- -D warnings`.
3. If a preexisting lint baseline fails, identify exact unrelated diagnostics.
4. Run `git diff --check` on `lib.rs` and ticket work artifacts.
5. Inspect the source diff for accidental Claude or adapter behavior changes.

Verification:

- Workspace tests pass, or any unrelated baseline issue is documented precisely.
- No warning originates in the ticket-owned change.
- Diff whitespace validation passes.

## Step 12: write implementation progress

Create `progress.md` before committing source and record:

- completed research/design/structure/plan phases;
- enum and state-map implementation;
- schedule classification behavior;
- cleanup behavior;
- new test coverage;
- exact verification results;
- deviations from this plan, if any;
- source paths intended for the ticket commit.

Verification:

- Progress distinguishes completed work from remaining commit/review tasks.

## Step 13: commit the source through Lisa

Run exactly:

```text
lisa commit-ticket \
  --ticket-id T-033-01-01 \
  --message "feat: model recycled Codex seat assignments" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not include unrelated paths, the ticket file, or broad directories.

Verification:

- Command exits successfully and prints a commit ID.
- `crates/lisa-plugin/src/lib.rs` is clean after the transaction.
- The source path has no ordinary-index staged entry.
- Unrelated worktree and ordinary-index content remains untouched.

Atomic outcome: ticket-owned source is durable through the isolated transaction.

## Step 14: perform final review

1. Inspect the committed diff.
2. Re-run focused tests if the commit transaction reconciled worktree content.
3. Confirm the ticket frontmatter was not manually edited.
4. Confirm source ownership and staging invariants.
5. Write `review.md` with changes, tests, coverage, concerns, and commit identity.
6. Stop after `review.md` is complete.

## Acceptance checklist

- [ ] `SeatAssignmentState` names pending, owned, and recovering.
- [ ] Recycled/reused Codex schedules as `AssignedPendingAck`.
- [ ] Pending Codex reports not owned.
- [ ] Fresh Codex bookkeeping is explicit.
- [ ] Reused Claude remains owned and follows its existing clear handshake.
- [ ] Clear timeout preserves pending assignment.
- [ ] Exit timeout preserves pending assignment.
- [ ] Release clears assignment state.
- [ ] No adapter, UI, core type, or ticket-frontmatter change.
- [ ] Focused and workspace tests pass.
- [ ] Ticket-owned source is committed only through `lisa commit-ticket`.
- [ ] `review.md` provides the final handoff.
