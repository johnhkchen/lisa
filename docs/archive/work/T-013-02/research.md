# T-013-02 Research: Add dependency checks to `lisa loop`

## Current State

### loop_cmd.rs already gates on binaries

`loop_cmd.rs:11-15` already checks for `zellij` and `claude` before proceeding:

```rust
if !dry_run {
    check_binary("zellij", "Install zellij: ...")?;
    check_binary("claude", "Install Claude Code: ...")?;
}
```

The `check_binary` function (line 140-145) calls a `which` helper (line 147-155) that shells out to `which(1)`.

### doctor.rs has parallel logic

`doctor.rs` has its own `is_on_path` function (line 58-66) that does the exact same thing as `loop_cmd::which` — shells out to `which(1)` and checks exit status.

`doctor.rs` also has richer check functions (`check_zellij`, `check_claude`, `check_wasm_target`) that capture version strings and provide install hints.

### Duplication inventory

| Concept | loop_cmd.rs | doctor.rs |
|---------|-------------|-----------|
| PATH check | `which()` (line 147) | `is_on_path()` (line 58) |
| Binary gating | `check_binary()` (line 140) | `check_zellij/claude()` (line 68/77) |
| Required deps list | hardcoded in `run_loop` | `build_checks()` (line 116) |

Both modules define the same two required dependencies (zellij, claude) independently.

### Error messages

Current `loop_cmd` error on missing dep:
```
`zellij` not found in PATH. Install zellij: https://zellij.dev/documentation/installation
```

Ticket requires:
```
Error: Missing required dependencies.

Run `lisa doctor` for details and install instructions.
```

### Visibility and access

- `doctor.rs` types (`CheckResult`, `DependencyCheck`, `CheckReport`) are all private (no `pub`)
- `doctor.rs` functions (`build_checks`, `run_checks`, `has_failures`) are all private
- Only `run_doctor()` is `pub`
- `loop_cmd::which` is `pub(crate)` — used by `init.rs` for `--check-tools` validation

### init.rs dependency on loop_cmd::which

`init.rs` imports `crate::loop_cmd::which` for the `--check-tools` flag on `lisa validate`. This means `which` can't simply be removed from `loop_cmd` without providing an alternative.

### Test coverage

- `doctor.rs` has 11 tests using mock check functions — good pattern for testability
- `loop_cmd.rs` tests `run_loop` with real filesystem but don't directly test the binary gating (the `check_binary` calls happen before CLAUDE.md check, and the tests for missing CLAUDE.md pass because the `check_binary` calls succeed on CI where zellij/claude might not be installed — actually they fail first. Looking more carefully: `test_run_loop_missing_claude_md` creates a tempdir without zellij/claude, so `check_binary("zellij")` would fail before reaching the CLAUDE.md check. This means these tests only pass on machines where zellij and claude are installed.)

Wait — re-reading: the test creates a tempdir and calls `run_loop(dir.path(), &config, false)`. The `check_binary` calls happen first. On CI without zellij, this would error with "zellij not found" not "No CLAUDE.md found". The test asserts `contains("CLAUDE.md")`. This means these tests only pass on machines with both binaries installed, or there's something else going on. Actually, the tests run on the developer's machine where both are installed, so they pass. This is fragile.

### dry_run bypasses checks

`run_loop` skips binary checks in dry-run mode (line 11-15). This is deliberate — dry-run is for checking the DAG without runtime deps. The ticket doesn't change this behavior.

## Key files

| File | Role |
|------|------|
| `crates/lisa-cli/src/doctor.rs` | Verbose dependency checker, source of truth for checks |
| `crates/lisa-cli/src/loop_cmd.rs` | Loop launcher, currently has own binary checks |
| `crates/lisa-cli/src/init.rs` | Uses `loop_cmd::which` for `--check-tools` |
| `crates/lisa-cli/src/main.rs` | CLI entry, dispatches to doctor/loop/validate |

## Constraints

1. `doctor.rs` internals are all private — need to expose a subset for reuse
2. `loop_cmd::which` is used by `init.rs` — must relocate or re-export
3. Dry-run mode should continue to skip binary checks
4. The shared function should be testable with mocks (doctor.rs already has a good pattern)
5. `lisa loop` error message must point to `lisa doctor`, not provide install hints directly
