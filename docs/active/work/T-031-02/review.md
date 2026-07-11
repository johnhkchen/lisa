# Review: T-031-02 gate done on commit

## Outcome

T-031-02 now makes isolated commit verification the authority for publishing
ticket completion. Artifact, idle, stopped Review, finish-up, manual mark-done,
and externally observed Done flows all enter one asynchronous scheduler state
machine. Until the attributed native command succeeds, Lisa retains the thread,
seat, prior in-memory phase/status, blocked dependents, and live loop.

The native `lisa complete-ticket` command prepares both Done frontmatter fields,
commits the real suffixed ticket path and complete ticket work directory through
T-031-01's alternate-index transaction, and prints a verified commit ID. The
plugin publishes Done only after receiving that successful result and verifying
the durable ticket scan.

## Files modified

### `crates/lisa-core/src/ticket.rs`

- Added `update_ticket_done`.
- Transforms phase and status in memory before one filesystem write.
- Preserves the rest of the ticket document through existing frontmatter update
  behavior.
- Added success and malformed-frontmatter/no-partial-write tests.

### `crates/lisa-cli/src/commit_transaction.rs`

- Added `CompleteTicketRequest` and `complete_ticket`.
- Validates separate repository-relative Markdown ticket and work-directory
  paths.
- Saves exact original ticket bytes.
- Prepares Done, then delegates all Git work to the existing `commit_ticket`.
- Restores exact original bytes when the transaction fails.
- Added compensating rollback after `HEAD` advancement for reconciliation,
  verification, or cleanup failure.
- Rollback guardedly restores the previous `HEAD` and reconciles only committed
  ticket paths in the ordinary index.
- Added idempotent verification for an already committed Done ticket whose
  explicit ticket/work paths are clean.
- Added four completion-specific process regressions on top of T-031-01's
  existing transaction tests.

### `crates/lisa-cli/src/main.rs`

- Added `lisa complete-ticket`.
- Accepts repository root, ticket ID, message, real ticket file, and work dir.
- Prints only the commit ID on success.
- Uses the CLI's established actionable error/nonzero-exit behavior.

### `crates/lisa-plugin/src/lib.rs`

- Added `CompletionSource` for Artifact, Idle, Stopped, Manual, and ObservedDone.
- Added `PendingCompletion` with prior phase/status and source diagnostics.
- Added `pending_completions` to scheduler state.
- Added real-path conversion and argv-based native completion command building.
- Added a distinct `lisa_completion` command-result context.
- Added duplicate-request suppression.
- Added pending-Done masking during every DAG rebuild.
- Added pending guards to stale-slot and orphan-thread safety sweeps.
- Added centralized failure handling and centralized successful publication.
- Routed every plugin completion trigger through the shared request method.
- Removed direct plugin writes of phase/status Done.
- Removed generic poll teardown based solely on scanned Done.
- Updated old immediate-completion tests to assert the pending boundary.
- Added successful artifact completion, failed/retried manual completion,
  dependent gating, reused Codex seat, and exact-once provenance regressions.

## Files created

- `docs/active/work/T-031-02/research.md`
- `docs/active/work/T-031-02/design.md`
- `docs/active/work/T-031-02/structure.md`
- `docs/active/work/T-031-02/plan.md`
- `docs/active/work/T-031-02/progress.md`
- `docs/active/work/T-031-02/review.md`

No file was deleted. This session did not edit the ticket frontmatter.

## Completion state machine

### Request side

All trigger sites call `request_completion`.

The method:

1. Rejects duplicate in-flight attempts.
2. Checks dependencies and the actual discovered ticket path.
3. Captures prior phase/status.
4. Inserts pending state before host dispatch.
5. Runs the configured native Lisa executable with explicit ticket/work paths.

It does not complete a thread, emit Done provenance, release a slot, schedule a
dependent, notify all-done, or terminate the loop.

### Pending side

The native process briefly owns preparation and commit while the WASM event loop
can continue polling. If a scan sees Done before the result arrives,
`rebuild_dag` overlays the pending ticket's prior phase/status. This keeps all DAG
and scheduler consequences non-Done until the result receipt is consumed.

Repeated review artifacts, idle signals, stopped signals, or manual actions are
deduplicated by ticket ID.

### Success side

Success requires:

- attributed pending ticket;
- exit code zero;
- plausible 40- or 64-character hexadecimal commit ID;
- fresh scan with both phase and status Done.

Only then Lisa removes the pending mask, logs completion, completes the thread,
emits one Done provenance record, releases the slot, removes the thread, and
schedules ready dependents. Normal polling subsequently owns all-done
notification and loop termination.

### Failure side

On native failure Lisa removes pending state, rebuilds the restored non-Done
ticket, retains its thread and slot, emits no Done provenance, leaves dependents
blocked, and logs exit/stderr details with retry guidance.

The native transaction now compensates for failures after ref advancement. If
cleanup or post-commit verification fails, it guardedly moves `HEAD` back and
reconciles ticket paths before `complete_ticket` restores the original ticket
bytes.

## Acceptance criteria assessment

### One completion state machine

