# T-013-02 Structure: Add dependency checks to `lisa loop`

## Files modified

### `crates/lisa-cli/src/doctor.rs`

**Add:**
- `pub(crate) fn which(name: &str) -> bool` — relocated from `loop_cmd.rs`, replaces private `is_on_path`
- `pub(crate) fn check_required_deps() -> Result<(), Vec<String>>` — runs zellij + claude checks, returns missing names

**Modify:**
- `is_on_path()` → replaced by `which()` (same body, new name + visibility)
- Internal callers of `is_on_path` (`check_wasm_target`) updated to call `which`

**Tests added:**
- `test_check_required_deps_all_found` — mock all deps as found, assert Ok
- `test_check_required_deps_one_missing` — mock zellij missing, assert Err contains "zellij"
- `test_check_required_deps_all_missing` — mock both missing, assert Err contains both names

To make `check_required_deps` testable with mocks, extract it as a function that takes a `Vec<DependencyCheck>` parameter (like `run_checks` does), with a public wrapper that calls `build_checks()` internally:

```
fn check_required_deps_inner(checks: Vec<DependencyCheck>) -> Result<(), Vec<String>>
pub(crate) fn check_required_deps() -> Result<(), Vec<String>>
```

### `crates/lisa-cli/src/loop_cmd.rs`

**Remove:**
- `fn check_binary(name: &str, install_hint: &str) -> Result<(), String>` (line 140-145)
- `pub(crate) fn which(name: &str) -> bool` (line 147-155)

**Modify:**
- `run_loop()`: Replace the two `check_binary()` calls (lines 13-14) with:
  ```rust
  if !dry_run {
      crate::doctor::check_required_deps().map_err(|missing| {
          format!(
              "Missing required dependencies: {}\n\nRun `lisa doctor` for details and install instructions.",
              missing.join(", ")
          )
      })?;
  }
  ```

### `crates/lisa-cli/src/init.rs`

**Modify:**
- Line 346: `crate::loop_cmd::which("zellij")` → `crate::doctor::which("zellij")`
- Line 354: `crate::loop_cmd::which("claude")` → `crate::doctor::which("claude")`

## Files unchanged

- `main.rs` — no changes needed, dispatching is the same
- `detect.rs`, `templates.rs`, `config.rs` — unrelated
- All `lisa-core` and `lisa-plugin` code — unrelated

## Module boundaries

```
doctor.rs (source of truth for dep checking)
  ├── pub(crate) which()           ← used by init.rs, doctor.rs internals
  ├── pub(crate) check_required_deps()  ← used by loop_cmd.rs
  └── pub run_doctor()             ← used by main.rs (unchanged)

loop_cmd.rs (consumer)
  └── calls check_required_deps() in run_loop()

init.rs (consumer)
  └── calls which() in run_validate()
```

## Ordering

1. Modify `doctor.rs`: add `which`, `check_required_deps`, update `is_on_path` callers, add tests
2. Modify `loop_cmd.rs`: remove `check_binary`/`which`, use `check_required_deps`
3. Modify `init.rs`: update import path
4. Run `cargo test --workspace` to verify
