# Plan — T-048-02-01 status-and-unblock-ux

## Execution rules

Work continuously through Implement and Review.

Do not edit the active ticket's phase or status.

Keep RDSPI artifacts in the current attempt-private work directory.

Use `apply_patch` for source edits.

Do not use ordinary `git add`, `git commit`, or a broad index operation.

Commit each meaningful source unit with `lisa commit-ticket`, the exact ticket
ID, and exact repository-relative include paths.

Preserve all unrelated modified/untracked paths already in the worktree.

Record execution results and deviations in `progress.md` before Review.

## Baseline

Focused baseline commands completed before source edits.

### Core disposition baseline

Command:

```text
cargo test -p lisa-core disposition::tests --no-fail-fast
```

Result:

- 14 passed;
- 0 failed.

### CLI status baseline

Command:

```text
cargo test -p lisa-cli status::tests --no-fail-fast
```

Result:

- 11 matching binary unit tests passed;
- 0 failed.

The matching set includes status and preownership status tests because Cargo's
name filter matches both modules.

### CLI help baseline

Command:

```text
cargo test -p lisa-cli --test help_surface --no-fail-fast
```

Result:

- 6 passed;
- 0 failed.

### Plugin UI baseline

Command:

```text
cargo test -p lisa-plugin ui::tests --no-fail-fast
```

Result:

- 47 passed;
- 0 failed.

## Step 1: add parked-remedy discovery

Files:

- create `crates/lisa-core/src/parking.rs`;
- modify `crates/lisa-core/src/lib.rs`.

Actions:

1. Define owned `ParkedRemedy` with ticket ID, typed owner, ask, and check.
2. Implement `collect_parked_remedies` over borrowed tickets and resolved work
   directory.
3. Filter to durable blocked tickets.
4. Parse only canonical `review-disposition.json`.
5. Retain only valid Block variants.
6. Preserve ask/check string contents.
7. Sort by ticket ID.
8. Export the module from core.
9. Add unit fixtures for structured, legacy, invalid, pass, absent, open, and
   ordering cases.

Independent verification:

```text
cargo test -p lisa-core parking::tests --no-fail-fast
cargo test -p lisa-core disposition::tests --no-fail-fast
cargo check -p lisa-core
```

Acceptance evidence:

- one shared meaning is available to status and dashboard;
- legacy fallback remains operator-owned;
- invalid documents do not fabricate human/check payloads.

## Step 2: lead status with waiting lines

File:

- modify `crates/lisa-cli/src/status.rs`.

Actions:

1. Import the collector and typed owner.
2. Add a helper that turns operator/world remedies into final lines.
3. Preserve the complete ask string after ticket identity.
4. Add only the self-check suffix for world owners.
5. Suppress agent-owned remedies from this human section.
6. Call discovery after ticket scan.
7. Print the section before the DAG header.
8. Keep no-remedy output unchanged.
9. Add helper-level string tests if useful; the binary fixture in Step 7 owns
   end-to-end stdout assertions.

Independent verification:

```text
cargo test -p lisa-cli status::tests --no-fail-fast
```

Acceptance evidence:

- heading precedes DAG detail;
- line contains only identity, ask, and world promise as applicable.

## Step 3: add dashboard waiting projection and renderer

Files:

- modify `crates/lisa-plugin/src/ui.rs`;
- modify `crates/lisa-plugin/src/lib.rs`.

Actions:

1. Define `WaitingItem` in UI.
2. Add `waiting_items` to `PluginState` and Default.
3. Update any complete `PluginState` literals that do not use struct update.
4. Add `render_waiting_on_you` using existing line-vector conventions.
5. Render operator asks without labels or extra detail.
6. Render world asks with the exact Lisa self-check promise.
7. Call the renderer first in Operations content.
8. In `to_ui_state`, collect durable parked remedies from DAG/work data.
9. Map only operator/world items and preserve sorted order.
10. Leave thread, timer, scheduler, provenance, and other view paths unchanged.
11. Add direct UI tests for content, omission, and ordering.
12. Add a projection/file-boundary test if required to prove canonical work is
    actually consumed.

Independent verification:

```text
cargo test -p lisa-plugin ui::tests --no-fail-fast
cargo test -p lisa-plugin waiting --no-fail-fast
cargo check -p lisa-plugin
```

Acceptance evidence:

- dashboard first content section matches status semantics;
- durable parked tickets do not require retained `ParkedThread` state.

## Step 4: format and verify source unit 1

Actions:

1. Run Rust formatting on the workspace.
2. Inspect only the five owned paths.
3. Confirm no unrelated diff was introduced.
4. Run focused tests from Steps 1–3 again.
5. Run `git diff --check` on exact paths.

