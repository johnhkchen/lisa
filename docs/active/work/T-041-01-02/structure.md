# Structure: total completion reducer

## Source changes

Modify exactly one repository source file:

`crates/lisa-core/src/completion.rs`

No files are created or deleted in the crate. `lib.rs`, manifests, lockfile,
plugin source, and scheduler source remain unchanged.

## Public API additions

Add two variants to `CompletionRejection`:

```rust
UnexpectedEvent {
    state: &'static str,
    event: &'static str,
}

CorrelationMismatch {
    expected: CorrelationId,
    actual: CorrelationId,
}
```

Add the public reducer alongside `Transition` and before the test module:

```rust
pub fn reduce(
    state: CompletionState,
    event: CompletionEvent,
) -> Result<Transition, CompletionRejection>
```

The signature uses owned values consistently with the existing event/state
types. It returns the already-public transition and rejection types.

## Private helpers

Add private exhaustive label helpers:

- `state_name(&CompletionState) -> &'static str`
- `event_name(&CompletionEvent) -> &'static str`
- `unexpected(&CompletionState, &CompletionEvent) -> CompletionRejection`

Each label helper names every current enum variant explicitly. The reducer uses
these only before consuming values on invalid arms. No public formatting or
serialization contract is introduced.

Add private transition constructors only if they reduce duplication without
hiding the matrix:

- a request helper may build `Requested + LaunchCompletion`;
- a failure helper may build `Rejected + no effect`.

The reducer's state/event arms remain visible even when construction is shared.

## Reducer organization

The top-level match has one arm per state:

```text
CompletionState::Eligible
CompletionState::Requested
CompletionState::CommandInFlight { correlation }
CompletionState::Rejected { reason, retryability }
CompletionState::Confirmed
```

Within each arm, match all event variants explicitly. Do not use `_`, `..` to
hide enum variants, or combined outer-state catch-alls.

The legal edge map is:

```text
Eligible --Request/effect--> Requested
Requested --CommandLaunched--> CommandInFlight(correlation)
Requested --CommandLaunchFailed--> Rejected(retryable)
CommandInFlight --matching CommandSucceeded--> Confirmed
CommandInFlight --matching CommandFailed--> Rejected(event retryability)
Rejected(retryable) --Request/effect--> Requested
```

`Rejected(action-required) + Request` returns its retained reason. Duplicate
requests in requested/in-flight/confirmed return `AlreadyPending`. Mismatched
results return `CorrelationMismatch`. Remaining matrix cells return
`UnexpectedEvent`.

## Test organization

Extend the existing `#[cfg(test)] mod tests` in the same file. Preserve the
predecessor's vocabulary tests.

Add small fixture constructors for stable IDs and failures only where they keep
assertions readable. Tests should compare complete `Result` or `Transition`
values.

Legal-edge tests:

1. eligible request emits the expected sole launch effect;
2. launch acknowledgement records correlation without effect;
3. launch failure enters retryable rejected state without effect;
4. matching command success confirms without effect;
5. matching command failure preserves source and retryability without effect;
6. retryable rejection accepts a new request and emits one effect.

Illegal/refused-edge tests:

1. duplicate requests in requested/in-flight/confirmed are `AlreadyPending`;
2. action-required rejection returns its stored named reason;
3. success/failure correlation mismatch reports expected and actual IDs;
4. every remaining illegal state/event pair returns exact state/event names in
   `UnexpectedEvent`.

Extend the predecessor's rejection exhaustiveness test to include both new
variants. This ensures every public rejection has display text and an explicit
match arm.

## Dependency boundaries

The production module continues to depend only on `std` and `thiserror`.
`reduce` neither imports nor accepts any type from lisa-plugin, scheduler, CLI,
Zellij, filesystem adapters, process adapters, or async runtimes.

The function returns `EffectCommand` as inert data. Execution remains beyond
the crate and outside this ticket.

## Artifact and commit boundaries

Research, design, structure, plan, progress, and review artifacts live only in
`.lisa/attempts/T-041-01-02/1/work/`. They are not source includes.

The one meaningful source unit is committed with:

```text
lisa commit-ticket --ticket-id T-041-01-02 \
  --message "feat(core): add total completion reducer" \
  --include crates/lisa-core/src/completion.rs
```

If the installed binary lacks the subcommand, use the repository-built Lisa CLI
with the same ticket, message, and exact include. Do not stage or commit through
the ordinary index.

## Verification ordering

1. Format the changed Rust file through workspace formatting.
2. Run focused lisa-core completion tests.
3. Run `cargo test --workspace`.
4. Run the repository quick check if the environment supports the WASM target.
5. Inspect the exact source diff.
6. Commit the exact source path through Lisa's isolated transaction.
7. Verify the ticket-owned path is clean and unrelated worktree state remains.
8. Write final progress and review artifacts.
