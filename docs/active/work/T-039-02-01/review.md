# Review: T-039-02-01

## Outcome

The ticket is complete. A named test-only characterization suite now pins the
current behavior of all eight `check_*_signals` consumers before the structural
ingestion refactor in `T-039-02-02`. The suite covers the requested poll order,
payload admission, legacy filename handling, deletion timing, and per-consumer
effects. No product behavior was changed.

## Commit

- Commit: `ac8959b5a8838d262635f328ed81c51a81638cef`.
- Message: `T-039-02-01: characterize signal consumers`.
- The commit was created with `lisa commit-ticket`.
- Exact include: `crates/lisa-plugin/src/lib.rs`.
- Exact include:
  `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.
- The ordinary Git index was not used.
- The ordinary index is empty after the transaction.
- Both ticket-owned source paths are clean.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

- Added `mod signal_consumer_characterization;` inside the existing test module.
- This is the only edit to the runtime source file.
- The declaration is under `#[cfg(test)]` through its parent module.
- No scheduler method, signal consumer, state type, helper, or constant changed.
- No production visibility changed.

### `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

- Added a 399-line focused unit-test module.
- Added two compact fixture helpers.
- Added 11 characterization tests.
- All fixtures use temporary directories.
- Private scheduler state is observed through the existing unit-test boundary.
- No new production seam was introduced solely for testing.

## Characterized poll order

One test reads the `poll_tick` source region and locks these consumer calls in
their current relative order:

1. `check_heartbeat_signals`
2. `check_awaiting_signals`
3. `check_process_start_signals`
4. `check_shell_ready_signals`
5. `check_codex_ack_signals`
6. `check_idle_signals`
7. `check_transition_signals`
8. `check_error_signals`

The assertion permits the existing non-consumer operations between these calls.
It will fail if a consumer disappears or crosses another consumer in the order.
This directly protects the ordering contract needed by heartbeat clocks,
awaiting injection gates, observable process readiness, artifact publication,
and timeout precedence.

## Characterized payload admission

### Heartbeat

- Current filenames are pane-scoped `.heartbeat` records.
- Bodies must parse as `AttemptLease` JSON.
- Slot ticket, slot lease, and current lease must all match.
- Stale generations are consumed but do not refresh activity or clear gates.
- Exact current generations refresh slot/thread activity.
- Exact current generations clear awaiting-human and attention debounce state.

### Process start

- Current filenames are pane-scoped `.started` records.
- Bodies must parse as `AttemptLease` JSON.
- A stale generation is consumed without changing `Starting`.
- The exact current starting lease becomes `ReadyForAssignment`.
- Process-start evidence does not directly establish ticket ownership.

### Shell ready

- Current filenames are pane-scoped `.shell-ready` records.
- Bodies must parse as `AttemptLease` JSON.
- A predecessor lease is consumed without crossing the reset boundary.
- Only the exact current reset successor triggers the same-pane relaunch.
- The result is replacement `Starting` with the successor generation and bounded
  relaunch count.

### Codex acknowledgement

- Current filenames are pane-scoped `.ack` records.
- The scanner reads raw UTF-8 provider content.
- Downstream admission requires exact tagged `UserPromptSubmit` JSON.
- A stale tag is consumed without ownership or activity refresh.
- An exact tag promotes the pending seat to `Owned`.
- Exact admission refreshes activity and logs acknowledgement.

### Awaiting

- Current filenames are pane-scoped `.awaiting` records.
- Body content is ignored; presence is sufficient.
- Admission inserts the pane into `awaiting_human`.
- Awaiting intentionally does not refresh activity.

### Idle

- Current filenames may be pane-scoped `.idle` records.
- Legacy ticket-named `.idle` records remain accepted.
- Body content is ignored.
- The positive fixture uses a legacy Research signal with no artifact.
- Its effect is consumption plus one idle-without-artifact alert.
- The phase remains Research without the required artifact.

### Transition

- One consumer recognizes pane-scoped `.stopped` and `.cleared` records.
- Body content is ignored.
- A valid stopped record refreshes pane activity before state handling.
- The fixture uses an idle state to avoid native host input while observing the
  dispatch effect.
- `.idle` remains untouched for the idle consumer.
- Existing focused tests continue to cover active stopped/cleared state edges.

