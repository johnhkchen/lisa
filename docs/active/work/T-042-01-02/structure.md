# Structure: typed completion source folding

## File inventory

One repository source file is modified:

`crates/lisa-plugin/src/lib.rs`

No source file is created or deleted. No manifest, core reducer, CLI,
serialization format, or public API changes. Private phase artifacts are
created only under `.lisa/attempts/T-042-01-02/1/work/` and are not part of the
ticket source commit.

## Completion adapter vocabulary

Extend private `CompletionInput` near `CompletionSource` with three variants:

```text
Idle {
    ticket_id: TicketId,
    source_lease: AttemptLease,
}

ObservedDone {
    ticket_id: TicketId,
    source_lease: Option<AttemptLease>,
}

Manual {
    ticket_id: TicketId,
    authority: Option<CompletionAuthority>,
}
```

Artifact and Stopped remain unchanged. The enum becomes the complete typed
plugin input vocabulary for all currently existing production completion
entry points.

`CompletionSource` remains the diagnostic/storage vocabulary and retains its
current variants. No source value is removed because pending completion and
result diagnostics use it.

## Dispatcher organization

`State::dispatch_completion` remains immediately after Review admission and
before the effect executor. Its match expands from a three-value tuple to an
internal normalized request description:

```text
ticket_id
source
authority
review_lease
```

Artifact, Stopped, and Idle set `review_lease` to the exact attempt lease and
authority to that same lease. ObservedDone and Manual set no Review lease and
carry their existing optional authority.

The dispatcher admits a passing Review only when `review_lease` is present.
This keeps evidence-source policy inside the typed seam.

It derives core aggregate state from `pending_completions`, creates one
`CompletionEvent::Request`, calls `reduce_completion`, logs reducer rejection,
and sends only a returned effect to the executor.

Attempt ID derivation becomes authority-based:

- Attempt lease → decimal attempt generation;
- Operator → `operator`;
- no authority → `missing-authority`.

Completion ID remains the ticket ID.

## Deleted helpers

Delete `State::request_review_completion` entirely.

Delete `State::request_completion` entirely.

No compatibility alias or alternate non-boolean wrapper is added. The typed
dispatcher is the only request gateway, and the executor is reachable from
production request code only inside that method.

## Effect executor boundary

`State::execute_completion_effect` keeps its location and signature. It
continues to:

- exhaustively unpack `EffectCommand::LaunchCompletion`;
- validate effect identity against ticket and authority;
- reject duplicate pending requests;
- require a current attempt lease or authorized Manual operator;
- require dependencies to be Done;
- resolve ticket file and prior state;
- insert `PendingCompletion`;
- record accepted effects in native tests;
- build normalized completion argv;
- invoke the sole host command launch;
- log the pending transaction.

No caller besides `dispatch_completion` may invoke it in production code.
Test-only direct calls may remain for executor-specific failure contracts.

## Idle call sites

In `check_idle_signals`, the Implement-to-Review catch-up branch currently has
`source_lease: Option<AttemptLease>`. After confirming Review exists:

- Some lease → dispatch `CompletionInput::Idle`;
- None → log `Rejected completion ... (Idle): no attempt lease`.

The Review-next-Done branch already reacquires an optional lease. Apply the
same branch structure and typed input.

The caller no longer passes `CompletionSource` or chooses Review admission.

## Observed Done reconciliation call site

In `poll_tick`, retain the snapshot of running threads whose rescanned DAG
ticket is Done. Replace the legacy request call with:

```text
dispatch_completion(CompletionInput::ObservedDone {
    ticket_id,
    source_lease,
})
```

The optional lease reaches the executor as optional attempt authority. This
preserves fail-closed reconciliation for malformed thread state.

Update the nearby comment to identify the typed adapter and clarify that this
post-timeout/post-reload scan is the current observed-Done reconciliation
source.

## Manual UI call site

`mark_ticket_done` retains its current authority selection:

- existing thread → optional attempt lease;
- no thread → Operator.

It dispatches `CompletionInput::Manual { ticket_id, authority }`. It returns
unit as before. Modal selection and key handling do not change.

## Existing tests to migrate

Tests that call the deleted `request_completion` must be rewritten according
to their boundary:

- stale/current attempt request test → dispatch an input that does not add
  unrelated Review admission, or invoke the executor only if specifically
  testing executor authority;
- fenced replacement authoritative completion → dispatch ObservedDone or
  create the typed input appropriate to its established evidence;
- split-brain stale predecessor request → dispatch ObservedDone to test the
  full gateway without requiring the predecessor's private Review;
- effect identity mismatch → remain a direct executor test because it tests
  the executor's returned-effect trust boundary.

Where a test is semantically about Artifact, ensure its private passing Review
disposition exists before using `CompletionInput::Artifact`.

## New behavioral tests

Add a focused test that creates a valid current lease and uses the dispatcher
for remaining sources in isolated State fixtures or resets:

- Idle with passing Review emits one exact effect and stores source Idle;
- ObservedDone emits one exact effect and stores source ObservedDone;
- Manual via `mark_ticket_done` emits one exact effect and stores source Manual.

Existing Artifact/Stopped coverage remains the proof for those variants.

Use exact `EffectCommand::LaunchCompletion` assertions with AttemptId and
CompletionId to show reducer-produced identity crosses the executor seam.

## New source invariant test

Add a test near other completion adapter tests:

```text
completion_has_one_typed_request_gateway
```

The test reads `include_str!("lib.rs")`, truncates at the `#[cfg(test)] mod
tests` marker, and asserts the legacy method declarations are absent.

It locates `fn dispatch_completion` and `fn execute_completion_effect`. The
dispatcher slice must contain exactly one
`self.execute_completion_effect(` call. The production prefix must contain
exactly one such call total. This proves no second production caller bypasses
the reducer.

It also asserts the executor slice contains exactly one
`run_command_with_env_variables_and_cwd(` call, preserving one completion
command launch location.

The invariant intentionally does not ban unrelated Rust functions returning
bool. It targets the architectural completion names and edges, avoiding false
positives across a large plugin file.

## Verification boundaries

First run formatting and a focused plugin test filter for the gateway and
remaining source tests. Then run the full plugin suite to expose call-site
compatibility issues. Run full workspace tests because lisa-plugin now has a
dev dependency on lisa-cli and connected transaction coverage.

Run:

- `cargo fmt --all -- --check`;
- focused `cargo test -p lisa-plugin ...`;
- `cargo test -p lisa-plugin --no-fail-fast`;
- `cargo test --workspace --no-fail-fast`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `git diff --check`.

## Transaction boundary

After tests, update private `progress.md`, inspect the exact diff, and run:

```text
lisa commit-ticket
  --ticket-id T-042-01-02
  --message "refactor(plugin): route all completion sources through typed adapter"
  --include crates/lisa-plugin/src/lib.rs
```

If the installed binary does not expose the command, use the repository-built
`target/debug/lisa` with identical arguments. Do not use the ordinary index.

After the isolated transaction, the source path must be absent from ordinary
staged, modified, and untracked status. Unrelated Lisa-managed and pre-existing
untracked paths remain preserved.
