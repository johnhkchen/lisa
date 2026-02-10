# Progress: T-009-04 LLM-Driven Validate Loop

## Completed

### Step 1-2: Types, validate() extraction, and output formatting
- Added `Severity`, `ValidationDiagnostic`, `ValidationResult` types (private to init.rs)
- Extracted `validate(root, check_tools) -> ValidationResult` from `run_validate`
- Converted all 17 error/warning push sites to structured `ValidationDiagnostic` with path, category, message, severity
- Replaced `print_results` with `print_diagnostics` using `{path}: {category}: {message}` format
- `run_validate` is now a thin wrapper: calls `validate()` then `print_diagnostics()`
- Success summary shows ticket count and ready count

### Step 3: Existing tests
- All 86 existing CLI tests pass unchanged (now 95 with new tests)
- All 77 core tests pass
- All 94 plugin tests pass
- Total: 266 tests, 0 failures

### Step 4: New diagnostic tests
Added 9 new tests calling `validate()` directly:
- `test_diagnostics_clean_project` — 0 errors, correct counts
- `test_diagnostics_missing_claude_md` — path="CLAUDE.md", category="structure"
- `test_diagnostics_ticket_parse_error` — category="frontmatter", path contains filename
- `test_diagnostics_missing_dependency` — category="dependency", message contains missing ID
- `test_diagnostics_no_ready_tickets` — category="readiness"
- `test_diagnostics_format_error` — exact format: `path: category: message`
- `test_diagnostics_format_warning` — exact format: `path: category (warning): message`
- `test_diagnostics_hook_structure_errors` — 2 errors for missing hook infra
- `test_diagnostics_success_counts` — ticket_count=3, ready_count=2

### Step 5: Final verification
- `cargo test --workspace` — 266 tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM builds clean

## Files Changed
- `crates/lisa-cli/src/init.rs` — only production file changed

## No Deviations from Plan
Implementation followed the plan exactly.
