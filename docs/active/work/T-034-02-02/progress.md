# Progress: T-034-02-02 gate completion on current lease

## Status

Implementation is complete and verified. The ticket-owned source change is
ready for Lisa's isolated commit transaction.

## Step 1 — model completion authority

Added private `CompletionAuthority` with two explicit forms:

- `Attempt(AttemptLease)` for every attempt-originated completion event;
- `Operator` for the existing manual recovery action when no active thread
  exists.

Changed `PendingCompletion` from Copy to Clone and stored the validated
authority on the pending record.

This retains the identity that crossed the completion boundary for diagnostics
and later attempt-aware provenance work.

## Step 2 — gate the request boundary

Extended `request_completion` with optional authority evidence.

After duplicate-pending suppression and before dependency/file/command work,
the method now:

1. accepts Attempt authority only when its lease is exactly current for the
   requested ticket;
2. accepts Operator authority only with `CompletionSource::Manual`;
3. rejects missing, stale, cross-ticket, or invalidly paired authority;
4. logs a warning and returns without creating pending state.

The method does not mutate lease state during validation.

## Step 3 — carry authority from all callers

Updated every `request_completion` caller.

Artifact completion snapshots the active logical thread lease.

Idle completion resolves the active logical thread lease in both Review paths.

Stopped Review completion resolves the exact slot matching both pane ID and
ticket, then uses that slot's attempt lease.

Observed-Done reconciliation snapshots the active logical thread lease.

Manual completion uses the active thread lease when one exists. When no thread
exists, it uses explicit Operator authority, preserving the pre-existing UI
recovery path for orphaned tickets. An existing unleased thread is not upgraded
to operator authority.

## Step 4 — preserve the T-031 transaction

No changes were made to completion command construction, command context,
native `complete-ticket`, alternate-index staging, commit-ID validation,
durable Done verification, failure recovery, successful teardown, or dependent
scheduling.

Completion result logs now include the stored authority for diagnosis.

## Step 5 — direct acceptance regression

Added:

`request_completion_rejects_stale_attempt_and_accepts_current_lease`

The test mints two leases for one Review ticket and installs the successor as
current.

It proves the predecessor:

- fails `is_current`;
- is rejected by the real request boundary;
- creates no pending completion;
- leaves the thread and slot assigned;
- emits a visible rejection warning.

It then proves the successor:

- passes the same boundary;
- creates pending completion state;
- is retained as Attempt authority;
- leaves frontmatter at Review pending native transaction preparation.

## Step 6 — preserve operator recovery

Added:

`test_mark_done_without_active_attempt_uses_operator_authority`

This test proves a manual action on a ticket with no active attempt continues
to enter the existing commit-gated transaction with explicit Operator
authority and does not publish Done early.

## Step 7 — update realistic completion fixtures

Added test-only `install_current_attempt`, which mirrors dispatch by:

- minting from lease high-water;
- inserting the same lease as high-water and current;
- stamping a matching thread;
- stamping a matching assigned slot.

Eleven existing completion fixtures initially failed because they constructed
pre-lease threads/slots directly. Updated only the fixtures that intentionally
model a real authoritative attempt.

Covered paths include:

- artifact Review completion;
- all-artifact catch-up;
- Implement-to-Review-to-completion catch-up;
- idle completion;
- stopped completion and dependency rejection;
- manual completion;
- failed transaction retry;
- verified transaction success;
- Codex artifact-only phase advancement.

## Verification results

### Focused acceptance

```text
cargo test -p lisa-plugin request_completion_rejects_stale_attempt_and_accepts_current_lease
```

Result: passed, 1 passed.

### Plugin suite

```text
cargo test -p lisa-plugin
```

Result: passed, 270 passed, 0 failed.

### Workspace suite

```text
cargo test --workspace
```

Result: passed across CLI, core, plugin, and doc-test targets.

### WASM target

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Result: passed.

### Formatting

```text
cargo fmt --all -- --check
```

Result: passed.

### Plugin Clippy

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Result: passed.

### Workspace Clippy

```text
cargo clippy --workspace --all-targets -- -D warnings
```

Result: blocked by existing out-of-scope warnings:

- twelve `clippy::unnecessary_to_owned` findings in
  `crates/lisa-core/src/dag.rs` tests;
- one `clippy::needless_borrows_for_generic_args` finding in
  `crates/lisa-cli/src/init.rs`.

The ticket-owned plugin target is clean under the same warning policy.

### Diff checks

```text
git diff --check -- crates/lisa-plugin/src/lib.rs
git diff --check -- docs/active/work/T-034-02-02
```

Result: passed.

## Deviation from the original plan

The first design draft treated every completion as attempt-originated. During
implementation, review of `open_mark_done_modal` showed that manual Done is
intentionally available for tickets with no running thread. Rejecting that path
would leave a visible but nonfunctional recovery action.

The design was refined to distinguish current Attempt authority from explicit
Operator authority. Operator is accepted only for Manual requests and cannot
be produced by artifact, idle, stopped, or observed-Done callers.

The Design, Structure, and Plan artifacts were updated to reflect this
compatibility-preserving boundary.

## Remaining implementation work

None.

T-034-02-03 remains responsible for authoritative attribution of shared
artifact publication and heartbeat signals. This ticket supplies the explicit
authority input that its artifact admission path can use.

## Isolated source commit

Committed the exact ticket-owned source path through Lisa:

```text
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-02-02 \
  --message "Gate completion on current attempt lease" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit: `b5a87227d15d002e531dd7a69ec333cf36d4422d`

No ordinary-index staging or ordinary Git commit command was used.
