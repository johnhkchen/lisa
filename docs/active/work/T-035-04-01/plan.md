# T-035-04-01 Plan — implementation and verification

## Preconditions

1. Preserve all unrelated dirty worktree paths.
2. Do not edit ticket phase/status frontmatter.
3. Write phase artifacts only under the exact attempt work directory.
4. Use `apply_patch` for source edits.
5. Use only `lisa commit-ticket` with exact ticket-owned paths for source commits.
6. Check ordinary-index and worktree state before and after every ticket commit.

## Step 1 — generalize prompt acknowledgement

Rename `crates/lisa-plugin/src/codex_ack.rs` to `assignment_ack.rs` through an
`apply_patch` add/delete operation.

Rename crate-private classifier types/functions and update documentation to state that
both native providers can emit the minimal `UserPromptSubmit` envelope.

Keep the marker prefix and JSON schema byte-compatible.

Retain all malformed, stale-ticket, stale-generation, wrong-event, and whole-line tests.
Add or adapt one test proving unknown provider-specific payload fields do not matter.

Verification:

```text
cargo test -p lisa-plugin assignment_ack
```

Expected: classifier tests compile under the new module and all stale evidence fails
closed.

## Step 2 — split provider launch from assignment construction

Add `AgentAdapter::assignment_text` and a bounded tagged assignment-reference method.

Implement provider-specific full instruction text through `ticket_prompt` and each
provider's context filename.

Change Claude fresh launch construction to omit ticket/artifact inputs and positional
prompt.

Change Codex `interactive_line` to omit assignment prompt while retaining flags,
lifecycle identity, model selection, Lisa binary, and `.error` fallback.

Build reused prompts from `assignment_text`, retaining Codex tagging only where the
existing scheduler supplies a generation.

Update adapter tests to prove:

- bare launch has no RDSPI/ticket-read instruction;
- long assignment content cannot affect launch command length;
- both providers retain exact lifecycle identity and model flags;
- provider-specific assignment text names the correct context file;
- bounded reference contains the assignment path and exact marker;
- stale marker classifier behavior remains compatible.

Verification:

```text
cargo test -p lisa-plugin adapter
```

## Step 3 — install Claude prompt evidence

Update the shared acknowledgement hook comments to provider-neutral language.

Add Claude `UserPromptSubmit` binding to generated settings and idempotent merge.

Update template tests for creation, merge, coexistence, and repeat-merge behavior.

Do not modify `init.rs` unless a failing test proves installation does not already include
`on-ack.sh`.

Verification:

```text
cargo test -p lisa-cli templates
```

Expected: Claude and Codex each produce exactly one guarded prompt hook.

## Step 4 — atomically publish the assignment file

Add deterministic `assignment.md` path construction and atomic write/rename helper beside
fresh launch preparation.

In scheduling, after minting the exact attempt lease and before lifecycle input for a
fresh provider, construct provider-specific assignment text and publish it.

On publication failure:

- revoke the just-created current lease;
- restore the pane title where applicable;
- log one dispatch error;
- submit no launch, `/clear`, or `/exit` input;
- leave no partial temporary file.

Update launch preparation tests to assert assignment bytes separately from launch bytes.
Use long and quote-heavy content to prove exact persistence and bounded bare script size.

Verification:

```text
cargo test -p lisa-plugin prepare_assignment
cargo test -p lisa-plugin prepare_fresh_launch
```

## Step 5 — add ReadyForAssignment and Delivering

Extend `SeatAssignmentState` with ready, delivering, and terminal delivery failure.

Extract a common Enter-aware acknowledgement deadline calculator.

Change exact process-start admission to `Starting -> ReadyForAssignment` only.

Add `deliver_ready_assignments` before process-start signal consumption in the poll loop.
It must validate the exact current lease and assignment file, construct the bounded
provider message, submit it, and move to `Delivering` with retry count zero.

Generalize active generation lookup, acknowledgement admission, `.ack` signal consumption,
and activity messages to both providers.

Preserve `AssignedPendingAck` and `Recovering` behavior for existing reused Codex paths.

Verification with focused native tests:

```text
cargo test -p lisa-plugin fresh_dispatch
cargo test -p lisa-plugin process_start
cargo test -p lisa-plugin assignment_ack
```

Expected fresh sequence for each provider:

