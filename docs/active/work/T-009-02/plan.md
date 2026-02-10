# Plan: Init Hardening for External Projects

## Step 1: Add project type label to CLAUDE.md template

**File:** `crates/lisa-cli/src/templates.rs`

- Add a `type_label()` helper or inline match on `project.project_type` in `generate_claude_md()`
- Insert label into the format string: `{name} ({type_label}) — TODO: ...`
- Update existing test `test_generate_claude_md_rust` to assert "Rust" appears
- Add test `test_generate_claude_md_node` for Node.js project

**Verify:** `cargo test -p lisa-cli -- templates`

## Step 2: Promote hook validation from warnings to errors

**File:** `crates/lisa-cli/src/init.rs`, in `run_validate()`

- Change `.claude/settings.local.json` missing check: `warnings.push(...)` → `errors.push(...)`
- Add: if file exists, read it and check `content.contains("idle_prompt")` → error if missing
- Change `.lisa/hooks/on-idle.sh` missing check: `warnings.push(...)` → `errors.push(...)`
- Add unix-only: if file exists, check executable permission → error if not set

**Verify:** `cargo test -p lisa-cli -- init`

## Step 3: Add new validation tests

**File:** `crates/lisa-cli/src/init.rs`, in `mod tests`

Add these tests:
1. `test_validate_missing_settings_json` — no settings file → error
2. `test_validate_settings_json_without_idle_hook` — file exists, no idle_prompt → error
3. `test_validate_missing_idle_hook_script` — no on-idle.sh → error
4. `test_validate_idle_hook_not_executable` — on-idle.sh exists, not executable (unix) → error
5. `test_validate_invalid_ticket_type_value` — ticket with `type: ticket` → error
6. `test_validate_invalid_phase_value` — ticket with `phase: coding` → error

**Verify:** `cargo test -p lisa-cli -- init`

## Step 4: Add init→validate round-trip tests

**File:** `crates/lisa-cli/src/init.rs`, in `mod tests`

1. `test_init_then_validate_roundtrip_rust` — create Cargo.toml, run_init, add ready ticket, run_validate → Ok
2. `test_init_then_validate_roundtrip_node` — create package.json, run_init, add ready ticket, run_validate → Ok

These are the most important tests: they prove that init output passes validate for external projects.

**Verify:** `cargo test -p lisa-cli -- init`

## Step 5: Full test suite

**Verify:** `cargo test --workspace` — all existing + new tests pass

## Testing Strategy

- All tests use `tempfile::tempdir()` for isolation
- Round-trip tests simulate the exact user workflow: init a project, add tickets, validate
- No mocking needed — all functions operate on filesystem
- Unix-specific tests guarded with `#[cfg(unix)]`
