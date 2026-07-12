# Plan: T-039-02-02

## Objective

Create a typed signal-ingestion boundary, route all eight existing signal loops
through it, preserve characterized behavior, commit the coherent source unit via
Lisa's isolated transaction, and verify all repository gates.

## Step 1: record the baseline

1. Inspect the ordinary working tree without modifying unrelated files.
2. Note Lisa-owned ticket and provenance changes already present.
3. Run the focused T-039-02-01 characterization suite before editing.
4. Record the test count and result in `progress.md`.
5. Do not stage any file.

Verification:

- `cargo test -p lisa-plugin signal_consumer_characterization`
- The suite passes before the refactor.

## Step 2: create the typed signal module

1. Add `crates/lisa-plugin/src/signal.rs`.
2. Define the closed `SignalRequest` enum.
3. Define `IdleTarget` for pane versus legacy ticket identity.
4. Define `SignalRecord` with distinct lease, provider-payload, presence, idle,
   stopped, cleared, and error variants.
5. Implement the single `ingest` operation.
6. Preserve silent behavior for unreadable/missing directories and entry errors.
7. Preserve directory order by collecting directly from `read_dir` iteration.

Verification:

- The module compiles when declared from `lib.rs`.
- No scheduler state type is passed into the ingestion function.
- The module does not import `codex_ack` or adapter behavior.

## Step 3: implement exact recognition and deletion policies

1. Move strict pane filename parsing into `signal.rs`.
2. For heartbeat, started, shell-ready, ack, awaiting, and error, parse a valid
   pane ID before deletion.
3. For the three lease families, read and deserialize before returning a record,
   but delete every strictly recognized file even when parsing fails.
4. For ack, preserve the raw UTF-8 payload and delete even when reading fails.
5. For awaiting and error, never read the body.
6. For idle, recognize the broad `.idle` suffix and delete before pane parsing.
7. Preserve the legacy ticket target only for non-pane idle names.
8. For transitions, recognize both stopped and cleared during one scan and
   delete before pane-number parsing.
9. Leave unrelated suffixes untouched.

Verification:

- Unit tests exercise strict invalid-name retention.
- Unit tests exercise malformed lease deletion.
- Unit tests exercise malformed broad idle/transition deletion.
- Unit tests exercise legacy idle and both transition variants.

## Step 4: route lease consumers through the boundary

1. Add the private module declaration and imports to `lib.rs`.
2. Rewrite `check_heartbeat_signals` to iterate heartbeat records.
3. Keep the existing slot ticket, slot lease, and current lease checks unchanged.
4. Keep activity and gate-clearing effects unchanged.
5. Rewrite `check_process_start_signals` to dispatch typed lease records.
6. Rewrite `check_shell_ready_signals` to dispatch typed lease records.
7. Capture the current time at the same downstream dispatch point.

Verification:

- Focused heartbeat, process-start, and shell-ready characterization tests pass.
- Existing startup and lease-fencing tests pass.

## Step 5: route provider and presence consumers

1. Rewrite `check_codex_ack_signals` to iterate raw provider payload records.
2. Keep exact tagged acknowledgement parsing downstream.
3. Keep activity refresh and logging conditional on successful admission.
4. Rewrite `check_awaiting_signals` to iterate presence records.
5. Preserve its no-activity-refresh behavior.
6. Rewrite `check_error_signals` to iterate presence records.
7. Preserve recovery-first and running-thread authority branches.

Verification:

- Ack characterization covers stale and exact provider tags.
- Awaiting characterization covers arbitrary body and no activity refresh.
- Error characterization covers reclaim effects.

## Step 6: route idle and transition consumers

1. Keep `idle_alerts.clear()` before ingestion.
2. Rewrite the top of `check_idle_signals` to match `IdleTarget`.
3. Preserve transition-state checking for pane targets.
4. Preserve pane activity refresh and notification pane identity.
5. Preserve legacy ticket targets without pane activity.
6. Leave the downstream phase/artifact match unchanged.
7. Rewrite transition scanning to match stopped and cleared records.
8. Preserve activity refresh before each handler call.
9. Ensure transition still performs a single directory scan.

Verification:

- Legacy-name characterization matrix passes unchanged.
- Delete-timing matrix passes unchanged.
- Idle phase behavior tests pass.
- Stopped and cleared state-machine tests pass.

## Step 7: remove obsolete parsing code

1. Remove `pane_id_from_signal_filename` from `lib.rs`.
2. Move its direct unit coverage into `signal.rs` as appropriate.
3. Search for all remaining direct `read_dir(&self.signal_dir)` calls in the
   eight consumer region.
4. Confirm each consumer invokes `signal::ingest`.
5. Confirm no consumer directly reads or removes its signal file.
6. Confirm the poll call order is textually unchanged.

Verification:

- `rg` shows eight ingestion calls in the eight loops.
- `rg` shows the old helper only inside the new boundary if retained privately.
- `git diff` shows no change to the characterization module.

## Step 8: format and run focused tests

1. Run `cargo fmt --all`.
2. Run the new signal-module tests.
3. Run the unchanged characterization suite.
4. If a failure occurs, compare it to the exact researched deletion, filename,
   payload, lease-admission, or poll-order behavior.
5. Fix the boundary without weakening or editing characterization assertions.

Verification:

- All new boundary tests pass.
- All 11 characterization tests pass unchanged.
- Formatting is stable.

## Step 9: run full repository gates

1. Run `cargo test --workspace`.
2. Run Clippy for all workspace targets with warnings denied, or the repository
   `just lint` task if it is the canonical equivalent.
3. Run the repository formatting check.
4. Run the repository quick check if needed to include WASM checking.
5. Run `git diff --check`.
6. Record exact commands and results in `progress.md`.

Verification:

- Workspace tests are green.
- Clippy is green with warnings denied.
- WASM/plugin check is green.
- Formatting and whitespace checks are green.

## Step 10: inspect the source diff

1. Review `crates/lisa-plugin/src/signal.rs` in full.
2. Review only the ticket-owned `lib.rs` diff.
3. Confirm typed variants keep payload families explicit.
4. Confirm current-attempt admission remains in scheduler methods.
5. Confirm no provider capability was broadened.
6. Confirm no legacy filename route was broadened.
7. Confirm recognized malformed records remain one-shot according to their
   original recognition policy.
8. Confirm no unrelated user change is included.

Verification:

- `git diff -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/signal.rs`
- `git diff -- crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`
  is empty.

## Step 11: commit the coherent source unit

1. Update `progress.md` before committing with completed work and any deviation.
2. Invoke Lisa's isolated transaction with the ticket ID, one meaningful message,
   and exactly the two ticket-owned source paths.
3. Do not use `git add`, `git commit`, or the ordinary index.

Planned command shape:

```text
lisa commit-ticket \
  --ticket-id T-039-02-02 \
  --message "T-039-02-02: add typed signal ingestion boundary" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/signal.rs
```

Verification:

- Lisa reports a successful isolated commit.
- Both exact source paths are clean afterward.
- The ordinary index contains no ticket-owned staged entries.
- Lisa-owned ticket/provenance mutations remain preserved.

## Step 12: post-commit verification and review

1. Re-run the most relevant focused characterization gate after commit if the
   transaction changes the worktree state.
2. Inspect the resulting commit and exact path list.
3. Write `review.md` in the attempt-private work directory.
4. Summarize architecture, behavior preservation, tests, and commit details.
5. List open concerns or explicitly state that none are known.
6. Stop on this ticket after the review artifact exists.

Verification:

- `review.md` is complete and self-contained.
- No ticket phase or status was manually edited.
- No ticket-owned source file remains modified, staged, or untracked.
- No next ticket is started.

## Atomicity rationale

The new module and eight consumer rewrites form one meaningful compilation unit.
Committing the module without consumers would add an unused boundary; committing
consumers without the module would not compile. Unit tests live with the module
and are part of the same owned source file. One exact two-path Lisa transaction
is therefore the smallest durable ticket-owned source unit.

## Deviation policy

- If a characterized behavior was misunderstood, update `progress.md` before
  revising implementation.
- Do not edit the characterization suite to make a refactor pass.
- If additional source files become necessary, document why before editing and
  add only exact ticket-owned paths to the Lisa transaction.
- If unrelated failures occur, distinguish them from ticket regressions in the
  review and avoid modifying unrelated code without scope authority.