```text
Starting -> ReadyForAssignment -> Delivering -> Owned
```

Exact `.started` must not imply ownership. Stale `.ack` must leave Delivering unchanged.

## Step 6 — bound missing chat acknowledgement

Add `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1`.

Add a same-attempt redelivery helper that revalidates the current lease and assignment
file, resubmits only the bounded reference, increments retries, and replaces the deadline.

Add `fail_assignment_delivery` mirroring the retained semantics of StartupFailed:

- terminal state written first;
- thread failed;
- slot, ticket, attempt, and current lease retained;
- one error alert;
- explicit reset guidance;
- no provider relaunch, lease mint, release, or timer rearm.

Extend deterministic timeout evaluation:

- first Delivering expiry retries once;
- second expiry becomes DeliveryFailed;
- later evaluations do nothing.

Add tests proving no Owned edge, no fresh launch count increase, exactly one redelivery,
terminal stale-ack rejection, and no infinite loop.

Verification:

```text
cargo test -p lisa-plugin missing_fresh_assignment
cargo test -p lisa-plugin delivery_failed
```

## Step 7 — expose dashboard states

Add UI status variants, labels, colors, and exhaustive conversion.

Update the full fresh-state native regression to inspect rendered rows after Starting,
ReadyForAssignment, Delivering, and Owned.

Add direct status coverage for red DeliveryFailed.

Verification:

```text
cargo test -p lisa-plugin dashboard
cargo test -p lisa-plugin seat_assignment
```

## Step 8 — preserve predecessor behavior

Run focused regressions by stable test-name filters:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin recycled_codex
cargo test -p lisa-plugin split_brain
cargo test -p lisa-plugin fresh_start
cargo test -p lisa-plugin pane_title
cargo test -p lisa-cli start_hook
```

Adjust only expectations made obsolete by the explicit new fresh state sequence or bare
launch shape. Do not weaken stale-attempt, single-recovery, lease-retention, or pane-name
assertions.

## Step 9 — format and full verification

Run formatting without staging:

```text
cargo fmt --all -- --check
```

If it reports changes, run `cargo fmt --all`, then inspect every modified path and ensure
only ticket-owned source was mechanically touched.

Run:

```text
cargo test --workspace
just check
```

If `just check` duplicates the suite, retain both results in progress because it also
checks the WASM target. Document any environmental failure exactly rather than claiming
coverage.

## Step 10 — inspect diff and commit meaningful source units

Inspect:

```text
git diff -- <exact ticket-owned paths>
git diff --check -- <exact ticket-owned paths>
git status --short
git diff --cached --name-only
```

Prefer one coherent source commit if the classifier rename, adapter trait, scheduler
module declaration, and state machine cannot compile independently.

Commit only with:

```text
lisa commit-ticket \
  --ticket-id T-035-04-01 \
  --message "feat(plugin): split provider start from chat assignment" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/assignment_ack.rs \
  --include crates/lisa-plugin/src/codex_ack.rs \
  --include crates/lisa-plugin/src/ui.rs \
  --include crates/lisa-cli/src/templates.rs
```

Omit any path that did not actually change. Include both old and new classifier paths if
the transaction requires explicit deletion plus addition.

After commit, verify every ticket-owned source path is clean and no ticket-owned path is
staged or untracked. Unrelated pre-existing dirty paths must remain untouched.

## Step 11 — progress artifact

Write `progress.md` in the private attempt directory throughout implementation. Record:

- completed plan steps;
- exact source paths changed;
- deviations and rationale before taking them;
- focused and full test commands/results;
- exact Lisa commit command and resulting commit ID;
- final ticket-owned cleanliness check.

Do not include progress.md in the source transaction.

## Step 12 — review artifact

Review the committed diff and write `review.md` in the private attempt directory.

Cover:

- outcome against every acceptance criterion;
- files created, modified, renamed, or deleted;
- exact state sequence and evidence boundaries;
- atomic assignment and bounded transport behavior;
- stale evidence and recovery behavior;
- test coverage and commands;
- source commit ID and included paths;
- open concerns, limitations, and T-035-04-02 boundary;
- confirmation that ticket frontmatter was not edited;
- confirmation that ticket-owned source is clean.

After `review.md`, remain on T-035-04-01 and stop. Do not publish artifacts, mark Done,
release the seat, or start another ticket.
