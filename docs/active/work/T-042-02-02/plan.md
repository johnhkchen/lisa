# Plan: Durable completion journal reconstruction

## 1. Preserve the concurrent-work baseline

Inspect status and the exact diff in `crates/lisa-plugin/src/lib.rs`.

Record that T-042-01-03 currently owns the uncommitted reconciliation changes.

Do not revert, stage, commit, or otherwise consume those changes.

Implement the new module independently while that ticket finishes.

Before integrating or committing `lib.rs`, require that the concurrent ticket's
source change is admitted and the file is clean relative to the new HEAD.

Verification: unrelated provenance, ticket, work-artifact, and plugin docs paths
remain byte-preserved and excluded.

## 2. Create the journal schema module

Add `crates/lisa-plugin/src/completion_journal.rs`.

Define schema version 1.

Define the typed runtime transition enum.

Define private serde record shapes with tagged state names.

Define the reconstructed aggregate with key, state, prior mask facts, and
optional confirmed commit.

Verification: record serialization is one compact line with explicit version
and component identity fields.

## 3. Implement record conversion

Convert runtime transitions to serializable records without parsing Display.

Convert parsed records back to typed CompletionGenerationId, CorrelationId,
Phase, TicketStatus, and Retryability values.

Reject unknown schema versions explicitly.

Preserve arbitrary UTF-8 completion/attempt/reason values through JSON escaping.

Verification: serialize/deserialize conversion preserves every typed component.

## 4. Implement deterministic aggregate folding

Add one fold helper over a ticket-indexed aggregate map.

Use the core reducer for Request, CommandLaunched, CommandLaunchFailed,
CommandFailed, and CommandSucceeded transitions.

Check generation-key equality before non-Request events.

Check correlation equality through reducer events.

Carry prior phase/status from Requested through later states.

Carry confirmed commit only in Confirmed.

Verification: invalid order, mismatched key, mismatched correlation, and an
event after Confirmed all fail with contextual errors.

## 5. Implement strict journal loading

Return an empty aggregate map for a missing path.

Read existing bytes without modification.

Require newline termination for every non-empty history.

Reject empty interior records and malformed JSON.

Fold lines in order and attach 1-based line numbers to errors.

Verification: a valid three-transition history reconstructs deterministically;
torn and malformed histories fail closed.

## 6. Implement atomic logical append

Load and validate the complete existing journal.

Validate the proposed transition against that reconstructed state.

Append one compact JSON object plus newline in memory.

Create the parent directory when required.

Publish the complete byte vector through RustPublication with a nonce-named
sibling temporary.

Return the exact post-transition aggregate only after rename succeeds.

Verification: destination bytes are the old complete history plus exactly one
line, and no temporary file remains.

## 7. Add focused journal-module tests

Test Requested reconstruction.

Append CommandInFlight and simulate restart through `load`.

Append Confirmed and simulate another restart.

Assert exact typed aggregate equality at every boundary.

Test retryable Rejected followed by another Requested.

Test malformed final line, unknown schema, invalid order, key mismatch, and
correlation mismatch.

Use a hostile but valid parent-directory name to exercise path handling.

Verification: all completion_journal module tests pass with no filesystem
residue.

## 8. Integrate the module and State fields

After the concurrent `lib.rs` change is committed, add the module declaration
and imports.

Add journal path, health, and aggregate map to State.

Enrich PendingCompletion with generation key and correlation.

Update only direct PendingCompletion test literals required by compilation.

Verification: State::default remains usable and empty-path native fixtures do
not perform disk I/O.

## 9. Add restoration and append helpers

Implement `restore_completion_journal`.

On successful missing/valid load, install the map and mark health true.

On invalid history, clear/retain fail-closed state as designed, mark health
false, and log one operator-visible error.

Implement `journal_completion_transition`.

Allow no-op only when the path is empty for pre-load tests.

Reject non-empty unhealthy operation.

Insert the returned aggregate only after successful atomic publication.

Verification: a failed append cannot mutate the State aggregate map.

## 10. Make aggregate state durable-first

Update `completion_state` to return reconstructed state first.

Retain pending-map Requested fallback for old tests and disabled-journal state.

Retain durable DAG Confirmed fallback when no journal aggregate exists.

Verification: State can distinguish reconstructed Requested,
correlation-bearing CommandInFlight, retryable Rejected, and Confirmed.

## 11. Mask DAG construction from reconstructed transactions

Factor the existing PendingCompletion phase/status mask into a helper.

Fall back to Requested or CommandInFlight aggregate prior values when no live
pending entry exists.

