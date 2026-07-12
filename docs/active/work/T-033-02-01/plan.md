# Plan: surface handoff state in the dashboard

## Implementation goal

Thread the existing scheduler-owned recycled-Codex assignment state through the
UI projection and render distinct status indicators for pending acknowledgment,
acknowledged ownership, and recovery. Prove the complete source-to-dashboard
path with a deterministic snapshot test.

## Step 1: add the UI assignment-status type

File: `crates/lisa-plugin/src/ui.rs`

- Add `SeatAssignmentStatus` beside slot presentation data.
- Include pending, owned, recovering, and recovery-failed variants.
- Derive copy/equality traits needed for mapping and tests.
- Add exact label and color methods.

Verification:

- compiler enforces exhaustive label/color matches;
- labels exactly match ticket vocabulary;
- no scheduler generation/deadline details leak into the UI type.

## Step 2: extend slot presentation data

File: `crates/lisa-plugin/src/ui.rs`

- Add optional assignment status to `SlotInfo`.
- Update every existing direct `SlotInfo` literal with `None`.
- Keep ticket, slot number, and transitioning semantics unchanged.

Verification:

- UI tests compile;
- existing fixtures preserve their prior behavior;
- absence of explicit state continues to render legacy status.

## Step 3: render assignment indicators

File: `crates/lisa-plugin/src/ui.rs`

- Make the Threads-section renderer crate-visible for the integrated test.
- In active rows, retain awaiting-human as the highest-priority status.
- Otherwise render explicit assignment label and color.
- Fall back to `Running` when assignment status is absent.
- Expand STATUS column width for `assigned-pending-ack`.
- Adjust every row shape and separator consistently.

Verification:

- existing Running and Awaiting tests pass;
- a focused UI state with explicit assignment status renders its exact label;
- no table field runs into TIME at the longest label.

## Step 4: map scheduler truth to UI truth

File: `crates/lisa-plugin/src/lib.rs`

- Add an exhaustive conversion from `SeatAssignmentState` to
  `ui::SeatAssignmentStatus`.
- In `to_ui_state`, look up assignment state by each slot's physical pane ID.
- Populate `SlotInfo.assignment_status` only from that lookup.
- Do not use route, terminal text, pane name, ticket reservation, thread state,
  transition state, or activity history as an ownership source.

Verification:

- direct projection test can observe each scheduler variant;
- a slot missing from `seat_assignments` yields `None`;
- compiler rejects unmapped future scheduler variants.

## Step 5: add the scheduler-driven dashboard snapshot

File: `crates/lisa-plugin/src/lib.rs`

- Reuse `pane_name_schedule_state` to schedule a ready ticket onto a resident
  Codex session.
- Render the Threads dashboard section after scheduling and capture pending.
- Inject an exact ticket/generation acknowledgment and capture owned.
- Build a second identical state, arm its deadline through actual prompt
  delivery, expire it deterministically, and capture recovering.
- Strip only ANSI SGR sequences and trim row ends.
- Assert one exact multiline snapshot containing all three labeled rows.

Verification:

- changing only terminal contents cannot affect the test;
- the pending and owned rows come from the same scheduler state instance;
- recovery comes from the real timeout transition;
- expected rows contain `codex`, ticket ID, slot, phase, and exact indicator;
- failure messages show the complete actual snapshot.

## Step 6: focused regression tests

Commands:

```text
cargo test -p lisa-plugin dashboard_snapshot_shows_recycled_codex_handoff_states
cargo test -p lisa-plugin render_threads
cargo test -p lisa-plugin recycled_codex_ownership
cargo test -p lisa-plugin bounded_ack_wait
```

Criteria:

- new snapshot passes;
- legacy UI table cases pass;
- acknowledgment exact-once behavior remains unchanged;
- bounded timeout/recovery behavior remains unchanged.

## Step 7: formatting and static verification

