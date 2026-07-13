# Review: pane-time ownership lookup

## Disposition

Pass.

The ticket acceptance criterion is implemented, the complete plugin and
workspace test suites pass, the source transaction contains exactly the two
ticket-owned paths, and no owned source remains dirty or staged.

No blocking correctness, test, formatting, scope, or transaction concern was
found during self-review.

## Change summary

This ticket adds a plugin-local pane-time ownership lookup over the scheduler's
durable terminal execution records.

The lookup answers which unique ticket owned one physical pane at one capture
timestamp. It uses the pane ID and closed start/end interval recorded in each
`ProvenanceRecord`.

The ticket does not change any capture writer or usage consumer behavior. It
provides the contract that downstream `T-043-03-01` can call after the append-only
capture schema and writer land.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

Added one private module declaration:

```rust
mod ownership;
```

No other crate-root, scheduler, state, provenance, or usage code changed.

### `crates/lisa-plugin/src/ownership.rs`

Added a focused 120-line module containing:

- module documentation;
- crate-visible `owner_at`;
- deterministic provenance fixture construction;
- recycled-pane acceptance coverage;
- duplicate and conflicting-overlap policy coverage.

No files were deleted or renamed.

## Public/internal interface review

The new function is:

```rust
pub(crate) fn owner_at<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceRecord>,
    pane_id: u32,
    captured_at: u64,
) -> Option<&'a str>
```

`pub(crate)` is the correct current visibility. The downstream consumer lives in
the same plugin crate, while no external library API is requested.

The iterator boundary is useful and appropriately general. It accepts a slice,
vector iterator, reverse iterator, or a chain of persisted records and the
current record. This avoids forcing a filesystem policy or allocation into the
lookup contract.

The borrowed result avoids an unnecessary ticket-ID clone. Callers that need
owned storage can clone at their boundary.

## Source-of-truth review

The lookup accepts `ProvenanceRecord` rather than `Thread`, `AgentSlot`, current
lease state, or environment data.

This is the correct durable source:

- terminal execution rows preserve the earlier owner after thread removal;
- append behavior preserves multiple tickets on a recycled pane;
- pane and epoch timestamps share the exact units needed by capture records;
- records carry explicit ticket identity.

The lookup does not accept mixed `ProvenanceLedgerRecord`, so pre-ownership
`AssignmentTransitionRecord` values cannot accidentally become owned intervals.
The type boundary enforces the schema's documented distinction.

## Interval behavior review

A record matches only when:

```text
record.pane_id == pane_id
record.started_at <= captured_at
captured_at <= record.ended_at
```

Both endpoints are inclusive.

This is appropriate for the schema's epoch-second precision. A capture and
attempt edge can occur within the same second; an exclusive check would create
an artificial attribution gap.

An invalid reversed interval cannot satisfy both comparisons and therefore does
not match.

## Uniqueness and conflict review

The implementation scans all supplied records and retains a unique ticket
identity.

Repeated matching rows for one ticket return that ticket. This handles duplicate
append evidence or overlapping attempts associated with the same ticket without
inventing ambiguity at the ticket-attribution level.

Matching rows for different tickets return `None` immediately.

Failing closed is preferable to first-row or last-row wins because the schema
contains no authority rule for different-ticket overlap. It also makes lookup
behavior independent of ledger or iterator order.

The downstream quarantine story can surface `None` as unattributable instead of
silently blending usage into an arbitrary owner.

## Complexity review

The implementation is O(n) time and O(1) additional space.

It allocates nothing, performs no I/O, and does not sort or build an index.

That is a sound contract implementation for the current ledger scale. A future
capture loader can group execution rows by pane or batch captures without
changing these lookup semantics if repeated full-ledger scans become material.

## Acceptance criterion mapping

Ticket acceptance requires a plugin unit test over pane recycling
`A(t0..t1) -> B(t2..t3)`.

