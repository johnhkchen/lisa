# Review: quarantine unattributable usage

## Disposition

Pass.

The ticket acceptance criterion is implemented and covered by a focused
regression.

No ticket-owned source change remains modified, staged, or untracked.

All required source changes were committed through Lisa's isolated transaction.

No critical issue requires human intervention before completion.

## What changed

The plugin now preserves valid usage captures that pane-time ownership cannot
confidently assign.

Instead of silently dropping those rows or assigning them to a shared fallback,
it writes each one to:

```text
.lisa/<client>/quarantine/<encoded-session-id>.jsonl
```

Codex and Claude retain distinct provider namespaces.

Each quarantine file is keyed by the capture's observed provider session ID.

Ordinary ASCII session IDs remain recognizable in filenames.

Unsafe or non-ASCII bytes are reversibly percent-encoded.

Slash, backslash, dot, and percent cannot become path traversal syntax.

An empty ID uses a collision-safe encoded sentinel.

No `last`, `last.usage.json`, or provider-wide quarantine bucket is created.

## Files created

### `crates/lisa-plugin/src/quarantine.rs`

This private module owns session path derivation and durable append mechanics.

It defines `QuarantinedCaptureRecord` with:

- `source_line`, the 1-based physical row in `captures.jsonl`;
- `capture`, the unchanged `CaptureRecord` observation.

The original pane, session, timestamp, and token values remain intact.

The source line is the durable idempotence key.

Because the capture ledger is append-only, a physical row keeps the same
identity across plugin rescans and reloads.

An exact repeated source row returns `AlreadyPresent`.

It does not append another row or request another operator warning.

Two byte-identical capture values on distinct source lines remain two distinct
quarantine records.

If one source line already contains different capture data, the store fails
closed with invalid-data rather than silently overwriting or conflating it.

The writer creates the session directory as needed and opens the file in append
mode.

It never truncates or rewrites existing rows.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

The new module is registered privately.

`CaptureRecord` is imported for explicit consumer and helper types.

`State::read_usage` now takes `&mut self` so it can raise activity events.

The capture ledger scan enumerates physical source lines before JSON parsing.

Malformed rows remain skipped and cannot provide a reliable session key.

The consumer retains its current physical-pane boundary.

This avoids evaluating captures from another pane whose live ownership interval
may not yet be present in durable provenance.

Same-pane captures later than `current.ended_at` are left pending.

This avoids prematurely quarantining a later ticket's capture when the ledger
is preloaded but that later execution record has not yet closed.

Eligible valid captures now have an explicit three-way ownership branch:

1. A capture owned by the current ticket enters checked token summation.
2. A capture uniquely owned by another ticket is skipped for this record.
3. A capture for which `owner_at` returns `None` enters quarantine.

Both missing ownership and conflicting overlapping ownership return `None` by
the existing `owner_at` contract.

Neither case fabricates a ticket assignment.

`State::quarantine_capture` connects persistence to activity reporting.

A newly appended quarantine row logs `ActivityEvent::Warning` containing:

- provider client;
- raw escaped session ID;
- pane ID;
- capture timestamp;
- destination path.

Generic warning events already convert to `ui::ActivityType::Warning`, so the
event appears in the dashboard activity feed and textual state snapshot.

A repeated rescan logs nothing because no new quarantine occurred.

A quarantine read, validation, directory, serialization, or append failure logs
`ActivityEvent::Error` with the same capture identity and target path.

Such a failure never sends the tokens into current provenance.

## Acceptance coverage

The new regression is:

```text
provenance_unattributable_capture_is_quarantined_by_session_and_visible
```

It creates one syntactically valid Codex capture on the current physical pane.

Its timestamp is before the current ticket's interval and absent from every
prior ownership interval.

The test verifies `owner_at`'s `None` path through the real usage consumer.

It asserts returned tokens and cost are all null.

This proves the capture does not blend into the current ticket.

It reads the expected session-specific quarantine file.

It parses exactly one `QuarantinedCaptureRecord`.

It compares source line 1 and the complete original capture.

It asserts a provider-wide `quarantine.jsonl` does not exist.

It asserts `last` and `last.usage.json` do not exist.

It finds the new `ActivityEvent::Warning`.

It passes that event through `activity_event_to_ui_entry` and matches a visible
`ui::ActivityType::Warning` carrying the session identity.

It scans the capture ledger a second time.

It then verifies both the quarantine row count and matching warning count remain
one.

This covers the acceptance criterion plus rescan idempotence.

