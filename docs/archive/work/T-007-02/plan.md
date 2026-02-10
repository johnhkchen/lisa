# Plan: T-007-02 validate-pre-loop-readiness

## Steps

### Step 1: Make `which` pub(crate) in loop_cmd.rs

Change `fn which(name: &str) -> bool` to `pub(crate) fn which(name: &str) -> bool`.

Verify: `cargo check -p lisa-cli`

### Step 2: Add `--check-tools` flag to CLI in main.rs

Add `check_tools: bool` field to `Commands::Validate`. Update the match arm to pass it to `run_validate`.

### Step 3: Rewrite `run_validate` in init.rs

Replace the body of `run_validate` with the comprehensive check:

1. Change signature to `pub fn run_validate(root: &Path, check_tools: bool) -> Result<(), String>`
2. Tool checks (conditional)
3. CLAUDE.md check (error) — keep existing
4. rdspi-workflow.md check — upgrade to error
5. .lisa.toml validation — keep existing
6. Config-aware ticket directory resolution
7. Ticket directory existence check (error)
8. `scan_tickets_with_diagnostics` — surface per-file parse errors
9. Empty tickets check (error)
10. DAG build + cycle detection (error) — keep existing
11. Ready tickets check (error)
12. Acceptance criteria check (warning) — keep existing
13. Grouped output: errors, warnings, summary
14. Update `run_init` call to pass `false`

Verify: `cargo check -p lisa-cli`

### Step 4: Update existing tests

- `test_validate_missing_claude_md` — add `false` arg
- `test_validate_valid_setup` — add `false` arg
- `test_validate_valid_lisa_toml` — add `false` arg
- `test_validate_invalid_lisa_toml` — add `false` arg
- `test_validate_with_tickets` — add `false` arg
- `test_validate_detects_missing_dependency` — add `false` arg

### Step 5: Add new tests

- `test_validate_missing_rdspi_workflow` — error when docs/rdspi-workflow.md missing
- `test_validate_empty_ticket_dir` — error when no .md files
- `test_validate_no_ready_tickets` — all tickets done, error
- `test_validate_ticket_parse_error` — malformed ticket file surfaces as error
- `test_validate_acceptance_criteria_warning` — returns Ok but has warning
- `test_validate_check_tools_false` — check_tools=false doesn't fail even if tools missing
- `test_validate_ready_for_loop` — full valid setup with ready ticket passes

### Step 6: Run full test suite

`cargo test --workspace`

## Testing Strategy

All tests are unit tests against `run_validate` directly, using `tempfile::tempdir()` to create isolated project structures. Tests assert:
- Return value (Ok vs Err)
- Error message content where relevant (via `unwrap_err().contains(...)`)

Tool checks cannot be reliably tested (depends on what's installed). The `check_tools=false` test ensures the flag is respected.
