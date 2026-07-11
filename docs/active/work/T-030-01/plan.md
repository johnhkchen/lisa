# Plan: ownership-aware init planning

## Implementation strategy

Implement the safety contract from the inside out: capture exact historical
evidence, add one classifier for static templates, migrate every whole-file
template call site, then exercise planning and real execution. Preserve existing
structured merge code and prove it remains non-destructive.

## Step 1: capture historical template evidence

1. Read tagged release content from local Git history.
2. Identify distinct outgoing bytes rather than duplicating identical content
   for every tag.
3. Add the pre-v0.2.3 workflow snapshot under CLI data.
4. Add v0.3 shell template literals for the stop, clear, and heartbeat scripts.
5. Add the v0.3 one-line Lisa gitignore literal.
6. Define an explicit legacy slice for every static target, using an empty slice
   when no distinct historical bytes exist.
7. Add template-level assertions that historical entries differ from current
   content and retain expected identifying lines.

Verification:

- `cargo test -p lisa-cli templates::tests`
- Compare snapshot hashes/bytes against the relevant Git tag where practical.

Atomic outcome: the distributed binary has self-contained, reviewable evidence
for safe upgrades before planner behavior changes.

## Step 2: add the static-template ownership classifier

1. Add private `plan_owned_template` in `init.rs` near `InitAction`.
2. Return `CreateFile` for an absent path.
3. Read an existing path as UTF-8 text.
4. Return current no-op skip on exact current equality.
5. Return `UpdateFile` only on exact membership in known-prior templates.
6. Return a specific preservation skip for unknown readable content.
7. Return a distinct preservation skip for read failures/non-UTF-8 content.
8. Keep all content writes owned by returned actions; the helper performs no
   mutation itself.

Verification:

- Compile the CLI test target.
- Focused action tests cover all five classifier outcomes.

Atomic outcome: one function embodies the replacement authorization rule.

## Step 3: migrate all static plain-text targets

1. Replace the workflow's inline exists/read/update branch with the classifier.
2. Extend hook tuple definitions to include the matching legacy slice.
3. Route all five hook files through the classifier.
4. Replace `.lisa/.gitignore`'s inline branch with the classifier.
5. Confirm no remaining `UpdateFile` fallback exists for a static plain-text
   template.
6. Leave the future T-030-02 call site obvious for append-only replacement.

Verification:

- Search planner source for all `templates::` static targets.
- Inspect every `UpdateFile` branch and classify it as proven template or
  format-aware merge.

Atomic outcome: the complete plain-text init action set shares the safe default.

## Step 4: revise conflicting tests

1. Change the arbitrary `# Old RDSPI content` test to expect preservation.
2. Add a separate known-prior workflow upgrade test using the historical fixture.
3. Change arbitrary hook `old content` tests to expect preservation.
4. Use exact v0.3 hook content in tests that are specifically about legitimate
   stale-template upgrades.
5. Keep current-template skip tests unchanged except for stronger reason checks.
6. Ensure test names distinguish `known_prior` from `unknown_modified`.

Verification:

- `cargo test -p lisa-cli init::tests::test_plan_init`

Atomic outcome: tests stop equating “different” with “Lisa-owned.”

## Step 5: add field-regression coverage

1. Construct a workflow fixture containing recognizable project-only Story Layer
   and read-the-story rules.
2. Save its exact bytes.
3. Call `plan_init_actions` and assert the workflow action is a preservation
   `Skip` with no disk changes.
4. Call real `run_init` and compare exact post-run bytes.
5. Construct locally modified active hook and notification sample fixtures.
6. Assert planning preserves them.
7. Run real init and assert exact content after the run.
8. Include `.lisa/.gitignore` custom secret rule preservation if it does not
   duplicate T-030-02's append-only behavior.

Verification:

- Run the named regression tests individually with `--nocapture` if output helps
  diagnose action plans.

Atomic outcome: both planning and execution permanently protect the reported
class of project additions.

## Step 6: cover legitimate upgrades and failures

