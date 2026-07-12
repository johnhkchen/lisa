# Plan: T-038-02-01 cargo-fmt-clean

## Objective

- Verify and preserve canonical formatting across the full Cargo workspace.
- Produce evidence for the exact acceptance criterion.
- Avoid source churn because the researched baseline is already clean.
- Finish without ticket-owned working-tree residue.

## Step 1: Establish the implementation baseline

- Run `git status --short` from the repository root.
- Record every visible changed or untracked path.
- Separate Lisa workflow-managed paths from Rust source paths.
- Confirm whether any `*.rs` file is modified, staged, or untracked.
- Do not alter workflow-managed paths.

### Verification

- Status should contain no ticket-owned Rust source path.
- Known Lisa ticket/provenance modifications are acceptable.
- Any unexpected source path pauses formatting writes until ownership is known.

### Commit boundary

- This is read-only inspection.
- No commit is created.

## Step 2: Execute the acceptance command

- Run `cargo fmt --all -- --check` from the repository root.
- Capture the process exit status.
- Capture any formatter diagnostic or diff output.
- Treat status 0 as the canonical-format predicate.

### Verification

- The command must exit 0.
- No formatter diff should be emitted.
- All workspace members must remain selected through `--all`.
- The command spelling must match the ticket criterion exactly.

### Commit boundary

- Check mode is read-only.
- No commit is created when it succeeds.

## Step 3: Apply the failure contingency only if needed

- This step is conditional on Step 2 failing.
- Review formatter output to identify the affected paths.
- Recheck Git ownership for every affected path.
- Run `cargo fmt --all` only when paths are safe to rewrite.
- Inspect the resulting Git diff in full.
- Confirm the diff changes layout and whitespace only.
- Reject behavioral, generated, or unrelated changes.

### Verification

- Rerun `cargo fmt --all -- --check`.
- Require exit status 0.
- Use Git diff inspection to confirm formatting-only content.
- Confirm no non-Rust file was unexpectedly rewritten.

### Commit boundary

- Gather exact repository-relative paths for rewritten Rust files.
- Run `lisa commit-ticket --ticket-id T-038-02-01`.
- Use a message describing canonical workspace formatting.
- Add one `--include` per exact owned path.
- Do not use a directory path, wildcard, or broad include.
- Do not use ordinary `git add` or `git commit`.

## Step 4: Record implementation progress

- Create the attempt-private `progress.md` artifact.
- Mark baseline inspection complete.
- Mark the exact formatter verification complete.
- State whether the contingency was required.
- State whether any source commit was created.
- Explain a no-op outcome if the tree remained canonical.
- List any deviations from this plan.

### Verification

- Progress accurately matches observed commands and statuses.
- It does not claim a source change that did not occur.
- It names any commit only if Lisa successfully created it.

### Commit boundary

- The progress artifact is workflow-managed.
- It is not committed through the implementation transaction.

## Step 5: Inspect final repository state

- Run `git status --short` again.
- Confirm no ticket-owned Rust source remains modified.
- Confirm no ticket-owned Rust source remains staged.
- Confirm no ticket-owned Rust source remains untracked.
- Preserve expected Lisa metadata and ticket-state changes.
- Optionally inspect `git diff --check` for whitespace errors.

### Verification

- The source tree must be clean with respect to this ticket.
- Any source commit from Step 3 must have consumed all owned changes.
- The ordinary index must not contain ticket-owned entries.

### Commit boundary

- Read-only inspection creates no commit.

## Step 6: Perform review verification

- Rerun `cargo fmt --all -- --check` if any time or state change occurred.
- Record the final exit status.
- Compare the implementation outcome against the single acceptance criterion.
- Assess whether further runtime tests have relevant coverage value.
- Identify open concerns or limitations.

### Verification

- Final formatter status must remain 0.
- The final source diff must be empty or formatting-only.
- For the selected no-op design, the expected source diff is empty.

### Commit boundary

- Verification creates no commit.

## Step 7: Produce the Review artifact

- Create `.lisa/attempts/T-038-02-01/1/work/review.md`.
- Summarize created, modified, and deleted source files.
- Explain that the inherited tree was already canonical if still true.
- List exact validation commands and outcomes.
- Describe test coverage and why it is proportionate.
- Surface any toolchain caveat.
- State whether human attention is required.

### Verification

- Review is sufficient to assess the ticket without reading every artifact.
- Review clearly distinguishes source files from private workflow artifacts.
- Review does not update ticket frontmatter.

### Commit boundary

- Lisa owns final artifact publication and completion commit.
- Stop after writing Review.
- Do not start another ticket.

## Test strategy

- Primary test: `cargo fmt --all -- --check`.
- Scope: all Rust targets in all workspace members.
- Expected result: status 0 with no diff output.
- Secondary inspection: `git status --short`.
- Expected result: no ticket-owned source residue.
- Optional inspection: `git diff --check`.
- Expected result: no whitespace errors in visible diffs.
- Unit tests: not applicable because no logic changes.
- Integration tests: not applicable because no behavior changes.
- Build tests: not required for a formatting-only or no-op result.

## Risk controls

- Do not run write-mode rustfmt unless check mode fails.
- Do not touch paths already modified by another owner.
- Do not include Lisa metadata in a source transaction.
- Do not create a ceremonial empty commit.
- Do not interpret an empty diff as missing work when acceptance already passes.
- Revalidate after any observed working-tree change.

## Expected result

- The exact formatter check exits 0.
- No Rust source edit is necessary.
- No ticket-owned source commit is necessary.
- The implementation artifact documents the verified no-op.
- The review artifact hands the result back to Lisa.
