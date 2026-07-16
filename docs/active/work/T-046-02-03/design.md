# Design: XDG-aware Zellij pre-grant and cache

## Design objective

- Make Lisa resolve the same cache directory as Zellij 0.43.
- Apply that resolved directory uniformly to permission pre-grant and cleanup.
- Preserve the current default paths on supported Unix platforms.
- Add regression tests that exercise configured and unconfigured environments.
- Keep the existing best-effort runtime behavior.
- Keep filesystem algorithms and loop orchestration unchanged.

## Governing external contract

- Zellij 0.43.1 resolves its cache with the `directories` crate.
- Its exact project tuple is `("org", "Zellij Contributors", "Zellij")`.
- Its cache path is `ProjectDirs::cache_dir()` for that tuple.
- The implementation should treat this tuple as the compatibility contract.
- Reusing that library and tuple avoids Lisa maintaining a second interpretation.
- The direct dependency version should stay on major version 5.
- Version 5 is the version used by the resolved Zellij 0.43 dependency source.

## Option 1: manually add `XDG_CACHE_HOME` handling

- Retain the current `cfg!(target_os = "macos")` structure.
- On non-macOS targets, read `XDG_CACHE_HOME` before `HOME`.
- Append `zellij` to the configured XDG root.
- Fall back to `$HOME/.cache/zellij`.
- Continue spelling the macOS path manually.

### Benefits

- The source change is very small.
- It does not add a direct package dependency.
- Tests can target a straightforward local function.
- The visible Linux bug would be addressed for ordinary absolute overrides.

### Costs and risks

- It duplicates Zellij's directory semantics.
- It must separately implement absolute-path validation.
- Empty and relative override behavior could diverge.
- Home-directory discovery would remain less capable than `dirs-sys`.
- Non-Linux Unix behavior would remain an undocumented approximation.
- Future path changes in the shared library would not flow into Lisa.
- The ticket asks to match Zellij's resolution exactly, not only its happy path.

### Decision

- Reject this option.
- It repairs the reported case but retains the source of semantic drift.

## Option 2: use `dirs::cache_dir` plus a suffix

- Add or reuse the `dirs` crate.
- Resolve a platform cache base with `dirs::cache_dir()`.
- Append `zellij` on Linux.
- Append the bundle identifier on macOS.

### Benefits

- Base-directory environment semantics are delegated to a library.
- Linux XDG behavior becomes correct for common cases.
- The implementation remains compact.

### Costs and risks

- Zellij uses `directories::ProjectDirs`, not `dirs::cache_dir` directly.
- Lisa would still own platform-specific project suffix rules.
- The currently locked `dirs` major version differs from `directories` internals.
- Organization/application normalization could drift.
- This is close to, but not identical to, Zellij's stated implementation.

### Decision

- Reject this option.
- It needlessly reconstructs the project-level half of the contract.

## Option 3: use the exact `ProjectDirs` construction

- Declare `directories = "5"` in `lisa-cli`.
- Import `directories::ProjectDirs` in `doctor.rs`.
- Construct the same qualifier, organization, and application as Zellij.
- Clone `cache_dir()` into Lisa's existing owned `PathBuf` return type.
- Leave all callers on the existing private resolver.

### Benefits

- The production expression mirrors Zellij 0.43 source directly.
- Absolute `XDG_CACHE_HOME` handling is inherited on Linux.
- Unset and invalid override fallback behavior is inherited.
- macOS bundle-ID formatting remains library-owned.
- Home-directory discovery matches the shared implementation.
- All three existing consumers receive the correction at once.
- The change is local and preserves current internal interfaces.

### Costs and risks

- `lisa-cli` gains a direct dependency.
- The lockfile records that direct dependency for the CLI package.
- A future Zellij upgrade may use a different directories major version.
- The project tuple is still repeated textually between projects.
- Environment-based tests require care around process-global variables.

### Decision

- Choose this option.
- It provides the strongest correspondence to the explicit 0.43 requirement.

## Option 4: depend on `zellij-utils`

- Add a direct dependency on Zellij's utility crate.
- Reuse its `ZELLIJ_CACHE_DIR` constant.

### Benefits

- The cache constant would come directly from Zellij code.
- The tuple and library version would not be repeated locally.

### Costs and risks

- `zellij-utils` is a large dependency for one directory path.
- It brings unrelated configuration, protocol, and asset concerns.
- Its global lazy value complicates tests that vary environment state.
- The first access freezes the environment-derived value for the process.
- Native CLI build time and dependency surface would grow substantially.
- Lisa already depends on Zellij APIs in the WASM package, not this CLI layer.

### Decision

- Reject this option.
- The coupling and testability cost is disproportionate to the path contract.

## Selected production design

