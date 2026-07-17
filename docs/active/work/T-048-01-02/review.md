# Review — T-048-01-02 park-instead-of-churn

## Outcome

The ticket is ready to complete.

Blocking Review dispositions now produce bounded scheduler behavior instead of
indefinite seat churn.

Operator- and world-owned remedies park immediately.

Agent-owned remedies receive exactly two fresh Review re-attempts during one
loop process, then park if the third attempt still blocks.

Parked tickets are durably represented by `status: blocked`, remain visible in
the DAG/board, and own neither a running thread nor an assigned seat.

Changing status back to `open` restores ordinary DAG eligibility and records an
Unpark transition without a parked allow-list or alternate scheduler authority.

## Files modified

### `crates/lisa-core/src/provenance.rs`

Advanced the provenance schema from version 4 to version 5.

Extended `ParkingTransitionType` with `Retry` alongside `Park` and `Unpark`.

Extended `ParkingTransitionRecord` with additive fields:

- optional `retry_count`;
- optional `retry_limit`;
- defaulted `recheck_eligible`.

The optional fields are omitted from compact JSON when absent.

The boolean is omitted when false and explicit when true.

Schema-4 park/unpark rows remain deserializable because all new fields have
defaults.

Documentation now describes the type as blocked-work retry/parking evidence,
not only point transitions into and out of parked state.

### `crates/lisa-plugin/src/lib.rs`

Added the fixed scheduler constant `MAX_AGENT_BLOCK_RETRIES = 2`.

Added a pure owner/count decision function returning Retry or Park metadata.

Added per-loop retry counters keyed by ticket ID. They are policy memory only;
ticket status remains scheduling authority.

Added `apply_review_block_policy`, which:

- scans running Review threads;
- requires an exact current attempt lease;
- uses the existing Review input admission path;
- requires both admitted Review artifacts;
- acts only on a valid typed block disposition;
- emits bounded retry evidence for agent-owned blocks;
- writes blocked status before releasing a park;
- retains a live attempt when the durable blocked write fails;
- releases the slot and removes the thread after successful retry/park;
- rebuilds the DAG after parking.

Added a block transition provenance writer that validates current attempt
authority before appending retry or park rows.

Added Unpark reconciliation that replays the latest parking transition, detects
durable open status, appends one interval row, and is idempotent because the new
Unpark row becomes the latest transition.

Unpark clears the old per-loop retry count before best-effort provenance append,
so repository status starts a fresh scheduling episode even when the ledger is
temporarily unwritable.

Integrated block policy after artifact advancement and before completion,
timeout, and stale-reseat consequences.

Integrated Unpark observation after DAG rebuild and during initial plugin load.

## Durable-state assessment

The central authority choice is intentionally small:

- `status: blocked` prevents scheduling through the existing DAG rule;
- `status: open` permits scheduling through that same rule;
- canonical `review-disposition.json` carries reason, owner, ask, steps, check,
  and unstructured fallback semantics;
- provenance observes Retry/Park/Unpark history;
- in-memory counters only apply the per-loop agent bound.

No parallel parked-ticket file or persisted allow-list was introduced.

No `ThreadStatus::Parked` entry is retained after policy action. This avoids a
stale in-memory thread defeating status-driven unpark and guarantees the logical
seat count is released.

Status-first park ordering is safety-critical. If the frontmatter write fails,
the attempt is not torn down and the ticket cannot be silently exposed as open.

Lease revocation still occurs through the existing shared slot-release path.

The completion reducer, completion journal, isolated Done transaction, and Done
provenance authority were not changed.

## Structured-block assessment

The scheduler does not duplicate structured block fields into another store.

It invokes `review_completion_inputs`, which admits the current attempt's
`review.md` and `review-disposition.json` into canonical work before policy.

The block therefore remains readable after its thread and lease are released.

Unstructured legacy blocks already parse as operator-owned through the
T-048-01-01 contract, so this policy parks them immediately rather than
retrying indefinitely.

World blocks set `recheck_eligible: true` in Park provenance.

The `check` string remains inert. No shell parser, process execution, or remedy
evaluation was added.

## Retry semantics

The first agent-owned blocking attempt emits Retry 1/2 and releases.

