# Design: T-031-02 gate done on commit

## Decision summary

Introduce one scheduler completion state machine backed by a native
`lisa complete-ticket` command. Every scheduler path that wants to move a ticket
to Done will enqueue the same pending completion, launch the native command, and
wait for its attributed `RunCommandResult`.

The native command will prepare `phase: done` and `status: done` and then invoke
T-031-01's isolated transaction with the real ticket file and ticket work
directory as explicit includes. The scheduler will publish Done only after the
command exits successfully with a verified commit ID. Failure leaves or restores
non-Done disk content, preserves the thread/slot, blocks dependents, emits no Done
provenance, and records an actionable error.

## Goals

- One request path for artifact, idle, stopped Review, finish-up, manual, and
  externally observed completion.
- One success publisher for thread completion, provenance, slot release, DAG
  unblocking, dependent scheduling, and loop termination.
- No in-memory Done before the native transaction succeeds.
- No duplicate command for repeated signals while a request is pending.
- Exact correlation between a host result and its ticket attempt.
- Provider-neutral behavior for Claude, Codex, and future adapters.
- Preserve T-031-01's alternate-index and foreign-staging guarantees.

## Option 1: write Done in WASM, then call `commit-ticket`

The plugin could save the old ticket bytes, call the existing two frontmatter
writers, then launch `lisa commit-ticket`.

Advantages:

- Reuses the current CLI without a new subcommand.
- Keeps all phase mutation visibly in the scheduler.

Disadvantages:

- Done exists uncommitted between the WASM write and native lock acquisition.
- A poll can rebuild the DAG during the asynchronous host command and unblock
  dependents before the command result.
- Failure restoration occurs outside the serialized native critical section.
- A plugin crash after writing but before launching recreates the field failure.
- Two frontmatter writes can leave phase Done with status open.

Rejected. It cannot make prepare-and-commit a coherent native operation.

## Option 2: native completion wrapper around the isolated transaction

Add a completion-specific native wrapper which saves the original ticket bytes,
prepares both Done fields, and calls the existing transaction.

Advantages:

- Preparation and staging are adjacent inside one native invocation.
- Failure can restore exact original ticket bytes before returning nonzero.
- The existing transaction remains the only Git commit implementation.
- The plugin receives a simple success/failure boundary.
- The command is independently process-testable.

Disadvantages:

- Adds a second CLI command with some argument overlap.
- The ticket is briefly Done on disk while the command runs; the plugin must mask
  the pending transition from its in-memory DAG until the result arrives.
- A post-ref cleanup failure in the underlying transaction is intrinsically
  ambiguous because `HEAD` may already have advanced. The command must report the
  failure accurately and avoid claiming scheduler success.

Chosen. It gives the narrowest reliable mutation window and cleanest scheduler
contract available across WASM and host process boundaries.

## Option 3: move Git transaction code into the WASM plugin

The plugin could implement alternate-index/ref operations directly.

Advantages:

- Fewer asynchronous state transitions.
- Direct access to scheduler state.

Disadvantages:

- WASI cannot use the native process and locking implementation unchanged.
- It duplicates T-031-01 and weakens the provider-neutral CLI boundary.
- Native Git subprocess behavior and filesystem locking are already solved.

Rejected as duplication and an unsuitable runtime boundary.

## Option 4: ask the agent to commit after Lisa publishes Done

This is the pre-existing compatibility behavior described by the ticket.

Rejected. The ticket explicitly forbids publish-first/commit-later semantics.

## Native command contract

Add:

```text
lisa complete-ticket \
  --path <repo-root> \
  --ticket-id <id> \
  --ticket-file <repo-relative-real-path> \
  --work-dir <repo-relative-ticket-work-dir> \
  --message <message>
```

The command will:

1. Validate the ticket/work paths are repository-relative and ticket-scoped.
2. Read and retain the exact original ticket bytes.
3. Update phase and status to Done using a single ticket-module operation.
4. Invoke `commit_ticket` with explicit includes for the ticket file and work dir.
5. Print the verified commit ID on success.
6. Restore original ticket bytes when the transaction fails before durable
   completion and return an actionable error.

The CLI wrapper delegates all Git staging, locking, ref update, index preservation,
and verification to T-031-01. It does not create another Git pathway.

## Atomic frontmatter preparation

The current ticket module exposes separate phase and status writers. Add a small
core helper that transforms both fields and performs one filesystem write.

This avoids an intermediate `phase: done` / `status: open` state and gives native
tests a single semantic operation. Existing single-field helpers remain for
non-Done phase advancement and reset behavior.

The completion wrapper retains original bytes rather than reconstructing old
frontmatter. Rollback therefore preserves comments, ordering, line endings, and
all unrelated fields exactly.

## Scheduler state model

Add a per-ticket pending map:

```rust
struct PendingCompletion {
    prior_phase: Phase,
    source: CompletionSource,
}

enum CompletionSource {
    Artifact,
    Idle,
    Stopped { pane_id: u32 },
    Manual,
    ObservedDone,
}
```

Only one entry may exist per ticket. A repeated request is a no-op with no second
host process. The prior phase is used to keep the in-memory DAG non-Done while the
native command is in flight.

The source is diagnostic only. It must not select different completion logic.

## Request transition

`request_completion(ticket_id, source)` will:

