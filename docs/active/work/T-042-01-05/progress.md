# Progress: Git-root-aware completion command

## Status

Implementation and verification are complete. The meaningful three-file source
unit is committed through the required isolated transaction.

## Completed work

### Configuration transport

- Added `PluginConfig.git_root`.
- Kept an empty default for legacy layouts and direct unit fixtures.
- Parsed `git_root` from the Zellij plugin configuration map.
- Added a focused default/round-trip unit test.

### Native discovery

- Added `discover_git_root` to the native loop launcher.
- Discovery invokes Git from the Lisa project root and canonicalizes the
  returned repository root.
- Real loop startup fails with a named error when discovery fails.
- Dry-run preserves its pre-repository utility by displaying the project root
  as a fallback only when discovery is unavailable.
- Layout generation now receives and emits the absolute Git root.
- Added a nested `games/midsummer` discovery regression using a temporary Git
  repository.

### Plugin root model

- Added a separate `State.git_root` beside `State.project_root`.
- Plugin load retains the configured Git root.
- Project-root uses for hook cwd and host execution remain unchanged.

### Completion path mapping

- Replaced the project-relative mapper with a completion-specific
  Git-root-relative mapper.
- `/host/...` sandbox paths are projected through the Lisa project root.
- Relative paths are anchored at the Lisa project root.
- Host absolute paths remain host absolute.
- Paths and Git root are lexically normalized without requiring enclosing host
  filesystem access from WASI.
- Empty/root selections and outside-root paths are rejected.
- Outside failures carry the stable name `completion path outside Git root`.

### Completion argv

- `complete-ticket --path` now receives the Git root.
- `--ticket-file` and `--work-dir` now receive normalized Git-root-relative
  paths.
- Host command cwd remains the Lisa project root.
- Added an exact argv regression proving the nested prefix
  `games/midsummer/docs/active/...`.
- Added a named outside-root rejection regression.

## Verification completed

`cargo check -p lisa-core -p lisa-cli -p lisa-plugin`

Passed.

`cargo test -p lisa-core test_config_git_root_round_trip --no-fail-fast`

Passed: 1; failed: 0.

`cargo test -p lisa-cli loop_cmd --no-fail-fast`

Passed: 19; failed: 0.

`cargo test -p lisa-plugin --lib completion_command --no-fail-fast`

Passed: 2; failed: 0.

`cargo fmt --all`

Applied; subsequent diff inspection shows formatted source.

`git diff --check`

Passed.

## Repository hygiene

The ordinary index is empty. Ticket-owned source modifications are limited to:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Lisa-managed ticket/provenance and admitted artifact paths are outside the
source unit. The pre-existing untracked `crates/lisa-plugin/docs/` tree is
preserved.

## Deviations from plan

Dry-run does not fail outside Git. Existing dry-run tests and the command's
inspection-only role support use in a freshly initialized project. Production
loop startup remains strict, and only production layout state can launch a
completion command.

No other deviation occurred.

## Remaining work

None in implementation.

## Source transaction

The installed Lisa binary did not expose `commit-ticket`. The freshly built
repository CLI performed the same required isolated transaction:

`target/debug/lisa commit-ticket --ticket-id T-042-01-05 --message "fix(plugin): build completion commands from Git root" --include crates/lisa-core/src/types.rs --include crates/lisa-cli/src/loop_cmd.rs --include crates/lisa-plugin/src/lib.rs`

Commit: `f48134cdb7112eb66181120c73d2917e7cd31da7`.

The commit contains exactly the three named source paths. All three are clean,
and the ordinary index remains empty.

## Final verification

`cargo test --workspace --no-fail-fast`

Passed. The CLI suite ran 280 tests, core ran 192 unit tests plus its integration
regressions, and the plugin ran 343 tests. The declared real-Zellij environment
test remained ignored by its contract; no executed test failed.

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

Passed.

`cargo fmt --all -- --check`

Passed.

Final commit/stat/status inspection passed. Only Lisa-managed ticket,
provenance, and admitted-work state plus the pre-existing untracked plugin docs
tree remain visible outside the clean source paths.
