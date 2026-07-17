# Progress — T-048-01-02 park-instead-of-churn

## Baseline

Read `CLAUDE.md`, the ticket, RDSPI workflow, parent story, predecessor ticket
and artifacts, scheduler implementation, DAG eligibility, disposition parser,
provenance schema, ticket mutation helpers, UI projection, and nearby tests.

The ticket names a historical `scheduler.rs`; current scheduler behavior lives
in `crates/lisa-plugin/src/lib.rs`.

The ordinary worktree began with Lisa-managed state and unrelated source/work
changes. They remain excluded from ticket commits.

Baseline verification passed:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
14 passed

cargo test -p lisa-plugin --no-run
passed
```

## Research, Design, Structure, Plan

Completed all four pre-implementation artifacts in the private attempt work
directory.

Selected design:

- use durable `status: blocked` as the only scheduling authority;
- preserve the canonical admitted Review disposition as structured block data;
- permit two agent-owned Review re-attempts per loop;
- park operator/world blocks immediately;
- explicitly mark world park records recheck-eligible without running checks;
- release/remove parked attempts so they consume no seat;
- reconstruct unpark provenance from the ledger while scheduling naturally from
  `status: open`.

## Implementation unit 1 — bounded block provenance

Modified `crates/lisa-core/src/provenance.rs`.

Changes:

- advanced the ledger schema from 4 to 5;
- added `ParkingTransitionType::Retry`;
- added optional `retry_count` and `retry_limit` fields;
- added defaulted/compact `recheck_eligible`;
- preserved deserialization of schema-4 park rows without additive fields;
- broadened record documentation to retry/park/unpark blocked-work transitions;
- added serialization and round-trip coverage for agent retry metadata;
- added explicit world recheck eligibility coverage;
- retained mixed execution/assignment/parking ledger replay.

Focused verification passed:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
16 passed
```

`cargo fmt --all` also formatted unrelated active CLI work. Those paths are not
owned by this ticket and were not included in the ticket commit.

Committed through Lisa's isolated transaction:

```text
319ac06c7c0b106153c3e1f6068fb836e522b4c4
feat(core): record bounded block retries
```

Exact included path:

- `crates/lisa-core/src/provenance.rs`.

No ordinary-index Git command was used.

## Review hardening follow-up

The source review identified that durable `status: open` should start a fresh
block episode even if the best-effort Unpark provenance append fails.

Moved per-loop retry-counter clearing before the ledger append. Scheduling was
already status-driven; this also prevents a later block from inheriting the old
episode's exhausted count when provenance storage is unavailable.

Reverification passed:

```text
cargo test -p lisa-plugin agent_owned_block --no-fail-fast
1 passed

cargo test -p lisa-plugin --no-fail-fast
399 passed
```

Committed through Lisa's isolated transaction:

```text
9109e7a016309a3fcdc3a424c709decd13acb5e4
fix(plugin): reset block retries on reopen
```

Exact included path:

- `crates/lisa-plugin/src/lib.rs`.

## Full verification

The complete command passed:

```text
cargo test --workspace --no-fail-fast
```

Relevant suite totals included:

- `lisa-cli` library: 19 passed;
- `lisa-cli` binary: 322 passed;
- CLI integration suites: all runnable tests passed;
- `lisa-core`: 216 passed;
- core completion-state and recorded-livelock integrations: 2 passed;
- `lisa-plugin`: 399 passed;
- doc tests: passed;
- real-Zellij delivery boundary: 1 expected ignored test because the external
  toolchain/wasm target gate is not part of the native suite.

Final source audit:

- `crates/lisa-core/src/provenance.rs` is clean and unstaged;
- `crates/lisa-plugin/src/lib.rs` is clean and unstaged;
- the ordinary Git index has no staged ticket paths;
- each Lisa commit contains only its exact declared include path;
- unrelated Lisa journal, ticket, and published-work changes remain untouched.

## Remaining

- write `review.md`;
- write exact `review-disposition.json`;
- remain on this ticket for Lisa's completion transaction.

## Remaining

- run workspace verification;
- audit ticket-owned source cleanliness;
- write Review artifacts.

## Implementation unit 2 — scheduler retry, park, and unpark policy

Modified `crates/lisa-plugin/src/lib.rs`.

Added a fixed two-retry per-loop agent block bound and a pure owner/count policy:

- agent count 0 → Retry 1/2;
- agent count 1 → Retry 2/2;
- agent count 2+ → Park 2/2;
- operator → immediate Park;
- world → immediate Park with recheck eligibility.

Added per-loop agent retry memory that is explicitly not scheduling authority.

Added Review block processing after current-attempt artifact admission:

- requires current lease plus both Review artifacts;
- retains the structured block in canonical admitted work;
- appends Retry evidence before an agent re-attempt;
- writes `status: blocked` before releasing a parked attempt;
- leaves the attempt/seat intact when blocked-status mutation fails;
- releases the slot and removes the thread on successful retry/park;
- rebuilds the DAG after parks so its existing blocked exclusion applies.

Added unpark reconciliation:

- replays the latest parking transition per ticket;
- recognizes a prior Park whose current durable status is open;
- appends one Unpark row with stranded duration;
- preserves owner, attempt, bound, and recheck facts;
- becomes idempotent because Unpark is then the latest row;
- never gates scheduling, which uses only normal DAG status.

Integrated policy into `poll_tick` before Review timeout/reseat consequences and
integrated unpark observation after DAG rebuild. Initial plugin load also
observes status reopened while the loop was stopped.

Added three focused tests:

1. exact owner and retry-bound decision table;
2. 2026-07-16 two-seat replay with operator/world blocks plus two ready tickets;
3. agent Retry 1/2 → Retry 2/2 → Park 2/2 → status-open Unpark sequence.

The two-seat replay asserts both block tickets become durably blocked, neither
is ever reseated, both seats go to ready tickets, both Park rows exist, and only
the world row is recheck eligible.

Focused verification passed:

```text
cargo test -p lisa-plugin review_block_policy --no-fail-fast
1 passed

cargo test -p lisa-plugin park_instead_of_churn --no-fail-fast
1 passed

cargo test -p lisa-plugin agent_owned_block --no-fail-fast
1 passed
```

Complete plugin and workspace-check verification passed:

```text
cargo test -p lisa-plugin --no-fail-fast
399 passed

cargo check --workspace
passed
```

Committed through Lisa's isolated transaction:

```text
dd56fbaf506a27c8a65f1b26f0ca066f7c745e0c
feat(plugin): park blocked reviews without occupying seats
```

Exact included path:

- `crates/lisa-plugin/src/lib.rs`.

No ordinary-index Git command was used.
