# Review: level-triggered completion eligibility

## Disposition

Pass. The pure completion domain now re-derives request obligation from durable
current-attempt artifact admission and the explicit typed pass disposition,
suppresses duplicate work once a transaction is pending or confirmed, and
surfaces unresolved in-flight work as a bounded correlation-tagged actionable
outcome. The single owned source file is committed and all verification is
green.

## Commit

```text
35847e3f9df71113cf9c8af8af28a746cff8e1ab
feat(core): add completion reconciliation
```

The commit contains exactly:

- `crates/lisa-core/src/completion.rs`

No source file was created, deleted, or renamed. No manifest or lockfile
changed.

## Public API added

### `CurrentLeaseArtifactAdmission`

This value represents the positive result of the adapter's artifact/lease
authority check. It carries the admitted `AttemptId` and `CompletionId` needed
for an eventual completion transaction.

The core module does not inspect paths or leases itself. Constructing this
value is the later adapter's assertion that the artifact belongs to the
authoritative current attempt.

### `DurableCompletionInputs`

This aggregate contains:

- optional `CurrentLeaseArtifactAdmission`;
- existing typed `ReviewDisposition`.

Eligibility requires both an admission and exact `ReviewDisposition::Pass`.
Block and Invalid remain fail-closed. The implementation consumes the E-040
typed verdict rather than duplicating JSON parsing or representing pass as a
boolean.

### `Reconciliation`

The result enum has three independently matchable outcomes:

- `Effect(EffectCommand)` requests one inert adapter command;
- `None` means no new action is required;
- `CommandInFlightActionRequired { correlation }` preserves the exact identity
  of unresolved asynchronous work.

The actionable in-flight outcome cannot exist without a correlation ID and
contains no launch effect.

### `reconcile`

The public signature is:

```text
reconcile(&DurableCompletionInputs, &CompletionState) -> Reconciliation
```

It borrows all input, clones only identities returned in output, performs no
I/O, mutates no external state, and executes no effect.

## Behavior assessment

### Eligible obligation

An admitted current-attempt artifact plus exact Pass and `Eligible` aggregate
state returns the precise `LaunchCompletion` effect with the admission's
attempt and completion identities.

Because reconciliation derives this result from current facts each time, a
lost in-memory edge does not erase the obligation. A future poll/load adapter
can call it again and observe the same request requirement until state records
that a transaction exists.

### Ineligible inputs

Missing artifact admission returns None from a requestable state. Explicit
Block and Invalid dispositions also return None. A stale in-memory Eligible
state therefore cannot bypass either current-attempt admission or E-040's
explicit pass gate.

### Pending and confirmed transactions

Requested and Confirmed return None even while durable eligibility remains
true. This is the level-triggered deduplication boundary: after the adapter
applies the first request and records Requested, repeated reconciliation emits
no second command.

### Rejected states

A retryable rejection with still-eligible durable facts returns one fresh
request effect. An action-required rejection returns None and is never retried
automatically.

This matches the existing reducer's retry policy while keeping the durable
eligibility gate in force.

### Command in flight

CommandInFlight never becomes a launch request. It returns
`CommandInFlightActionRequired` with the state's exact correlation ID.

This result remains actionable even if the original admission or disposition
inputs are temporarily unavailable. The uncertainty belongs to a command that
already launched; hiding it would lose transaction state, while retrying it
would risk duplicate completion.

The decision is bounded: one invocation returns one named no-launch outcome
and contains no retry loop. Actual deadline persistence, journal replay, and
idempotent commit convergence remain explicitly owned by the follow-on
durability story.

## Shared effect construction

The reducer and reconciler both use a private `request_effect` constructor.
This preserves one definition of the attempt/completion payload without
calling the reducer from reconciliation and inventing a panic or impossible
error fallback.

The existing reducer's public signature and state/event behavior are unchanged.

## Provider parity

No production or test branch mentions Claude or Codex. The durable contract is
expressed only with completion identities, typed disposition, state, and
effect values. Provider adapters may discover evidence differently, but after
current-lease admission they feed the same function.

## Test coverage

Six new colocated unit tests cover all acceptance cases:

1. eligible admitted/pass request returns exact effect;
2. absent admission is ineligible;
3. Block and Invalid dispositions are ineligible;
4. Requested and Confirmed suppress effects;
5. retryable rejection emits an exact effect and action-required rejection
   emits none;
6. unresolved in-flight work yields exact correlation-tagged actionable state,
   even with unavailable durable inputs.

Existing completion reducer tests continue to cover legal transitions,
duplicate requests, correlation mismatches, launch failures, retry policy, and
the exhaustive illegal-event matrix.

## Verification results

The following passed after the final implementation:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
cargo clippy -p lisa-core --all-targets -- -D warnings
git diff --check -- crates/lisa-core/src/completion.rs
cargo test --workspace --quiet
cargo check -p lisa-plugin --target wasm32-wasip1 --quiet
```

Observed results:

- lisa-core unit tests: 191 passed, 0 failed;
- lisa-cli unit tests: 279 passed, 0 failed;
- lisa-plugin unit tests: 341 passed, 0 failed;
- integration suites: 1 + 3 + 1 passed, 0 failed;
- core doctests: 0 failed;
- real-Zellij integration: one existing environment-gated ignored test;
- clippy: passed with warnings denied;
- plugin WASM check: passed;
- formatting and diff checks: passed.

## Repository preservation

The source unit was committed only through `lisa commit-ticket` with the exact
repository-relative include path. Ordinary `git add`, `git add -A`, and
ordinary `git commit` were not used.

Post-commit verification shows `crates/lisa-core/src/completion.rs` clean and
the ordinary index empty. Orchestration-owned changes to the active ticket and
provenance remain untouched. Untracked plugin docs and Lisa-published work
artifacts remain outside this source commit.

## Open concerns and limitations

No blocking concern was found.

`CurrentLeaseArtifactAdmission` has public fields, so the pure core cannot
prevent an incorrect adapter from constructing it before validating the lease.
That is intentional at this story boundary: adapter integration and its
current-attempt tests are explicitly deferred to E-042.

`CommandInFlightActionRequired` does not contain a clock deadline. The pure
story has no durable time source or journal, and the named follow-on durability
story owns bounded deadlines and replay convergence. This ticket establishes
the no-retry, correlation-retaining decision contract that layer consumes.

Reconciliation returns a decision but not the next aggregate state. The future
adapter must apply the emitted request through the reducer and persist/store
the resulting Requested state before another reconciliation call. That wiring
is explicitly outside this ticket and is tested later at the adapter seam.

## Critical issues requiring human attention

None.

## Human review focus

A reviewer should confirm that the three-way `Reconciliation` outcome is the
desired adapter boundary and that treating unresolved in-flight state as
actionable even when original eligibility inputs are unavailable matches the
planned durability layer. The implementation otherwise stays within the
settled pure-domain story boundary and does not change runtime plugin behavior.

