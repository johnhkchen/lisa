# T-038-02-02 Plan: Clippy Zero Warnings

## Goal

Demonstrate that the current checkout produces zero Clippy warnings on the native
workspace and the `wasm32-wasip1` plugin target, record exact command evidence,
and preserve behavior by avoiding source changes unless a final lint requires
one.

## Step 1: Confirm implementation starting state

1. Run `git status --short`.
2. Distinguish Lisa-controlled workflow changes from ticket-owned source files.
3. Confirm that no Rust, Cargo, CI, or developer-command file is already modified
   by this ticket.
4. Record the current commit identifier.

Verification:

- Only expected Lisa workflow files may be dirty at the start.
- Any unrelated user change must remain untouched.
- No ordinary Git staging or commit command is used.

## Step 2: Run the final native warning-strict lint gate

Run:

```text
cargo clippy --workspace -- -D warnings
```

Capture:

- the exact command;
- complete meaningful stdout/stderr;
- exit status;
- whether any warning or error diagnostic appears;
- the workspace packages covered.

Verification:

- Exit status is 0.
- Warning count is zero.
- No diagnostic is suppressed or ignored.

If the command fails with a diagnostic:

1. Identify the lint and exact source path.
2. Inspect the surrounding code and associated tests.
3. Apply the smallest semantics-preserving fix.
4. Document this deviation before proceeding.
5. Rerun focused Clippy for the affected package.
6. Commit the exact changed source file through `lisa commit-ticket`.
7. Rerun the full native gate.

## Step 3: Run the final WASM warning-strict lint gate

Run:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Capture:

- the exact command;
- complete meaningful stdout/stderr;
- exit status;
- whether any target-specific warning or error diagnostic appears.

Verification:

- Exit status is 0.
- Warning count is zero.
- The explicit target is `wasm32-wasip1`.
- The selected package is `lisa-plugin`.

If the command fails with a diagnostic:

1. Determine whether the lint is WASM-specific or shared with host compilation.
2. Inspect target-gated code and behavior constraints.
3. Apply the smallest semantics-preserving fix.
4. Document the plan deviation.
5. Rerun focused native and WASM Clippy as appropriate.
6. Commit only the exact changed source path through `lisa commit-ticket`.
7. Rerun both full primary gates.

## Step 4: Verify formatting without rewriting files

Run:

```text
cargo fmt --all -- --check
```

Verification:

- Exit status is 0.
- No file is rewritten by the check command.
- Any pre-existing unrelated formatting issue is reported rather than silently
  modified.

## Step 5: Run the native behavior regression suite

Run:

```text
cargo test --workspace
```

Capture:

- exit status;
- package/test target summaries;
- test pass/fail/ignored counts;
- any warnings emitted during compilation.

Verification:

- Exit status is 0.
- All non-ignored tests pass.
- No compiler warnings appear.
- Any intentionally ignored environment-dependent test remains identified as
  ignored rather than misreported as run.

## Step 6: Verify ordinary WASM compilation

Run:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Verification:

- Exit status is 0.
- No warning appears.
- The plugin remains compilable outside Clippy's driver.

## Step 7: Evaluate source and commit requirements

1. Run `git status --short` after all gates.
2. Compare with the implementation starting state.
3. Confirm no ticket-owned Rust, Cargo, workflow, or developer-command file was
   created or modified.
4. Confirm the ordinary Git index has not been used for ticket work.

Expected outcome:

- No source changes exist.
- No `lisa commit-ticket` invocation is needed because there is no meaningful
  source unit to commit.
- Attempt artifacts remain private for Lisa publication.

Conditional outcome if a lint remediation was required:

- Every changed source unit has already been committed with an exact include.
- No ticket-owned source file remains modified, untracked, or staged.
- The progress artifact lists each isolated transaction command and result.

## Step 8: Write `progress.md`

Record implementation in execution order:

1. Starting repository state.
2. Native Clippy command, output, status, and warning count.
3. WASM Clippy command, output, status, and warning count.
4. Formatting check result.
5. Workspace test result.
6. WASM check result.
7. Source-change and transaction result.
8. Final repository state.
9. Any deviations from this plan.

Verification:

- Both commands explicitly required by the acceptance criterion are visible.
- Output is recorded, not merely summarized as “passed.”
- The absence of source changes is explicit and justified by the zero-warning
  result.

## Step 9: Perform self-review

Review:

- the ticket and acceptance criterion;
- every created phase artifact;
- final command evidence;
- `git diff` for ticket-owned source paths;
- final status for modified, staged, and untracked source files;
- possible gaps caused by target selection or caching.

Verification:

- The native command covers the workspace.
- The WASM command covers the actual plugin deliverable.
- Warning denial proves zero warnings.
- Tests and ordinary WASM checking support the no-behavior-change claim.
- No unrelated file was claimed or modified.

## Step 10: Write `review.md` and stop

The review artifact will include:

- file/change summary;
- acceptance-criterion mapping;
- exact primary commands and outcomes;
- supporting verification results;
- test coverage assessment;
- transaction hygiene;
- open concerns and limitations;
- final readiness assessment.

After `review.md` exists:

- Do not update ticket phase/status fields.
- Do not publish artifacts directly to `docs/active/work/T-038-02-02/`.
- Do not start another ticket.
- Remain on this ticket for Lisa to verify the lease, publish admitted artifacts,
  create the completion commit, and release the seat.

## Atomicity model

The expected plan has no source commit because validation succeeds without code
changes. If remediation becomes necessary, one meaningful lint-owned source unit
is one isolated transaction. Artifacts are not combined into an ordinary Git
commit by this agent; Lisa owns their admission and final completion transaction.

## Completion checklist

- [ ] Native workspace Clippy exits 0 with warnings denied.
- [ ] WASM plugin Clippy exits 0 with warnings denied.
- [ ] Exact primary command output is recorded.
- [ ] Formatting check passes.
- [ ] Workspace tests pass.
- [ ] WASM `cargo check` passes without warnings.
- [ ] No behavior-changing or speculative source edit exists.
- [ ] Every conditional source edit, if any, is isolated-transaction committed.
- [ ] No ticket-owned source remains staged, modified, or untracked.
- [ ] `progress.md` is complete.
- [ ] `review.md` is complete.
- [ ] Agent stops on the current ticket after Review.
