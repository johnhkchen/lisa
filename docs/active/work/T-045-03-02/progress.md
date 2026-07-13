# Progress — T-045-03-02 evidence tiers: hook and artifact

## Status

Implementation is complete.

Focused tests, the complete plugin suite, and the workspace suite pass.

The ticket-owned source unit is ready for the Lisa isolated commit transaction.

Review remains after the commit and final repository checks.

## Baseline completed

Before source edits, the existing primary-evidence regression passed:

```text
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
```

Result:

- 1 passed;
- 0 failed.

The existing stale-attempt artifact regression passed:

```text
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact
```

Result:

- 1 passed;
- 0 failed.

The existing exact acknowledgment regression passed:

```text
cargo test -p lisa-plugin test_recycled_codex_ownership_requires_matching_ack_exactly_once
```

Result:

- 1 passed;
- 0 failed.

The baseline confirmed that claim-only ownership and predecessor fencing were
green before this ticket changed artifact behavior.

## Production implementation completed

Modified:

`crates/lisa-plugin/src/lib.rs`

Added `State::admit_artifact_ownership`.

The method accepts a ticket ID and exact attempt lease after filesystem
artifact admission has succeeded.

It requires:

- candidate ticket equality;
- exact current lease authority;
- a physical slot reserved for the ticket;
- the slot's exact candidate lease;
- an active delivered assignment generation;
- generation equality with the candidate attempt.

Only after all guards pass does it insert `SeatAssignmentState::Owned`.

It returns the pane ID only when it performs the pending-to-owned edge.

Startup, ready, already-owned, terminal, stale, unleased, and unmatched states
therefore remain no-ops.

Added `State::record_artifact_ownership`.

This wrapper receives an artifact that has already passed `admit_artifact`.

On the one successful ownership edge it:

- bumps pane and thread activity;
- emits an information event;
- names pane, ticket, attempt, and artifact.

Rejected or redundant evidence produces no success activity.

## Artifact integration completed

Updated the Implement `progress.md` publication branch.

The branch now distinguishes:

- admitted current bytes;
- absent progress;
- rejected publication.

An admitted leased `progress.md` may provide bounded fallback ownership.

It still does not advance Implement and does not set the phase loop's
`advanced_any` flag.

Updated the phase-edge artifact branch.

The `Ok(true)` path now records fallback ownership before phase advancement.

Missing and rejected artifacts retain the prior behavior.

Ticket-file update, phase activity, completion dispatch, and catch-up looping
remain unchanged.

## Evidence order documented

Updated `poll_tick` comments without reordering consumers.

The order remains:

1. exact assignment claim;
2. matching provider hook;
3. admitted current-attempt workflow artifact;
4. later timeout policy.

The claim gets the first opportunity to own when several evidence forms coexist.

The hook remains the supplemental fast path while a claim is pending.

The artifact is the last bounded positive fallback before timeout evaluation.

## Hook acceleration test completed

Added:

`matching_hook_accelerates_pending_claim_ownership`

The test uses the scheduled Codex fixture and real fresh-delivery helpers.

It proves:

- the seat begins in `Delivering`;
- no claim file exists;
- a matching `UserPromptSubmit` record is consumed once;
- the hook alone changes the seat to `Owned`;
- the acknowledgment activity event is emitted;
- the claim remains absent.

Focused command:

```text
cargo test -p lisa-plugin matching_hook_accelerates_pending_claim_ownership
```

Result:

- 1 passed;
- 0 failed.

## Artifact fallback and stale evidence test completed

Added:

`current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored`

The test schedules a predecessor, releases its authority, and schedules a
monotonic replacement for the same ticket.

It drives the replacement to a real `Delivering` state.

It then supplies:

- a predecessor-generation hook routed to the replacement pane;
- predecessor `research.md` bytes in the old private attempt directory.

It proves stale evidence:

- the hook is consumed once;
- the replacement remains pending;
- replacement pane and thread activity clocks do not move;
- the predecessor artifact remains private;
- no canonical artifact appears;
- direct predecessor admission fails the current-lease boundary;
- the workflow phase does not advance.

The test then writes distinct `research.md` bytes under the replacement private
directory.

It proves current fallback:

