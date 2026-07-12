# Review: T-034-02-02 gate completion on current lease

## Outcome

The completion boundary now rejects attempt-originated requests unless their
source lease exactly matches the scheduler's current lease for that ticket.

A stale predecessor cannot create pending completion state or launch the native
`complete-ticket` transaction.

A current lease follows the existing T-031 isolated transaction and durable
publication state machine unchanged.

The source implementation is committed through Lisa's isolated transaction at:

`b5a87227d15d002e531dd7a69ec333cf36d4422d`

## Files changed

### Modified source

- `crates/lisa-plugin/src/lib.rs`

### Created workflow artifacts

- `docs/active/work/T-034-02-02/research.md`
- `docs/active/work/T-034-02-02/design.md`
- `docs/active/work/T-034-02-02/structure.md`
- `docs/active/work/T-034-02-02/plan.md`
- `docs/active/work/T-034-02-02/progress.md`
- `docs/active/work/T-034-02-02/review.md`

No source files were created or deleted.

No CLI, configuration, hook, signal payload, serialized public type, or native
transaction interface changed.

The ticket frontmatter was not edited by this agent. Lisa advanced it from the
written phase artifacts.

## Authority model

Added private `CompletionAuthority`:

```text
Attempt(AttemptLease)  attempt-originated scheduler event
Operator               manual UI action with no active thread
```

`PendingCompletion` retains the authority that was admitted.

This makes the completion request describe both:

- diagnostic origin (`CompletionSource`);
- authority to perform the transition (`CompletionAuthority`).

Keeping these separate avoids conflating an Artifact, Idle, Stopped, Manual,
or ObservedDone trigger with the attempt identity that produced it.

## Admission behavior

`request_completion` still suppresses duplicate requests first.

It then validates authority before dependency checks, ticket file lookup,
pending-state insertion, command construction, or host launch.

Attempt authority is accepted only when:

```text
source_lease.is_current(current_leases[ticket_id]) == true
```

Exact lease equality covers both ticket ID and attempt ID.

The following fail closed:

- missing source authority;
- a prior attempt ID;
- a lease for another ticket;
- absent current authority;
- Operator authority paired with a non-Manual source;
- an existing active thread with no lease.

Rejection logs a scheduler warning and returns false.

It does not:

- create `PendingCompletion`;
- run `lisa complete-ticket`;
- change frontmatter;
- mutate either lease map;
- complete or remove the thread;
- release or rename the slot;
- emit Done provenance;
- unblock dependents.

## Caller coverage

All completion callers now provide explicit authority.

### Artifact

`check_artifact_advances` snapshots the logical thread's attempt lease and
passes it at Review-to-Done admission.

### Idle

Both idle-triggered Done paths resolve and pass the logical thread lease.

### Stopped

`auto_complete_review` resolves the exact physical slot matching the stopped
pane ID and ticket. It passes that slot's lease, so a stopped event is not
silently relabeled with a different pane's thread identity.

### Observed Done

The polling reconciliation snapshot carries each active thread's lease into
the same completion boundary.

### Manual

Manual completion with an active thread uses that thread's lease.

Manual completion with no active thread retains the existing operator recovery
behavior through explicit Operator authority. This exception is narrow:

- only `mark_ticket_done` creates it;
- only Manual source accepts it;
- attempt-originated callers cannot use it;
- an existing but unleased thread is rejected instead of being treated as an
  operator action.

## T-031 transaction preservation

The current-attempt path continues through the same code after admission.

Unchanged behavior includes:

- dependency validation;
- concrete ticket file resolution;
- pending Done masking during DAG rebuild;
- `build_completion_command` arguments;
- the `lisa_completion` result context;
- native `lisa complete-ticket` invocation;
- explicit ticket and work-directory transaction inputs;
- alternate-index commit serialization;
- commit-ID format validation;
- durable phase/status Done verification;
- recoverable failed-command handling;
- PhaseCompleted and TicketPhaseChanged publication;
- Done provenance timing;
- slot release;
- thread removal;
- dependent scheduling.

Only completion logs gained the admitted authority for diagnosis.

## Direct acceptance test

Added:

`request_completion_rejects_stale_attempt_and_accepts_current_lease`

The test creates attempt 1 and attempt 2 for one Review ticket, leaving attempt
2 current.

