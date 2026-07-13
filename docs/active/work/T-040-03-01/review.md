# Review: T-039-06-02 blocking Review regression

## Disposition

Pass.

The implementation satisfies T-040-03-01's acceptance criterion and is ready
for Lisa's completion transaction.

## Summary

The plugin test suite now contains a dedicated, searchable regression for the
T-039-06-02 field boundary.

The scenario starts with an assigned current attempt already in Review,
`review.md` present, and a valid machine-readable disposition whose value is
`block`. A second ticket depends on the reviewed ticket.

The test drives the production `check_artifact_advances` scheduler consumer and
proves the block is a refusal, not completion intent.

## File changed

Modified:

```text
crates/lisa-plugin/src/lib.rs
```

Added one test:

```text
tests::test_t039_06_02_blocking_review_never_prepares_done
```

No production function, public interface, manifest, dependency, generated
asset, or configuration was changed. No file was created or deleted in product
source.

## Acceptance criterion trace

### Reproduces review.md plus disposition=block

The fixture installs a current `AttemptLease` across the runtime thread, pane
slot, and scheduler lease map. It writes attempt-private `review.md` and:

```json
{"disposition":"block","reason":"resolve the hostile review finding"}
```

The JSON is a valid block under the core disposition contract, so the test
exercises the deliberate block arm rather than an invalid-document fallback.

### Ticket stays assigned

After the production artifact poll, the test asserts:

- the reviewed thread still exists;
- its phase remains Review;
- its status remains Running;
- pane 39 still owns the reviewed ticket;
- the pane retains the installed attempt lease;
- the scheduler's current lease map retains the same lease.

These checks cover both visible slot assignment and the authority state that
makes the assignment usable.

### Must not prepare Done or commit

The test asserts the reviewed ticket is absent from `pending_completions`.

That is the scheduler boundary immediately before construction/launch of the
native `complete-ticket` command. If no pending request exists, the plugin has
not prepared or launched the commit transaction.

This is the assertion that would have failed against the pre-T-040-01-03
unconditional Review-to-Done path. That path admitted `review.md` and called
`request_completion` without interpreting the block file, inserting a pending
completion even though durable Done had not yet appeared.

The regression's assertion message records that historical discriminator so a
future failure explains why pending state matters.

### No Done publication or provenance

The test reads ticket frontmatter after the poll and requires both:

```text
status: review
phase: review
```

It configures an isolated provenance ledger path and requires that the path was
never created. The fixture begins without a ledger and performs no other
terminal transition, so this proves there is no authoritative Done provenance
row or any other completion residue.

### Dependents stay blocked

The real parsed DAG contains `T-DEPENDENT` with
`depends_on: [T-REVIEW]`.

The test requires `Dag::all_dependencies_done(T-DEPENDENT)` to remain false and
also requires that no runtime thread was created for the dependent. This pins
both readiness computation and the absence of accidental downstream
scheduling.

### Blocking reason remains actionable

The activity log must contain `Completion blocked` and the exact fixture
reason. The retained assignment therefore has a visible reason for operator or
agent follow-up.

## Design assessment

A dedicated historical regression was preferable to adding another assertion
inside the existing generic disposition table.

The existing test remains useful for broad block/pass/invalid policy coverage.
The new test independently combines assignment retention, no transaction
preparation, no provenance, and dependent blocking under the incident ID. That
makes it harder for generic fixture refactoring to erase field evidence.

The test uses the real scheduler state path rather than a parser-only unit.
Parser-only coverage would have passed before the bug was fixed and could not
observe transaction intent or dependent readiness.

Not launching a real Git subprocess is intentional. For a block, the relevant
safety property is that the command boundary is never reached. Existing plugin
and CLI tests cover successful transaction/result publication separately.

## Verification evidence

Formatting:

```text
cargo fmt --all -- --check
```

Passed.

Focused historical test:

```text
cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done
```

Passed: 1 test, 0 failed.

Adjacent disposition policy:

```text
cargo test -p lisa-plugin review_disposition
```

Passed: 1 test, 0 failed.

Complete native workspace:

```text
cargo test --workspace
```

Passed, including:

- 279 `lisa-cli` unit tests;
- CLI integration suites;
- 169 `lisa-core` tests;
- 337 `lisa-plugin` tests;
- doc tests.

The declared real-Zellij environment test remained ignored because its host
prerequisites are outside the ordinary workspace test run.

WASM target:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Passed.

Diff hygiene:

```text
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Passed.

## Commit and ownership review

The ticket executed only the required isolated source transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-03-01 \
  --message "Pin blocking Review completion regression" \
  --include crates/lisa-plugin/src/lib.rs
```

Resulting commit:

```text
b6a574abd4471f8a361b005ddfbac306cf98dffe
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs` and adds 98 test
lines. No ordinary Git index add or ordinary commit was used.

The ticket-owned source path is clean after the transaction. Unrelated
Lisa-managed provenance, ticket, generated docs, and canonical work paths were
preserved and excluded.

## Test coverage assessment

Coverage is strong for the acceptance boundary because one deterministic test
combines all required effects under the real plugin scheduler:

- authoritative attempt artifact admission;
- explicit valid blocking disposition;
- absence of completion preparation;
- retained assignment and lease;
- unchanged durable Review state;
- absence of Done provenance;
- real DAG dependent blocking;
- operator-visible reason.

Broader neighboring tests continue to cover pass, invalid disposition,
stopped-session completion, successful pending completion, transaction result
publication, slot release, and provenance emission.

## Limitations

The test does not execute a live Zellij host or external Git command. A correct
block is required not to launch that command, so the deterministic pending-map
boundary is both faster and more directly tied to the historical fault.

The provenance assertion requires the ledger path not to exist rather than
parsing an existing mixed ledger. This fixture emits no other terminal events,
making nonexistence the strongest expected state. Mixed-ledger parsing and
authoritative flags are covered elsewhere.

The test duplicates some setup from the generic disposition table. The
duplication is deliberate for a self-contained historical regression and does
not introduce a production maintenance surface.

## Open concerns

No blocking implementation, correctness, test, formatting, or ownership issue
remains.

Lisa still owns admission of these attempt-private artifacts, final Done
publication, the completion commit, and release of the current seat. This
attempt must remain on T-040-03-01 until that confirmation.
