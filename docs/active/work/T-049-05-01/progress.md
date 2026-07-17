# Progress: level-triggered block parking

## Completed phases

- Read `AGENTS.md`, `CLAUDE.md`, the complete RDSPI workflow, ticket, story,
  source incident, and current implementation.
- Wrote generation-two Research, Design, Structure, and Plan artifacts in the
  private attempt directory.
- Kept all phase artifacts out of the shared `docs/active/work` path.

## Implemented source unit

Modified only `crates/lisa-plugin/src/lib.rs`.

### Durable generation discovery

- Added a shared private-attempt root resolver.
- Added numeric durable attempt high-water discovery.
- Only positive direct generation directories with a real `work` child count.
- Scheduling merges durable high water into the process-local predecessor map
  immediately before minting.
- A restarted scheduler therefore creates generation N+1 rather than reusing
  generation one over an existing attempt tree.
- Durable discovery never restores `current_leases` authority by itself.

### Canonical generation correlation

- Orphan reconciliation resolves the current live lease when one survives, or
  the newest durable attempt generation otherwise.
- It requires private and canonical `review-disposition.json` bytes to match.
- It parses the canonical file through the existing disposition parser.
- Structured blocks and legacy unstructured blocks therefore share one
  validation boundary.
- A newer attempt directory with no disposition makes an older canonical block
  stale.
- An operator-edited canonical verdict cannot be overwritten by an old private
  block during reconciliation.

### Provenance consumption and attribution

- Extracted latest parking-transition replay into one shared helper.
- Retry, Park, or Unpark at the same/newer generation consumes the old block.
- This keeps live agent retries from being converted into immediate orphan
  parks before a successor is scheduled.
- Added an explicit-attempt transition append boundary.
- The existing live emitter still validates its thread/current lease before
  passing the lease to that append boundary.
- Orphan parking attributes the Park row to reconstructed durable generation
  evidence without restoring execution authority.
- Clippy initially rejected the append helper's argument count; the helper now
  derives ticket identity from the supplied lease instead of accepting a
  redundant ticket parameter.

### Level-triggered parking

- Added `reconcile_orphaned_review_blocks`.
- It scans schedulable Review tickets that lack an authoritative live Review
  thread.
- A current, matching canonical Block writes `status: blocked` first.
- It appends Park provenance, releases any retained slot/current lease, removes
  residual thread and finish-up state, and rebuilds the DAG once.
- World-owned orphan parks remain recheck eligible.
- No retry count is fabricated when no agent retry was durably observed.

### Observation and scheduling boundaries

- Plugin load invokes orphan reconciliation before permission or pane events
  can schedule.
- Every poll invokes orphan reconciliation in addition to the unchanged live
  Review block policy.
- `schedule_ready_tickets` invokes reconciliation before all scheduling early
  returns and before collecting ready tickets.
- Direct permission, pane, completion, world-recheck, and keep-working
  scheduling calls therefore share the same admission fence.
- Existing blocked ticket status remains the durable DAG scheduling authority.

### Unpark behavior

- Existing Unpark reconciliation now reuses the latest-transition helper.
- An Open status plus latest Park still appends one Unpark row.
- That Unpark consumes the old generation's block.
- Scheduling then restores ordinary Review behavior and mints the next durable
  attempt generation.
- Once a new attempt exists, the prior canonical disposition remains harmless.

## Regression coverage added

Added four production-path native regressions:

1. `orphaned_legacy_block_parks_at_load_boundary_without_spawning`
   - uses the preserved T-046-06-03 legacy field reason;
   - starts with no live thread or lease;
   - verifies disk/DAG blocked status, one Operator Park row, no spawned thread,
     and a Waiting-on-you entry;
   - verifies repeat reconciliation is idempotent.
2. `orphaned_block_appearing_after_thread_loss_parks_and_releases_seat`
   - removes the writing thread while retaining its lease and seat;
   - verifies mid-run parking, provenance, lease revocation, and seat release.
3. `scheduling_parks_durable_block_then_unpark_seats_fresh_generation`
   - invokes scheduling directly against a durable block;
   - verifies no reviewer is seated;
   - reopens and reconciles Unpark;
   - verifies generation two is seated normally.
4. `stale_prior_generation_disposition_does_not_park_fresh_attempt`
   - retains generation-one canonical/private Block bytes;
   - creates a newer durable attempt without a verdict;
   - verifies no Park row or blocked status;
   - verifies later scheduling continues monotonically.

Existing E-048 live-thread coverage remained unchanged and passes, including:

- exact owner and retry policy;
- two-seat operator/world parking replay;
- two bounded Agent retries followed by Park;
- status-open Unpark and fresh scheduling;
- dashboard Waiting-on-you projection;
- world-owned recheck behavior.

## Verification completed

- `cargo fmt --all` — passed and source is formatted.
- Focused orphan filters — 3 passed.
- Scheduling/unpark regression — 1 passed.
- Stale-generation regression — 1 passed.
- `cargo test -p lisa-plugin --no-fail-fast` — 427 passed.
- `cargo test --workspace --no-fail-fast` — all workspace unit,
  integration, and doc tests passed; one environment-dependent real-Zellij
  test remained intentionally ignored.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

## Deviation and recovery note

Generation one was still running briefly after generation two launched and had
left same-ticket uncommitted source work in the shared worktree.

The older process exited before commit.

Generation two retained and audited that same-ticket work, then tightened the
generation predicate to require exact canonical/private equality, moved the
scheduling guard before early returns, required real attempt work directories,
and repaired the regression fixtures to model canonical admission.

No unrelated source was adopted or modified.

## Repository hygiene

- Ticket-owned source path: `crates/lisa-plugin/src/lib.rs`.
- No ordinary `git add` or `git commit` was used.
- Lisa-managed journal, provenance, ticket, and shared work changes remain
  outside the source include.
- Concurrent T-049-03-02 completed independently and its commit is the current
  source baseline; this ticket does not claim its files.

## Remaining

- Write Review and exact disposition artifacts.

## Commit and final verification

- Committed through `lisa commit-ticket` with exact include
  `crates/lisa-plugin/src/lib.rs`.
- Commit: `5b423801dbbdc873b87d062e42cbc59715d0a38e`.
- Subject: `fix(plugin): reconcile orphaned review blocks`.
- `just check` passed after the commit, including the WASM target check and
  complete workspace test suite.
- `git diff -- crates/lisa-plugin/src/lib.rs` is empty.
- The ordinary index has no staged paths.
- Only Lisa-managed journal, provenance, ticket, and shared work state remains
  dirty/untracked.
