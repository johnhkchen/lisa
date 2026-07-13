# Review: completion effect adapter seam

## Disposition

Pass.

T-042-01-01 introduces the required typed plugin adapter seam for artifact and
stopped Review completion. Both inputs now produce E-041 typed request events,
the pure reducer decides whether a launch effect exists, and one exhaustive
executor owns the completion host-command launch.

The focused exactly-one-effect proof, plugin suite, workspace suite, WASM
Clippy, and formatting are green. No blocking issue remains.

## Source change

Modified:

- `crates/lisa-plugin/src/lib.rs`

No file was created or deleted in production source.
No manifest or lockfile changed.
No E-041 `lisa-core` source changed.

The implementation commit is:

`bedf313426fe0350a4a240bb0ef6385ee83ed079`

It was created with `lisa commit-ticket` and includes only the exact source
path above.

## Architecture delivered

`CompletionInput` provides the ticket-local scheduler evidence admitted in this
slice:

- Artifact with ticket and current attempt lease;
- Stopped with ticket, pane, and current attempt lease.

`State::dispatch_completion` is the shared adapter dispatch function. It:

1. admits current-attempt `review-disposition.json`;
2. requires a passing disposition;
3. maps the existing pending map to core Eligible or Requested state;
4. constructs typed `AttemptId` and `CompletionId` values;
5. constructs `CompletionEvent::Request`;
6. calls E-041's pure reducer;
7. logs typed rejection text;
8. executes only the optional effect returned by the transition.

Artifact polling and stopped Review auto-completion both call this function.
Neither source constructs or launches a completion command directly.

`State::execute_completion_effect` exhaustively matches
`EffectCommand::LaunchCompletion`. It validates effect identity against the
scheduler ticket and attempt authority, then applies the existing pending,
lease, dependency, ticket lookup, path, command construction, and activity
behavior.

The `complete-ticket` Zellij launch is present in this executor only. The other
host `run_command` call in `lib.rs` remains the unrelated notification hook.

## Compatibility

Disposition admission messages and canonical publication behavior are
preserved through the factored `admit_passing_review` helper.
The completion result path is unchanged: commit output and current authority
are checked, the DAG is rebuilt, and durable Done is verified before thread and
seat consequences occur.

Idle, externally observed Done, and manual operator origins remain behind a
temporary legacy wrapper. That wrapper delegates to the same effect executor
and contains no host launch. Migrating those sources and deleting/quarantining
the boolean wrapper is explicitly assigned to successor T-042-01-02.

## Acceptance mapping

### Single adapter dispatch

Satisfied by `State::dispatch_completion`, which is the only dispatcher for the
two owned input variants and always invokes `reduce_completion` with a typed
Request event.

### Artifact and stopped routing

Satisfied by `check_artifact_advances` constructing Artifact input and
`auto_complete_review` constructing Stopped input from the assigned slot lease.
A missing lease fails visibly before any request can be fabricated.

### Exactly one effect execution site

Satisfied by `State::execute_completion_effect`. Search inspection found one
completion-specific `run_command_with_env_variables_and_cwd` call at this
boundary. No artifact/stopped caller launches directly.

### Unit proof

`test_check_artifact_advances_review_to_done` now asserts a passing leased
Review produces exactly one recorded `LaunchCompletion` with the correct
attempt generation and ticket completion identity. It then drives the stopped
source and asserts the effect count remains one because the reducer receives
Requested state and rejects the duplicate request.

The recording vector is `cfg(test)` only and therefore absent from production
WASM.

## Verification results

Focused adapter proof:

`cargo test -p lisa-plugin --lib test_check_artifact_advances_review_to_done --no-fail-fast`

Passed: 1; failed: 0.

Focused disposition compatibility:

`cargo test -p lisa-plugin --lib review_disposition_gates_artifact_completion_and_dependents --no-fail-fast`

Passed: 1; failed: 0.

Plugin library suite:

`cargo test -p lisa-plugin --lib --no-fail-fast`

Passed: 341; failed: 0.

Workspace suite:

`cargo test --workspace --no-fail-fast`

All executed tests passed, including 279 CLI, 191 core, 341 plugin tests, and
the core generated and recorded completion regressions. The existing real
Zellij environment test remained ignored by its declared contract.

WASM lint:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

Passed.

Formatting:

`cargo fmt --all -- --check`

Passed.

## Repository review

The ticket-owned source path is clean after the isolated transaction.
The ordinary Git index is empty.
No ordinary `git add` or `git commit` was used.

Lisa-managed `.lisa/provenance.jsonl`, ticket phase frontmatter, and admitted
work publication remain outside the source commit. The pre-existing untracked
`crates/lisa-plugin/docs/` path was preserved.

## Open concerns and limitations

No blocking concern exists for this ticket.

The bridge currently represents pending aggregate state through the existing
`pending_completions` map and maps it to core Requested during dispatch. Full
CommandInFlight, Rejected, Confirmed, and correlation persistence/rendering are
intentionally later story work.

The temporary boolean wrappers remain for sources not included in this first
slice. They cannot launch outside the centralized executor. Their removal is a
named acceptance criterion of T-042-01-02.

Completion IDs currently use ticket identity because the plugin has one pending
aggregate per ticket. Durable generation/idempotency identity belongs to
S-042-02.

Nested Git-root command normalization is not changed here; T-042-01-05 and
T-042-01-06 own that already identified defect and regression.

## Critical issues requiring human attention

None.

## Human review focus

Confirm the staged-migration boundary is appropriate: Artifact and Stopped are
reducer-backed now, all completion launch I/O is centralized, and the remaining
legacy sources are preserved only until their explicitly dependent successor.

Review is complete. This attempt remains on T-042-01-01 for Lisa to admit the
Review artifacts, prepare the completion commit, publish Done, and release the
seat.