### Error

- Current filenames are pane-scoped `.error` records.
- Body content is ignored.
- A matching running thread is removed and made retryable.
- Its slot reservation is released while the resident session remains.
- Error alert and activity effects are asserted.

## Characterized filename compatibility

The suite submits a ticket-named candidate to every consumer. Heartbeat,
process-start, shell-ready, ack, awaiting, transition, and error leave those
unrecognized records untouched. Idle alone consumes the legacy
`{ticket_id}.idle` family. This negative/positive matrix prevents the next
refactor from accidentally granting uniform legacy support to pane-only signals
or removing the one compatibility route that exists.

## Characterized delete timing

The suite asserts one-shot deletion for a recognized current record from every
consumer even when parsing or downstream state admission cannot complete:

- malformed lease JSON for heartbeat, process start, and shell ready;
- malformed provider JSON for ack;
- inapplicable state for presence-only awaiting, idle, and error records;
- a pane-prefixed stopped name with nonnumeric pane ID for transition.

The transition case is especially precise because it observes deletion before
pane-ID parsing. Together, the cases lock the externally visible
delete-before-admission contract and prevent rejected signals from replaying on a
later poll.

## Test coverage

### New suite

- Command: `cargo test -p lisa-plugin signal_consumer_characterization`.
- Result: 11 passed, 0 failed.
- Each of the eight consumers has a positive effect assertion.
- Each of the eight consumers participates in the deletion matrix.
- Each of the eight consumers participates in the legacy-name matrix.
- Poll order is covered once as a complete ordered sequence.

### Repository gates

- `just check`: green.
- WASM check for `wasm32-wasip1`: green.
- Workspace native tests: 733 passed, 0 failed, 1 ignored.
- The ignored test is the existing real-Zellij environment-gated boundary.
- `just lint`: green for plugin, core, and CLI with warnings denied.
- `just fmt-check`: green.
- `git diff --check`: green.

## Review of acceptance criteria

- “All eight consumers”: satisfied by the named module and matrices.
- “Poll order”: satisfied by the `poll_tick` relative-order assertion.
- “Payload admission”: satisfied by exact/stale lease, exact/stale tagged JSON,
  and presence-body fixtures.
- “Legacy filename handling”: satisfied across all eight consumers.
- “Delete timing”: satisfied by rejected/inapplicable one-shot fixtures.
- “Per-consumer effects”: satisfied by eight focused positive paths.
- “Passes on the unmodified tree”: runtime behavior was not edited; the tests
  pass against the preexisting consumer implementations.
- “No product source changed”: no product logic changed; the only `lib.rs` edit
  is a test-only module declaration.

## Open concerns and limitations

- The poll-order test is source-structural rather than a full `poll_tick` host
  simulation. This is intentional because a full poll invokes unrelated Zellij
  and scheduling behavior. It precisely catches relative call reordering.
- The transition positive fixture observes stopped dispatch in an idle slot.
  Existing tests separately cover the host-writing `WaitingForStop` and
  `WaitingForClear` edges. The characterization suite avoids duplicating those
  host-sensitive fixtures.
- Filesystem iteration order is intentionally not asserted. Current consumers
  call `read_dir`, whose order is not a product guarantee.
- Read failures are not deterministically forced. Malformed payloads cover the
  stable observable deletion/admission boundary without platform-specific file
  permission behavior.
- Non-UTF-8 filename grammar remains covered by the existing shared parser test,
  not duplicated in this suite.
- The source-order assertion names the following method as its slice boundary;
  moving or renaming that boundary method will require a test-only adjustment.
- These are characterization tests, not an endorsement that every current
  asymmetry is ideal. `T-039-02-02` may change structure but should keep this
  suite green unchanged.

## Critical issues

None found. No product regression, test failure, lint failure, formatting issue,
staged ticket-owned file, or uncommitted ticket-owned source path remains.

## Handoff

The next structural ticket can use the named Cargo filter as a tight regression
gate while introducing typed signal ingestion. It should preserve the three
distinct payload families, idle-only legacy naming, delete-before-admission
semantics, and the exact relative polling order captured here. Lisa should now
perform the completion publication and commit; this agent remains on
`T-039-02-01` and does not start another ticket.

