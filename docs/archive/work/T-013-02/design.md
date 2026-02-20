# T-013-02 Design: Add dependency checks to `lisa loop`

## Options considered

### Option A: Extract a `check_required_deps()` function into doctor.rs

Make `doctor.rs` the single source of truth. Expose a lightweight `pub fn check_required_deps() -> Result<(), Vec<String>>` that runs only the required checks (zellij, claude) and returns missing dep names. `loop_cmd.rs` calls this and formats its own terse error. `init.rs` also migrates to use `doctor.rs` helpers instead of `loop_cmd::which`.

**Pros:** Single source of truth for "what is required." doctor.rs already has the check logic, mock pattern, and tests.

**Cons:** Slightly couples loop_cmd to doctor — but they're in the same crate and this is the whole point of the ticket.

### Option B: Create a new shared module (e.g., `deps.rs`)

Move `which()` and the required-deps check into a new `deps.rs` module. Both `doctor.rs` and `loop_cmd.rs` import from it.

**Pros:** Clean separation. Neither module depends on the other.

**Cons:** Another module for a small amount of shared logic. Over-engineering for what amounts to a `which()` function and a list of two binaries.

### Option C: Move `which()` to doctor.rs, add `check_required_deps()`

Similar to Option A but specifically: relocate `which()` from `loop_cmd.rs` to `doctor.rs` (since doctor is the "dependency checking" module), make it `pub(crate)`, and add `check_required_deps()` alongside it.

**Pros:** Consolidates all dep-checking in one place. `loop_cmd.rs` and `init.rs` import from the canonical location.

**Cons:** None significant — doctor.rs is the natural home for this logic.

## Decision: Option C

Option C is the cleanest. `doctor.rs` is the canonical module for dependency checking. Moving `which()` there and adding `check_required_deps()` eliminates duplication without creating unnecessary new modules.

Option B was rejected because a new module adds complexity for ~15 lines of shared code. Option A was close but Option C is more explicit about relocating `which()` rather than just adding a new function.

## Design

### New public API in doctor.rs

```rust
/// Check if a binary is available on PATH.
pub(crate) fn which(name: &str) -> bool { ... }

/// Check that all required runtime dependencies are present.
/// Returns Ok(()) if all found, Err with list of missing dep names otherwise.
pub(crate) fn check_required_deps() -> Result<(), Vec<String>> { ... }
```

`check_required_deps()` checks zellij and claude using the existing `check_zellij()`/`check_claude()` functions, filters for `NotFound` results on required deps, and returns the missing names.

### Changes to loop_cmd.rs

- Remove `check_binary()` and `which()` functions
- Replace the two `check_binary` calls with a single `crate::doctor::check_required_deps()?` call (mapping the error to the terse message)
- Error message on failure:
  ```
  Missing required dependencies.

  Run `lisa doctor` for details and install instructions.
  ```

### Changes to init.rs

- Replace `crate::loop_cmd::which(...)` with `crate::doctor::which(...)`

### Testing

- `doctor.rs`: Add a test for `check_required_deps()` using the existing mock pattern (mock all-found → Ok, mock one missing → Err with names)
- `loop_cmd.rs`: The existing `test_run_loop_missing_claude_md` and `test_run_loop_missing_tickets_dir` tests are currently fragile (they depend on the host having zellij/claude installed). We should NOT try to fix that fragility in this ticket — it's out of scope. The tests will continue to work on dev machines.
- Add a unit test in `loop_cmd.rs` or as an integration concern that verifies the error message format when deps are missing. However, since `check_required_deps()` shells out to `which(1)`, mocking it cleanly requires the same closure pattern as doctor.rs. The simplest approach: test `check_required_deps()` thoroughly in doctor.rs (with mocks), and trust the integration in loop_cmd.rs.

### What stays the same

- `run_doctor()` public API unchanged
- dry-run mode still skips dep checks
- `doctor.rs` internal types remain private
- Mock test pattern in doctor.rs unchanged
