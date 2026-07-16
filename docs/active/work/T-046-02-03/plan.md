# Plan: XDG-aware Zellij pre-grant and cache

## Goal

- Replace Lisa's hardcoded Zellij cache lookup with Zellij 0.43-equivalent resolution.
- Prove pre-grant and cleanup use the selected runtime path in both environment states.
- Preserve all existing fallback paths and best-effort behavior.

## Implementation unit strategy

- Treat the dependency, resolver, and regression tests as one atomic source unit.
- The dependency is required for the resolver to compile.
- The tests define the acceptance boundary of the resolver change.
- Use one `lisa commit-ticket` transaction with three exact source paths.
- Do not include workflow artifacts in that source transaction.
- Do not touch the ordinary Git index.

## Step 1: establish a baseline

- Run the existing focused `doctor` unit tests before editing.
- Confirm cleanup-inner tests pass.
- Confirm pre-grant-inner tests pass.
- Record any pre-existing failure in `progress.md`.
- Do not change source to compensate for unrelated failures.

### Verification

- `cargo test -p lisa-cli doctor::tests::test_clean_cache`
- `cargo test -p lisa-cli doctor::tests::test_pregrant`
- Expected: all selected current tests pass.

## Step 2: declare the direct dependency

- Edit `crates/lisa-cli/Cargo.toml` with `apply_patch`.
- Add `directories = "5"` under runtime dependencies.
- Keep existing dependencies and features unchanged.
- Run Cargo metadata/check so the lockfile package edge updates.
- Inspect the lockfile diff for only the CLI dependency entry.

### Verification

- `cargo check -p lisa-cli`
- `git diff -- crates/lisa-cli/Cargo.toml Cargo.lock`
- Expected: the manifest and CLI lock entry mention `directories`.
- Expected: no unrelated package-version churn.

## Step 3: replace the production resolver

- Import `directories::ProjectDirs` in `doctor.rs`.
- Replace direct `HOME` lookup and OS branching.
- Use qualifier `org`.
- Use organization `Zellij Contributors`.
- Use application `Zellij`.
- Map the optional project directories to an owned cache path.
- Preserve the function name, visibility, and return type.

### Verification

- `cargo check -p lisa-cli`
- Expected: the CLI compiles without consumer changes.
- Inspect all `zellij_cache_dir` call sites.
- Expected: cleanup, pre-grant, and doctor still call the same helper.

## Step 4: add scoped environment test support

- Work inside the existing `doctor.rs` test module.
- Add a static mutex for the two new environment cases.
- Add a restoration guard that captures `HOME` and `XDG_CACHE_HOME`.
- Store environment values as `OsString`-capable options.
- Restore variables in `Drop`.
- Recover a poisoned mutex instead of cascading failures.
- Keep all support code behind `#[cfg(test)]` through module placement.

### Verification

- Ensure production builds do not warn about test support.
- Ensure environment values are restored after successful assertions.
- Review the guard for unwind-time restoration.

## Step 5: add the configured-environment regression

- Create a temporary test root.
- Create controlled home and XDG cache paths beneath it.
- Set `HOME` to the controlled home.
- Set `XDG_CACHE_HOME` to the absolute controlled XDG root.
- Derive the expected platform-specific Zellij cache path.
- Assert the resolver returns that exact path.
- Create a nested matching Lisa plugin cache entry beneath the expected path.
- Call `clean_zellij_plugin_cache()`.
- Assert the matching entry is removed.
- Call `pregrant_plugin_permissions()` with a fake plugin path.
- Read `permissions.kdl` from the expected path.
- Assert the fake plugin key and requested permissions are present.
- Identify a non-selected candidate path.
- Assert no `permissions.kdl` is written there.

### Linux expected result

- Resolver: `<temp>/xdg-cache/zellij`.
- Cleanup target: `<temp>/xdg-cache/zellij/.../lisa-plugin-*.wasm`.
- Pre-grant file: `<temp>/xdg-cache/zellij/permissions.kdl`.
- Fallback home candidate remains unused.

### macOS expected result

- Resolver: `<temp>/home/Library/Caches/org.Zellij-Contributors.Zellij`.
- Cleanup and pre-grant operate there.
- The configured XDG candidate remains unused.

## Step 6: add the unconfigured-environment regression

- Create a fresh temporary test root.
- Set `HOME` to its controlled home child.
- Remove `XDG_CACHE_HOME`.
- Derive the expected platform-specific fallback path.
- Assert the resolver returns that exact path.
- Create a nested matching Lisa plugin cache entry.
- Call the environment-resolving cleanup wrapper.
- Assert the entry is removed.
- Call the environment-resolving pre-grant wrapper.
- Assert `permissions.kdl` is written under the expected fallback root.
- Assert the content names the fake plugin and all requested permissions.

### Linux expected result

