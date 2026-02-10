# Design: Init Hardening for External Projects

## Decision Summary

Incremental hardening of existing code. No new modules, no architectural changes.
All changes stay within `init.rs` and `templates.rs`. Test coverage is the main
deliverable.

## Approach: Targeted Fixes + Round-Trip Tests

### Option A: Minimal targeted fixes (chosen)
- Add project type label to CLAUDE.md template
- Promote hook checks from warnings to errors in validate
- Add content validation for settings.local.json (check for idle_prompt key)
- Add executable check for on-idle.sh on unix
- Add init→validate round-trip test
- Add validate tests for each specific failure case in the requirements

### Option B: Refactor validate into a diagnostic collector
- Create a `ValidationDiagnostic` enum with typed variants
- More testable, but over-engineering for the current scope
- Rejected: the current string-based approach is clear enough and the test suite
  already verifies specific error messages

### Option C: Add JSON parsing for settings validation
- Pull in serde_json to validate settings.local.json structure
- Rejected: serde_json is already a dependency (via lisa-core types.rs serde),
  but we don't need full structural validation. A simple string contains check
  for "idle_prompt" is sufficient and avoids coupling to JSON structure.

## Design Decisions

### 1. CLAUDE.md template: Add project type label
In `generate_claude_md()`, change the project header from:
```
{name} — TODO: add a one-line project description here.
```
to:
```
{name} ({type_label}) — TODO: add a one-line project description here.
```
where `type_label` is "Rust", "Node.js", "Go", "Python", or "unknown type".

### 2. Validate hook infrastructure as errors, not warnings
The ticket says "catch and report clearly." Currently these are warnings.
Promote to errors:
- Missing `.claude/settings.local.json` → error
- Missing `.lisa/hooks/on-idle.sh` → error
- `settings.local.json` exists but doesn't contain `idle_prompt` → error
- `on-idle.sh` exists but not executable (unix only) → error

Rationale: Without hook infrastructure, `lisa loop` will not receive idle signals
and threads will appear stuck. This is not optional — it's required for correct
operation.

### 3. settings.local.json content validation
Read the file and check `content.contains("idle_prompt")`. Simple string match,
no JSON parsing needed. This catches the case where the file exists but was
created manually without the hook.

### 4. on-idle.sh executable check
On unix: check file permissions for executable bit.
On non-unix: skip (already handled by `#[cfg(unix)]`).

### 5. Round-trip test: init → validate
Single test that runs `run_init()` on a tempdir then `run_validate()` on the
same directory with a ticket file added. Verifies the complete pipeline works
for each detected project type.

### 6. Test coverage for each validation case
Add specific tests for:
- settings.local.json missing → error
- settings.local.json without idle_prompt → error
- on-idle.sh not executable → error (unix only)
- Ticket with `type: ticket` → parse error surfaced
- Ticket with invalid phase → parse error surfaced
- CLAUDE.md template includes project type for Rust and Node

## What We're NOT Doing

- Not adding JSON parsing for settings validation
- Not restructuring validate into typed diagnostics
- Not adding config file merging for settings.local.json (existing settings + hook)
- Not changing the init plan-then-execute pattern
- Not modifying ticket parsing in lisa-core (already catches invalid types/phases)
