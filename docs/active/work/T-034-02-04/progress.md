# Progress: T-034-02-04 one authoritative provenance record

## Status

Implementation, verification, and the isolated source commit are complete.

Ticket frontmatter phase/status were not edited.

## Completed: Research

- Read `CLAUDE.md`, the ticket, and the RDSPI workflow.
- Mapped `AttemptLease`, dispatch stamping, revocation, fencing, and release.
- Mapped `ProvenanceRecord`, every publisher call site, and ledger tests.
- Mapped completion request admission and asynchronous result publication.
- Identified that provenance discarded thread lease and concrete fence result.
- Identified that pending completion was not excluded from timeout reclamation.
- Identified that result publication logged but did not revalidate authority.
- Wrote `research.md`.

## Completed: Design

- Evaluated partial attempt IDs, optional leases, lifecycle event sourcing, and
  required schema-v2 terminal records.
- Selected required complete lease, fence, and authority fields.
- Selected pending completion as a lease-critical section.
- Selected defense-in-depth current checks at result and ledger publication.
- Preserved append-only predecessor history.
- Wrote `design.md`.

## Completed: Structure

- Limited source ownership to core provenance, plugin scheduler, and ledger docs.
- Defined the schema-v2 public record shape.
- Defined publisher, result, timeout, and fence interfaces.
- Defined focused test placement and exact isolated commit paths.
- Wrote `structure.md`.

## Completed: Plan

- Sequenced schema, publisher, fencing, completion stability, fixture migration,
  acceptance regression, documentation, verification, and isolated commit.
- Defined focused and workspace test commands.
- Wrote `plan.md`.

## Completed: core provenance schema

- Bumped `SCHEMA_VERSION` from 1 to 2.
- Added required `attempt_lease: AttemptLease`.
- Added required `authoritative: bool`.
- Added required `fenced: bool`.
- Updated the core sample fixture and compact JSON assertions.
- Preserved enum wire values, metrics, usage parsing, and append helper.

## Completed: scheduler publisher

- Extended `emit_provenance` with explicit confirmed fence state.
- Made the method return whether a record was appended.
- Rejected active threads without a stamped attempt lease.
- Rejected Done when the stamped lease is not current.
- Derived authoritative status only for accepted Done.
- Populated the complete schema-v2 record.
- Preserved non-fatal ledger I/O behavior.

## Completed: timeout and fencing history

- Reordered session timeout to revoke/fence before record append.
- Reordered hard-silence reclaim the same way.
- Classified `Fenced` and `AlreadyFenced` as confirmed fence history.
- Classified `NoAssignedPane` as not confirmed.
- Kept the stamped thread alive through record construction.
- Passed `fenced: false` for error-signal failure and normal completion.

## Completed: completion authority stability

- Excluded pending completion tickets from session/per-phase timeout reclaim.
- Excluded pending completion tickets from hard-silence reclaim.
- Revalidated pending attempt authority at asynchronous result handling.
- Rejected stale results before thread completion, provenance, slot release, or
  dependent scheduling.
- Retained existing operator-only manual behavior.
- Retained isolated command success and durable-Done verification.

## Completed: fixture migration

- Updated direct provenance tests to install real current attempts.
- Updated all publisher calls with fence state.
- Added assertions for lease, fence, and authoritative fields.
- Preserved Claude/Codex usage, route, append, no-op, and frontmatter coverage.

## Completed: acceptance regression

Added `fenced_attempt_and_replacement_publish_one_authoritative_done_record`.

The test proves:

- predecessor attempt one is revoked and physically fenced;
- its TimedOut history is appended with exact lease one;
- its row is fenced and non-authoritative;
- replacement attempt two is strictly newer;
- direct stale Done ledger publication is rejected;
- a stale predecessor completion callback is rejected;
- pending replacement completion cannot be timeout-reclaimed;
- replacement completion publishes exact lease two;
- replacement Done is authoritative and not fenced;
- a duplicate callback does not append again;
- exactly one row is both Done and authoritative.

## Completed: documentation

- Updated the ledger to schema version 2.
- Updated the example JSON and field table.
- Documented attempt versus ticket-authoritative semantics.
- Documented confirmed fence state.
- Updated the successful-run query to select authoritative Done.
- Documented mixed schema-v1/v2 handling.

## Focused verification completed

Commands completed successfully:

```text
cargo test -p lisa-core provenance
  7 passed

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

`cargo fmt --all` was run before focused verification.

## Deviations from plan

- The acceptance regression directly exercises both timeout-critical-section
  behavior and stale result publication rather than creating two separate tests.
  This keeps the entire predecessor-to-winner invariant in one readable timeline.
- No separate fence-classification helper was introduced; the two teardown sites
  use identical small `matches!` expressions, avoiding a public/internal API for
  a two-variant classification.
- The publisher returns false when the ledger path is unset. Existing callers do
  not branch on the return, and the existing no-op test remains green.

## Repository hygiene

- The worktree contained unrelated modified and untracked files before work.
- No unrelated path has been intentionally edited.
- `cargo fmt --all` did not alter the pre-existing dirty CLI path.
- Planned ticket-owned commit paths remain exactly:
  - `crates/lisa-core/src/provenance.rs`;
  - `crates/lisa-plugin/src/lib.rs`;
  - `docs/knowledge/provenance-ledger.md`.
- No ordinary `git add` or `git commit` command was used.

## Full verification completed

```text
cargo fmt --all -- --check
  passed

cargo test --workspace
  lisa-cli:    270 passed
  lisa-core:   155 passed
  lisa-plugin: 272 passed
  doc tests:   passed

cargo check -p lisa-plugin --target wasm32-wasip1
  passed
```

## Isolated source commit completed

The installed `/opt/homebrew/bin/lisa` predates the `commit-ticket` subcommand.

The repository's current CLI exposes it, so the exact equivalent isolated
transaction was run through Cargo:

```text
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-02-04 \
  --message "Attribute authoritative provenance to attempt leases" \
  --include crates/lisa-core/src/provenance.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include docs/knowledge/provenance-ledger.md
```

Commit:

```text
7cd864365193f7ff5d14fce3004d8b9cf6cf3b79
```

Commit contents:

```text
crates/lisa-core/src/provenance.rs
crates/lisa-plugin/src/lib.rs
docs/knowledge/provenance-ledger.md
```

Post-commit checks confirm all three paths are clean and the ordinary Git index
contains no staged paths.

## Remaining

- No implementation work remains.
- `review.md` is complete; stop for Lisa's completion handling.