For attempt 1 it asserts:

- the lease is not current;
- `request_completion` returns false;
- no pending completion exists;
- thread and slot remain assigned;
- a stale-authority warning is logged.

For attempt 2 it asserts:

- the lease is current;
- the same request returns true;
- pending state exists with Attempt authority for attempt 2;
- the diagnostic source remains Artifact;
- ticket frontmatter remains Review before native transaction preparation.

This directly covers both halves of the ticket acceptance criterion at the
real scheduler boundary.

## Additional regression coverage

Added `test_mark_done_without_active_attempt_uses_operator_authority` to prove
the existing orphan-ticket manual recovery action still enters commit-gated
pending state without publishing Done early.

Added a test-only `install_current_attempt` helper that mirrors production
dispatch across:

- lease high-water;
- current authority;
- logical thread stamp;
- physical slot stamp.

Updated existing completion fixtures to model authoritative scheduled attempts
instead of legacy unstamped threads and slots.

The existing verified-success regression still simulates native preparation,
supplies a valid commit ID, verifies durable Done, releases the slot, removes
the thread, and observes the dependent-ready state.

The existing failed-manual-completion regression still proves retryability,
no early release, and no duplicate provenance.

## Verification

Passed:

```text
cargo test -p lisa-plugin request_completion_rejects_stale_attempt_and_accepts_current_lease
cargo test -p lisa-plugin
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check -- crates/lisa-plugin/src/lib.rs
git diff --check -- docs/active/work/T-034-02-02
```

Plugin result: 270 passed, 0 failed.

The workspace suite passed across CLI, core, plugin, and doc-test targets.

The plugin is clean under Clippy with warnings denied and builds for
`wasm32-wasip1`.

## Workspace-wide Clippy baseline

`cargo clippy --workspace --all-targets -- -D warnings` remains blocked by
pre-existing findings outside the ticket-owned file:

- twelve unnecessary `to_string` calls in `crates/lisa-core/src/dag.rs` tests;
- one needless generic-argument borrow in `crates/lisa-cli/src/init.rs`.

These files were not modified or committed for this ticket. The owned plugin
target passes the identical warning policy.

## Commit and worktree integrity

The source unit was committed with:

```text
lisa commit-ticket \
  --ticket-id T-034-02-02 \
  --message "Gate completion on current attempt lease" \
  --include crates/lisa-plugin/src/lib.rs
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs`.

The source path is clean after commit.

No ticket-owned source path is staged, modified, or untracked.

No ordinary `git add` or `git commit` command was used.

Unrelated pre-existing modified and untracked worktree paths were excluded.

## Open concerns and deferred scope

### Shared artifact attribution

Artifact completion currently receives the active thread lease, but shared-path
file existence does not prove which attempt wrote the file. T-034-02-03 owns
attempt-scoped artifact publication/admission and will supply enforceable writer
identity through this boundary.

This is the principal known limitation, explicitly sequenced by the story DAG.

### Liveness attribution

Heartbeat and other liveness signals are not changed here. T-034-02-03 owns
their stale-attempt rejection.

### Provenance authority

Pending completion now retains admitted authority, but provenance schema and
single authoritative Done attribution remain T-034-02-04 scope.

### In-flight lease change

This ticket validates authority before command launch, which is the required
commit admission boundary. It does not redesign timeout/release behavior while
a native completion command is in flight. Existing pending masking and the
short serialized transaction remain unchanged.

### Plugin restart

Lease high-water/current state remains process-local as established by the
prerequisite tickets. Persistence is outside this slice.

## Human review focus

A reviewer should confirm:

1. Attempt authority cannot reach command launch without exact current equality.
2. The stopped path uses the originating physical slot rather than a replacement
   logical thread.
3. Operator authority is reachable only from the manual no-thread recovery path.
4. The admitted current path below the new guard matches T-031 behavior.
5. T-034-02-03 consumes the explicit authority seam for attributable artifact
   publication instead of continuing to trust shared-path existence.

## Final assessment

The ticket acceptance criterion is satisfied with direct stale/current
coverage. The implementation is provider-neutral, preserves the isolated commit
transaction, keeps manual operator recovery functional, and leaves no
ticket-owned source changes outside Lisa's isolated source commit.

No critical issue requires human intervention before the next dependent ticket.