Commands:

```text
cargo fmt --all
cargo test -p lisa-core parking::tests --no-fail-fast
cargo test -p lisa-cli status::tests --no-fail-fast
cargo test -p lisa-plugin ui::tests --no-fail-fast
git diff --check -- crates/lisa-core/src/lib.rs \
  crates/lisa-core/src/parking.rs \
  crates/lisa-cli/src/status.rs \
  crates/lisa-plugin/src/lib.rs \
  crates/lisa-plugin/src/ui.rs
```

If workspace formatting touches an unrelated concurrent path, do not include
that path; inspect and report rather than overwrite user work.

## Step 5: commit source unit 1

Use installed Lisa, not ordinary Git commit.

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-048-02-01 \
  --message "T-048-02-01: show parked asks in status and dashboard" \
  --include crates/lisa-core/src/lib.rs \
  --include crates/lisa-core/src/parking.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

Post-commit checks:

```text
git show --stat --oneline -1
git show --check -1
git status --short -- <the five exact paths>
git diff --cached --name-only
```

Record the commit ID in `progress.md`.

## Step 6: implement disposable check runner

Files:

- create `crates/lisa-cli/src/unblock.rs`;
- modify `crates/lisa-cli/Cargo.toml`;
- modify `Cargo.lock` only as generated by Cargo dependency resolution.

Actions:

1. Promote `tempfile` to a runtime dependency.
2. Add Unix `libc` dependency for process-group kill.
3. Define `CheckResult` and `UnblockOutcome`.
4. Implement Git-visible current-file enumeration with NUL parsing.
5. Implement safe regular-file snapshot copying.
6. Implement non-Git small-tree fallback with heavy/control directory skips.
7. Ensure symlinks cannot route a check back into the live project.
8. Make snapshot entries read-only.
9. Fingerprint sorted paths, kinds, permissions, and contents.
10. Create anonymous stdout/stderr capture files.
11. Launch `/bin/sh -c` in a new process group with snapshot cwd.
12. Give temporary variables a disposable scratch directory while leaving HOME
    unchanged.
13. Poll to an injected deadline.
14. Kill the full group on Unix at timeout; wait/reap the wrapper.
15. Compare fingerprints after exit.
16. Read only bounded output.
17. Reduce output to one sanitized observation.
18. Return Passed, Failed, TimedOut, or ChangedFiles.
19. Restore snapshot owner-write permissions only so temporary cleanup works.
20. Add short unit tests for every outcome and string.

Independent verification:

```text
cargo test -p lisa-cli unblock::tests --no-fail-fast
cargo check -p lisa-cli
```

Safety evidence:

- live fixture bytes are unchanged;
- new live sentinel is absent;
- write attempt is declined even if it exits zero;
- timeout returns near the injected deadline;
- output is bounded to one plain line.

## Step 7: implement project unblock flow

File:

- continue `crates/lisa-cli/src/unblock.rs`.

Actions:

1. Resolve configured ticket/work directories.
2. Scan tickets and find exact ID.
3. Require blocked status before parsing or spawning.
4. Require a valid canonical Block disposition.
5. Run the optional check with the production five-second timeout.
6. Convert nonzero/check observation to exact decline copy.
7. Convert timeout to exact plain deadline copy.
8. Convert detected mutation to exact plain change-attempt copy.
9. Do not write status on any decline.
10. On Passed or absent check, call `update_ticket_status(Open)`.
11. Return exact success copy.
12. Unit-test status preservation and updates where direct testing is simpler.

Independent verification:

```text
cargo test -p lisa-cli unblock::tests --no-fail-fast
```

Acceptance evidence:

- verification precedes reopening;
- no-check is a valid explicit reopen;
- unblock never performs the remedy itself.

## Step 8: wire command and help surface

Files:

- modify `crates/lisa-cli/src/main.rs`;
- modify `crates/lisa-cli/tests/help_surface.rs`.

Actions:

1. Declare `mod unblock`.
2. Add visible `Unblock` command after Status.
3. Add positional ticket ID and default project path.
4. Use purpose-first, jargon-free command description.
5. Add example text.
6. Dispatch Reopened to stdout/success.
7. Dispatch Declined directly to stderr/nonzero without `Error:`.
8. Preserve generic error handling for operational failures.
9. Update exact top-level snapshot.
10. Update all-command count/inventory.
11. Add exact unblock help snapshot.
12. Update visible ordering assertions.
13. Ensure plumbing grouping remains unchanged.

Independent verification:

```text
cargo test -p lisa-cli --test help_surface --no-fail-fast
```