`owner_at_resolves_each_ticket_window_on_a_recycled_pane` creates:

- A on pane 7 during 100 through 199;
- B on pane 7 during 300 through 399.

It proves:

- `owner_at(..., pane 7, time 150) == A`;
- `owner_at(..., pane 7, time 350) == B`;
- time 50 before both is `None`;
- time 250 between both is `None`;
- time 450 after both is `None`.

The same test additionally proves all four inclusive endpoints and rejects a
different pane at an otherwise matching time.

The stated acceptance criterion is fully covered.

## Additional regression coverage

`owner_at_accepts_duplicate_identity_but_rejects_conflicting_overlap` covers
policy not explicitly required by the acceptance statement but necessary for
safe downstream attribution.

It verifies:

- two overlapping A records return A;
- overlapping A and B records return `None`;
- reversing the A/B record order still returns `None`.

This guards against future simplification to an order-dependent `.find()` or
last-match implementation.

## Verification results

Formatting:

```text
cargo fmt --all
cargo fmt --all -- --check
```

Result: pass.

Focused lookup tests:

```text
cargo test -p lisa-plugin owner_at
```

Result:

```text
2 passed; 0 failed; 0 ignored; 375 filtered out
```

Full plugin package:

```text
cargo test -p lisa-plugin
```

Result:

```text
377 passed; 0 failed; 0 ignored
```

Full workspace:

```text
cargo test --workspace
```

Result: pass with exit status 0. All CLI, core, plugin, integration, and doc-test
targets completed without failure.

## Source transaction review

The meaningful unit was committed through the mandated isolated command, not the
ordinary index.

Commit:

```text
ace7af7d0d4030b62cdd9806fcd22a9ca4516818
feat(plugin): add pane-time ownership lookup
```

`git show --name-status` reports exactly:

```text
M crates/lisa-plugin/src/lib.rs
A crates/lisa-plugin/src/ownership.rs
```

Post-commit checks show both source paths have empty staged and unstaged diffs.

Lisa-managed ticket frontmatter and published work artifacts were present in the
shared working tree but were not included in the source transaction.

No `git add`, `git add -A`, ordinary `git commit`, destructive Git command, or
broad ownership include was used.

## Scope review

The implementation intentionally does not:

- define or write `CaptureRecord`;
- change the legacy capture key;
- change Stop hooks;
- load the mixed provenance ledger;
- replace `State::read_usage`;
- reorder `State::emit_provenance`;
- aggregate token counts;
- quarantine unmatched captures;
- log unattributable activity;
- add a second scheduler timeline;
- alter core provenance schema or version.

These exclusions match the story's contract-only boundary and leave named work
to `S-043-02` and `S-043-03`.

## Open concerns and limitations

There is no blocking concern.

Known intentional limitations:

- The caller must parse the mixed ledger and supply only execution records.
- The caller must include the current terminal record if it has not yet been
  appended and captures within that attempt need attribution.
- Different-ticket overlaps collapse to `None`; the current `Option` contract
  does not distinguish a gap from conflicting evidence.
- Lookup is linear in supplied record count.
- Writer and consumer behavior remain legacy until downstream tickets land.

These are deliberate boundaries rather than defects in this ticket.

## Human reviewer focus

A reviewer should primarily confirm:

1. inclusive bounds match desired capture semantics;
2. same-ticket duplicate evidence should remain attributable;
3. different-ticket overlap should fail closed;
4. iterator input is suitable for combining persisted and current records;
5. downstream code filters mixed ledger rows to execution records.

No migration, configuration, runtime deployment, or live metered verification is
required for this contract-only unit-test ticket.

## Final assessment

The plugin now exposes a narrow, deterministic pane-time ownership seam grounded
in durable scheduler facts. It correctly distinguishes two tickets that reuse
one pane, preserves no-owner gaps, avoids order-dependent conflict resolution,
and is ready for downstream capture attribution.
