# Progress: total completion reducer

## Status

Implementation is complete, verified, and committed through Lisa's isolated
ticket transaction.

Source commit: `eae63f07ddb4ada49b8ba9cc44abf323b4343944`

## Completed work

- Read the repository guidance, ticket, RDSPI workflow, parent story, epic, and
  predecessor completion-domain design.
- Mapped the existing completion states, events, identities, effect command,
  transition, and rejection types.
- Confirmed the ticket owns only `crates/lisa-core/src/completion.rs`.
- Preserved pre-existing Lisa-owned worktree changes and unrelated untracked
  files.
- Added public pure `reduce(CompletionState, CompletionEvent)`.
- Implemented an explicit outer arm for every completion state.
- Implemented explicit inner arms for every completion event in every state.
- Added no state- or event-hiding wildcard arm.
- Implemented initial eligible request emission.
- Implemented requested-to-in-flight launch acknowledgement.
- Implemented launch failure to retryable rejected state.
- Implemented correlated command success to confirmed state.
- Implemented correlated command failure to rejected state.
- Preserved command-failure source and retryability.
- Implemented retryable rejected-state request behavior.
- Implemented action-required request refusal using the retained typed reason.
- Implemented duplicate request refusal without emitting another effect.
- Added typed unexpected-event rejection for invalid lifecycle ordering.
- Added typed correlation-mismatch rejection for stale/misdirected results.
- Kept all effect commands as inert returned data.
- Added no I/O, process launch, global mutation, scheduler access, or adapter
  dependency.
- Extended rejection exhaustiveness/display coverage.
- Added exact-value unit tests for every legal lifecycle edge.
- Added exact-value tests for all duplicate-request illegal edges.
- Added exact-value tests for both mismatched-result illegal edges.
- Added an illegal callback matrix covering every remaining callback/state
  combination.
- Added retryability branch coverage.
- Formatted the workspace.
- Ran focused, workspace, and WASM-oriented verification.
- Committed the single ticket-owned source path with `lisa commit-ticket`.
- Confirmed no ticket-owned source file remains modified, staged, or untracked.

## Transition coverage

### Accepted transitions

| From | Event | To | Effect |
| --- | --- | --- | --- |
| Eligible | Request | Requested | one LaunchCompletion |
| Requested | CommandLaunched | CommandInFlight | none |
| Requested | CommandLaunchFailed | Rejected/Retryable | none |
| CommandInFlight | matching CommandSucceeded | Confirmed | none |
| CommandInFlight | matching CommandFailed | Rejected/event policy | none |
| Rejected/Retryable | Request | Requested | one LaunchCompletion |

### Typed refusals

| Condition | Rejection |
| --- | --- |
| Request while requested | AlreadyPending |
| Request while in flight | AlreadyPending |
| Request after confirmed | AlreadyPending |
| Request after action-required rejection | retained named rejection |
| Success with wrong correlation | CorrelationMismatch |
| Failure with wrong correlation | CorrelationMismatch |
| Inapplicable callback/state cell | UnexpectedEvent |

## Verification results

### Formatting

Command:

`cargo fmt --all -- --check`

Result: passed after applying standard cargo formatting. The final source is
formatter-clean.

### Focused tests

Command:

`cargo test -p lisa-core completion`

Result: passed, 16 completion tests, zero failures. This includes the six
accepted edge tests and all typed-refusal categories.

### Workspace tests

Command:

`cargo test --workspace`

Result: passed. Final run completed all workspace targets with zero failures;
the plugin suite reported 341 passing tests and lisa-core completion tests were
green.

### Repository quick check

Command:

`just check`

Result: passed. `cargo check -p lisa-plugin --target wasm32-wasip1` succeeded,
then the full workspace test suite succeeded.

### Diff hygiene

Command:

`git diff --check`

Result: passed before commit.

After commit, `git diff -- crates/lisa-core/src/completion.rs` is empty.

## Commit transaction

Command:

```text
lisa commit-ticket --ticket-id T-041-01-02 \
  --message "feat(core): add total completion reducer" \
  --include crates/lisa-core/src/completion.rs
```

Result:

`eae63f07ddb4ada49b8ba9cc44abf323b4343944`

Only the exact source path was included. No ordinary `git add`, ordinary
`git commit`, or broad staging command was used.

## Deviations from plan

The implementation matched the planned public API and transition design.

One test-strengthening adjustment occurred after the first green workspace run:
the illegal callback test was expanded from representative cells to every
remaining invalid callback/state cell. Focused and workspace tests were rerun
after that adjustment, and the source was committed only after the final green
run.

The repository automatically admitted phase artifacts into the shared active
work path while this attempt continued. The attempt wrote artifacts only to its
private work directory and did not directly edit the shared copies.

## Remaining work

No work remains for T-041-01-02.

T-041-01-03 owns level-triggered eligibility and reconciliation. Plugin effect
execution remains explicitly outside this story slice.
