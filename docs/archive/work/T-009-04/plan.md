# Plan: T-009-04 LLM-Driven Validate Loop

## Step 1: Add types and extract `validate()` function

1. Add `Severity`, `ValidationDiagnostic`, and `ValidationResult` types at top of `init.rs` (private).
2. Create `fn validate(root: &Path, check_tools: bool) -> ValidationResult` by extracting the body of `run_validate`.
3. Convert every `errors.push(string)` → `diagnostics.push(ValidationDiagnostic { path, category, message, severity: Error })`.
4. Convert every `warnings.push(string)` → `diagnostics.push(ValidationDiagnostic { ..., severity: Warning })`.
5. For ticket file paths, use `path.strip_prefix(root)` to produce relative paths.
6. Track `ticket_count` and `ready_count` in the result.
7. Handle early returns: when ticket dir is missing or scan fails, return the diagnostics collected so far (with ticket_count=0, ready_count=0).

**Verify:** `cargo check -p lisa-cli`

## Step 2: Rewrite output formatting

1. Replace `print_results` with `print_diagnostics(result: &ValidationResult) -> Result<(), String>`.
2. Print errors first, then warnings, each on its own line:
   - Error: `{path}: {category}: {message}`
   - Warning: `{path}: {category} (warning): {message}`
3. Summary line:
   - Errors present: `\n{N} error(s) found. Fix and re-run \`lisa validate\`.`
   - No errors: `All checks passed. {total} tickets, {ready} ready, DAG valid. Run \`lisa loop\` to start.`
4. Update `run_validate` to call `validate()` then `print_diagnostics()`.

**Verify:** `cargo check -p lisa-cli`

## Step 3: Run existing tests

Run `cargo test --workspace` to confirm all existing tests pass unchanged. The tests check `is_ok()` / `is_err()` on `run_validate`, which still returns `Result<(), String>`.

**Verify:** `cargo test --workspace` — all tests pass

## Step 4: Add new tests for structured diagnostics

Add tests that call `validate()` directly and assert on diagnostics:

1. `test_validate_diagnostics_clean` — valid project → 0 errors, correct ticket_count/ready_count
2. `test_validate_diagnostics_missing_claude_md` — missing CLAUDE.md → has diagnostic with path="CLAUDE.md", category="structure"
3. `test_validate_diagnostics_ticket_parse_error` — malformed ticket → diagnostic with category="frontmatter"
4. `test_validate_diagnostics_missing_dependency` — broken depends_on → diagnostic with category="dependency"
5. `test_validate_diagnostics_no_ready_tickets` — all tickets done → diagnostic with category="readiness"
6. `test_validate_diagnostics_format` — test that `format_diagnostic` produces `{path}: {category}: {message}` exactly

**Verify:** `cargo test --workspace` — all tests pass including new ones

## Step 5: Final verification

1. `cargo test --workspace` — all tests pass
2. `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM still builds
3. Manual sanity: the output format matches the ticket's example