- the replacement changes to `Owned`;
- Research advances to Design;
- canonical output contains only replacement bytes;
- predecessor bytes remain unchanged;
- activity clocks move;
- the fallback event names pane 10, attempt 2, and `research.md`.

Focused command:

```text
cargo test -p lisa-plugin current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored
```

Result:

- 1 passed;
- 0 failed.

## Focused regression results

Ran:

```text
cargo test -p lisa-plugin claim
```

Result:

- 8 passed;
- 0 failed.

Ran:

```text
cargo test -p lisa-plugin ack
```

Result:

- 26 passed;
- 0 failed.

Ran:

```text
cargo test -p lisa-plugin artifact_advances
```

Result:

- 9 passed;
- 0 failed.

Ran:

```text
cargo test -p lisa-plugin stale_attempt
```

Result:

- 3 passed;
- 0 failed.

These filters cover the primary claim, supplemental hook, fallback artifact,
phase behavior, and predecessor fencing together.

## Formatting and diff checks

The initial `cargo fmt --all -- --check` reported one mechanical wrapping
difference in the new progress admission match.

Ran `cargo fmt --all`.

Subsequent formatting check passed:

```text
cargo fmt --all -- --check
```

Ran:

```text
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Result: clean.

Diff scope before commit:

- one source file;
- 219 insertions;
- 16 deletions;
- production methods, integration, comments, and two tests only.

## Complete plugin suite

Ran:

```text
cargo test -p lisa-plugin
```

Result:

- 393 passed;
- 0 failed;
- 0 ignored.

This includes claim, acknowledgment, artifact, timeout, split-brain,
completion, UI, signal-ingestion, and provider compatibility coverage.

## Complete workspace suite

Ran:

```text
cargo test --workspace
```

Result:

- 896 passed across workspace unit and integration suites;
- 0 failed;
- 1 ignored real-Zellij environment test.

The ignored test declares that it requires real Zellij, zsh, script, jq, and
the `wasm32-wasip1` target.

CLI claim tests passed all three current, wrong-nonce, and prior-attempt cases.

Core claim serialization and named rejection tests passed.

## Deviations from plan

No semantic deviation was required.

The plan allowed a small post-admission wrapper, which was used to keep progress
and phase-edge behavior identical.

The formatter was run after its check found the expected mechanical wrapping
difference.

No new evidence enum, signal schema, UI type, or module was introduced.

## Remaining implementation actions

1. Run repository `just check`.
2. Reconfirm the exact source diff and ordinary-index status.
3. Commit only `crates/lisa-plugin/src/lib.rs` using `lisa commit-ticket`.
4. Verify the source path is clean after the isolated transaction.
5. Write Review artifacts and final disposition.

## Ticket-owned source state

The only ticket-owned source unit is:

`crates/lisa-plugin/src/lib.rs`

The working tree also contains unrelated Lisa-managed and planning paths that
were present or changed outside this source unit.

They are not included in this ticket's source transaction.

No ordinary `git add` or `git commit` command has been used.

## Repository check gate

Ran:

```text
just check
```

Result: passed.

The command successfully completed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

The WASM target check completed without warnings or errors.

The repeated workspace suite retained the same green results, including all 393
plugin tests and the unchanged intentionally ignored real-Zellij environment
test.

Remaining work is the isolated source commit and Review artifacts only.

## Isolated source commit completed

Before committing, inspected the ordinary index and exact source diff.

The ordinary index contained no staged path owned by this ticket.

Ran exactly:

```text
lisa commit-ticket \
  --ticket-id T-045-03-02 \
  --message "feat(plugin): tier hook and artifact ownership evidence" \
  --include crates/lisa-plugin/src/lib.rs
```

Result: success.

Commit:

```text
de308795c2e2af37d240e392cf8192dedaf08c2b
```

Commit subject:

```text
feat(plugin): tier hook and artifact ownership evidence
```

Commit contents:

- exactly `crates/lisa-plugin/src/lib.rs`;
- 219 insertions;
- 16 deletions.

Post-commit checks confirmed the ticket-owned source path has no working-tree or
ordinary-index diff.

The unrelated Lisa-managed provenance, journal, planning, ticket, and work
paths remain outside the source transaction.

Ran the artifact/stale-evidence regression again after the commit.

Result:

- 1 passed;
- 0 failed.

Implementation and source durability are complete.
