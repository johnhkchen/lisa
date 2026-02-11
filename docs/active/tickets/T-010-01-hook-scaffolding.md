---
id: T-010-01
title: Add Stop and SessionStart hook scaffolding
type: task
phase: done
status: done
priority: high
story: S-010
created: 2026-02-11
---

# T-010-01: Add Stop and SessionStart hook scaffolding

## Objective

Add the `Stop` and `SessionStart[clear]` hook scripts to Lisa's CLI scaffolding system. These hooks write signal files that enable event-driven session transitions.

## Tasks

### 1. Add hook script constants to `templates.rs`

File: `crates/lisa-cli/src/templates.rs`

Add two new constants alongside `ON_IDLE_HOOK`:

```rust
/// The on-stop hook script, called by Claude Code's Stop event.
/// Fires when Claude finishes responding (ready for input).
pub const ON_STOP_HOOK: &str = r#"#!/bin/sh
# Lisa stop signal hook — called by Claude Code when it finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi
"#;

/// The on-clear hook script, called by Claude Code's SessionStart[clear] event.
/// Fires after /clear is processed (context cleared).
pub const ON_CLEAR_HOOK: &str = r#"#!/bin/sh
# Lisa clear signal hook — called by Claude Code after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#;
```

### 2. Update `settings_local_json()` template

File: `crates/lisa-cli/src/templates.rs`

Expand the template to include all three hooks:

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".lisa/hooks/on-stop.sh"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "clear",
        "hooks": [
          {
            "type": "command",
            "command": ".lisa/hooks/on-clear.sh"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": ".lisa/hooks/on-idle.sh"
          }
        ]
      }
    ]
  }
}
```

### 3. Generalize hook merge logic

File: `crates/lisa-cli/src/templates.rs`

Currently `merge_idle_prompt_hook()` only handles `Notification[idle_prompt]`. Refactor to support multiple hook event types:

- Rename to `merge_hooks()` or keep separate functions for each
- Support merging `Stop`, `SessionStart`, and `Notification` hooks
- Preserve existing user hooks (don't overwrite, only add if missing)
- Use the `matcher` field for `SessionStart` and `Notification`, omit for `Stop`

### 4. Update init scaffolding

File: `crates/lisa-cli/src/init.rs`

In `plan_init_actions()`, add scaffold actions for the new hooks:

```rust
// Scaffold on-stop.sh
if !hooks_dir.join("on-stop.sh").exists() {
    actions.push(ScaffoldAction::WriteFile {
        path: hooks_dir.join("on-stop.sh"),
        content: templates::ON_STOP_HOOK.to_string(),
        executable: true,
    });
}

// Scaffold on-clear.sh
if !hooks_dir.join("on-clear.sh").exists() {
    actions.push(ScaffoldAction::WriteFile {
        path: hooks_dir.join("on-clear.sh"),
        content: templates::ON_CLEAR_HOOK.to_string(),
        executable: true,
    });
}
```

### 5. Update validation logic

File: `crates/lisa-cli/src/init.rs`

In `validate_project()`, check for the new hooks:

- `.lisa/hooks/on-stop.sh` should exist and be executable
- `.lisa/hooks/on-clear.sh` should exist and be executable
- `~/.claude/settings.local.json` should contain `Stop` and `SessionStart` hooks

Add to validation results:

```rust
if !hooks_dir.join("on-stop.sh").exists() {
    issues.push(ValidationIssue {
        severity: Severity::Error,
        category: Category::Hooks,
        message: "Missing .lisa/hooks/on-stop.sh".to_string(),
        fix_hint: Some("Run `lisa init` to scaffold missing hooks".to_string()),
    });
}

// Similar for on-clear.sh
```

### 6. Tests

Add tests for:
- `ON_STOP_HOOK` and `ON_CLEAR_HOOK` constants are well-formed shell scripts
- `settings_local_json()` produces valid JSON with all three hooks
- Hook merge logic preserves existing user hooks
- Validation detects missing hooks

## Acceptance Criteria

- [ ] `on-stop.sh` and `on-clear.sh` scripts defined in `templates.rs`
- [ ] `settings_local_json()` includes `Stop`, `SessionStart[clear]`, and `Notification[idle_prompt]` hooks
- [ ] Hook merge logic handles all three hook types without clobbering user hooks
- [ ] `lisa init` scaffolds the new hook scripts (executable, 0755 permissions)
- [ ] `lisa validate` checks for all three hooks and reports missing ones
- [ ] All tests pass (`cargo test --workspace`)

## Files Modified

- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-cli/src/init.rs`

## Notes

- The `Stop` hook fires on every turn completion (whenever Claude finishes responding)
- The `SessionStart[clear]` hook fires only when `/clear` is processed (not on `startup`, `resume`, or `compact`)
- Both hooks rely on the `LISA_PANE_ID` environment variable, which persists across `/clear` (same process)
- Signal files are deleted by the plugin after processing (handled in T-010-02)
