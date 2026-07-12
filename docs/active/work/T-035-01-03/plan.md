# T-035-01-03 Plan — gate Owned on observed start

## Step 1 — extend assignment vocabulary

Modify `crates/lisa-plugin/src/lib.rs`:

- add `SeatAssignmentState::Starting { generation }`;
- document that it is a reserved fresh process awaiting exact start observation;
- keep `seat_is_owned` limited to `Owned`;
- keep E-033 active generations limited to ack/recovery states.

Modify `crates/lisa-plugin/src/ui.rs`:

- add `SeatAssignmentStatus::Starting`;
- render label `starting`;
- use yellow pending color.

Extend `to_ui_state` mapping in `lib.rs`.

Verification:

- `cargo check -p lisa-plugin` catches exhaustive-match omissions.

## Step 2 — implement exact start admission

Add `State::acknowledge_process_start` near assignment helpers.

Checks in order:

- seat is currently `Starting`;
- starting generation equals candidate attempt ID;
- pane has an assigned ticket and lease;
- candidate equals the slot lease and ticket;
- candidate is current in `current_leases`.

Only then insert `Owned` and return true.

Verification:

- method is inert for malformed context, stale authority, and duplicate delivery;
- no E-033 acknowledgment path calls or accepts it.

## Step 3 — consume `.started` signals

Add `State::check_process_start_signals` beside heartbeat consumption.

- scan `signal_dir`;
- recognize only `pane-<u32>.started`;
- parse `AttemptLease`;
- remove each recognized file before admission;
- pass valid candidates to the admission method.

Call it near the top of `poll_tick`, before later health/timeout decisions.

Verification:

- absent signal directory is harmless;
- malformed/stale files are consumed without ownership;
- exact signal promotes once.

## Step 4 — gate fresh dispatch

Within `schedule_ready_tickets`, track whether the selected route submits a fresh
provider process launch.

- unused physical seat: fresh;
- cross-provider recycle: fresh after exit;
- `FreshExec` reset: fresh;
- `ClearHandshake` reuse: not fresh.

Choose assignment state with fresh start taking precedence:

- fresh -> `Starting { attempt_id }`;
- reused Codex -> `AssignedPendingAck`;
- same-process accepted reuse -> `Owned`.

Keep prompt-ack deadline arming unchanged for the E-033 states.

Verification:

- immediately after a truly fresh schedule, the seat is reserved and not owned;
- same-process Claude reuse remains owned;
- recycled Codex reuse remains pending ack.

## Step 5 — add native acceptance coverage

Create a native test in `lib.rs` that executes the scheduler seam:

1. build one ready ticket and one empty compatible pane;
2. point `signal_dir` at the test directory;
3. call `schedule_ready_tickets`;
4. read the scheduler-minted current lease;
5. assert `Starting` with that generation;
6. assert `seat_is_owned` is false;
7. snapshot the dashboard row and require `starting`;
8. write the exact lease to `pane-<id>.started`;
9. call `check_process_start_signals`;
10. assert the file is gone and the state is `Owned`;
11. require the dashboard row to show `owned`.

Add stale/malformed signal checks before the matching signal if fixture clarity permits.

## Step 6 — focused regression verification

Run formatting:

```text
cargo fmt --all -- --check
```

Run the new test by exact or distinctive name.

Run E-033 coverage:

- recycled Codex ownership exact-ack test;
- dropped-ack bounded recovery test;
- reused Claude ownership test;
- consecutive reused panes regression if runtime is reasonable.

Run E-034 coverage:

- split-brain fencing test;
- relevant stale lease/heartbeat tests.

Then run:

```text
cargo test -p lisa-plugin
cargo test --workspace
```

If unrelated working-tree edits affect tests, document evidence without modifying them.

## Step 7 — isolated source commit

Build the current Lisa CLI if the installed binary lacks `commit-ticket`, then invoke the
repository-built `lisa commit-ticket` implementation.

Use exactly:

```text
--ticket-id T-035-01-03
--include crates/lisa-plugin/src/lib.rs
--include crates/lisa-plugin/src/ui.rs
```

Use one meaningful source unit because the state model, consumer, UI label, and native
test form one indivisible contract. Do not include ticket or attempt artifacts.

Verify afterward:

- source paths are clean;
- ordinary staged entries are unchanged;
- no ticket-owned source remains modified or untracked.

## Step 8 — progress and review handoff

Write `progress.md` in the private attempt work directory with:

- completed implementation units;
- commit identifier;
- tests and outcomes;
- deviations from this plan.

Write `review.md` in the same directory with:

- source file inventory;
- behavior summary;
- test coverage;
- open concerns, especially that missing-start timeout belongs to T-035-01-04.

Do not update ticket phase/status and do not publish artifacts to the shared work path.
After `review.md`, remain on T-035-01-03 and stop for Lisa's completion transaction.
