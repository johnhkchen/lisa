# Progress: persist pre-ownership failure row

## Status

Implementation is complete.

The ticket-owned source unit is ready for the isolated Lisa commit.

## Completed work

- [x] Read the ticket, project instructions, and complete RDSPI workflow.
- [x] Mapped the core provenance schema and plugin terminal transition sites.
- [x] Wrote `research.md` in the attempt-private work directory.
- [x] Evaluated emission boundaries and selected guarded helper-local emission.
- [x] Wrote `design.md` in the attempt-private work directory.
- [x] Defined the single-file source and test structure.
- [x] Wrote `structure.md` in the attempt-private work directory.
- [x] Sequenced implementation, verification, and isolated commit steps.
- [x] Wrote `plan.md` in the attempt-private work directory.
- [x] Added the assignment-transition schema imports to the plugin.
- [x] Added one shared `emit_assignment_transition` writer.
- [x] Wired `fail_assignment_delivery` to `DeliveryFailed` evidence.
- [x] Wired `fail_assignment_recovery` to `RecoveryFailed` evidence.
- [x] Wired `fail_startup` to `StartupFailed` evidence.
- [x] Added heterogeneous ledger test reading.
- [x] Added complete three-path field and exact-once coverage.
- [x] Added coexistence coverage with a later authoritative Done row.
- [x] Updated the real recovery-timeout characterization to expect durable
  evidence and reject duplicates.
- [x] Ran focused tests.
- [x] Ran full plugin tests.
- [x] Ran full workspace tests.
- [x] Ran formatting and diff hygiene checks.

## Source changes

`crates/lisa-plugin/src/lib.rs` imports the schema vocabulary introduced by
T-040-02-01:

- `AssignmentState`;
- `AssignmentTransitionRecord`;
- `ProvenanceRecordType`.

The three named terminal helpers now invoke the common writer after their
source-state guard succeeds and after the thread is marked failed.

Each call passes the stable durable state and the exact original reason.

The shared writer:

- treats an unset ledger path as a no-op;
- resolves the slot by exact pane and ticket;
- requires an attempt lease on that slot;
- requires a matching thread pane and attempt lease;
- derives `anthropic` or `openai` from the snapshotted thread client;
- uses the thread attempt start and current observation time;
- computes duration with saturating subtraction;
- constructs `AssignmentTransitionRecord` with schema version 3;
- appends through the core assignment-transition append API;
- logs inconsistent evidence as a warning;
- logs filesystem failure as an error without changing scheduler policy.

The named terminal-state guard remains the exact-once boundary.

After the first call stores a terminal state, a repeated call no longer matches
the helper's accepted source state and cannot reach the writer.

## Test changes

`read_mixed_ledger` reads both assignment-transition and execution rows through
`ProvenanceLedgerRecord`.

`preownership_failure_state` builds internally consistent test state with a
bound pane, ticket, thread, lease, current/high-water maps, client, source
assignment state, and temporary ledger.

`preownership_terminal_transitions_append_once_and_coexist_with_later_done`
drives all three real helper methods.

For every path it asserts:

- exactly one physical JSONL line;
- schema version 3;
- `assignment-transition` record type;
- ticket `T-NAME`;
- exact attempt lease;
- pane 10;
- derived vendor identity;
- exact named durable state;
- exact caller reason;
- coherent timestamps and saturating duration;
- absence of `authoritative`;
- absence of execution `outcome`;
- no second append from a repeated terminal call.

The delivery case uses Claude and pins provider `anthropic`.

The recovery and startup cases use Codex and pin provider `openai`.

The same test mints a later attempt for the delivery ticket, emits an
authoritative Done execution record, and verifies both heterogeneous rows
remain in order.

`assignment_recovery_failure_retains_authority_for_operator_reset` continues to
drive the real deadline evaluator.

It now asserts one `RecoveryFailed` row with the exact successor attempt and
production timeout reason.

Its second timeout evaluation confirms no duplicate row is added.

All previous retained-state and operator-reset authority assertions remain.

## Focused verification

Command:

```text
cargo test -p lisa-plugin preownership_terminal_transitions_append_once_and_coexist_with_later_done
```

Result:

```text
1 passed; 0 failed
```

Command:

```text
cargo test -p lisa-plugin assignment_recovery_failure_retains_authority_for_operator_reset
```

Result:

```text
1 passed; 0 failed
```

## Full verification

Command:

```text
cargo test -p lisa-plugin
```

Result:

```text
334 passed; 0 failed
```

Command:

```text
cargo test --workspace
```

Result:

```text
workspace unit, integration, and doc tests passed
real_zellij_delivery_boundary remained intentionally ignored by its existing environment gate
```

The workspace output included:

- 276 CLI library tests passed;
- 169 core tests passed;
- 334 plugin tests passed;
- atomic provider contract integration passed;
- help surface integration passed;
- doc tests passed.

Formatting command:

```text
cargo fmt --all -- --check
```

Result: passed.

Diff hygiene command:

```text
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Result: passed.

## Deviations

The planned writer consistency checks were implemented as one evidence lookup
rather than several branch-specific warning messages.

This keeps the failure behavior fail-closed and produces one actionable
diagnostic without changing the designed invariant.

The tests also retain ledger-existence assertions with activity-log diagnostics
before field parsing, making a missing append failure easier to diagnose.

No production-scope deviation remains.

`fail_startup_recovery` is unchanged as planned; only the acceptance-named
`fail_startup` site emits `StartupFailed` here.

## Ownership

The only ticket-owned source path is:

```text
crates/lisa-plugin/src/lib.rs
```

Unrelated ticket files, the live provenance ledger, and shared work artifacts
were not edited or staged as ticket source work.

## Isolated commit

Command:

```text
lisa commit-ticket \
  --ticket-id T-040-02-02 \
  --message "feat(plugin): persist pre-ownership failures" \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
a7e4a0037a98aee90b4b38ee44ee5e7a6255c199
```

Only the exact ticket-owned source path was included.

## Remaining

- [x] Commit the source file with `lisa commit-ticket` and the exact include.
- [x] Verify the ticket-owned source changes are committed; post-commit audit
  found concurrent unrelated changes on the same path.
- [ ] Write `review.md`.
- [ ] Write valid `review-disposition.json`.
- [ ] Remain on this ticket for Lisa completion publication.

## Post-commit concurrency finding

Immediately after the isolated commit, `git status --short --
crates/lisa-plugin/src/lib.rs` still reported the shared file modified.

The remaining two hunks are rustfmt-only changes in T-040-01-03 code and are
not owned by this ticket.

More importantly, comparison of commit `a7e4a00` with its parent shows the
commit contains T-040-01-03's substantive review-disposition work alongside
this ticket's provenance work.

The foreign hunks include:

- the `lisa_core::disposition` import;
- `State::request_review_completion`;
- review completion call-site rewiring.

T-040-01-03 is currently in Implement and its ticket explicitly owns those
changes.

This occurred because both tickets concurrently edited the same repository
path and `lisa commit-ticket --include` isolates by path, not by hunk.

The RDSPI concurrency guidance identifies same-file concurrent ownership as a
missing dependency edge.

No attempt was made to revert, rewrite, stage, or recommit the other ticket's
work because that would overwrite or consume another active agent's changes.

Review disposition must therefore block completion until the shared-path
ownership is reconciled and each ticket's commit boundary is restored.