Acceptance evidence:

- operator can discover exact requested syntax;
- every new help string is pinned and checked by jargon tests.

## Step 9: add black-box parked UX fixtures

File:

- create `crates/lisa-cli/tests/parked_ux.rs`.

Actions:

1. Add minimal project/ticket/disposition fixture builders.
2. Invoke real binary for status and unblock.
3. Assert Waiting on you is the first output section.
4. Assert operator ask verbatim and no raw reason/owner/schema vocabulary.
5. Assert world self-check promise.
6. Assert failing check exact nonzero stderr and blocked status retention.
7. Assert passing check exact stdout and open status.
8. Rescan/rebuild DAG and assert ready eligibility after pass.
9. Assert no-check follows the same reopen/DAG path.
10. Assert relative write attempt has no live effect, emits plain decline, and
    remains blocked.
11. Assert stderr has no generic prefix or stack-like text for declines.

Independent verification:

```text
cargo test -p lisa-cli --test parked_ux --no-fail-fast
```

## Step 10: resolve focused regressions

Run together:

```text
cargo test -p lisa-core parking::tests --no-fail-fast
cargo test -p lisa-cli unblock::tests --no-fail-fast
cargo test -p lisa-cli status::tests --no-fail-fast
cargo test -p lisa-cli --test parked_ux --no-fail-fast
cargo test -p lisa-cli --test help_surface --no-fail-fast
cargo test -p lisa-plugin ui::tests --no-fail-fast
```

For each failure:

- determine whether implementation or expectation violates the artifact
  decisions;
- patch only ticket-owned paths;
- document material deviation before proceeding;
- rerun the smallest failing group, then the combined focused set.

## Step 11: format and verify source unit 2

Commands:

```text
cargo fmt --all
cargo test -p lisa-cli unblock::tests --no-fail-fast
cargo test -p lisa-cli --test parked_ux --no-fail-fast
cargo test -p lisa-cli --test help_surface --no-fail-fast
cargo check -p lisa-cli
git diff --check -- Cargo.lock \
  crates/lisa-cli/Cargo.toml \
  crates/lisa-cli/src/main.rs \
  crates/lisa-cli/src/unblock.rs \
  crates/lisa-cli/tests/help_surface.rs \
  crates/lisa-cli/tests/parked_ux.rs
```

Inspect final strings explicitly with `rg` and test assertions.

## Step 12: commit source unit 2

Use exact includes, omitting `Cargo.lock` if Cargo did not change it.

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-048-02-01 \
  --message "T-048-02-01: verify and reopen parked tickets" \
  --include Cargo.lock \
  --include crates/lisa-cli/Cargo.toml \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/unblock.rs \
  --include crates/lisa-cli/tests/help_surface.rs \
  --include crates/lisa-cli/tests/parked_ux.rs
```

Post-commit checks mirror Step 5.

Record commit ID and exact contents in `progress.md`.

## Step 13: workspace verification

Run in increasing scope:

```text
cargo check --workspace
cargo test -p lisa-core --no-fail-fast
cargo test -p lisa-cli --no-fail-fast
cargo test -p lisa-plugin --no-fail-fast
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
just check
```

`just check` includes the WASM plugin check and workspace tests.

If an environment-gated real-Zellij test remains ignored, record that fact and
why this ticket does not require it.

If concurrent work changes unrelated files during the suite, report the
specific warning/failure and do not absorb it into this ticket.

## Step 14: final diff and ownership audit

Commands:

```text
git log --oneline -5
git show --stat --oneline <commit-1>
git show --stat --oneline <commit-2>
git show --check <commit-1>
git show --check <commit-2>
git status --short
git diff --cached --name-only
```

Requirements:

- all ticket-owned source paths clean;
- ordinary index empty or exactly unchanged from baseline;
- no attempt artifact committed as a source unit;
- unrelated modified/untracked paths preserved;
- ticket frontmatter not manually changed beyond Lisa's phase management;
- no shared `docs/active/work/T-048-02-01/` writes.

## Step 15: write Review artifacts

Write `review.md` in the attempt-private work directory.

Include:

- behavior summary;
- files and commits;
- durable authority assessment;
- check isolation/timeout assessment;
- exact user strings;
- focused and workspace test results;
- gaps and platform limitations;
- ownership audit;
- human review focus.

Then write exactly:

```json
{"disposition":"pass","reason":null}
```

to `review-disposition.json` only if all acceptance and ownership checks pass.

Otherwise write a valid block shape with a non-empty actionable reason.

After both Review artifacts exist, stop on this ticket and wait for Lisa's
completion transaction.
