# Progress: T-039-02-01

## Status

- Research is complete.
- Design is complete.
- Structure is complete.
- Plan is complete.
- Implementation is complete.
- Narrow and repository-wide verification are green.
- The ticket-owned source unit is ready for isolated commit.
- Review remains after commit verification.

## Completed implementation work

### Test-only module boundary

- Added a child module declaration inside the existing `#[cfg(test)] mod tests`.
- Created `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.
- Kept all new helpers and assertions under the test configuration.
- Did not change any `State` runtime method.
- Did not change any signal filename parser behavior.
- Did not change any scheduler call ordering.
- Did not add or change a public interface.
- Did not change dependencies or Cargo manifests.

### Poll-order characterization

- Added `poll_tick_preserves_the_eight_consumer_order`.
- The test inspects only the `poll_tick` source region.
- It asserts a monotonically increasing sequence of eight call expressions.
- The asserted order is heartbeat, awaiting, process start, shell ready, ack,
  idle, transition, and error.
- Unrelated scheduler operations between the calls remain permitted.
- Missing or reordered calls produce a consumer-specific failure.

### Cross-consumer deletion characterization

- Added `recognized_records_are_one_shot_before_payload_or_state_admission`.
- The test covers all eight consumers in isolated temp directories.
- Heartbeat, started, and shell-ready use malformed lease JSON.
- Ack uses malformed provider JSON.
- Awaiting, idle, and error use arbitrary presence bodies.
- Transition uses a pane-prefixed stopped filename with a nonnumeric pane ID.
- Every recognized record is asserted deleted.
- The rejected fixtures demonstrate deletion does not wait for semantic admission.

### Cross-consumer legacy-name characterization

- Added `idle_alone_admits_the_legacy_ticket_filename_family`.
- The test covers a ticket-named legacy candidate for all eight consumers.
- Seven pane-scoped consumers are asserted to leave the ticket-named file alone.
- Idle is asserted to recognize and delete `T-LEGACY.idle`.
- This makes the unique compatibility route explicit.

### Heartbeat characterization

- Added a slot, running thread, current lease, and stale clocks fixture.
- A stale generation is consumed without refreshing activity.
- A stale generation does not clear awaiting-human state.
- A stale generation does not clear attention debounce.
- An exact current lease is consumed and refreshes both activity clocks.
- An exact current lease clears awaiting-human and attention debounce state.

### Process-start characterization

- Added a current leased seat in `Starting`.
- A stale generation is consumed without changing the starting state.
- The exact current lease promotes the seat to `ReadyForAssignment`.
- The test asserts process-start does not directly make the seat `Owned`.

### Shell-ready characterization

- Reused the scheduler fixture to create a genuine timed-out startup.
- Captured the revoked predecessor and current reset successor.
- Asserted predecessor shell proof is consumed without relaunch.
- Asserted the reset state remains pending after predecessor proof.
- Asserted exact successor proof is consumed and relaunches in the same pane.
- Asserted the replacement `Starting` state carries successor generation.
- Asserted the bounded relaunch count is preserved.

### Codex acknowledgement characterization

- Added a current leased seat pending acknowledgement.
- A stale tagged provider payload is consumed without ownership.
- Stale evidence does not bump pane activity.
- An exact `UserPromptSubmit` assignment tag promotes to `Owned`.
- Successful acknowledgement bumps activity.
- Successful acknowledgement emits the existing informational activity record.

### Awaiting characterization

- Submitted arbitrary non-JSON body content.
- Asserted the record is consumed by presence.
- Asserted the pane is inserted into `awaiting_human`.
- Asserted awaiting does not refresh pane activity.

### Idle characterization

- Created a temporary Research ticket and running thread.
- Omitted `research.md` deliberately.
- Submitted a legacy ticket-named idle record with arbitrary body content.
- Asserted the record is consumed.
- Asserted the thread remains in Research.
- Asserted one idle-without-artifact alert names the missing artifact.

### Transition characterization

- Submitted an arbitrary stopped body for a valid pane.
- Asserted the stopped record is consumed by presence.
- Asserted the pane activity clock is refreshed.
- Used an idle slot so no native host input call was required.
- Asserted an idle file remains untouched for the idle consumer.
- The cross-consumer deletion test separately locks delete-before-pane-parse.

### Error characterization

- Submitted arbitrary non-JSON error detail for a running pane.
- Asserted the signal is consumed by presence.
- Asserted the running thread is removed.
- Asserted its slot reservation is released while the session stays resident.
- Asserted the error alert and error activity effect are recorded.

## Deviations and fixture corrections

### Poll source boundary

- The initial test expected a nonexistent next-method name after `poll_tick`.
- The narrow run failed before behavioral assertions.
- The boundary was corrected to the actual following method,
  `format_activity_event`.
- No production code changed.

### Thread phase initialization

- The initial legacy-idle fixture assumed `Thread::new` inherited Research.
- Current `Thread::new` initializes `current_phase` to Ready.
- The fixture now explicitly sets `Phase::Research` to match its ticket.
- This correction documents current constructor behavior and changes test data only.

### Plan refinement

- The cross-consumer negative/deletion behavior was consolidated into two table
  tests rather than repeated inside every positive test.
- Positive tests then focus on payload admission and observable effects.
- This preserves every planned dimension with less duplicated fixture code.
- Existing focused transition tests continue to cover host-writing state edges.

## Verification evidence

### Narrow suite

- Command: `cargo test -p lisa-plugin signal_consumer_characterization`.
- Result: 11 passed, 0 failed.
- The first run identified the two fixture corrections above.
- The corrected run completed in approximately 0.01 seconds.

### Formatting

- Command: `cargo fmt --all` during implementation.
- Command: `cargo fmt --all -- --check` at the final gate.
- Result: green.

### Diff validation

- Command: `git diff --check`.
- Result: green.
- Manual diff inspection confirms the runtime file change is only the test child
  module declaration.

### WASM check

- Command: `just check`.
- WASM component: `cargo check -p lisa-plugin --target wasm32-wasip1`.
- Result: green.

### Workspace tests

- Command: `cargo test --workspace` through `just check`.
- `lisa-cli`: 274 unit tests passed.
- CLI integration tests: 4 passed across active integration binaries.
- Real-Zellij delivery boundary: 1 ignored by its documented environment gate.
- `lisa-core`: 155 tests passed.
- `lisa-plugin`: 303 tests passed, including all 11 new tests.
- Doc tests: 0 failures.
- Total executed tests: 733 passed, 0 failed, 1 ignored.

### Clippy

- Command: `just lint`.
- `lisa-plugin` WASM target with `-D warnings`: green.
- `lisa-core` with `-D warnings`: green.
- `lisa-cli` with `-D warnings`: green.

## Source ownership

Ticket-owned source paths are exactly:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

Lisa-owned concurrent paths observed in the worktree include:

- `.lisa/provenance.jsonl`
- `docs/active/tickets/T-039-02-01.md`
- `docs/active/work/T-039-02-01/`

Those Lisa-owned paths are excluded from the source transaction. No ordinary
`git add`, broad add, or ordinary `git commit` has been used.

## Isolated commit result

- Command: `lisa commit-ticket --ticket-id T-039-02-01` with exact repeated
  `--include` paths for the module declaration and characterization file.
- Message: `T-039-02-01: characterize signal consumers`.
- Commit: `ac8959b5a8838d262635f328ed81c51a81638cef`.
- Commit inventory: 2 files changed, 401 insertions.
- `crates/lisa-plugin/src/lib.rs` contributes only two test-module lines.
- The new characterization module contributes 399 test-only lines.
- `git diff --cached --name-only` is empty after the transaction.
- Both ticket-owned source paths are clean after the transaction.
- Lisa-owned ticket, provenance, and shared publication paths remain outside the
  transaction as required.

## Remaining

- Write `review.md` with commit evidence and final concerns.
- Stop on this ticket for Lisa's completion transaction.

