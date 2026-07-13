# Structure: level-triggered completion eligibility

## Change inventory

One source file is modified:

- `crates/lisa-core/src/completion.rs`

No module declaration, manifest, lockfile, plugin, CLI, scheduler, or dashboard
file changes. No source file is created or deleted. Phase artifacts remain in
the attempt-private work directory for Lisa publication.

## Import boundary

`completion.rs` adds:

```rust
use crate::disposition::ReviewDisposition;
```

This is a sibling core-domain type. Production imports remain limited to
`std`, `thiserror`, and `lisa-core` modules. No Zellij, WASM, client, plugin,
filesystem, process, or clock type enters the module.

## Durable admission type

Add `CurrentLeaseArtifactAdmission` after the identity definitions and before
the completion lifecycle values. It is a positive domain fact containing:

```text
attempt_id: AttemptId
completion_id: CompletionId
```

The adapter that eventually constructs this value is responsible for checking
that the artifact belongs to the authoritative current attempt. The type does
not carry a boolean lease flag because its construction is the positive fact.

Derives: `Debug`, `Clone`, `PartialEq`, `Eq`.

## Durable input aggregate

Add `DurableCompletionInputs` beside the admission type. It contains:

```text
artifact_admission: Option<CurrentLeaseArtifactAdmission>
disposition: ReviewDisposition
```

`None` means no current-lease artifact has been admitted. Only
`ReviewDisposition::Pass` satisfies the second eligibility condition.

Derives: `Debug`, `Clone`, `PartialEq`, `Eq`.

## Reconciliation output

Add `Reconciliation` after `Transition`, where decision outputs are already
defined. Its variants are:

```text
Effect(EffectCommand)
None
CommandInFlightActionRequired { correlation: CorrelationId }
```

`Effect` is the single request effect. `None` is the literal absence of a new
obligation action. The in-flight variant retains the mandatory identity of the
unresolved command and never carries a launch effect.

Derives: `Debug`, `Clone`, `PartialEq`, `Eq`.

## Reconcile function

Add the public free function after `reduce` and before private reducer helpers:

```text
reconcile(
    durable_inputs: &DurableCompletionInputs,
    state: &CompletionState,
) -> Reconciliation
```

The function first checks durable eligibility. If admission is absent or the
disposition is not exact Pass, it returns `Reconciliation::None`.

For eligible inputs it exhaustively matches `CompletionState`:

- Eligible delegates a Request event to `reduce`.
- retryable Rejected delegates a Request event to `reduce`.
- Requested returns None.
- Confirmed returns None.
- action-required Rejected returns None.
- CommandInFlight clones its correlation into the actionable outcome.

## Internal request helper

A small private helper may accept the borrowed state and admission, invoke
`reduce`, and extract the single effect. It returns `Reconciliation::Effect`
when the reducer accepts the request. The helper is called only for the two
states whose reducer request edges are legal.

The helper must not execute the returned effect or update caller state. Its
purpose is to prevent manual duplication of `request_transition` behavior.

## Test placement

Add tests to the existing `#[cfg(test)] mod tests` in the same file. Add fixture
helpers only if they reduce repetition without hiding eligibility inputs.

The primary cases are:

1. current-lease admission + Pass + Eligible returns exact launch effect;
2. missing admission returns None;
3. Block and Invalid each return None;
4. Requested returns None;
5. Confirmed returns None;
6. retryable Rejected returns a fresh exact effect;
7. action-required Rejected returns None;
8. CommandInFlight returns the exact correlation-tagged actionable outcome.

Requested and Confirmed may share a table test because both establish the
no-effect pending/confirmed invariant. Block and Invalid may share a table test
because both prove fail-closed disposition handling.

## Ordering

1. Import the typed disposition.
2. Add durable admission/input types.
3. Add reconciliation output.
4. Add `reconcile` and its private reducer-delegating helper.
5. Add focused tests.
6. Format and run focused verification.
7. Run workspace and WASM checks.
8. Commit the single source path through Lisa's isolated transaction.

## Public compatibility

All changes are additive. Existing state, event, effect, rejection, and reducer
signatures remain intact. Downstream code need not adopt reconciliation until
the follow-on plugin adapter story.

Because `ReviewDisposition` already derives clone and equality, the new durable
input aggregate requires no changes to `disposition.rs`.

## Ownership and commit boundary

The meaningful source unit is exactly:

```text
crates/lisa-core/src/completion.rs
```

The commit command must include only that repository-relative path. Active
ticket frontmatter, provenance, unrelated plugin docs, and attempt work
artifacts are excluded from the source commit.

## Non-changes

- No plugin call site is folded into the reducer.
- No real poll/reload behavior is introduced.
- No command executor is added.
- No durable journal or deadline is added.
- No manual `[d]one` behavior changes.
- No dashboard output changes.
- No provider-specific conditional is added.

