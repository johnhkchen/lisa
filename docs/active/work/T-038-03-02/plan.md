# Plan: implement and prove the four cleanups

## Preconditions

1. Confirm the assignment ticket remains `T-038-03-02`, generation 1.
2. Confirm no ticket-owned source path is already modified or staged.
3. Treat Lisa-managed ticket and provenance changes as external workflow state.
4. Use only the private attempt directory for phase artifacts.
5. Keep C-05 through C-14 unchanged throughout implementation.

## Step 1: add the pure signal filename parser

File:

`crates/lisa-plugin/src/lib.rs`

Actions:

1. Select a module-level location available to scheduler methods and tests.
2. Add `pane_id_from_signal_filename(&OsStr, &str) -> Option<u32>`.
3. Implement UTF-8 conversion, exact prefix strip, exact suffix strip, and
   `u32` parsing as one `Option` chain.
4. Replace the repeated parse chain in heartbeat scanning.
5. Replace the repeated parse chain in process-start scanning.
6. Replace the repeated parse chain in shell-readiness scanning.
7. Replace the repeated parse chain in Codex acknowledgement scanning.
8. Replace the repeated parse chain in awaiting-human scanning.
9. Replace pane-id parsing in the stopped transition branch.
10. Preserve stopped-file removal before acting on the parse result.
11. Replace pane-id parsing in the cleared transition branch.
12. Preserve cleared-file removal before acting on the parse result.
13. Replace the repeated parse chain in error scanning.
14. Do not touch idle legacy-name parsing.
15. Do not factor directory scanning or payload handling.

## Step 2: add focused parser tests

File:

`crates/lisa-plugin/src/lib.rs`

Actions:

1. Add a table-driven UTF-8 grammar test in the existing test module.
2. Include valid zero, ordinary, maximum, and leading-zero ids.
3. Include wrong prefix and wrong suffix cases.
4. Include a suffix that appears before trailing text.
5. Include empty, non-numeric, negative, and whitespace ids.
6. Include an overflowing decimal id.
7. Include a suffix-mismatch case proving caller suffix specificity.
8. Add a Unix-only non-UTF-8 `OsString` rejection test.
9. Keep expected values independent of consumer logic.

Verification:

1. Run `cargo fmt --all -- --check` after formatting as needed.
2. Run the two focused parser tests.
3. Run existing signal-related tests if a useful name filter is available.
4. Inspect the diff to verify no scanner behavior moved.

Commit:

`lisa commit-ticket --ticket-id T-038-03-02 --message "Centralize pane signal filename parsing" --include crates/lisa-plugin/src/lib.rs`

Success criteria:

- Parser tests pass.
- Existing focused plugin tests pass.
- The diff changes only parser expressions and focused tests.
- `lib.rs` is clean after the Lisa transaction.

## Step 3: add native adapter defaults

File:

`crates/lisa-plugin/src/adapter.rs`

Actions:

1. Give `AgentAdapter::reset_strategy` the current `ClearHandshake` body.
2. Clarify in its comment that non-native integrations override the default.
3. Give `AgentAdapter::follow_up` the current typed prompt body.
4. Clarify in its comment that other delivery mechanisms override the default.
5. Remove Claude's duplicate reset method.
6. Remove Claude's duplicate follow-up method.
7. Remove Codex's duplicate reset method.
8. Remove Codex's duplicate follow-up method.
9. Leave all provider-specific methods unchanged.
10. Leave all existing provider assertions independent and unchanged.

Verification:

1. Format and check the Rust source.
2. Run the adapter test module.
3. Confirm Claude reset and follow-up tests pass.
4. Confirm Codex reset and follow-up tests pass.
5. Confirm resolver and mixed-route tests still exercise trait objects.
6. Inspect the diff for only trait bodies and duplicate-method deletion.

Commit:

`lisa commit-ticket --ticket-id T-038-03-02 --message "Default shared native adapter policies" --include crates/lisa-plugin/src/adapter.rs`

Success criteria:

- All adapter tests pass.
- Both providers inherit the exact prior results.
- Override enum variants remain available.
- `adapter.rs` is clean after the Lisa transaction.

## Step 4: extract deterministic event counting

File:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Actions:

1. Add `event_count` before the comparison predicates.
2. Accept an event kind as its only argument.
3. Print zero if the event log is absent.
4. Return immediately after printing the missing-file fallback.
5. Otherwise run the existing `awk` expression unchanged.
6. Replace duplicated setup in `event_count_is` with command substitution.
7. Replace duplicated setup in `event_count_at_least` likewise.
8. Preserve equality and arithmetic comparisons exactly.
9. Change no harness call site.
10. Do not touch the live-provider harness.

Verification:

1. Run `bash -n` on the fixture.
2. Run the ignored real-Zellij Rust integration test explicitly.
3. Require successful test status.
4. Require the harness output to contain its PASS receipt via the Rust test.
5. Inspect the diff to verify it is one local extraction.

Commit:

`lisa commit-ticket --ticket-id T-038-03-02 --message "Centralize deterministic harness event counts" --include crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Success criteria:

- Shell syntax passes.
- Real-Zellij integration passes.
- Exact and lower-bound assertions continue to execute.
- The fixture path is clean after the Lisa transaction.

## Step 5: integrated verification

Run in this order:

1. `cargo fmt --all -- --check`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings`

If a command fails:

1. Determine whether failure is caused by ticket-owned changes.
2. Record the failure and diagnosis in `progress.md`.
3. Fix only ticket-owned behavior within the authorized candidate boundary.
4. Re-run the focused proof for the affected unit.
5. Commit the fix through `lisa commit-ticket` with its exact source path.
6. Re-run all integrated verification commands.

Do not use an ordinary Git index or ordinary commit for any fix.

## Step 6: scope and ownership audit

1. Inspect commits created for this ticket.
2. Confirm source changes are limited to the three planned paths.
3. Confirm C-05 whole scanner loops remain separate.
4. Confirm C-06 failure/reclaim paths remain separate.
5. Confirm C-07 timeout/liveness loops remain separate.
6. Confirm C-08 atomic publication remains separate.
7. Confirm C-09 cross-harness helpers remain duplicated.
8. Confirm C-10 historical artifacts are unchanged.
9. Confirm C-11 hook schemas remain explicit.
10. Confirm C-12 broad scheduler fixtures remain unchanged.
11. Confirm C-13 provider assignment logic remains separate.
12. Confirm C-14 provider assertions remain independent.
13. Run `git diff --cached --name-only` to inspect the ordinary index.
14. Run `git status --short` to inspect the worktree.
15. Ensure all three ticket-owned source files are clean.
16. Leave workflow-managed changes untouched.

## Step 7: progress artifact

Write `.lisa/attempts/T-038-03-02/1/work/progress.md` with:

- completion state of C-01 through C-04;
- exact source files changed;
- focused proof commands and results;
- exact Lisa ticket commits and commit identifiers;
- integrated formatting, workspace test, and clippy results;
- deviations from this plan and rationale;
- ownership/status audit;
- explicit list of C-05 through C-14 left in place.

Writing `progress.md` completes Implement; continue immediately to Review.

## Step 8: review artifact

Write `.lisa/attempts/T-038-03-02/1/work/review.md` with:

- concise change summary by candidate and file;
- behavior-preservation analysis;
- focused and integrated test coverage;
- deferred/larger repetition named for the report;
- test gaps, if any;
- open concerns and known limitations;
- final ticket-owned file cleanliness;
- any critical issue needing human attention.

After `review.md` exists, remain on `T-038-03-02` and stop. Do not update the
ticket, publish artifacts, release the seat, or begin another ticket.
