# Progress: ownership-aware init planning

## Status

Implementation and verification are complete. Review handoff is the only
remaining artifact.

## Completed: RDSPI preparation

- Read `AGENTS.md`, the canonical `CLAUDE.md`, the RDSPI workflow, ticket, and
  parent story.
- Located the ticket at its actual descriptive filename after the short path in
  the prompt did not exist.
- Confirmed ticket frontmatter begins at `phase: research`.
- Wrote `research.md`, `design.md`, `structure.md`, and `plan.md` continuously.
- Committed those four artifacts as scoped commit `0e67320`.
- Did not edit ticket phase or status fields.

## Completed: historical ownership evidence

- Inspected local release tags from v0.2.0 through v0.4.0-rc.5.
- Identified two byte-distinct workflow generations.
- Added `crates/lisa-cli/data/legacy/rdspi-workflow-v0.2.md`.
- Verified the added snapshot's Git blob hash exactly matches the v0.2.0 tagged
  workflow blob: `cbe4974f4acbc1348be06219928b67ad22c56cd2`.
- Added exact v0.3 stop, clear, and heartbeat hook literals.
- Added the v0.3 Lisa gitignore literal (`signals/\n`).
- Added explicit empty historical slices for idle and notification templates,
  whose relevant v0.3 bytes equal current content.

## Completed: ownership-aware planner

- Added private `plan_owned_template` beside `InitAction`.
- The helper produces `CreateFile` for an absent path.
- It produces a no-op skip for exact current bytes.
- It produces `UpdateFile` only for exact known-prior bytes.
- It produces `preserved: content is not a known Lisa template` for unknown
  readable content.
- It produces `preserved: existing file is unreadable` when text reading fails.
- Migrated workflow planning to the helper.
- Migrated all five hook templates to the helper with per-target historical
  slices.
- Migrated `.lisa/.gitignore` to the helper pending T-030-02's append-only merge.
- Left create-only context files and structured TOML/JSON merges intact.

## Completed: regression tests

- Replaced tests that treated arbitrary `old content` as safely replaceable.
- Added an all-current static-template no-op test covering workflow, five hook
  files, and Lisa gitignore.
- Added known-prior update coverage for workflow, stop, clear, heartbeat, and
  Lisa gitignore.
- Added unknown-content preservation coverage for every hook template.
- Added a committed-additions-style workflow fixture with Story Layer text.
- The same real-run fixture covers a customized stop hook, notification sample,
  and a Lisa gitignore containing `hooks/ntfy-topic`.
- The fixture asserts exact bytes after planning and after real `run_init`.
- Added deterministic non-UTF-8 read-failure coverage for workflow and hook
  content, asserting no update action.
- Added a real-run known-prior stop hook upgrade to current bytes.
- Retained fresh initialization, context preservation, TOML merge, JSON merge,
  malformed JSON, and init/validate round-trip tests.

## Focused verification completed

- `cargo fmt --all -- --check` initially reported only formatting needed in the
  new helper/test code.
- Ran `cargo fmt --all` as the planned mechanical correction.
- `cargo test -p lisa-cli init::tests --no-fail-fast` passed:
  65 passed, 0 failed, 183 filtered out after the final malformed-structure
  regression was added.
- `git diff --check` passed.
- Scoped diff inspection confirms production changes are limited to
  `crates/lisa-cli/src/init.rs`, `crates/lisa-cli/src/templates.rs`, and the new
  legacy workflow data file.

## Deviations from plan

### No production policy enum

As designed, the implementation did not add a policy enum used only for tests.
The complete policy remains visible through dedicated planner branches and the
shared static-template helper.

### Historical shell templates remain inline

The short legacy shell scripts are raw constants in `templates.rs`; only the
larger workflow is a separate data file. This follows Structure and keeps the
runtime registry dependency-free.

### Field fixture is constructed in test code

The vend-style additions are deterministic test strings rather than separate
fixture files. This makes byte ownership obvious at the assertion site and
avoids another fixture loader. The historical workflow itself remains a real
tag-exact file because its complete bytes are runtime evidence.

### No permission-based unreadable test

The failure test uses invalid UTF-8 because tests may run with privileges that
make permission denial unreliable. It exercises the exact `read_to_string`
error branch used for unreadable or non-text existing files.

## Working-tree safety

- The repository began with unrelated modified hook scripts, ticket files, and
  untracked Lisa/runtime/story files.
- Those paths have not been staged for this ticket.
- `cargo fmt` changed only Rust source in the scoped implementation; the unrelated
  status entries were already present.
- Implementation commits will explicitly name only T-030-01 production/test and
  artifact paths.

## Full verification completed

- `cargo test --workspace` passed before the final test-only addition:
  - `lisa-cli`: 247 passed;
  - `lisa-core`: 145 passed;
  - `lisa-plugin`: 234 passed;
  - doc tests: 0 failures.
- `just check` passed, including:
  - `cargo check -p lisa-plugin --target wasm32-wasip1`;
  - the complete workspace test suite with the same 626 passing tests.
- The final malformed-structure regression passed individually and the complete
  init subset then passed 65/65.
- `cargo fmt --all` completed and subsequent source is formatted.
- `git diff --check` passed.

## Lint result

- `cargo clippy --workspace --all-targets -- -D warnings` does not pass on the
  baseline because twelve old `unnecessary_to_owned` findings remain in
  `crates/lisa-core/src/dag.rs` tests.
- A narrower `cargo clippy -p lisa-cli --all-targets -- -D warnings` reaches the
  CLI and reports one old `needless_borrows_for_generic_args` finding at the
  pre-existing `&format!` in `init.rs`'s config-upsert test.
- `git blame` attributes that line to commit `55e85c14`, predating this ticket;
  the core findings date to `ba92ee43`.
- No lint finding points to the new ownership helper, historical constants, or
  regressions. These unrelated baseline cleanup items were not changed.

## Commits completed

- `0e67320 docs: design ownership-aware init planning`
- `d2d06b3 fix: preserve unowned init templates`

The final malformed-input regression and closing artifacts will be committed as
the verification/handoff unit.

## Remaining

- Write `review.md` with final results and open concerns.
- Commit the final regression and closing artifacts without staging unrelated
  working-tree changes.