1. Return if the ticket already has a pending request.
2. Locate the ticket and verify direct dependencies are Done.
3. Require configured `lisa_bin` and a known project root.
4. Derive repository-relative real ticket/work paths by stripping `/host` and
   validating they are under the project root/configured roots.
5. Insert pending state before launching the command, closing duplicate-trigger
   races within the plugin event loop.
6. Launch `complete-ticket` with a context key containing the ticket ID.
7. Log that commit-gated completion is pending.

No thread, slot, provenance, or DAG-ready state changes at request time.

## Pending-Done masking

The native command can finish its disk write before Zellij delivers the result.
During that interval a poll may scan the ticket as Done.

`rebuild_dag` will replace a pending ticket's scanned Done phase/status with its
prior non-Done in-memory values before constructing scheduling state. This makes
the pending map the authority until the command result is consumed.

The mask prevents:

- dependent readiness;
- done-thread teardown;
- stale-slot release;
- all-done notification;
- termination.

After success removes the pending entry, a fresh rebuild reads durable Done.

## Successful result transition

The `RunCommandResult` handler recognizes a distinct completion context key.
Success requires:

- exit code zero;
- stdout containing a plausible hexadecimal Git object ID;
- the pending entry still existing for that ticket;
- a fresh ticket scan showing both phase and status Done.

Then, in order:

1. Remove the pending entry.
2. Rebuild the DAG and confirm the durable Done state.
3. Log phase completion/change.
4. Complete the thread if present.
5. Emit exactly one Done provenance record while the thread remains available.
6. Release the assigned slot.
7. Remove the thread.
8. Schedule newly ready dependents.

All-done notification and termination remain in `poll_tick`, which now sees only
verified Done transitions.

## Failed result transition

For nonzero exit, missing exit code, malformed success output, or failed durable
state verification:

1. Remove the pending entry so the ticket is retryable.
2. Rebuild the DAG from restored non-Done disk state.
3. Keep the thread and its slot assigned.
4. Emit no provenance.
5. Schedule no dependent on this ticket.
6. Log stderr/exit details as an activity error with recovery guidance.

If the native wrapper cannot restore ticket bytes, its combined error must expose
that critical condition. The scheduler still refuses to publish Done.

## Routing every completion trigger

- `check_artifact_advances`: when Review's next phase is Done, request completion
  instead of writing phase.
- `check_idle_signals`: both Review idle and Implement-idle-with-existing-review
  request completion.
- `handle_stopped_signal`: Review stop requests completion.
- `mark_ticket_done`: manual confirmation requests completion.
- Finish-up timeout: remains a prompt; its eventual artifact/idle/stop enters the
  same request function.
- Generic observed Done: a running thread whose scanned disk ticket becomes Done
  without a verified pending result is routed through the same transaction rather
  than being torn down directly.

No call site writes phase Done independently.

## Source ownership and final commit contents

The transaction includes the real ticket path and entire ticket work directory,
which contains all six artifacts. Those are the loop-owned outstanding paths at
completion.

Arbitrary shared-worktree source changes cannot be safely inferred by the
scheduler; capturing them would risk stealing concurrent ticket or human work.
The RDSPI Implement phase already requires incremental commits after meaningful
units. The completion state machine therefore requires source implementation to
be committed before Review completion and atomically commits all remaining
loop-owned ticket changes with the Done frontmatter.

This preserves T-031-01's explicit-ownership invariant. T-031-03 can reinforce
the provider prompt/contract and live-run evidence.

## Provenance exact-once rule

Only the successful result publisher emits `RunOutcome::Done`.

Request sites, failed results, observed pending Done scans, stale-slot sweeps, and
generic audit paths never emit Done. Because the pending entry is consumed before
publication and the thread is removed in the same synchronous event handler, a
duplicate command result cannot find a publishable attempt.

## Reused-seat behavior

The slot retains its ticket ID, provider ownership, and resident-session state
while completion is pending. No transition handshake or next-ticket reservation
begins until success releases the slot. This applies equally to reused Codex and
Claude sessions.

On failure the seat remains recoverable: it is still attached to its Review
thread, the dashboard shows the transaction error, and a later stop/idle/manual
trigger can retry.

## Test design

### Core tests

- Combined Done update changes both fields in one resulting document.
- Missing/malformed fields return errors without partial content.

### CLI transaction tests

- Completion commit contains Done frontmatter plus all six work artifacts.
- Foreign staged content remains staged and excluded.
- A forced commit failure restores exact original ticket bytes.
- Successful stdout is a commit ID and included paths are clean afterward.

### Plugin state tests

- Automatic Review artifact completion creates one pending request and does not
  complete/release before result success.
- Finish-up/Review-stop flow routes to the same pending state.
- Manual mark-done uses the same request.
- Failure retains non-Done DAG state, thread, slot, and zero Done provenance.
- Success publishes once and duplicate results do not duplicate provenance.
- A reused Codex slot stays assigned until success.
- A dependent remains blocked while pending and schedules only after success.
- Observed external Done does not trigger the old generic teardown path.

### Required verification

- Focused core/CLI/plugin tests.
- `cargo test --workspace`.
- WASM release build.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings` or the repository's
  accepted plugin Clippy invocation.
- `git diff --check` on ticket-owned changes.

## Rejected compatibility behavior

There will be no code path that writes/publishes Done and asks an agent to commit
afterward. There will also be no fallback that releases a seat merely because a
ticket file happens to say Done while its completion command is pending or has
failed.
