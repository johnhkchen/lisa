# Progress: Init Hardening for External Projects

## Completed

### Step 1: Project type label in CLAUDE.md template
- Added `type_label` derivation in `generate_claude_md()` (Rust, Node.js, Go, Python, unknown type)
- Template now produces: `{name} ({type_label}) — TODO: ...`
- Updated `test_generate_claude_md_rust` to assert "(Rust)" appears
- Added `test_generate_claude_md_node` for Node.js
- Updated `test_generate_claude_md_unknown` to assert "(unknown type)"

### Step 2: Promoted hook validation from warnings to errors
- `.claude/settings.local.json` missing → error (was warning)
- `.claude/settings.local.json` exists but no `idle_prompt` → new error
- `.lisa/hooks/on-idle.sh` missing → error (was warning)
- `.lisa/hooks/on-idle.sh` not executable (unix) → new error
- Updated all existing validate tests that expected `is_ok()` to include hook infrastructure

### Step 3: New validation tests
- `test_validate_missing_settings_json` — error when settings.local.json missing
- `test_validate_settings_json_without_idle_hook` — error when file exists without idle_prompt
- `test_validate_missing_idle_hook_script` — error when on-idle.sh missing
- `test_validate_idle_hook_not_executable` — error on unix when not executable
- `test_validate_invalid_ticket_type_value` — ticket with `type: ticket` → parse error
- `test_validate_invalid_phase_value` — ticket with `phase: coding` → parse error

### Step 4: Round-trip tests
- `test_init_then_validate_roundtrip_rust` — init with Cargo.toml + ticket → validate passes
- `test_init_then_validate_roundtrip_node` — init with package.json + ticket → validate passes

### Step 5: Full test suite
- `cargo test -p lisa-cli -p lisa-core`: 163 tests pass (86 + 77)
- lisa-plugin has pre-existing compilation errors (removed struct fields still referenced in tests)

## Test Count
- Before: 22 init tests, 7 template tests
- After: 30 init tests (+8), 8 template tests (+1)
- Total new tests: 9

## Files Modified
- `crates/lisa-cli/src/templates.rs` — project type label (5 lines changed + 1 test added)
- `crates/lisa-cli/src/init.rs` — hook validation hardening + 8 new tests + helper function

## Deviations from Plan
None. All steps executed as planned.
