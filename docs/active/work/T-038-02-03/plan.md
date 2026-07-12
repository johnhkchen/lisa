# T-038-02-03 Plan: Execute and Record Final Green Gates

## Goal

Prove on the current tightened tree that native workspace tests and the WASM
compilation boundary pass, record reproducible evidence, and finish with no
ticket-owned source residue.

## Step 1: Capture the implementation baseline

### Actions

- Record the current `HEAD` commit.
- Inspect short working-tree status.
- Inspect the ordinary index separately.
- Identify Lisa-owned ticket/provenance changes without modifying them.
- Confirm the current Rust toolchain and installed `wasm32-wasip1` target if a
  target error makes that relevant.

### Verification

- `HEAD` is at or after the completed Clippy predecessor.
- No ticket-owned source file is already modified, staged, or untracked.
- The ordinary index has no entry attributable to this ticket.

### Commit boundary

- No commit; this is read-only observation.

## Step 2: Reconfirm formatting cleanliness

### Command

```text
cargo fmt --all -- --check
```

### Actions

- Run from the repository root.
- Capture complete output and exit status.
- Do not invoke rewriting format mode.

### Verification

- Exit status is zero.
- No source file is rewritten.
- Any output is evaluated for formatting drift.

### Failure response

- Identify reported files and ownership.
- Do not rewrite concurrent or unrelated work.
- If drift is ticket-owned and blocks acceptance, document the deviation before
  considering a minimal correction.

### Commit boundary

- Expected: no source unit and no commit.

## Step 3: Reconfirm native warning cleanliness

### Command

```text
cargo clippy --workspace -- -D warnings
```

### Actions

- Run after formatting succeeds.
- Capture output and status.
- Count warning/error diagnostics, distinguishing Cargo coordination messages from
  compiler diagnostics.

### Verification

- Exit status is zero.
- Warning count is zero because warnings are denied.
- All workspace members are selected on the host target.

### Failure response

- Record package, source path, and lint name.
- Determine whether a predecessor/concurrent change altered the baseline.
- Make no suppression or speculative refactor.

### Commit boundary

- Expected: no source unit and no commit.

## Step 4: Reconfirm WASM warning cleanliness

### Command

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

### Actions

- Run after native Clippy succeeds.
- Capture complete output and status.
- Confirm the named target is used.

### Verification

- Exit status is zero.
- Warning count is zero.
- No target installation or target-specific compilation error appears.

### Failure response

- Separate environment/toolchain setup issues from source defects.
- Preserve the exact target-specific diagnostic.

### Commit boundary

- Expected: no source unit and no commit.

## Step 5: Execute the canonical acceptance gate

### Command

```text
just check
```

### Expanded commands

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

### Actions

- Run once, sequentially through the Just recipe.
- Capture the recipe's emitted commands and Cargo/test output.
- Capture the composite exit status.
- Extract each test binary's result summary from the output.
- Sum passed, failed, and ignored values for a workspace-level record.

### Verification

- The WASM check exits zero.
- Workspace tests execute only after the WASM check succeeds.
- Every executed test binary reports `ok`.
- Aggregate failed count is zero.
- The composite `just check` status is zero.
- Any ignored test is identified and assessed rather than hidden.

### Failure response

- If WASM check fails, diagnose it before tests can run.
- If a test fails, record target and test name, then use a focused rerun only for
  diagnosis.
- After any correction, rerun the entire `just check` gate.

### Commit boundary

- Expected: no source unit and no commit.
- If a real defect requires a correction, commit the smallest meaningful source
  unit through `lisa commit-ticket` with exact includes before final verification.

## Step 6: Build the release WASM artifact

### Command

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

### Actions

- Run after `just check` passes.
- Capture output, target, profile, and status.
- Treat output under `target/` as generated build state.

### Verification

- Exit status is zero.
- Cargo completes the `release` profile.
- No warning or error diagnostic is emitted.
- The command exercises compilation/code generation/linking for the shipped WASM
  target.

### Failure response

- Record whether the failure is compilation, code generation, linking, or local
  environment related.
- Do not claim acceptance based only on the earlier `cargo check` if release build
  was attempted and failed.

### Commit boundary

- Generated target output is never committed.

## Step 7: Reconcile source and transaction state

### Actions

- Inspect `git status --short`.
- Inspect `git diff --cached --name-status`.
- Inspect tracked source diffs outside known Lisa-managed paths.
- Inspect untracked files while excluding ignored build state and the private
  attempt artifacts from source ownership assessment.

### Verification

- No ticket-owned source file is staged.
- No ticket-owned source file remains modified.
- No ticket-owned source file remains untracked.
- Existing Lisa-owned provenance/ticket/work publication entries are not mistaken
  for ticket source residue.
- No ordinary-index command was used.

### Commit boundary

- If no source changed, do not invoke an empty isolated commit.
- If source changed, ensure each meaningful unit was already committed using:

```text
lisa commit-ticket --ticket-id T-038-02-03 --message <message> --include <exact-path>...
```

## Step 8: Write implementation progress

### File

```text
.lisa/attempts/T-038-02-03/1/work/progress.md
```

### Content

- Baseline commit and state.
- Exact command sequence.
- Complete short outputs and compact long-output summaries.
- Exit statuses.
- Test counts and ignored-test explanation.
- WASM check and release-build outcome.
- Deviations from this plan.
- Source-change and isolated-transaction result.
- Final repository hygiene.

### Verification

- Every acceptance statement can be traced to an actual command in this attempt.
- No result is copied forward as if freshly observed.

## Step 9: Perform final self-review

### Actions

- Re-read the ticket acceptance criterion.
- Compare actual results against every planned verification criterion.
- Confirm all six phase artifacts exist in the private work directory.
- Confirm no agent-authored artifact was written to shared active work.
- Reconfirm ticket-owned source cleanliness.

### File

```text
.lisa/attempts/T-038-02-03/1/work/review.md
```

### Review content

- Overall outcome.
- File change summary.
- Acceptance mapping.
- Test coverage and counts.
- WASM target/profile coverage.
- Open concerns, limitations, and critical issues.
- Transaction hygiene and handoff readiness.

## Planned deviations policy

- Any change to command scope or order will be recorded in `progress.md` before
  relying on its result.
- Diagnostic reruns supplement but never replace the full final gate.
- Source fixes require explicit rationale and exact isolated transaction paths.
- Environmental blockers are recorded distinctly from product defects.

## Completion criteria

- Formatting check passes.
- Native workspace Clippy passes with warnings denied.
- WASM plugin Clippy passes with warnings denied.
- `just check` passes.
- Its `cargo test --workspace` component reports zero failed tests.
- Its `cargo check -p lisa-plugin --target wasm32-wasip1` component passes.
- Release WASM build passes.
- Results are recorded in `progress.md` and summarized in `review.md`.
- No ticket-owned source residue exists.
- Ticket phase/status remain unedited by the agent.
- Work stops after Review pending Lisa's completion commit.