Use the helper in `rebuild_dag`.

Restore the journal before initial scan-to-DAG construction in load.

Apply the same mask to startup scan results.

Verification: a fresh State with CommandInFlight journal plus Done ticket bytes
constructs a DAG at the prior phase/status, while Confirmed exposes Done.

## 12. Journal request and launch before host execution

Keep existing effect identity, authority, dependency, and path checks.

Build the completion command before accepting durable transitions.

Construct the generation-derived correlation once.

Append Requested with prior phase/status.

Append CommandInFlight with exact correlation.

If either append fails, log LaunchFailed and do not call the host.

Insert enriched PendingCompletion only after both appends.

Retain native inert-effect recording at the accepted boundary.

Verification: the production adapter produces exactly two ordered complete
records before command invocation can be observed.

## 13. Journal command rejection before retry

Read exact key/correlation from PendingCompletion.

On nonzero exit or invalid commit output, append retryable Rejected.

Only after success remove live pending state and rebuild.

On append failure, retain pending and masking state and surface the durability
failure.

Apply the same fail-closed principle to stale-authority result handling.

Verification: a normal failed command remains retryable; restarting after the
failure reconstructs Rejected rather than CommandInFlight.

## 14. Journal confirmation before downstream success

Preserve current valid commit-output and durable Done verification.

Append Confirmed with exact correlation and commit ID.

If publication fails, restore PendingCompletion and the prior DAG mask.

Do not emit phase completion, authoritative provenance, thread completion,
release, or scheduling before Confirmed publication succeeds.

Verification: injected confirmation-publication failure leaves the scheduler
blocked and emits no Done provenance.

## 15. Add the restart-reconstruction acceptance test

Create a temporary configured State and current AttemptLease.

Use real ticket/work paths inside one temporary Git-root-shaped directory.

Dispatch a completion through the production adapter.

Assert Requested and CommandInFlight journal records exist in order.

Construct a fresh State, use the production restoration helper, and assert its
aggregate exactly equals the original state.

Assert initial DAG masking retains prior non-Done truth if ticket bytes are
already Done while the journal is in-flight.

Deliver valid durable Done plus a commit-ID result to the original state.

Construct another fresh State and assert Confirmed plus exact commit ID.

Assert only three accepted-state records and no temporary residue.

Verification: removing any transition append or restoration step makes the
test fail.

## 16. Verify provenance compatibility

Point the acceptance fixture at a temporary provenance path.

After Confirmed, parse the execution record through the existing
ProvenanceLedgerRecord enum.

Assert no completion-specific required field or new record variant appears.

Run the existing schema-v2/schema-v3 mixed-ledger tests unchanged.

Verification: prior JSON examples still deserialize and new completion state is
stored only in its dedicated journal.

## 17. Run focused verification

Run journal module tests.

Run the restart acceptance test by exact name.

Run existing completion dispatch, command builder, result success/failure,
provenance, DAG masking, and level-triggered reconciliation tests.

Run the structural one-gateway test.

Verification: exact transition counts, correlation values, and side-effect
ordering assertions all pass.

## 18. Run full verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin --lib --no-fail-fast
cargo test --workspace --no-fail-fast
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
just check
```

If environment/tooling makes commands redundant, record exact executed results
and any substitution in progress.md.

Verification: native workspace and production WASM surfaces are green.

## 19. Inspect the final diff and ownership

Confirm source changes are limited to:

- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Confirm no concurrent hunks remain uncommitted in `lib.rs` before this ticket's
transaction.

Confirm ordinary index is empty.

Verification: exact source ownership is auditable from status and diff.

## 20. Record implementation progress

Write attempt-private `progress.md` before the isolated transaction.

Document completed steps, deviations, exact tests, concurrent ownership wait,
and remaining work.

Do not publish it to shared work directly.

## 21. Commit the meaningful source unit

Use only:

```text
lisa commit-ticket --ticket-id T-042-02-02 \
  --message "feat(plugin): persist completion aggregate journal" \
  --include crates/lisa-plugin/src/completion_journal.rs \
  --include crates/lisa-plugin/src/lib.rs
```

The two files form one adapter contract and should compile together.

Do not use ordinary git add or git commit.

Verification: the commit contains exactly those paths and each is clean
afterward.

## 22. Review and stop

Inspect the committed diff and rerun any risk-focused test if HEAD moved during
the transaction.

Write `review.md` with acceptance mapping, test coverage, source commit, and
open concerns.

Write exactly one valid `review-disposition.json` shape.

Do not update ticket phase/status, publish Done, release the seat, or start the
next ticket.
