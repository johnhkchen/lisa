# Structure: Init Hardening for External Projects

## Files Modified

### 1. `crates/lisa-cli/src/templates.rs`

**Modify `generate_claude_md()`:**
- Add `type_label` parameter derivation from `project.project_type`
- Insert type label into the project header line

**Change scope:** ~5 lines in the format string, plus type_label computation.

**Modify test `test_generate_claude_md_rust`:**
- Assert the output contains "Rust" type label

**Add test `test_generate_claude_md_node`:**
- Verify Node.js project generates correct type label and commands

### 2. `crates/lisa-cli/src/init.rs`

**Modify `run_validate()` — hook validation section (lines ~279-291):**
- Change `.claude/settings.local.json` check from warning to error
- Add content check: read file and verify it contains "idle_prompt"
- Change `.lisa/hooks/on-idle.sh` check from warning to error
- Add unix executable permission check for on-idle.sh

**No changes to `plan_init_actions()` or `run_init()`** — these are already correct.

**Add tests (in `mod tests`):**

1. `test_validate_missing_settings_json` — error when settings.local.json missing
2. `test_validate_settings_json_without_hook` — error when file exists but no idle_prompt
3. `test_validate_missing_on_idle_hook` — error when on-idle.sh missing
4. `test_validate_on_idle_hook_not_executable` — error on unix when not executable
5. `test_init_then_validate_roundtrip_rust` — init + add ticket → validate passes
6. `test_init_then_validate_roundtrip_node` — init + add ticket → validate passes
7. `test_validate_invalid_ticket_type` — ticket with `type: ticket` → parse error
8. `test_validate_invalid_phase` — ticket with `phase: coding` → parse error

## Module Boundaries

- No changes to `lisa-core` — ticket parsing already handles invalid types/phases
- No changes to `config.rs` — config validation is already correct
- No changes to `detect.rs` — detection logic is already correct
- No new files created

## Public Interface Changes

None. `generate_claude_md()` signature unchanged (takes `&DetectedProject`).
`run_validate()` signature unchanged. Only behavior changes:
- CLAUDE.md output now includes project type label
- Validate promotes hook checks from warnings to errors
- Validate adds content/permission checks for hook files