Commands:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
git diff --check -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
```

If formatting check fails due to this ticket, run `cargo fmt --all`, inspect the
scope, and repeat the check. Formatting changes must not spill into unrelated
files.

Criteria:

- Rust formatting is clean;
- plugin code has no warnings;
- ticket source diff has no whitespace errors.

## Step 8: broad verification

Commands:

```text
cargo test --workspace
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Criteria:

- every workspace unit/integration/doc test passes;
- the production WASM target builds;
- dashboard-only visibility additions do not alter scheduler behavior.

If broad verification reveals an unrelated pre-existing failure, record the
exact command and evidence in `progress.md` and `review.md`; do not modify
unrelated files.

## Step 9: review source ownership

- Inspect `git diff` only for the two ticket-owned Rust files.
- Inspect `git status --short` and distinguish pre-existing unrelated changes.
- Confirm no ordinary-index entries belong to this ticket.
- Confirm the ticket frontmatter was not edited.
- Confirm work artifacts exist through `progress.md` before committing source.

Criteria:

- source change matches the Structure artifact;
- no terminal-content parsing was added;
- no unrelated worktree content is included.

## Step 10: commit the implementation unit

Use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-033-02-01 \
  --message "feat: surface Codex handoff state in dashboard" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

If the installed Lisa executable lacks the command, use the repository-built
CLI after verifying its help output. Do not use ordinary Git staging or commit.

Criteria:

- commit contains exactly the two source paths;
- ordinary index remains untouched;
- both source paths are clean after the transaction;
- unrelated worktree changes remain present and uncommitted.

## Step 11: write Review

File: `docs/active/work/T-033-02-01/review.md`

- summarize behavior and exact files;
- record the isolated source commit;
- evaluate the acceptance criterion explicitly;
- list focused, workspace, lint, formatting, and WASM results;
- document open concerns and limitations;
- confirm no ticket phase/status edit;
- confirm ticket-owned source cleanliness.

## Test matrix

| Case | Source transition | Expected dashboard status |
|---|---|---|
| Recycled Codex scheduled | scheduler inserts `AssignedPendingAck` | `assigned-pending-ack` |
| Exact ack | scheduler replaces state with `Owned` | `owned` |
| Ack deadline expires | scheduler begins `Recovering` | `recovering` |
| Awaiting-human | existing scheduler awaiting set | `Awaiting` |
| No assignment entry | legacy UI projection | `Running` |
| Parked thread | existing thread state | `Parked` |
| Idle slot | no active/parked thread | `Idle` |

## Risks and mitigations

### Risk: status text truncation or overlap

Mitigation: expand the fixed-width STATUS column to the longest required label
and snapshot the complete row.

### Risk: UI accidentally infers handoff state

Mitigation: make the UI field typed and populate it at exactly one production
site using `seat_assignment(pane_id)`.

### Risk: acknowledgment and timeout test flakiness

Mitigation: use existing direct acknowledgment injection and injected absolute
deadline evaluation; do not sleep.

### Risk: breaking existing dashboard semantics

Mitigation: preserve Awaiting precedence and the None-to-Running fallback, then
run all existing render-thread tests.

### Risk: unrelated dirty worktree contamination

Mitigation: inspect exact diffs and commit with two explicit `--include` paths
through Lisa's isolated transaction.

## Completion checklist

- [ ] UI semantic enum exists.
- [ ] Slot projection carries explicit assignment status.
- [ ] Required labels render in STATUS.
- [ ] Longest label fits the table.
- [ ] Snapshot crosses scheduler and renderer boundary.
- [ ] Pending snapshot is scheduler-sourced.
- [ ] Owned snapshot follows exact acknowledgment.
- [ ] Recovering snapshot follows deterministic timeout.
- [ ] Existing UI and scheduler regressions pass.
- [ ] Workspace tests pass.
- [ ] WASM build passes.
- [ ] Clippy and formatting pass.
- [ ] Source commit uses `lisa commit-ticket` with exact paths.
- [ ] Review handoff records tests and open concerns.
