# Review: XDG-aware Zellij pre-grant and cache

## Review outcome

- Disposition: pass.
- Ticket: `T-046-02-03`.
- Source implementation is complete.
- Acceptance criteria are covered by production behavior and regression tests.
- Focused, package, and workspace checks pass.
- Ticket-owned source changes are committed through Lisa's isolated transaction.
- No ticket-owned source path remains staged, modified, or untracked.
- No critical issue requires human intervention before completion.

## Source commit

- Commit: `485b7f78c3304537575571d76ba561bdf5390b1e`.
- Subject: `fix(cli): match Zellij cache directory resolution`.
- Commit mechanism: `lisa commit-ticket`.
- Ticket ID supplied to the transaction: `T-046-02-03`.
- Exact include: `Cargo.lock`.
- Exact include: `crates/lisa-cli/Cargo.toml`.
- Exact include: `crates/lisa-cli/src/doctor.rs`.
- The ordinary Git index was not used.
- The ordinary Git index is empty after the transaction.

## What changed

- The CLI now directly depends on `directories` major version 5.
- The workspace lockfile records `directories` in `lisa-cli`'s dependency list.
- The concrete package was already pinned at `directories` 5.0.1.
- `doctor.rs` imports `directories::ProjectDirs`.
- `zellij_cache_dir()` no longer manually reads `HOME`.
- It no longer manually branches between macOS and other Unix targets.
- It constructs project directories with qualifier `org`.
- It uses organization `Zellij Contributors`.
- It uses application `Zellij`.
- It returns the resulting `cache_dir()` as an owned `PathBuf`.

## Why the production change is correct

- Zellij 0.43.1 uses the same `directories` major version.
- Zellij's resolved source uses the same exact project tuple.
- Zellij derives its plugin permission cache from that project cache directory.
- Lisa now delegates path semantics to the same library contract.
- Linux absolute `XDG_CACHE_HOME` is honored by `directories` 5.
- The Linux application suffix produced by the tuple is `zellij`.
- The configured Linux path is therefore `$XDG_CACHE_HOME/zellij`.
- With no configured override, Linux falls back to `$HOME/.cache/zellij`.
- On macOS, the tuple produces `org.Zellij-Contributors.Zellij`.
- The macOS path remains `$HOME/Library/Caches/org.Zellij-Contributors.Zellij`.
- Relative XDG overrides follow library fallback semantics rather than custom Lisa behavior.
- Home discovery also follows the same `dirs-sys` behavior used by the library.

## Runtime consumer coverage

- `clean_zellij_plugin_cache()` still calls `zellij_cache_dir()`.
- `pregrant_plugin_permissions()` still calls `zellij_cache_dir()`.
- `run_doctor()` still calls `zellij_cache_dir()` for cleanup and reporting.
- No consumer keeps a separate cache-path implementation.
- No call-site signature changed.
- No startup ordering changed.
- Loop cleanup still runs before permission pre-grant.
- Both operations therefore select the same environment-derived cache root.

## Preserved behavior

- `zellij_cache_dir()` remains private.
- Its return type remains `Option<PathBuf>`.
- A failed directory resolution still produces `None`.
- Loop cleanup remains best-effort.
- Permission pre-grant remains best-effort.
- Doctor retains its existing resolution-failure diagnostic.
- The recursive cleanup algorithm is unchanged.
- Cache matching still removes names containing `lisa-plugin`.
- The permission KDL serialization is unchanged.
- Existing KDL entries are still preserved.
- Existing exact plugin grants remain idempotent.
- The requested permission set is unchanged.
- No CLI flag or configuration field was added.
- No plugin/WASM code changed.

## New test support

- Tests use a module-local mutex around directory-environment mutation.
- Tests save original `HOME` and `XDG_CACHE_HOME` values as `OsString`.
- A scoped guard restores both values on drop.
- Restoration occurs during normal return and panic unwinding.
- Poisoned mutex recovery prevents one failure from masking subsequent cases.
- Temporary directories isolate all generated cache content.
- Production code receives no test-only environment abstraction.
- The regression therefore exercises the actual `ProjectDirs` resolver.

## Configured-environment regression

- `test_zellij_cache_wrappers_honor_configured_environment` sets controlled `HOME`.
- It sets an absolute controlled `XDG_CACHE_HOME`.
- It asserts the exact path returned by `zellij_cache_dir()`.
- On Linux the expected path is `<xdg-cache>/zellij`.
- On macOS the expected path is the existing home bundle-ID cache.
- It creates a realistic nested `lisa-plugin-deadbeef.wasm` cache directory.
- It calls `clean_zellij_plugin_cache()` rather than the injected inner function.
- It asserts the selected fixture is removed.
- It calls `pregrant_plugin_permissions()` rather than the injected inner function.
- It asserts `permissions.kdl` appears under the selected cache root.
- It asserts the fake WASM key appears in the permission file.
- It asserts every requested permission appears in the file.
- It asserts the non-selected candidate receives no permission file.

## Unconfigured-environment regression

- `test_zellij_cache_wrappers_honor_unconfigured_environment` sets controlled `HOME`.
- It explicitly removes `XDG_CACHE_HOME` for the test scope.
- It asserts the exact path returned by `zellij_cache_dir()`.
- On Linux the expected path is `<home>/.cache/zellij`.
- On macOS the expected path is the existing home bundle-ID cache.
- It creates a nested Lisa plugin-cache fixture.
- It calls the runtime cleanup wrapper.
- It asserts the fixture is removed.
- It calls the runtime pre-grant wrapper.
- It verifies the selected `permissions.kdl` content.
- The test therefore covers both path resolution consumers in fallback state.

