# T-010-01 Structure: Hook Scaffolding

## File: crates/lisa-cli/src/templates.rs

### Add Constants (after ON_IDLE_HOOK, before LISA_GITIGNORE)

```
pub const ON_STOP_HOOK: &str = r#"..."#;
pub const ON_CLEAR_HOOK: &str = r#"..."#;
```

Both follow the exact same pattern as `ON_IDLE_HOOK`:
- Shebang `#!/bin/sh`
- Comment header
- `mkdir -p "$SIGNAL_DIR"`
- Write timestamped file: `pane-$LISA_PANE_ID.stopped` / `pane-$LISA_PANE_ID.cleared`

### Modify: settings_local_json()

Replace the current body (lines 28-44) with a JSON literal containing all three hook entries:
- `"Stop"`: array with one object, no matcher, command `".lisa/hooks/on-stop.sh"`
- `"SessionStart"`: array with one object, matcher `"clear"`, command `".lisa/hooks/on-clear.sh"`
- `"Notification"`: array with one object, matcher `"idle_prompt"`, command `".lisa/hooks/on-idle.sh"`

### Remove: merge_idle_prompt_hook()

Delete lines 49-87. Replaced by `merge_hooks()`.

### Add: merge_hooks()

```rust
pub fn merge_hooks(existing_json: &str) -> Result<String, String>
```

Internally uses a private helper:

```rust
fn ensure_hook(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event_type: &str,
    matcher: Option<&str>,
    command: &str,
)
```

`ensure_hook` logic:
1. Get or create `hooks_obj[event_type]` as array
2. Check if hook already exists:
   - If `matcher` is Some: check any entry's `matcher` field matches
   - If `matcher` is None: check any entry's `hooks[].command` matches the command
3. If not present, push a new entry (with or without matcher based on parameter)

`merge_hooks` calls `ensure_hook` three times:
- `ensure_hook(hooks, "Stop", None, ".lisa/hooks/on-stop.sh")`
- `ensure_hook(hooks, "SessionStart", Some("clear"), ".lisa/hooks/on-clear.sh")`
- `ensure_hook(hooks, "Notification", Some("idle_prompt"), ".lisa/hooks/on-idle.sh")`

### Test Additions (templates.rs)

- `test_on_stop_hook_content`: shebang, LISA_PANE_ID, .stopped extension
- `test_on_clear_hook_content`: shebang, LISA_PANE_ID, .cleared extension
- `test_settings_local_json`: expand to assert Stop, SessionStart, clear, idle_prompt, on-stop.sh, on-clear.sh, on-idle.sh
- `test_merge_hooks_empty_object`: all three hooks added
- `test_merge_hooks_with_existing_idle`: adds Stop + SessionStart, keeps idle
- `test_merge_hooks_already_complete`: no duplicates
- `test_merge_hooks_preserves_permissions`: unrelated keys preserved
- `test_merge_hooks_invalid_json`: returns Err
- Remove `test_merge_idle_prompt_hook_*` tests (4 tests) — replaced by merge_hooks tests

## File: crates/lisa-cli/src/init.rs

### Modify: plan_init_actions()

After the existing on-idle.sh block (lines 113-125), add two identical blocks for:
- `.lisa/hooks/on-stop.sh` → `templates::ON_STOP_HOOK`
- `.lisa/hooks/on-clear.sh` → `templates::ON_CLEAR_HOOK`

For `settings.local.json` merge (lines 141-180):
- Change the skip condition from `content.contains("idle_prompt")` to checking all three: `content.contains("idle_prompt") && content.contains("\"Stop\"") && content.contains("\"SessionStart\"")`
- Change `merge_idle_prompt_hook(&content)` to `merge_hooks(&content)`

### Modify: run_init()

Replace the single hardcoded chmod block (lines 246-255) with a loop over:
```rust
let hook_scripts = ["on-idle.sh", "on-stop.sh", "on-clear.sh"];
for script in &hook_scripts {
    let hook_path = root.join(format!(".lisa/hooks/{}", script));
    if hook_path.exists() {
        // chmod 0o755
    }
}
```

### Modify: validate()

After the existing on-idle.sh checks (lines 430-453), add parallel checks for:
- `.lisa/hooks/on-stop.sh` — exists + executable
- `.lisa/hooks/on-clear.sh` — exists + executable

For `settings.local.json` content check (line 410):
- Expand to check all three: `"idle_prompt"`, `"Stop"`, `"SessionStart"`
- Report specific missing hooks in the error message

### Modify: write_hook_infrastructure() test helper

Add `on-stop.sh` and `on-clear.sh` creation + chmod alongside `on-idle.sh`.

### Modify: test_plan_init_actions_empty_dir

Change expected count from 14 to 16 (2 additional hook files).

### Test Additions (init.rs)

- `test_validate_missing_stop_hook`: on-stop.sh absent → error
- `test_validate_missing_clear_hook`: on-clear.sh absent → error
- `test_plan_init_actions_existing_stop_hook`: on-stop.sh pre-exists → skip
- Update `test_diagnostics_hook_structure_errors`: expect 4 errors (was 2)

## Summary of Changes

| File | Lines Added (est.) | Lines Removed (est.) | Net |
|------|-------------------|---------------------|-----|
| templates.rs | ~80 | ~40 | +40 |
| init.rs | ~60 | ~15 | +45 |
| **Total** | ~140 | ~55 | +85 |
