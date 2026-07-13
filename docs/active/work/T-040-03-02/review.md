# Review: rc.6 pre-ownership evidence regression

## Disposition

PASS.

T-040-03-02 satisfies its acceptance criterion.

One deterministic native regression now drives a production pre-ownership
delivery miss, proves a durable failure row was appended, and retrieves that
same physical row through the implementation behind `lisa status --ticket`.

No blocking correctness, test, source-ownership, or commit-scope issue remains.

## Commit reviewed

```text
1d8d0ad20813ceed6dcb22bb13cb2929afbc0d7f
Pin pre-ownership CLI evidence regression
```

The isolated commit contains exactly:

```text
crates/lisa-cli/src/main.rs
crates/lisa-cli/src/preownership_status.rs
crates/lisa-cli/src/status.rs
crates/lisa-plugin/src/lib.rs
```

`git show --check` passes.

All four ticket-owned source paths are clean and unstaged after the transaction.

## Historical regression

The new plugin test is:

```text
rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

It constructs a real scheduler fixture rather than writing a static provenance
row.

The fixture binds ticket `T-NAME` to pane 10 with a Codex thread, current
attempt lease, retained seat reservation, and temporary ledger.

The seat begins in `Delivering` after its bounded retry allowance has been
consumed.

The test injects a time after the acknowledgement deadline into
`check_assignment_ack_timeouts_at`.

This drives the production timeout branch that calls
`fail_assignment_delivery` with the stable reason:

```text
provider did not acknowledge the bounded chat assignment
```

No provider process, model call, Zellij pane, wall-clock sleep, or network
service is involved.

## Scheduler outcome

The test requires exactly one `AssignmentDeliveryFailed` transition outcome.

It requires the retained seat state to be `DeliveryFailed`.

The assertion explicitly states that a missed assignment must remain failed
and never be treated as provider ownership.

This pins the lifecycle classification independently of provenance rendering.

## Durable evidence

The regression reads the physical JSONL ledger created by the scheduler.

It requires exactly one row for the terminal edge.

The row must decode through `ProvenanceLedgerRecord` as an
`AssignmentTransition`, not an execution record.

The test checks:

- current schema version;
- `assignment-transition` discriminator;
- ticket `T-NAME`;
- exact current attempt lease;
- pane 10;
- provider `openai` derived from Codex;
- durable state `DeliveryFailed`;
- exact production reason.

Raw JSON must omit `outcome` and `authoritative`, preserving the contract that
pre-ownership failure evidence is not fabricated execution authority.

The physical-row assertion is the historical discriminator.

Against the pre-S-040-02 scheduler, the miss would have produced no assignment
row. Reading the ledger would fail because it did not exist, or the CLI query
would find no matching pre-ownership evidence.

## CLI retrieval

The shipped pre-ownership ledger reader and renderer were mechanically moved
from the general DAG status module into:

```text
crates/lisa-cli/src/preownership_status.rs
```

`main.rs` now calls that module for the existing `Status` ticket-evidence mode.

The plugin's native test module includes that exact source file as a test-only
surface.

The regression passes the scheduler-created ledger path to
`write_preownership_status` and captures its output.

It requires a one-row report containing:

- queried ticket ID;
- attempt and historical pane;
- `delivery-failed` stable state;
- exact persisted reason;
- `openai` provider;
- start time;
- end time;
- wall-clock duration.

This is not a copied formatter or an independently generated fixture. The
producer and consumer operate on the same temporary physical file during one
test.

## CLI compatibility

The extraction did not change command arguments, help text, path resolution,
error behavior, or rendered output.

`lisa status --ticket <id>` still defaults to `.lisa/provenance.jsonl`.

`--ledger` still accepts an explicit fixture or archived ledger path.

Normal `lisa status` still calls the existing DAG report.

The existing binary-level integration test remains unchanged and passes with
exact stdout and empty stderr.

The three existing focused reader tests moved with the implementation:

- mixed execution/assignment ledger filtering;
- valid ledger with no matching assignment rows;
- malformed later row rejected before any output.

## Test-only source seam

The plugin crate is a private-state `cdylib`, while the CLI is a binary crate.

There is no reliable cross-package `CARGO_BIN_EXE_lisa` path for plugin unit
tests, and recursively building Cargo from a test would not be hermetic.

Including the small self-contained CLI module under the plugin's existing
`#[cfg(test)]` module provides one exact source of truth without a production
dependency or public scheduler test API.

The seam is restricted to native tests. It is absent from deployed WASM code.

The included stdout wrapper is unused in the plugin copy, so the embedding
module carries a scoped dead-code allowance. The production CLI copy uses the
wrapper normally and has no such allowance.

## Focused verification

Historical regression:

```text
cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

Result: 1 passed, 0 failed.

CLI report tests:

```text
cargo test -p lisa-cli preownership_status
```

Result: 3 passed, 0 failed.

Binary CLI fixture:

```text
cargo test -p lisa-cli --test preownership_status
```

Result: 1 passed, 0 failed.

## Full verification

```text
cargo fmt --all -- --check
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

All commands passed.

Observed native unit totals were:

- 279 CLI tests;
- 169 core tests;
- 341 plugin tests.

All enabled integration and doc-test targets passed.

The established real-Zellij delivery harness remained ignored by its explicit
environment gate; the new acceptance regression does not depend on it.

## Scope assessment

No production scheduler state transition changed.

No provenance schema or append behavior changed.

No provider readiness, acknowledgement, retry, or recovery behavior changed.

No static fixture changed.

No dependency was added.

No ticket frontmatter or canonical work artifact was directly edited by the
source transaction.

The only non-test production change is the behavior-preserving CLI module
extraction required to expose the exact consumer implementation to the native
producer regression.

## Coverage limits

The joined regression chooses the delivery-miss variant, which satisfies the
acceptance criterion's delivery/recovery/startup alternative.

Existing dependency coverage independently drives and validates recovery and
startup terminal persistence states.

The regression calls the exact ledger query/renderer used by CLI dispatch but
does not launch a second CLI process from the plugin test. Clap and process
dispatch remain covered by the unchanged black-box CLI integration test.

This split keeps the producer-to-consumer proof deterministic without relying
on Cargo build order or an installed binary.

## Open concerns

No open functional concern was found.

Cross-crate source inclusion is intentionally narrow and test-only. If the CLI
later becomes a reusable library, this seam can be replaced with a normal
dev-dependency without changing the regression's behavioral assertions.

The duplicated three report-module tests add negligible runtime and make the
included source's self-contained contract visible in the plugin suite.

No human intervention is required before completion.