## Additional unit coverage

`session_paths_are_safe_and_injective_for_opaque_ids` covers:

- ordinary provider session shapes;
- parent-directory traversal syntax;
- forward and backward slashes;
- literal dot and percent;
- Unicode UTF-8 bytes;
- empty identity;
- empty-sentinel collision avoidance.

Every derived path is asserted to remain directly under the provider's
`quarantine` directory.

`append_is_idempotent_by_source_line_and_preserves_identical_rows` covers:

- first append and envelope parse;
- byte-for-byte stability on an exact rescan;
- a second source line with identical capture values;
- preservation of both distinct source rows.

The existing recycled-pane test now also asserts that B's later captures do not
create quarantine while A's earlier interval is being closed.

Existing Codex and Claude owned-capture tests remain green.

Existing ownership gap and ambiguous-overlap tests remain green.

## Verification results

Baseline before implementation:

- ownership tests: 2 passed;
- recycled-pane attribution regression: 1 passed.

Focused after implementation:

- `cargo test -p lisa-plugin quarantine`: 3 passed;
- `cargo test -p lisa-plugin owner_at`: 2 passed;
- recycled-pane attribution regression: passed;
- Codex owned usage flow: passed;
- Claude owned usage flow: passed;
- acceptance regression rerun immediately before commit: passed.

Formatting:

- `cargo fmt --all -- --check`: passed.

Plugin suite:

- `cargo test -p lisa-plugin`: 381 passed, 0 failed, 0 ignored.

Workspace suite:

- `cargo test --workspace`: passed;
- the real-Zellij environment test retained its declared ignored status.

Repository quick gate:

- `just check`: passed;
- WASM target check passed;
- workspace tests passed again.

## Commits

Storage and compiled module registration:

```text
c7a05511a35ec8a192bae9a4b3033858944989db
feat(plugin): add session quarantine store
```

Consumer integration and acceptance test:

```text
309b282d1d3a3ac9a9e313871382663ce6bbb179
feat(plugin): quarantine unattributable captures
```

Both were created with `lisa commit-ticket` and exact repository-relative
include paths.

No ordinary-index add or commit command was used.

## Plan deviation

The plan initially described the first commit as only `quarantine.rs`.

Rust does not compile or discover that module until `lib.rs` declares it.

The first commit therefore included the new file and only the
`mod quarantine;` registration line from `lib.rs`.

That made the storage unit compiled and testable before behavioral integration.

The second commit then made the runtime and test changes in `lib.rs`.

This deviation was recorded in `progress.md` before the first commit.

## Open concerns and limitations

Malformed capture JSON is still skipped rather than quarantined.

That boundary is necessary because a malformed row cannot reliably supply the
session ID required by this ticket's storage contract.

Idempotence depends on the established append-only `captures.jsonl` contract.

External truncation or rewriting is treated as corruption rather than supported
input behavior.

Quarantine files are local ignored machine state, matching capture ledgers.

They are not a new durable Git reporting surface.

Quarantine currently occurs when terminal provenance consumption scans the
same pane.

A capture on a pane that never reaches any later terminal scan remains in the
original append-only capture ledger until such a scan occurs.

This ticket does not add a background global sweeper.

The bounded activity log can eventually evict old warnings, consistent with all
existing activity events.

New quarantine events are visible at occurrence and remain durably inspectable
through their session files.

The ticket does not reconcile quarantine back into provenance if new ownership
evidence appears later.

The closed-time filter avoids the known premature case; captures already proven
unowned are deliberately held aside rather than guessed.

The full six-overwrite field-incident reproduction, including quarantine and
no-capture surfacing together, remains explicitly assigned to `T-043-03-03`.

No live metered multi-pane run was performed; the story declares that boundary
separately, and deterministic native plus WASM compile gates pass here.

Quarantine persistence failure visibility is implemented but not given a
separate integration test in this ticket.

The acceptance path and storage success/idempotence paths are directly covered.

None of these limitations blocks the stated acceptance criterion.

## Final repository state

`crates/lisa-plugin/src/quarantine.rs` is committed.

All ticket-owned changes in `crates/lisa-plugin/src/lib.rs` are committed.

The ordinary Git index is empty.

Remaining status entries belong to Lisa's ticket transition, provenance,
completion journal, and admitted work-artifact machinery.

Those paths were not included in ticket source commits and were not reverted.

Lisa retains responsibility for publishing this Review, creating the completion
commit, moving the ticket to Done, and releasing the seat.
