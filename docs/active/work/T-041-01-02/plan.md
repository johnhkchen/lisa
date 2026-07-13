# Plan: total completion reducer

## Step 1: extend typed rejection coverage

In `crates/lisa-core/src/completion.rs`, add truthful rejection variants for an
unexpected lifecycle event and a mismatched command correlation. Give each a
`thiserror` display that includes enough context for diagnostics.

Update the existing exhaustive rejection unit test so the compiler forces both
new variants to be handled.

Verification: run the focused completion test module and confirm rejection
display assertions remain nonempty.

## Step 2: add exhaustive naming helpers

Add private helpers that map every `CompletionState` and `CompletionEvent`
variant to a static name. Implement them with explicit matches and no wildcard.
Add a small helper that constructs `UnexpectedEvent` from borrowed state/event
values.

Verification: compilation itself proves every current enum variant is named.
The illegal matrix test in Step 5 verifies the returned names.

## Step 3: implement request transitions

Add the public owned-value `reduce` function. Implement the `Eligible` state
first: Request produces `Requested` plus the exact `LaunchCompletion` effect;
all command events are refused as unexpected.

Implement request behavior for other states:

- Requested, in-flight, and confirmed return `AlreadyPending` and never emit.
- Retryable rejected accepts Request and produces a fresh request/effect.
- Action-required rejected returns the retained rejection and never emits.

Verification: exact unit assertions cover initial request, duplicate request,
retry request, and action-required refusal.

## Step 4: implement command lifecycle transitions

Implement `Requested + CommandLaunched` to create the correlation-carrying
in-flight state.

Implement `Requested + CommandLaunchFailed` to create a retryable rejected
state containing the exact source.

Implement matching in-flight command success and failure. Success confirms;
failure preserves both source and retryability in a rejected state. Neither
emits an effect.

Implement mismatched result correlation as `CorrelationMismatch`. Refuse every
other callback/state combination as `UnexpectedEvent`.

Verification: one exact-value test per legal lifecycle edge plus success and
failure mismatch assertions.

## Step 5: prove the illegal matrix

Create a table-driven test for illegal callback events across Eligible,
Requested, Rejected, and Confirmed states, excluding the separately tested
legal and specially rejected cells. Assert the exact `UnexpectedEvent` state
and event labels for every entry.

Keep explicit duplicate-request, action-required, and correlation-mismatch
tests separate because those outcomes carry semantic payloads beyond matrix
labels.

Verification: the combination of explicit reducer arms and tests must make it
impossible for any public state to be hidden by a catch-all.

## Step 6: format and run focused tests

Run workspace formatting. Run:

```text
cargo test -p lisa-core completion
```

Resolve compilation, equality, display, or ownership issues locally. Inspect
the source diff to ensure only the completion module changed.

## Step 7: run required workspace verification

Run:

```text
cargo test --workspace
```

This is the acceptance-gated command. If it exposes an unrelated environmental
failure, record the exact failure and continue only if ticket-owned tests can
still be established honestly.

Run `just check` as the broader repository validation when available; it adds
the WASM check used by project guidance. Record every command and result in
`progress.md`.

## Step 8: commit the source unit

Before committing, inspect `git status --short` and the exact diff for
`crates/lisa-core/src/completion.rs`. Preserve Lisa-owned ticket/provenance
changes and unrelated untracked files.

Commit only the reducer source unit:

```text
lisa commit-ticket --ticket-id T-041-01-02 \
  --message "feat(core): add total completion reducer" \
  --include crates/lisa-core/src/completion.rs
```

If necessary, invoke `target/debug/lisa` after building lisa-cli, retaining the
same arguments. Do not use ordinary Git staging or commit commands.

Verification: confirm the exact source path is no longer modified and inspect
the resulting commit. Unrelated pre-existing worktree entries must remain
unchanged.

## Step 9: complete implementation record

Write `progress.md` with completed steps, deviations, source commit identity,
test results, and remaining work. If implementation required a design change,
record its rationale before Review.

## Step 10: review and disposition

Review the committed diff against every acceptance clause:

- public pure reducer signature exists;
- state matching is exhaustive without a state-hiding catch-all;
- accepted transitions carry zero or one effect;
- illegal edges yield typed named rejections;
- tests cover every legal edge and illegal categories;
- workspace tests pass;
- no ticket-owned source remains dirty.

Write `review.md` with files, behavior, coverage, open concerns, and commit
details. Write exactly one valid `review-disposition.json` object. Use pass only
if the implementation is committed and the required test suite is green.

Stop on this ticket after the Review artifacts. Do not edit ticket frontmatter,
publish artifacts, or start successor work.
