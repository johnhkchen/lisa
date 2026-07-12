# Review: T-034-02-04 one authoritative provenance record

## Outcome

The ticket acceptance criterion is implemented.

A fenced predecessor and its replacement now produce independently attributable
schema-v2 provenance. The predecessor retains its timeout and confirmed-fence
history. Only the exact current replacement lease can publish an authoritative
Done record, and stale or duplicate completion outcomes append nothing.

Source commit:

```text
7cd864365193f7ff5d14fce3004d8b9cf6cf3b79
Attribute authoritative provenance to attempt leases
```

## Files modified

### `crates/lisa-core/src/provenance.rs`

- Bumped provenance `SCHEMA_VERSION` from 1 to 2.
- Added required `attempt_lease: AttemptLease` to every new record.
- Added required `authoritative: bool`.
- Added required `fenced: bool`.
- Updated schema documentation comments.
- Updated the sample fixture and serialization round-trip assertions.

No route, usage, timing, concurrency, pane, or append behavior was removed.

### `crates/lisa-plugin/src/lib.rs`

- Made provenance publication attempt-aware.
- Rejected records from unleased active threads.
- Rejected Done when the stamped thread lease is not current.
- Derived authoritative status only for accepted Done.
- Captured confirmed fence outcomes in timeout/stale records.
- Reordered fenced teardown so the record reflects actual fence result.
- Excluded pending completions from timeout and hard-silence reclamation.
- Revalidated completion authority when the asynchronous commit result returns.
- Updated existing ledger fixtures to use real attempt leases.
- Added the fenced-predecessor/replacement acceptance regression.

### `docs/knowledge/provenance-ledger.md`

- Updated the schema version and JSON example.
- Added attempt, fence, and authority field definitions.
- Documented attempt-history versus ticket-authoritative semantics.
- Updated the successful Codex query to require authoritative Done.
- Documented schema-v1/v2 mixed-ledger handling.

## Files created

The required phase artifacts were created in this ticket work directory:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

Lisa owns their final completion transaction.

No source file was deleted.

## Behavioral contract

Schema-v2 terminal records now have this identity spine:

```text
ticket_id
attempt_lease { ticket_id, attempt_id }
outcome
authoritative
fenced
```

For a timed-out attempt:

```text
outcome       = timed-out
authoritative = false
fenced        = true when pane fencing was confirmed
attempt_lease = the revoked predecessor stamp
```

For an accepted completion:

```text
outcome       = done
authoritative = true
fenced        = false
attempt_lease = the exact current winning stamp
```

For an ordinary adapter error:

```text
outcome       = failed
authoritative = false
fenced        = false
attempt_lease = the failed attempt stamp
```

Hard-silence failure records carry `fenced: true` when closure is confirmed.

## Authority defenses

The completion path now has three relevant checks.

First, `request_completion` retains the T-034-02-02 admission check: an attempt
must be current before a native completion transaction is launched.

Second, pending completion acts as a lease-critical section. Session timeout and
hard-silence detection do not revoke or replace that attempt while the isolated
transaction is outstanding.

Third, `handle_completion_result` revalidates the preserved pending authority
before publishing scheduler lifecycle state. A stale callback cannot complete a
replacement thread, emit Done provenance, release its slot, or schedule dependents.

Finally, `emit_provenance` independently rejects stale Done. This prevents a
future caller from bypassing result-boundary validation accidentally.

## Append-only history

No prior row is rewritten or removed.

Retries still append distinct rows.

The exact complete lease lets readers group attempts without inferring writer
identity from ticket ID, pane, time, or route.

`fenced` records confirmed scheduler action, not an intended action inferred
from `timed-out` text.

`authoritative` identifies the ticket-level successful publication. Failed and
timed-out rows remain valid historical facts while being non-authoritative as
ticket success outcomes.

## Acceptance regression

`fenced_attempt_and_replacement_publish_one_authoritative_done_record` drives:

