# Review: completion domain types

## Disposition

Pass. The implementation meets the ticket acceptance criterion and preserves
the ticket's pure-domain boundary. The source unit is committed through Lisa's
isolated transaction, required tests pass, and no ticket-owned source remains
staged, modified, or untracked.

## Change summary

Commit: `806c7c729407795c80b36cd4de5975dfa4506fec`

Message: `feat(core): add completion domain vocabulary`

The commit contains four exact paths:

- `crates/lisa-core/src/completion.rs` (created)
- `crates/lisa-core/src/lib.rs` (modified)
- `crates/lisa-core/Cargo.toml` (modified)
- `Cargo.lock` (modified)

No file was deleted.

## Public API delivered

The crate now exposes `lisa_core::completion`, a pure module containing:

- `AttemptId`
- `CompletionId`
- `CorrelationId`
- `Retryability`
- `LaunchFailure`
- `CompletionRejection`
- `CompletionState`
- `CompletionEvent`
- `EffectCommand`
- `Transition`

All three identity types are opaque newtypes rather than primitive aliases.
They provide construction, borrowing, formatting, conversion, comparison,
ordering, and hashing while preventing accidental interchange at compile time.

## State invariant review

The lifecycle contains exactly the required named states:

- Eligible
- Requested
- CommandInFlight
- Rejected
- Confirmed

`CommandInFlight` has a mandatory `CorrelationId` field. It has no default and
the correlation is not optional, so an uncorrelated in-flight state cannot be
constructed through the public enum.

`Rejected` has mandatory `CompletionRejection` and `Retryability` fields. A
caller cannot create a rejected state that omits either its reason or its retry
policy.

`Transition` contains one optional effect rather than a vector. An accepted
transition can therefore request zero or one external command, never multiple
commands. Constructing a transition performs no effect.

## Rejection review

Every required rejection is a distinct enum variant:

- `AlreadyPending`
- `StaleLease`
- `DispositionBlocked`
- `DependencyBlocked`
- `LaunchFailed`

The first two preserve the relevant completion or attempt identity. The block
variants preserve operator-visible detail. Launch failure preserves an owned,
adapter-neutral source error.

`thiserror` is a direct lisa-core dependency used for standard rejection/error
Display and source behavior. The module imports no plugin, process-launch,
Zellij, or WASM APIs.

## Event and effect review

Events describe request, launch, launch failure, correlated success, and
correlated failure facts. Asynchronous result events retain correlation. The
launch effect retains the attempt authority and completion identity needed by a
future adapter.

The module intentionally contains no `reduce` or `reconcile` function. Those
are the explicit scopes of dependent tickets T-041-01-02 and T-041-01-03.
Existing plugin completion paths remain unchanged as required by S-041-01's
honest boundary.

## Test coverage

Six focused completion unit tests cover:

- identity construction and formatting;
- mandatory in-flight correlation;
- rejected-state reason and retryability retention;
- zero-or-one transition effect cardinality;
- distinct matching and Display for all rejection variants;
- standard error source chaining for launch failure.

Verification results:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p lisa-core`: 175 passed, 0 failed.
- `cargo test --workspace`: passed across all crates and non-ignored
  integration tests.
- Observed workspace suites: 279 CLI, 175 core, 341 plugin, and 5 non-ignored
  integration tests passed.
- The pre-existing real-Zellij integration test was ignored by its explicit
  environment gate.
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passed.
- `git diff --check` for ticket-owned paths: passed.

## Repository preservation

The source commit was made only with `lisa commit-ticket` and exact include
paths. It did not consume or alter the ordinary Git index. Post-commit status
shows no ticket-owned source changes.

The working tree still contains unrelated orchestration-owned modifications to
Lisa provenance and the active ticket, plus unrelated untracked plugin docs and
Lisa-published work artifacts. These were intentionally excluded and remain
outside this ticket's source ownership.

## Open concerns and limitations

No blocking concern was found.

The identity values currently accept empty strings. The ticket requires nominal
identity separation, not content validation, and the existing repository has no
settled format for these new IDs. Validation can be added without exposing the
private representation if a durable schema later defines stricter rules.

The types do not derive serde. This avoids prematurely declaring a journal wire
format; persistence is explicitly deferred to a follow-on epic. The opaque
representations allow serde to be added deliberately when that schema lands.

`CommandFailed` reuses the adapter-neutral `LaunchFailure` message wrapper even
for post-launch failure detail. This keeps the public error surface small for
the vocabulary ticket; a later adapter or reducer ticket can rename/generalize
that wrapper if command-result failures acquire structured fields.

The reducer ticket must still define the exhaustive state/event legality table.
This review confirms only that the required vocabulary and structural
invariants are present and ready for that work.

## Human review focus

A human reviewer should primarily confirm that the chosen identity
representations and event names are suitable for T-041-01-02 before that ticket
builds exhaustive behavior on them. No runtime behavior changed in this commit,
so there is no migration or operational rollout concern for this ticket alone.
