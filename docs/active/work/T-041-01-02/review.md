# Review: total completion reducer

## Disposition

Pass. The ticket acceptance criterion is satisfied, the exact owned source unit
is committed, and all required verification is green.

## Summary

This ticket adds a pure, total completion-domain reducer to lisa-core. The
reducer accepts owned `CompletionState` and `CompletionEvent` values and returns
either an explicit `Transition` or a typed `CompletionRejection`.

The reducer is a handwritten exhaustive state/event matrix. Every public state
appears in the outer match, and every public event appears in each inner match.
There is no wildcard arm that could silently absorb a newly introduced state.

Accepted request transitions return `EffectCommand::LaunchCompletion` as data.
The function does not execute that command. All other accepted transitions
return no effect, so every transition continues to carry at most one command by
construction through `Option<EffectCommand>`.

## Files changed

### `crates/lisa-core/src/completion.rs`

Added:

- `CompletionRejection::UnexpectedEvent`
- `CompletionRejection::CorrelationMismatch`
- public `reduce(state, event)`
- private request and rejected transition constructors
- private exhaustive state/event naming helpers
- exact legal-edge tests
- duplicate-request tests
- retry-policy tests
- correlation mismatch tests
- complete invalid callback/state matrix coverage

No other source, manifest, lockfile, plugin, scheduler, or CLI file changed.

## Behavior reviewed

### Eligible request

An eligible aggregate accepts Request, enters Requested, and returns exactly one
launch effect with the request's attempt and completion identities.

### Launch acknowledgement

Requested accepts CommandLaunched, enters CommandInFlight with the supplied
correlation, and returns no effect. The existing state type continues to make
an in-flight state without correlation unrepresentable.

### Launch failure

Requested accepts CommandLaunchFailed as a lifecycle fact, enters Rejected with
the exact `LaunchFailed` source and retryable policy, and returns no effect.

### Command results

CommandInFlight accepts success only when its correlation matches, then enters
Confirmed. It accepts matching failure, preserves source and retryability, and
enters Rejected. Neither result emits an effect.

A mismatched result returns `CorrelationMismatch { expected, actual }` and
therefore cannot confirm or reject a different in-flight command.

### Retry and duplicate behavior

A retryable rejected state accepts a new Request and emits one fresh launch
effect. An action-required rejected state refuses Request with its retained
typed reason.

Requested, CommandInFlight, and Confirmed refuse duplicate Request with
`AlreadyPending`, returning no transition or effect. Confirmed is terminal.

### Invalid ordering

Callback events that do not apply to the current lifecycle state return
`UnexpectedEvent` with independently matchable state and event names. This
avoids falsely classifying ordering errors as lease, dependency, disposition,
or adapter failures.

## Acceptance assessment

- Pure `reduce(state, event) -> Result<Transition, CompletionRejection>` exists:
  satisfied.
- Exhaustive matching with no state-hiding catch-all: satisfied.
- Reducer launches nothing: satisfied; launch is returned only as inert data.
- Reducer mutates no external state: satisfied; it has no external reference or
  adapter dependency.
- Legal edges assert expected effect cardinality and value: satisfied.
- Illegal edges assert correct named rejection: satisfied across duplicate,
  action-required, mismatch, and complete callback matrix tests.
- `cargo test --workspace` green: satisfied.

## Test coverage

The completion module has 16 passing focused unit tests. New coverage directly
asserts:

- initial request effect payload;
- launch correlation storage;
- launch-failure source and retryability;
- success confirmation;
- command-failure source and retryability;
- retry request effect payload;
- duplicate requests in all pending/terminal states;
- action-required retained rejection;
- success and failure correlation mismatch;
- every remaining illegal callback/state pair;
- display/exhaustive handling for every rejection variant.

The final `cargo test --workspace` run passed with zero failures. `just check`
also passed, including the `wasm32-wasip1` lisa-plugin check. No runtime mock or
integration adapter test is needed because this ticket deliberately exposes no
runtime boundary.

## Purity review

Production additions use only owned domain values and existing pure types. The
reducer contains no filesystem operations, process commands, Zellij imports,
WASM imports, time reads, random values, global mutable state, scheduler calls,
or callbacks. Private helpers only construct values or map enum variants to
static labels.

## Commit and worktree review

Source commit:

`eae63f07ddb4ada49b8ba9cc44abf323b4343944`

It was created through `lisa commit-ticket` with exactly:

`--include crates/lisa-core/src/completion.rs`

The exact ticket-owned source path is clean after the transaction. The
remaining modified ticket/provenance files and untracked plugin docs/shared
work directory were pre-existing or Lisa-managed and were not included.

## Open concerns and limitations

`Requested` does not retain the previously accepted completion identity, so a
duplicate request rejection can only carry the incoming completion ID. This is
an inherited state-shape constraint from T-041-01-01, not a reducer regression.
Durable idempotency identity is explicitly deferred to a later epic.

Both pre-launch and post-launch adapter failures use the inherited
`CompletionRejection::LaunchFailed` variant. The event correlation and state
still distinguish their lifecycle position, and tests preserve the source and
retry policy.

`UnexpectedEvent` stores static enum labels rather than recursively storing a
state that could itself contain a rejection. These labels are diagnostics, not
a serialized wire format.

The reducer is not yet wired into lisa-plugin, by explicit story boundary.
T-041-01-03 will add level-triggered eligibility/reconciliation. Neither is a
blocker for this ticket.

No critical issue, TODO, or uncommitted ticket-owned source remains.