The second agent-owned blocking attempt emits Retry 2/2 and releases.

The third agent-owned blocking attempt emits Park 2/2 and writes blocked status.

Every transition carries the exact attempt lease, so ledger replay distinguishes
the three attempts rather than collapsing them into one ticket-level event.

The bound resets with the scheduler process, matching the ticket's “per loop”
scope.

An explicit reopen also clears the prior episode's consumed counter.

## Test coverage

### Core provenance tests

Added/updated coverage for:

- Retry discriminator serialization;
- agent remedy owner;
- exact retry count and retry limit;
- compact omission of absent policy fields;
- explicit world recheck eligibility;
- round-trip equality;
- schema-4 compatibility defaults;
- mixed execution/assignment/parking ledger replay;
- park/unpark append ordering.

Focused result: 16 provenance tests passed.

### Pure scheduler policy test

The decision table covers:

- agent at zero consumed → Retry 1/2;
- agent at one consumed → Retry 2/2;
- agent at two, three, and `u8::MAX` → Park 2/2;
- operator → immediate Park, not recheck eligible;
- world → immediate Park, recheck eligible.

### 2026-07-16 replay

The regression constructs a two-seat board with:

- one current operator-owned Review block;
- one current world-owned Review block;
- two independent queued tickets;
- exact attempt leases and private Review artifacts;
- a real temporary mixed provenance ledger.

It asserts:

- both blocks become durably blocked;
- both blocking threads are removed;
- both leases and seat assignments are released;
- neither blocker is selected on repeated scheduling;
- both queued tickets acquire the two seats;
- exactly two Park rows exist;
- only the world Park is marked recheck eligible.

### Agent bound and unpark replay

The regression drives real scheduler state through:

- attempt 1 → Retry 1/2;
- attempt 2 → Retry 2/2;
- attempt 3 → Park 2/2;
- repeated scheduling while blocked → no attempt;
- frontmatter status changed to open;
- Unpark append;
- attempt 4 scheduled through ordinary DAG eligibility.

It asserts exact ledger order, attempt IDs, bound metadata, counter reset, and
idempotent Unpark reconciliation.

## Verification results

The following passed:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
cargo test -p lisa-plugin review_block_policy --no-fail-fast
cargo test -p lisa-plugin park_instead_of_churn --no-fail-fast
cargo test -p lisa-plugin agent_owned_block --no-fail-fast
cargo test -p lisa-plugin --no-fail-fast
cargo check --workspace
cargo test --workspace --no-fail-fast
```

Notable totals:

- lisa-cli library: 19 passed;
- lisa-cli binary: 322 passed;
- lisa-core: 216 passed;
- lisa-plugin: 399 passed;
- all runnable integration and doc tests passed.

The real-Zellij delivery test remained its expected ignored external-toolchain
test. This ticket's scheduler replay runs natively through production methods.

## Commits and repository hygiene

Ticket-owned source was committed only through Lisa's isolated transaction:

1. `319ac06c7c0b106153c3e1f6068fb836e522b4c4` — core provenance contract;
2. `dd56fbaf506a27c8a65f1b26f0ca066f7c745e0c` — scheduler policy and tests;
3. `9109e7a016309a3fcdc3a424c709decd13acb5e4` — reopen resilience follow-up.

Each commit contains exactly its declared source path.

Both ticket-owned source files are clean and unstaged.

The ordinary index contains no ticket-owned entries.

Unrelated journal, ticket, source, and published-work changes were not included
or reverted.

## Open concerns and boundaries

World remedy checks are only marked eligible; execution belongs to
T-048-02-02 as required.

There is no new dashboard or CLI copy and no unblock command; those belong to
S-048-02. Existing blocked-ticket projection keeps parked work visible now.

Unpark reconciliation reads the mixed ledger when invoked. The current ledger
scale and five-second scheduler interval are acceptable for this slice, but a
future query/index layer may cache latest parking transitions if ledgers become
large. This is a performance consideration, not a correctness gap.

Physical pane reuse still honors existing cooldown, quiet, and provider
transition safety. Parking frees logical `max_threads` capacity immediately;
actual pane selection remains subject to those established anti-clobber guards.

No known correctness issue, TODO, or human action blocks completion.