1. Write exact known-prior workflow content and assert planned current update.
2. Write exact v0.3 stop/clear/heartbeat hooks and assert current updates.
3. Write the exact old Lisa gitignore and assert a current update.
4. Run real init for representative prior content and verify current disk bytes.
5. Write non-UTF-8 workflow and hook bytes.
6. Assert unreadable preservation reasons and absence of update actions.
7. Retain malformed JSON tests to prove structured targets skip instead of
   falling back to replacement.
8. Add or strengthen `.lisa.toml` preservation assertions for unrelated content.

Verification:

- Focused ownership and malformed-input test filters.

Atomic outcome: both safe-upgrade and safe-failure sides of the contract are
tested.

## Step 7: enforce complete policy coverage

1. Add a policy matrix comment/test listing all twenty fresh-init actions.
2. Map each path family to create-if-absent, preserve-if-present,
   replace-if-proven-pristine, or format-aware merge.
3. Use representative assertions for directories/context files.
4. Use per-target assertions for all seven static template files.
5. Use preserving-update/no-op/error assertions for TOML and both JSON files.
6. Confirm the fresh-init count remains compatible.

Verification:

- Review `plan_init_actions` top to bottom against the ticket acceptance list.
- Run all `init::tests`.

Atomic outcome: future additions to the planner have a visible policy precedent.

## Step 8: format and run focused verification

1. Run `cargo fmt --all -- --check`.
2. If formatting fails only on this change, run `cargo fmt --all` and recheck.
3. Run `cargo test -p lisa-cli init::tests`.
4. Run `cargo test -p lisa-cli templates::tests`.
5. Run `cargo clippy -p lisa-cli --all-targets -- -D warnings` if the local
   environment supports it.
6. Record exact commands and results in `progress.md`.

Success criteria:

- No destructive static-template expectation remains.
- All focused tests pass.
- No warning is introduced by legacy constants or helper signatures.

## Step 9: run full verification

1. Run `cargo test --workspace`.
2. Run `just check` if the wasm target/toolchain is installed.
3. Distinguish code failures from environment/toolchain gaps.
4. Inspect `git diff --check`.
5. Inspect scoped diff and repository status to avoid capturing unrelated user
   changes.

Success criteria:

- Full CLI/workspace suite passes.
- Fresh init remains compatible.
- Any unavailable wasm verification is documented, not hidden.

## Step 10: commit and hand off

1. Commit the pre-implementation RDSPI artifacts as one scoped unit.
2. Commit implementation and tests as a separate scoped unit.
3. Update `progress.md` after every meaningful implementation unit and document
   any deviation before following it.
4. Write `review.md` from the final diff and test evidence.
5. Include files changed, ownership behavior, test coverage, known gaps, and open
   concerns.
6. Do not edit ticket `phase` or `status` frontmatter.
7. Stop after `review.md` is written.

## Expected implementation checks

- Customized workflow is byte-for-byte stable after planning and real init.
- Customized hook scripts and samples are byte-for-byte stable after real init.
- Unknown plain text yields a specific safety skip.
- Non-UTF-8/read-failed content yields a specific unreadable safety skip.
- Known prior templates update to current content.
- Current templates remain no-ops.
- Structured JSON merge preserves unrelated hooks and keys.
- TOML update preserves unrelated project settings.
- Fresh init creates the same expected action set.
- Full CLI and workspace tests pass.

## Risk controls

- Use exact bytes, never normalized or fuzzy comparisons.
- Keep the classifier pure with respect to the filesystem: read and plan only.
- Avoid any overwrite fallback on an error arm.
- Scope commits by explicit paths because the working tree contains unrelated
  user/Lisa runtime changes.
- Do not add ticket frontmatter to implementation commits unless it becomes
  tracked independently by Lisa.
- Treat T-030-02 as a follow-up boundary, not a reason to leave gitignore unsafe
  in the current ticket.

## Planned deviations policy

If Git history shows additional distinct installed template generations, add
them to the registry and record the expansion in `progress.md`. If exact legacy
workflow content is too large for a maintainable Rust literal, keep it as an
included data fixture as specified in Structure. If permission-based unreadable
tests are unreliable under the test runner, use invalid UTF-8 as the stable
`read_to_string` failure fixture.