- Keep `fn zellij_cache_dir() -> Option<PathBuf>` private.
- Replace its manual `HOME` and platform branches.
- Call `ProjectDirs::from("org", "Zellij Contributors", "Zellij")`.
- Map the result to `project_dirs.cache_dir().to_path_buf()`.
- Preserve `None` if project directories cannot be constructed.
- Do not cache the result globally in Lisa.
- Resolving on each wrapper call preserves the existing call-time behavior.
- Do not change the cleanup wrapper signature.
- Do not change the pre-grant wrapper signature.
- Do not change `run_doctor` output.
- Do not change `run_loop` ordering.

## Why no Lisa-global cache value

- A global lazy path would resemble Zellij's internal constant.
- Lisa calls cleanup and pre-grant in one stable startup environment.
- Recomputing is negligible relative to filesystem traversal and process launch.
- A non-global function is easier to test under multiple environment states.
- Existing code already resolves per call.
- Preserving that shape avoids unrelated lifecycle changes.

## Test strategy options

### Pure reimplementation tests

- A helper could accept explicit home and XDG values.
- Tests would be deterministic without changing process environment.
- The helper would necessarily reproduce library behavior.
- That would test Lisa's duplicate algorithm instead of the selected production API.
- This approach is rejected because it weakens the regression guarantee.

### Integration tests through full CLI commands

- `lisa doctor` can exercise cleanup path selection.
- Full `lisa loop` can exercise both cleanup and pre-grant.
- Loop startup requires real dependencies, embedded WASM, project setup, and Zellij exec.
- Doctor also performs dependency checks unrelated to path selection.
- These tests would be slow and operationally brittle.
- This approach is rejected for the path-level regression suite.

### Unit tests around the actual wrappers

- Unit tests in `doctor.rs` can call private functions directly.
- Temporary directories can serve as isolated home and XDG roots.
- Tests can set `HOME` and `XDG_CACHE_HOME` for the duration of each case.
- A module-local mutex can serialize the new environment-mutating cases.
- A guard can restore original variable values even after assertions complete.
- The configured case expects `<xdg>/zellij` on Linux.
- The unconfigured case expects `<home>/.cache/zellij` on Linux.
- macOS expects the bundle-ID path in both states.
- Each case can invoke both pre-grant and cleanup wrappers.
- This approach directly exercises production resolution and filesystem routing.

### Decision

- Choose wrapper-level unit tests with scoped environment restoration.
- Keep tests platform-aware rather than asserting Linux behavior on macOS.
- Limit environment mutation to `HOME` and `XDG_CACHE_HOME`.

## Regression fixture design

- Create one temporary directory per test.
- Use a child `home` directory as the controlled home.
- Use a child `xdg-cache` directory as the configured override.
- Set or remove `XDG_CACHE_HOME` according to the test case.
- Resolve the expected path according to the target OS contract.
- Place a nested `lisa-plugin-*.wasm` directory only under the expected root.
- Call `clean_zellij_plugin_cache()`.
- Assert the fixture was removed.
- Call `pregrant_plugin_permissions()` with a stable fake WASM path.
- Assert `permissions.kdl` exists under the expected root.
- Assert the file contains the fake plugin entry.
- In the configured Linux case, assert the fallback root remains untouched.
- On macOS, assert the XDG-root candidate remains untouched.

## Environment restoration design

- Save each variable with `std::env::var_os` before mutation.
- Restore a previous value with `set_var`.
- Remove a variable that was previously absent.
- Hold a static `Mutex<()>` for the complete setup/action/assertion/restore scope.
- Prefer a helper that restores values before it returns.
- Avoid assertions before restoration when practical.
- A drop guard provides restoration during unwinding.
- Poisoned mutex state can be recovered with `into_inner` for later tests.

## Compatibility and failure behavior

- Linux with an absolute XDG override changes to the intended runtime root.
- Linux without the override remains unchanged.
- Linux with a relative override follows `directories` fallback semantics.
- macOS remains unchanged with or without XDG variables.
- Missing home discovery still returns `None`.
- Cleanup still silently does nothing when resolution fails.
- Pre-grant still silently falls back to Zellij prompting on failure.
- The KDL format and requested permissions remain unchanged.
- No public API or CLI surface changes.

## Verification design

- Run targeted `lisa-cli` unit tests for Zellij cache behavior first.
- Run the full `lisa-cli` test target next.
- Run workspace formatting checks.
- Run workspace tests in proportion to the small dependency change.
- Run `just check` if the repository toolchain and WASM target permit it.
- Inspect the exact diff before the ticket commit.
- Commit only `crates/lisa-cli/Cargo.toml`, `Cargo.lock`, and `doctor.rs` if changed.
- Recheck ticket-owned cleanliness after the isolated commit.

## Chosen outcome

- Lisa will share Zellij's directory-resolution mechanism and identity tuple.
- One corrected resolver will continue to serve every existing consumer.
- Regression tests will prove both wrappers route to the selected runtime root.
- No broader cache abstraction or CLI configuration will be introduced.
