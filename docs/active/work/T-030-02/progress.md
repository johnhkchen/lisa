# Progress: append-only ignore and mutation report

## Current state

- Research complete.
- Design complete.
- Structure complete.
- Plan complete.
- Implementation complete.
- Review verification complete.

## Completed

- Mapped the existing init planner, ownership helper, execution loop, tests, and
  README contract.
- Selected an immutable-prefix append-only merge for `.lisa/.gitignore`.
- Selected explicit `NoOp` and `SafetySkip` action categories.
- Selected a writer-injected execution path and successful-write mutation record.
- Defined the combined vend workflow plus `hooks/ntfy-topic` Git regression.
- Split ordinary no-ops from safety skips in `InitAction` and its display.
- Added `plan_append_only_gitignore`, preserving existing bytes as an exact prefix
  and appending only missing trimmed rules in template order.
- Removed the now-unused legacy whole-file gitignore ownership constant.
- Added writer-injected `run_init_with_writer` while retaining the public stdout
  wrapper.
- Added a successful-write record and final `Files changed` report with created
  versus updated labels.
- Limited executable-bit changes to active hook files created or updated by the
  current run; skipped project hooks are untouched.
- Revised the vend fixture to keep workflow/hook customizations, append required
  ignore rules, and verify `hooks/ntfy-topic` with `git check-ignore`.
- Added edge coverage for missing trailing newline, harmless spacing,
  idempotence, invalid UTF-8, all four output categories, exact before/after
  write-set reporting, and skipped-hook mode preservation.
- Documented the ownership, append-only, mutation-report, and pre-commit
  inspection contracts in README.

## Verification so far

- `cargo check -p lisa-cli`: passed after production implementation.
- Focused init tests: 68 passed, 0 failed.
- `cargo fmt --all`: completed.
- Full `cargo test -p lisa-cli`: 251 passed, 0 failed.
- Full `cargo test --workspace`: 630 passed, 0 failed
  (251 CLI, 145 core, 234 plugin; doc-tests passed).
- `just check`: passed, including the `wasm32-wasip1` plugin check and all 630
  workspace tests.
- `git diff --check`: passed before final review.
- The final test-only clippy cleanup was rechecked with its focused mutation
  report regression: 1 passed, 0 failed.
- Warning-strict CLI clippy reaches one pre-existing
  `needless_borrows_for_generic_args` diagnostic in the stale-version test near
  `init.rs:2032`. T-030-01 already documented this baseline issue. No clippy
  diagnostic remains in code or tests introduced by T-030-02.

## Remaining

- None. Review artifact is complete.

## Deviations

### Combined implementation unit

The action-category refactor, append-only planner, and mutation reporting landed
in one working unit before the first implementation commit. They share exhaustive
enum matches and end-to-end output tests, so separating them would have required
an intentionally incomplete intermediate state. The phase artifacts remain a
separate earlier commit.

## Commits

- `a2a85fc` — research, design, structure, plan, and initial progress artifacts.
- `8796cca` — append-only gitignore merge, action categories, exact mutation
  report, regressions, and README contract.
- Final review/handoff commit contains the test-only clippy cleanup and completed
  progress/review artifacts.