```text
attempt 1 current
  -> fail
  -> revoke and fence pane 1
  -> append timed-out history for lease 1
  -> release/remove
attempt 2 current on pane 2
  -> direct stale Done publication using lease 1 rejected
  -> stale completion callback carrying lease 1 rejected
  -> pending lease 2 protected from timeout reclaim
  -> durable Done result accepted for lease 2
  -> authoritative Done appended for lease 2
  -> duplicate callback ignored
```

The final assertions require exactly two rows:

- lease 1: TimedOut, fenced, non-authoritative;
- lease 2: Done, not fenced, authoritative.

The count of rows satisfying `outcome == Done && authoritative` must equal one.

## Test coverage

Focused core coverage:

```text
cargo test -p lisa-core provenance
7 passed
```

This covers schema-v2 serialization, complete lease JSON, new flags, enum wire
names, record round-trip, append preservation, usage extraction, and timestamps.

Focused plugin coverage:

```text
cargo test -p lisa-plugin \
  fenced_attempt_and_replacement_publish_one_authoritative_done_record
1 passed

cargo test -p lisa-plugin provenance --no-fail-fast
8 passed

cargo test -p lisa-plugin completion --no-fail-fast
5 passed

cargo test -p lisa-plugin timeout --no-fail-fast
26 passed
```

These suites cover both providers' usage flow, route retention, append behavior,
ticket-frontmatter isolation, no-ledger behavior, stale/current completion
admission, verified commit publication, retry behavior, timeout guards, and the
new complete predecessor/replacement timeline.

Full regression coverage:

```text
cargo test --workspace
lisa-cli:    270 passed
lisa-core:   155 passed
lisa-plugin: 272 passed
doc tests:   passed
```

WASM verification:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
passed
```

Formatting:

```text
cargo fmt --all -- --check
passed
```

## Compatibility

This is an intentional schema shape change, so the version is 2.

The runtime only appends ledger rows and does not deserialize historical rows.

Existing jq/SQL queries that ignore the new fields continue to read JSONL.

Rust or strongly typed readers of mixed historical ledgers must inspect
`schema_version`: schema-v1 rows cannot truthfully supply a missing attempt lease.

The documentation explicitly rejects manufacturing attribution for old rows.

Provider behavior is otherwise unchanged. Claude and Codex both use the same
scheduler-side lease invariant, route fields, and usage extraction paths.

## Repository and transaction hygiene

The repository was dirty before this ticket began.

Only these ticket-owned implementation paths entered the isolated commit:

```text
crates/lisa-core/src/provenance.rs
crates/lisa-plugin/src/lib.rs
docs/knowledge/provenance-ledger.md
```

The installed Homebrew Lisa binary lacked `commit-ticket`. The current repository
CLI exposed the required subcommand, so the transaction ran through:

```text
cargo run -q -p lisa-cli -- commit-ticket ...
```

The command returned commit `7cd864365193f7ff5d14fce3004d8b9cf6cf3b79`.

Post-commit status confirms all three ticket-owned source/documentation paths are
clean. The ordinary Git index contains no staged paths.

No ordinary `git add`, broad staging, or ordinary `git commit` was used.

## Open concerns

- Schema-v1 rows remain unattributed by design. Consumers that need one logical
  table must represent their attempt identity as unknown, not guess attempt one.
- Usage artifacts are still ticket-keyed. This ticket attributes the terminal
  scheduler record, but a future attempt-scoped usage capture could improve token
  fidelity when retries overlap unusually closely.
- Pending completion suppresses timeout reclamation until the native command
  returns. The command runner is already the completion durability boundary; if
  it can hang indefinitely in a future environment, it may need its own bounded
  transaction timeout that does not revoke the lease while the subprocess can
  still commit.
- Operator completion without an active thread still has no execution thread to
  attribute, so `emit_provenance` remains a no-op there. This matches the existing
  manual no-thread behavior and does not claim a fabricated execution attempt.
- The full live split-brain proof remains assigned to S-034-03. This ticket adds
  the deterministic scheduler-level provenance regression required for its input.

## Critical issues

None found.

All planned source work is committed, all verification is green, and no
ticket-owned source path remains modified or staged.
