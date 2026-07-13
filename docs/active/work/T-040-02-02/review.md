# Review: persist pre-ownership failure row

## Disposition

Block pending commit-ownership repair.

The requested behavior is implemented and all tests pass.

The source commit is not cleanly attributable to this ticket because a
concurrent ticket edited the same file and its hunks were captured by this
ticket's path-scoped isolated transaction.

This is a repository coordination issue, not a functional failure in the
pre-ownership provenance implementation.

## Functional summary

The plugin now persists durable evidence at the three acceptance-named
pre-ownership terminal transitions:

- `fail_assignment_delivery`;
- `fail_assignment_recovery`;
- `fail_startup`.

Each helper maps its retained scheduler state to the matching stable provenance
state:

- delivery failure → `DeliveryFailed`;
- recovery exhaustion → `RecoveryFailed`;
- startup failure → `StartupFailed`.

The exact caller-provided reason is retained verbatim.

The terminal helper guard remains the exact-once boundary.

Once the first call installs the terminal assignment state, a repeated call no
longer matches the accepted source state and cannot append a duplicate row.

## Shared writer

`State::emit_assignment_transition` centralizes row construction and append
behavior.

It treats an unset ledger path as a no-op, matching existing execution
provenance behavior in native tests and pre-load state.

It resolves the slot using both physical pane ID and ticket ID.

It requires a slot attempt lease whose ticket matches the supplied ticket.

It requires the matching thread to retain the same pane and attempt lease.

Malformed or incomplete state fails closed and produces an operator-visible
activity warning instead of fabricated evidence.

The vendor provider is derived from the snapshotted thread client through the
existing `Route::from_client` mapping.

Claude therefore records `anthropic` and Codex records `openai`.

The writer uses the thread attempt start and current terminal observation time.

It computes wall-clock duration with saturating subtraction.

The record uses schema version 3 and explicit record type
`assignment-transition`.

It appends through the core `append_assignment_transition_record` API.

Filesystem errors are logged and swallowed, preserving scheduler state and
loop availability.

## Authority semantics

The new rows are `AssignmentTransitionRecord`, not execution
`ProvenanceRecord` values.

They have no execution outcome.

They have no `authoritative` field.

This records a truthful pre-ownership failure without fabricating a failed
execution or ticket-level result.

A later terminal execution record uses the existing independent writer.

Both writers append to the same JSONL file, so the later row coexists with the
earlier transition and never overwrites it.

## Source file

The ticket intended to modify only:

```text
crates/lisa-plugin/src/lib.rs
```

No core schema changes were needed because dependency T-040-02-01 already
provided the record, enum vocabulary, mixed reader, and append function.

No ticket frontmatter, shared work artifact, live ledger, CLI reader, or
knowledge document was intentionally included as ticket source work.

## Test coverage

`preownership_terminal_transitions_append_once_and_coexist_with_later_done`
constructs complete pane/ticket/thread/lease state and directly drives all
three named helper transitions.

For each transition it asserts:

- one physical JSONL line after the first call;
- no second line after a repeated call;
- schema version 3;
- assignment-transition discriminator;
- exact ticket ID;
- exact attempt lease;
- exact pane ID;
- correct derived vendor;
- exact named state;
- exact reason;
- coherent started/ended/duration values;
- absence of `authoritative`;
- absence of execution `outcome`.

The provider assertions cover both mappings: delivery uses Claude and expects
`anthropic`; recovery/startup use Codex and expect `openai`.

The test then mints a later attempt for the same ticket and appends an
authoritative Done execution row.

The heterogeneous reader observes the original assignment row first and the
later authoritative execution row second.

`assignment_recovery_failure_retains_authority_for_operator_reset` exercises
the real deadline evaluator rather than only the helper in isolation.

It confirms the successor recovery lease is recorded with the production
timeout reason and that subsequent timeout scans do not append duplicates.

All prior assertions about retained seat, thread failure, current/high-water
lease, alert deduplication, and lack of automatic retry remain in place.

## Verification results

Focused transition test:

```text
1 passed; 0 failed
```

Focused real recovery test:

```text
1 passed; 0 failed
```

Full plugin suite:

```text
334 passed; 0 failed
```

Full workspace suite passed.

Observed suite totals included 276 CLI library tests, 169 core tests, and 334
plugin tests, plus the atomic provider contract and help-surface integrations.

The existing real-Zellij delivery test remained ignored by its environment
gate.

`cargo fmt --all -- --check` passed before commit.

`git diff --check -- crates/lisa-plugin/src/lib.rs` passed before commit.

`git show --check a7e4a00` passed after commit.

## Commit

The isolated command returned:

```text
a7e4a0037a98aee90b4b38ee44ee5e7a6255c199
```

Message:

```text
feat(plugin): persist pre-ownership failures
```

## Critical ownership issue

Commit `a7e4a00` is larger than the ticket's reviewed pre-commit diff.

Comparing it with its parent shows it also contains concurrent T-040-01-03
work:

- `lisa_core::disposition` imports;
- the new `request_review_completion` method;
- review-to-Done call-site gating.

T-040-01-03 is an active critical ticket in Implement and owns those changes.

After the commit, the shared file also retained two uncommitted rustfmt-only
hunks within that foreign code.

The cause is concurrent modification of the same file.

Lisa's isolated transaction protects the ordinary index and serializes commit
movement, but `--include crates/lisa-plugin/src/lib.rs` necessarily commits the
whole path rather than selected hunks.

The project workflow explicitly treats same-file concurrent tickets as a
missing dependency edge.

No destructive cleanup was attempted because reverting or rewriting the shared
file would risk discarding the other active agent's work.

## Required repair

Before this ticket can pass review, reconcile T-040-02-02 and T-040-01-03
ownership for `crates/lisa-plugin/src/lib.rs`.

The repair should ensure:

- this ticket's durable pre-ownership provenance changes are committed under
  this ticket;
- T-040-01-03's disposition-gating changes are committed under T-040-01-03;
- no active agent's working state is lost;
- the final shared file retains both compatible functional changes;
- the full plugin/workspace verification remains green;
- no ticket-owned source remains uncommitted.

Adding a dependency edge or otherwise serializing future same-file tickets is
recommended to prevent recurrence.

## Open functional concerns

No functional correctness gap was found in the provenance behavior.

The writer deliberately does not emit when ticket, pane, slot lease, and thread
lease cannot be reconciled; fabricating missing attempt identity would violate
the acceptance criterion.

The row's `started_at` is the common attempt start because the current private
state machine does not retain a dedicated transition-start timestamp across all
three named paths.

That timing choice is documented and covered for coherence, but not asserted
against an independently injected transition clock.

The blocking disposition is solely for commit ownership and coordination.
