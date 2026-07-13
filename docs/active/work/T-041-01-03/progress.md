# Progress: level-triggered completion eligibility

## Phase completion

Research, Design, Structure, and Plan are complete in the attempt-private work
directory. Implementation followed the planned single-file additive boundary.

## Completed: durable eligibility vocabulary

Modified `crates/lisa-core/src/completion.rs` to consume the existing typed
`ReviewDisposition` contract.

Added `CurrentLeaseArtifactAdmission`, a positive fact carrying the exact
`AttemptId` and `CompletionId` whose Review artifact passed the adapter's
current-lease admission boundary.

Added `DurableCompletionInputs`, containing an optional admitted artifact and
the fail-closed typed disposition. Absence of admission and every non-Pass
disposition prevent request effects.

No filesystem, scheduler, provider, process, Zellij, or WASM type entered the
new API.

## Completed: reconciliation outcome

Added public `Reconciliation` with three named outcomes:

- `Effect(EffectCommand)` for one request obligation;
- `None` when no new action is required;
- `CommandInFlightActionRequired { correlation }` for unresolved work that
  must not be blindly retried.

The in-flight outcome structurally requires a `CorrelationId` and carries no
effect. One reconciliation call always terminates with one value and contains
no retry loop.

## Completed: level-triggered reconcile

Added public, borrowing, pure:

```text
reconcile(&DurableCompletionInputs, &CompletionState) -> Reconciliation
```

Eligible and retryable-rejected aggregate states emit the existing completion
launch effect only when both durable eligibility facts hold. Requested and
Confirmed suppress duplicate effects. Action-required rejected state remains
non-retryable.

CommandInFlight returns its exact correlation as actionable even if admission
or disposition inputs later become unavailable. The already-launched
transaction remains uncertain independently of the original eligibility
facts, so losing those facts cannot silently hide it or cause a duplicate
launch.

Extracted private `request_effect` construction and reused it in both the
existing reducer request transition and reconciliation. This prevents effect
payload drift while avoiding unreachable reducer error handling.

## Completed: unit coverage

Added six focused tests covering:

1. admitted current-lease artifact + exact Pass + Eligible returns the exact
   request effect with both identities;
2. missing artifact admission returns None;
3. typed Block and Invalid dispositions both return None;
4. Requested and Confirmed states both return None;
5. retryable Rejected returns the exact fresh effect while ActionRequired
   returns None;
6. unresolved CommandInFlight with unavailable durable inputs returns the exact
   correlation-tagged actionable outcome and no effect.

The production implementation and fixtures contain no Claude/Codex branch or
provider value.

## Verification completed

Focused checks after the final source change:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
cargo clippy -p lisa-core --all-targets -- -D warnings
git diff --check -- crates/lisa-core/src/completion.rs
```

Results:

- formatting check passed;
- lisa-core: 191 passed, 0 failed;
- lisa-core doctests: 0 failed;
- clippy passed with warnings denied;
- diff whitespace check passed.

Full regression checks were run after the initial implementation:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

Results:

- workspace test command passed;
- observed CLI unit suite: 279 passed;
- observed core unit suite: 191 passed;
- observed plugin unit suite: 341 passed;
- non-ignored integration and doc-test suites passed;
- the existing explicitly gated real-Zellij test remained ignored;
- plugin WASM check passed.

The final workspace and WASM commands also passed after the shared-helper/test
tightening.

## Plan clarification

The initial design text said durable eligibility would be checked before every
state. During implementation, the in-flight policy was tightened: an already
launched unresolved command remains correlation-tagged actionable even when
the original eligibility inputs are unavailable. This better satisfies the
acceptance criterion's bounded in-flight requirement and never emits a request
effect. Research, Design, Structure, and Plan were updated to record the final
ordering before Review.

The initial design also considered invoking `reduce` from `reconcile` to reuse
request construction. The implementation instead shares a private
`request_effect` constructor between both functions. This preserves one
payload definition without adding a panic or impossible-error fallback to the
public pure reconciliation function. The design artifact records the final
choice.

## Ownership status

Ticket-owned source path:

```text
crates/lisa-core/src/completion.rs
```

The active ticket and `.lisa/provenance.jsonl` remain orchestration-owned
changes. Untracked plugin docs and Lisa-published work artifacts remain outside
this ticket's source ownership. No ordinary-index staging or commit command was
used.

## Source commit completed

Committed the exact source unit through Lisa's isolated transaction:

```text
35847e3f9df71113cf9c8af8af28a746cff8e1ab
feat(core): add completion reconciliation
```

The commit contains only `crates/lisa-core/src/completion.rs`. Post-commit
inspection reports that path clean and the ordinary index empty. Unrelated
orchestration and untracked paths remain untouched.

## Remaining

- Write Review and the pass/block disposition.