## Existing regression coverage retained

- Cleanup with no matching entry remains covered.
- Deeply nested cache removal remains covered.
- Cleanup of a nonexistent cache root remains covered.
- Permission-file content remains covered.
- Permission pre-grant idempotence remains covered.
- Preservation of existing permission entries remains covered.
- The new tests supplement rather than replace these operation-level tests.

## Verification performed

- `cargo test -p lisa-cli doctor::tests::test_clean_cache` passed before editing.
- Cleanup baseline: 3 passed, 0 failed.
- `cargo test -p lisa-cli doctor::tests::test_pregrant` passed before editing.
- Pre-grant baseline: 7 passed, 0 failed.
- `cargo fmt --all` completed successfully after editing.
- `cargo check -p lisa-cli` completed successfully.
- `git diff --check` completed successfully.
- The configured routing regression passed individually.
- The unconfigured routing regression passed individually.
- Cleanup tests passed again after editing: 3 passed, 0 failed.
- Pre-grant tests passed again after editing: 7 passed, 0 failed.

## Package test result

- `cargo test -p lisa-cli` passed.
- CLI library unit tests: 14 passed.
- CLI binary unit tests: 272 passed.
- CLI integration tests executed successfully.
- The real-Zellij delivery boundary remained intentionally ignored.
- No executed CLI test failed.
- The new environment tests passed inside the full parallel package suite.
- This provides evidence that scoped restoration does not disturb neighboring tests.

## Workspace test result

- `just check` passed.
- `cargo check -p lisa-plugin --target wasm32-wasip1` passed.
- `cargo test --workspace` passed.
- The workspace run included 19 `lisa-cli` library tests under feature unification.
- The workspace run included 272 CLI binary tests.
- The workspace run included 395 plugin tests.
- Core, CLI, plugin, integration, and doc-test targets completed without failures.
- The direct native CLI dependency did not break the WASM plugin check.

## Acceptance criterion mapping

### Criterion 1

- With `XDG_CACHE_HOME` set on Linux, `ProjectDirs` selects its absolute root.
- The `zellij` project suffix is appended by the same library Zellij uses.
- Pre-grant writes `<XDG_CACHE_HOME>/zellij/permissions.kdl`.
- Cleanup traverses `<XDG_CACHE_HOME>/zellij`.
- With the variable unset, the library retains `$HOME/.cache/zellij` on Linux.
- macOS retains `$HOME/Library/Caches/org.Zellij-Contributors.Zellij`.
- The shared resolver serves both runtime operations.
- Criterion 1 is satisfied.

### Criterion 2

- One regression covers the configured environment.
- One regression covers the unconfigured environment.
- Each regression asserts exact path resolution.
- Each regression exercises cleanup through its environment-resolving wrapper.
- Each regression exercises pre-grant through its environment-resolving wrapper.
- Platform-specific expected values cover Linux and macOS semantics.
- Criterion 2 is satisfied.

## Platform execution note

- The current development host is macOS.
- The macOS arms of both regression cases executed locally and passed.
- The configured macOS test verifies that XDG does not change the existing path.
- Linux expectations are guarded by `cfg(target_os = "linux")`.
- On Linux CI, the same tests exercise `$XDG_CACHE_HOME/zellij` and the fallback path.
- The Linux expectation directly reflects `directories` 5.0.1 resolved source.
- No cross-compilation runtime test was attempted because filesystem environment behavior is host-specific.
- This is a coverage note, not a completion blocker.

## Concurrency note

- Another ticket modified `Cargo.lock` while this ticket was active.
- Its first isolated commit temporarily included this ticket's generated dependency edge.
- That ticket recognized and removed the foreign edge in commit `2edf4e3`.
- This restored a clean ownership boundary before this ticket's source commit.
- The final ticket commit contains exactly the planned lockfile edge.
- No unrelated lockfile package or version change appears in `485b7f7`.
- No source overlap remains unresolved.

## Diff review

- `Cargo.lock` adds one `directories` line to the `lisa-cli` package entry.
- `crates/lisa-cli/Cargo.toml` adds one direct dependency declaration.
- `doctor.rs` removes eleven lines of manual resolver logic.
- `doctor.rs` adds the two-line `ProjectDirs` resolver expression.
- The remaining additions are isolated test support and two tests.
- `loop_cmd.rs` is unchanged.
- `lisa-plugin` is unchanged.
- Ticket frontmatter phase and status are unchanged by this agent.

## Open concerns

- Zellij may change its directory dependency or project identity in a future release.
- This ticket intentionally targets documented Zellij 0.43 semantics.
- A future Zellij upgrade should compare its resolver tuple and directories major version.
- The current code comment names the 0.43 compatibility boundary to make that review visible.
- There are no known correctness concerns for the current supported boundary.
- There are no TODOs introduced by this implementation.

## Final cleanliness

- `Cargo.lock` is clean after the isolated source commit.
- `crates/lisa-cli/Cargo.toml` is clean after the isolated source commit.
- `crates/lisa-cli/src/doctor.rs` is clean after the isolated source commit.
- `git diff --cached --name-only` is empty.
- Unrelated Lisa-managed worktree files remain untouched.
- Phase artifacts exist in the attempt-private work directory.
- Review publication and ticket completion remain Lisa's responsibility.

## Recommendation

- Admit the Review artifacts.
- Mark the disposition as pass.
- Allow Lisa to prepare and verify the final completion commit.
- No additional source work is required for `T-046-02-03`.
