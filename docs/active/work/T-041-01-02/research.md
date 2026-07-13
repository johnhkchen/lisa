# Research: total completion reducer

## Ticket boundary

T-041-01-02 starts after T-041-01-01 and owns the pure reducer for the
completion aggregate. Its acceptance criterion is one function,
`reduce(state, event) -> Result<Transition, CompletionRejection>`, plus focused
unit tests. The function must be total over the public state and event enums,
must expose illegal edges as typed rejections, and must perform no I/O.

The story deliberately excludes the lisa-plugin adapter, command execution,
durable journaling, scheduler mutation, dashboard rendering, and manual `[d]one`
behavior. Those existing completion callers remain untouched.

## Existing module

`crates/lisa-core/src/completion.rs` is the domain boundary created by the
predecessor ticket. `crates/lisa-core/src/lib.rs` publishes it as
`pub mod completion`. No plugin module currently imports its types.

The module imports only `std::fmt` and `thiserror::Error`. It contains no
Zellij, WASM, filesystem, process, scheduler, or async dependency. This makes a
free function over owned values naturally pure.

The three identity types are opaque string-backed newtypes:

- `AttemptId` identifies the attempt claiming completion authority.
- `CompletionId` identifies the completion aggregate/request instance.
- `CorrelationId` identifies a launched asynchronous command and its result.

Each supports construction, borrowing, display, and string conversion. The
identities are cloneable and equality-comparable, which is sufficient for
effect assertions and correlation checks.

## State vocabulary

`CompletionState` has five variants:

- `Eligible`: durable inputs currently authorize a request.
- `Requested`: a request was accepted and its launch effect was emitted.
- `CommandInFlight { correlation }`: launch was acknowledged and a correlated
  result is pending.
- `Rejected { reason, retryability }`: a request or command reached an explicit
  rejected outcome.
- `Confirmed`: authoritative completion succeeded.

Only `CommandInFlight` stores a correlation. Therefore a result can be checked
only while in that state, and an in-flight state without correlation is
unrepresentable.

`Requested` does not retain attempt or completion identity. Consequently the
reducer can reproduce those identities in the launch effect at request time,
but it cannot later report the identity of a previously accepted request.

`Rejected` retains a full `CompletionRejection` and a separate
`Retryability`. This permits a retry decision without parsing error text.

## Event vocabulary

`CompletionEvent` has five variants:

- `Request { attempt_id, completion_id }`
- `CommandLaunched { correlation }`
- `CommandLaunchFailed { source }`
- `CommandSucceeded { correlation }`
- `CommandFailed { correlation, source, retryability }`

Request is the only event carrying attempt and completion identities. Result
events carry correlations. Launch failure occurs before a correlation exists.
Command failure carries both an adapter-neutral owned failure and retry policy.

Events represent facts supplied to the aggregate. There is no callback or
function field through which the reducer could execute external behavior.

## Output vocabulary

`Transition` contains a next `CompletionState` and
`Option<EffectCommand>`. The option statically caps one accepted transition at
zero or one command.

The only effect today is `EffectCommand::LaunchCompletion`, carrying the
request attempt and completion identities. The reducer returns this value as
data; an adapter is responsible for execution in later work.

`CompletionRejection` currently has the five story-required domain outcomes:
already pending, stale lease, disposition blocked, dependency blocked, and
launch failed. Several represent eligibility facts that the next reconciliation
ticket will derive. `LaunchFailed` wraps `LaunchFailure`; the source chain is
already unit-tested.

The current rejection enum has no representation for a lifecycle event that is
inapplicable to a state or for a result whose correlation differs from the
in-flight correlation. Total reduction requires those cases to remain typed;
silently accepting them or mislabeling them as a lease/disposition/dependency
failure would weaken the public contract.

## Existing tests and patterns

The module-local tests use direct construction and exact `assert_eq!` checks.
They already cover identity values, the mandatory in-flight correlation,
rejected-state payloads, optional effect cardinality, distinct rejection
variants, and error source chaining.

Other lisa-core modules use ordinary handwritten matches and colocated unit
tests. There is no FSM macro or shared reducer framework. The epic explicitly
keeps the state machine handwritten and inspectable.

The workspace uses Rust 2021, stable cargo tests, and `thiserror` version 2 in
lisa-core. No new dependency is required for this ticket.

## Natural lifecycle edges

The type and variant documentation establish the forward lifecycle:

1. `Eligible + Request` accepts and emits `LaunchCompletion`.
2. `Requested + CommandLaunched` records the correlation and emits no effect.
3. `Requested + CommandLaunchFailed` records a rejected launch outcome.
4. `CommandInFlight + CommandSucceeded` confirms when correlation matches.
5. `CommandInFlight + CommandFailed` records rejection when correlation
   matches, preserving source and retryability.

The `Rejected` retryability payload also supplies enough information for a
retryable rejection to accept a later `Request`. An action-required rejection
can return its retained typed reason without emitting an effect.

A `Request` received after `Requested`, `CommandInFlight`, or `Confirmed` is a
duplicate against a pending or completed transaction. The incoming completion
identity is the only available identity for an `AlreadyPending` rejection.

Result correlation equality is a domain boundary. Accepting a mismatched
result would allow one command to confirm or reject another aggregate state.

## Repository state and ownership

The ordinary worktree already contains Lisa-owned changes to provenance and the
ticket frontmatter, plus an unrelated untracked plugin docs directory. They are
not owned by this ticket and must remain untouched.

The expected ticket-owned source path is only
`crates/lisa-core/src/completion.rs`. Attempt artifacts belong under
`.lisa/attempts/T-041-01-02/1/work/` and are not passed to `commit-ticket`.

The source unit must be committed with `lisa commit-ticket` and that exact
repository-relative include. Ordinary `git add` and `git commit` are forbidden
for this workflow.

## Verification constraints

The acceptance criterion explicitly requires `cargo test --workspace` to be
green. A focused `cargo test -p lisa-core completion` provides a faster first
check. Formatting should be checked before committing. Because the change is
pure lisa-core Rust and introduces no target-specific imports, the workspace
test is the principal required verification; the repository's broader quick
check can additionally validate the WASM target when available.

No phase or status frontmatter may be edited. After all source changes are
committed and tests pass, `progress.md`, `review.md`, and the exact review
disposition JSON remain private artifacts for Lisa to admit and publish.