Met. All plugin-originated and observed Done flows call `request_completion`;
there is one result handler and one successful publisher. `rg` finds no direct
plugin `update_ticket_phase(...Done)` or Done-status writer.

### Final durable commit contents

Met for loop-owned outstanding completion content. `complete-ticket` explicitly
includes the real ticket file and entire `docs/active/work/<ticket-id>` directory,
so all six artifacts and both Done fields are committed together.

RDSPI requires meaningful source implementation units to be committed during
Implement. The scheduler does not broadly infer arbitrary shared-worktree source
ownership, which would violate T-031-01 by stealing concurrent/human changes.

### Publish only after verified success

Met. Pending masking prevents in-memory Done, thread completion, provenance,
seat release, dependent readiness, all-done notification, and termination before
the successful result plus durable ticket scan.

### Recoverable transaction failure

Met in covered failure paths. Exact ticket bytes are restored, transaction ref
advancement is compensated, thread/seat remain, dependents remain blocked, and
activity includes an actionable error.

### No uncommitted loop-owned residue on success

Met. The transaction includes ticket/work paths and reconciles those exact paths
against new `HEAD`; foreign staged entries remain unchanged and excluded.

### Required regressions

Covered:

- automatic Review artifact completion;
- Review finish-up prompt plus stopped Review route;
- manual mark-done failure/retry;
- reused Codex seat retained until success;
- dependent blocked while pending and ready after success;
- existing idle/Implement-with-review completion;
- externally observed/pre-committed Done;
- failed-attempt zero provenance and eventual exact-once success.

### Provenance exact once

Met. Only successful publication emits Done. The combined regression fails an
attempt, retries successfully, delivers a duplicate result, and observes one
ledger record.

## Test coverage

### Core

- Combined phase/status update.
- Malformed ticket produces no partial write.

### CLI transaction

Twelve focused tests now cover original isolation behavior plus:

- completion commit containing Done frontmatter and all six artifacts;
- foreign staged entry preservation/exclusion during completion;
- exact non-Done byte restoration on commit failure;
- compensating rollback restoring `HEAD`, ticket working changes, and foreign
  staged bytes;
- already committed clean Done verification without creating another commit.

### Plugin

The 236-test plugin suite includes pending-boundary assertions for artifact,
idle, stopped, and manual triggers. New integration-style state tests cover
durable success, failure/retry, DAG masking, reused Codex slot retention,
dependent scheduling boundary, and provenance deduplication.

## Verification evidence

- Focused core ticket tests: 32 passed.
- Focused CLI transaction tests: 12 passed.
- Full plugin suite: 236 passed.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- `cargo clippy -p lisa-cli --bin lisa -- -D warnings`: passed.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: passed.
- `cargo test --workspace`: passed.
- Final `just check`: passed with 267 CLI, 147 core, 236 plugin tests, WASM
  check, and doc tests.
- `cargo run -q -p lisa-cli -- complete-ticket --help`: passed.
- Ticket-owned `git diff --check`: passed.

## Commits

- `2cd5089` — atomic combined frontmatter preparation.
- `52da264` — native completion command and process tests.
- `e85b313` — scheduler gate, routed triggers, and plugin regressions.
- `b8903cd` — compensating rollback for incomplete commit success.
- `ef5aa39` — idempotent externally committed Done verification.

All commits were created through T-031-01's exact-path alternate-index
transaction. Unrelated modified/untracked worktree files were not included.

## Open concerns and limitations

### Shared-worktree source ownership

The final completion transaction owns ticket/work paths, not every unstaged path
in the repository. Arbitrary code ownership cannot be inferred safely while
multiple ticket agents and humans share one worktree. Implementation code must
therefore be committed in RDSPI's required incremental units before Review.
T-031-03 should reinforce this provider contract and prove it in the live mixed
provider regression.

### Native preparation visibility

`complete-ticket` prepares Done immediately before entering `commit_ticket`.
There is a short on-disk interval before the transaction acquires its lock. Lisa's
pending DAG mask prevents any scheduler publication during that interval, and
failure restores exact bytes. An unrelated external process that ignores Lisa's
state/lock can still momentarily observe the prepared working-tree content.

### Catastrophic rollback failure

If guarded ref rollback itself fails because of concurrent `HEAD` movement or a
filesystem/Git failure, the command returns a combined critical error and Lisa
refuses to publish. This is intentionally fail-closed, but human Git repair may
be required. The successful rollback path is process-tested; deterministic
injection of rollback failure is not.

### Live Zellij process boundary

Native tests simulate attributed command results rather than launching a real
Zellij host command. The argv builder, event correlation, and state transitions
are covered, while T-031-03 owns the live provider-contract regression.

## Reviewer focus

Reviewers should concentrate on:

1. `handle_completion_result` ordering and exact-once teardown/provenance.
2. Pending phase/status masking in `rebuild_dag`.
3. `complete_ticket` byte restoration and already-committed verification.
4. `rollback_after_ref_advance` correctness for ordinary index reconciliation.
5. The explicit source-ownership boundary handed to T-031-03.

No known critical issue remains for this ticket's scheduler integration scope.
