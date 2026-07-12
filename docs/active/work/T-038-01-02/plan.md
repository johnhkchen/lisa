# Plan — T-038-01-02 startup-launch-timing-baseline

## Step 1 — confirm scope and state

- Re-read the ticket, parent story, epic context, and RDSPI workflow.
- Confirm the story permits work-artifact writes only.
- Capture `git status --short` before implementation.
- Treat existing active-ticket modifications as scheduler-owned and leave them untouched.
- Confirm there is no need for a source commit.

Verification:

- No planned path lies outside `.lisa/attempts/T-038-01-02/1/work/`.
- No ordinary-index operation is used.

## Step 2 — build the measured release candidate

Run from the repository root:

```bash
just build-cli
```

This builds release WASM first, touches its output to refresh embedding, then builds the release CLI.

Verification:

- Command exits zero.
- `target/wasm32-wasip1/release/lisa.wasm` exists.
- `target/release/lisa` exists and is executable.
- `target/release/lisa --version` exits zero.

## Step 3 — capture identity and environment

Capture:

```bash
git rev-parse HEAD
target/release/lisa --version
sw_vers
```

Also retain the working directory and date/time zone in the progress artifact.

Verification:

- Git identity is a full commit hash.
- CLI output identifies the RC version.
- Host identity is sufficient to qualify same-host reproducibility.

## Step 4 — run benchmark batch 1

Invoke the inline Ruby monotonic benchmark driver with:

- command vector `target/release/lisa`, `loop`, `--dry-run`, `--path`, `.`;
- 3 warmups;
- 30 recorded samples;
- stdout/stderr redirected to `File::NULL`;
- millisecond output;
- raw list plus min/median/mean/max.

Verification:

- All three warmups succeed.
- Exactly 30 raw sample values are printed.
- All 30 measured children succeed.
- Median calculation sorts a copy and averages the two central values.

## Step 5 — run independent benchmark batch 2

Invoke the exact same driver again as a new process, without rebuilding and without changing the checkout.

Verification:

- Exactly 30 new successful samples are produced.
- Batch 2 is not a re-summary of batch 1.
- Exact command, working directory, and input remain unchanged.

## Step 6 — evaluate tolerance

Calculate:

```text
delta_pct = abs(batch2_median - batch1_median) / batch1_median * 100
```

Compare to the declared ±20% same-host median tolerance.

Verification:

- Record both medians.
- Record absolute millisecond difference.
- Record relative percentage difference.
- Mark PASS only if the difference is no greater than 20%.
- If it fails, inspect drift and do not hide the failed pair.

## Step 7 — record path classifications

In `progress.md`, create distinct subsections for:

1. release CLI dry-run startup;
2. real-Zellij local stub launch;
3. native Codex startup;
4. native Claude startup.

For the CLI path, record the numeric result and exact commands.

For the real-Zellij path, state that the committed behavioral harness is deterministic but does not timestamp the focused boundary. Explain why total suite time and wait bounds are invalid substitutes.

For Codex and Claude, use the exact phrase “not deterministically measurable here” and identify external client/runtime/service variability. Note that no metered run was performed.

Verification:

- No path is omitted.
- No provider field observation is mislabeled as deterministic.
- No upper bound is mislabeled as observed latency.

## Step 8 — record implementation progress

Create `progress.md` after measurement with:

- completed steps;
- raw evidence;
- summary statistics;
- exact commands;
- tolerance result;
- deviations from the plan;
- source/commit status.

Verification:

- The baseline can be reproduced without reading code.
- Raw evidence permits independent median recalculation.
- Units and warm/cold semantics are explicit.

## Step 9 — repository integrity check

Run:

```bash
git status --short
git diff --cached --name-only
```

Compare with initial state.

Verification:

- No ticket-owned repository source file is staged, modified, or untracked.
- Existing scheduler-owned ticket changes remain untouched.
- Attempt-private artifacts are the only implementation writes.
- No `lisa commit-ticket` call is needed because no source unit exists.

## Step 10 — review

Write `review.md` containing:

- outcome and acceptance status;
- baseline and rerun medians;
- tolerance result;
- exact reproduction commands;
- measured-boundary definition;
- per-path deterministic notes;
- files created/modified/deleted;
- coverage and gaps;
- open concerns and limitations;
- repository integrity.

Verification:

- Review is self-contained.
- A release-report author can reuse the number with caveats intact.
- No claim extends from CLI dry-run startup to Zellij or provider startup.

## Step 11 — stop on the ticket

After `review.md` exists:

- do not edit ticket phase/status;
- do not publish artifacts manually;
- do not create a completion commit;
- do not start another ticket;
- remain available for Lisa's completion transaction.

## Test strategy summary

This measurement-only ticket adds no unit or integration tests because it changes no behavior. Verification is empirical and structural:

- fresh release build succeeds;
- two repeated benchmark distributions succeed;
- median tolerance passes;
- raw sample counts and calculations are inspectable;
- path caveats prevent overclaiming;
- Git integrity proves no product behavior was altered to obtain a number.

## Planned commit units

None. The parent story is artifact-only, and RDSPI artifacts are published by Lisa. If any product or harness modification becomes necessary, stop and document it as a deviation rather than silently broadening scope.
