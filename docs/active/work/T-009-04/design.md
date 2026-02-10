# Design: T-009-04 LLM-Driven Validate Loop

## Approach: Structured Error Collection + Flat Output

### Decision
Replace the `Vec<String>` error/warning collectors with a `Vec<ValidationDiagnostic>` that carries path, category, severity, and message. Then format each diagnostic as `{path}: {category}: {message}` on its own line. No grouping headers, no decorative prefixes.

### Why This Approach
1. **Minimal diff** — The validation logic (checks 1-13 in research.md) stays unchanged. Only the error collection and output formatting change.
2. **Structured internally, flat externally** — A `ValidationDiagnostic` struct gives us type safety inside the code while producing a simple one-line-per-error format that any LLM can parse with a basic regex.
3. **No dependencies** — No new crates needed. The struct is just `{path: String, category: &str, message: String, severity: Severity}`.

### Rejected Alternatives
- **JSON output mode** — Adds complexity, requires a `--json` flag, and LLMs parse `path: category: message` just as well. Can be added later if needed.
- **Keep Vec<String>, just reformat messages** — Loses structure. If we ever want JSON or programmatic access, we'd have to parse strings back apart.
- **New error type hierarchy** — Over-engineered. A flat struct with a category string is sufficient.

## Detailed Design

### New Types (in `init.rs`)

```rust
/// Severity of a validation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

/// A single validation finding.
#[derive(Debug, Clone)]
struct ValidationDiagnostic {
    /// Relative file path or logical location (e.g. "CLAUDE.md", "docs/active/tickets/")
    path: String,
    /// Category tag: frontmatter, dependency, structure, config, readiness
    category: &'static str,
    /// Human-readable description of the problem
    message: String,
    /// Whether this blocks readiness
    severity: Severity,
}
```

These types are private to `init.rs` — no need to export them.

### Output Format

Errors and warnings printed in order, one per line:
```
docs/active/tickets/T-001.md: frontmatter: missing required field 'phase'
docs/active/tickets/T-002.md: dependency: depends_on references unknown ticket 'T-999'
.claude/settings.local.json: config: missing idle_prompt hook configuration
```

Warnings get a `warning:` prefix in the category:
```
docs/active/stories: structure (warning): directory not found
```

Actually, simpler: just don't print warnings unless verbose. The ticket says "structured, actionable output" and warnings aren't actionable blockers. But the ticket's example includes no warnings. Let's keep warnings as-is for now — print them after errors with `(warning)` suffix on the category.

Final summary line:
- On failure: `\n{N} error(s) found. Fix and re-run \`lisa validate\`.`
- On success: `All checks passed. {total} tickets, {ready} ready, DAG valid. Run \`lisa loop\` to start.`

### Changes to `run_validate`

1. Replace `errors: Vec<String>` and `warnings: Vec<String>` with `diagnostics: Vec<ValidationDiagnostic>`.
2. Each `errors.push(...)` becomes `diagnostics.push(ValidationDiagnostic { path, category, message, severity: Severity::Error })`.
3. Each `warnings.push(...)` becomes `diagnostics.push(ValidationDiagnostic { ..., severity: Severity::Warning })`.
4. Replace `print_results` with `print_diagnostics` that formats each diagnostic as `{path}: {category}: {message}` (errors) or `{path}: {category} (warning): {message}` (warnings).
5. On success path (no errors), include ticket count and ready count in the summary.

### Changes to `print_results` → `print_diagnostics`

```rust
fn print_diagnostics(diagnostics: &[ValidationDiagnostic]) -> Result<(), String> {
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.severity == Severity::Warning).collect();

    for d in &errors {
        println!("{}: {}: {}", d.path, d.category, d.message);
    }
    for d in &warnings {
        println!("{}: {} (warning): {}", d.path, d.category, d.message);
    }

    if errors.is_empty() {
        // Success — caller provides summary
        Ok(())
    } else {
        let msg = format!("{} error(s) found. Fix and re-run `lisa validate`.", errors.len());
        println!("\n{}", msg);
        Err(msg)
    }
}
```

### Path Resolution

All paths should be relative to the project root. The ticket's example uses bare relative paths like `docs/active/tickets/T-001.md`. Since `run_validate` takes `root: &Path`, we'll express all paths relative to root. For ticket files, use `ticket.file_path.strip_prefix(root)` to get the relative path. For known files (CLAUDE.md, .lisa.toml, etc.) use hardcoded relative paths.

### Success Summary Enhancement

On success, we need ticket count and ready count. Currently `run_validate` builds the DAG and calls `dag.get_ready_tickets()`. We'll capture `dag.len()` and `ready.len()` and pass them to the summary:
```
All checks passed. 4 tickets, 2 ready, DAG valid. Run `lisa loop` to start.
```

### Test Impact

Existing tests check `result.is_ok()` / `result.is_err()` — these will continue to work unchanged. No test currently asserts on stdout content, so the output format change is non-breaking for tests.

New tests to add:
- Test that the error format matches `{path}: {category}: {message}` pattern (capture stdout or refactor to return diagnostics for testing).
- Actually, testing stdout capture is awkward. Better: make `run_validate` return `Vec<ValidationDiagnostic>` (or a result struct) and have `main.rs` handle printing. But that's a larger refactor. Simpler: add a `collect_diagnostics` function that returns the diagnostics, and have `run_validate` call it + print.

### Refactor Plan

Split `run_validate` into:
1. `validate(root, check_tools) -> ValidationResult` — returns structured result
2. `run_validate(root, check_tools) -> Result<(), String>` — calls validate, prints, returns exit status

```rust
struct ValidationResult {
    diagnostics: Vec<ValidationDiagnostic>,
    /// Only populated if no errors prevented DAG construction
    ticket_count: usize,
    ready_count: usize,
}
```

This makes the diagnostics testable without stdout capture.