- Resolver: `<temp>/home/.cache/zellij`.
- Existing Linux default is unchanged.

### macOS expected result

- Resolver: `<temp>/home/Library/Caches/org.Zellij-Contributors.Zellij`.
- Existing macOS default is unchanged.

## Step 7: format and run focused tests

- Run `cargo fmt --all`.
- Run the new configured test individually.
- Run the new unconfigured test individually.
- Run all cache-related `doctor` tests together.
- Run all pre-grant-related `doctor` tests together.
- If failures expose environment leakage, correct the test guard before proceeding.

### Verification

- Exact resolver assertions pass.
- Cleanup fixture removals pass.
- Permission-file location and content assertions pass.
- Existing inner-operation tests remain green.

## Step 8: run package verification

- Run `cargo test -p lisa-cli`.
- Run `cargo clippy -p lisa-cli --all-targets -- -D warnings` if consistent with project checks.
- Confirm no flaky environment interactions across the full package suite.
- Repeat the two new tests together if any concurrency concern appears.

### Verification

- All CLI unit and integration tests pass.
- No new warnings are emitted.
- No existing CLI behavior is broken.

## Step 9: run workspace verification

- Run `just check` per project guidance.
- This includes WASM checking and native workspace tests.
- If the command is unavailable or blocked by an external toolchain issue, run equivalent Cargo checks.
- Record exact commands and outcomes in `progress.md`.
- Distinguish ticket failures from infrastructure failures.

### Verification

- Workspace tests pass.
- WASM plugin still checks.
- The CLI's direct dependency does not affect the plugin target unexpectedly.

## Step 10: inspect ownership and diff

- Run `git diff --check` for whitespace errors.
- Inspect `git diff` for each ticket-owned source path.
- Confirm no change to `loop_cmd.rs`.
- Confirm no change to plugin permission requests.
- Confirm no unrelated lockfile churn.
- Run `git status --short`.
- Separate pre-existing Lisa-managed worktree entries from ticket-owned source changes.

### Verification

- Source diff is limited to:
  - `Cargo.lock`
  - `crates/lisa-cli/Cargo.toml`
  - `crates/lisa-cli/src/doctor.rs`
- Attempt artifacts exist only in the private attempt directory.

## Step 11: commit the meaningful source unit

- Invoke Lisa's isolated transaction from the repository root.
- Use ticket ID `T-046-02-03`.
- Use a concise message describing XDG-aware Zellij cache resolution.
- Include each exact repository-relative source path.
- Do not use `git add`.
- Do not use ordinary `git commit`.

### Command shape

```text
lisa commit-ticket \
  --ticket-id T-046-02-03 \
  --message "fix(cli): match Zellij cache directory resolution" \
  --include Cargo.lock \
  --include crates/lisa-cli/Cargo.toml \
  --include crates/lisa-cli/src/doctor.rs
```

### Verification

- Confirm the Lisa command reports a successful isolated commit.
- Inspect the resulting commit summary.
- Confirm ticket-owned paths have no staged, modified, or untracked changes.
- Confirm unrelated ordinary-index and worktree state remains untouched.

## Step 12: update implementation progress

- Create `progress.md` before source editing begins.
- Record the baseline result.
- Mark dependency and resolver changes complete.
- Mark both regression states complete.
- Record formatting, focused, package, and workspace verification.
- Record the isolated commit identifier.
- Document deviations from this plan before applying them.
- State that no implementation work remains when complete.

## Step 13: Review phase

- Inspect the committed diff.
- Summarize all changed source files.
- Explain the exact runtime behavior change.
- Map tests to both acceptance criteria.
- Identify platform coverage available in the current environment.
- Identify any remaining concern about future Zellij dependency upgrades.
- Confirm no ticket-owned source dirt remains.
- Write `review.md` in the attempt-private work directory.
- Write the exact one-line disposition JSON.

## Pass criteria

- Production resolver uses `ProjectDirs` with Zellij's exact tuple.
- Linux absolute `XDG_CACHE_HOME` resolves to its `zellij` child.
- Linux unset override resolves to `$HOME/.cache/zellij`.
- macOS continues resolving to the bundle-ID cache path.
- Both cleanup and pre-grant wrappers operate under the resolved root.
- Existing content, idempotence, traversal, and preservation tests pass.
- Package and workspace verification pass.
- All ticket-owned source changes are committed with exact Lisa includes.
- Review artifacts are complete and disposition is `pass`.

## Block criteria

- The exact Zellij 0.43 path cannot be represented through the dependency.
- Regression tests cannot safely isolate filesystem effects.
- Required package or workspace tests fail due to the ticket change.
- Lisa's isolated transaction cannot commit all owned source paths.
- Ticket-owned source changes remain dirty after repeated safe remediation.
- Any such block receives a non-empty actionable reason in disposition JSON.
