use crate::detect::DetectedProject;

/// The RDSPI workflow document, embedded at compile time
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");

/// The compiled WASM plugin, embedded at compile time via build.rs
pub const PLUGIN_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"));

/// The on-idle hook script, called by Claude Code's idle_prompt notification.
/// Writes a signal file so the plugin knows which session finished its work.
pub const ON_IDLE_HOOK: &str = r#"#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification.
# Writes a signal file so the plugin knows this session finished its work.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
fi
"#;

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

/// Gitignore content for the .lisa/ directory — ignores ephemeral signal files.
pub const LISA_GITIGNORE: &str = "signals/\n";

/// Generate .claude/settings.local.json with Stop, SessionStart, and Notification hooks.
pub fn settings_local_json() -> String {
    r#"{
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
"#
    .to_string()
}

/// Ensure a single hook entry exists in the hooks object.
/// For hooks with a matcher (SessionStart, Notification), deduplication checks the matcher value.
/// For hooks without a matcher (Stop), deduplication checks the command path.
fn ensure_hook(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event_type: &str,
    matcher: Option<&str>,
    command: &str,
) {
    let entries = hooks_obj
        .entry(event_type)
        .or_insert_with(|| serde_json::json!([]));
    let arr = match entries.as_array_mut() {
        Some(a) => a,
        None => return,
    };

    let already_exists = match matcher {
        Some(m) => arr
            .iter()
            .any(|entry| entry.get("matcher").and_then(|v| v.as_str()) == Some(m)),
        None => arr.iter().any(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map_or(false, |hooks| {
                    hooks
                        .iter()
                        .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(command))
                })
        }),
    };

    if !already_exists {
        let mut entry = serde_json::Map::new();
        if let Some(m) = matcher {
            entry.insert("matcher".to_string(), serde_json::json!(m));
        }
        entry.insert(
            "hooks".to_string(),
            serde_json::json!([{
                "type": "command",
                "command": command
            }]),
        );
        arr.push(serde_json::Value::Object(entry));
    }
}

/// Merge all Lisa hooks (Stop, SessionStart[clear], Notification[idle_prompt]) into
/// an existing settings.local.json. Returns the updated JSON string, or an error if
/// the JSON is malformed.
pub fn merge_hooks(existing_json: &str) -> Result<String, String> {
    let mut root: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| format!("invalid JSON in settings.local.json: {}", e))?;

    let obj = root
        .as_object_mut()
        .ok_or("settings.local.json root is not an object")?;

    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or("settings.local.json 'hooks' is not an object")?;

    ensure_hook(hooks_obj, "Stop", None, ".lisa/hooks/on-stop.sh");
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("clear"),
        ".lisa/hooks/on-clear.sh",
    );
    ensure_hook(
        hooks_obj,
        "Notification",
        Some("idle_prompt"),
        ".lisa/hooks/on-idle.sh",
    );

    serde_json::to_string_pretty(&root)
        .map_err(|e| format!("failed to serialize JSON: {}", e))
}

/// Generate a project-specific CLAUDE.md
pub fn generate_claude_md(project: &DetectedProject) -> String {
    use crate::detect::ProjectType;

    let type_label = match project.project_type {
        ProjectType::Rust => "Rust",
        ProjectType::Node => "Node.js",
        ProjectType::Go => "Go",
        ProjectType::Python => "Python",
        ProjectType::Unknown => "unknown type",
    };

    let build_section = if project.build_command.is_empty() {
        String::new()
    } else {
        format!(
            r#"### Build and Test

```bash
# Build
{}

# Run tests
{}

# Lint
{}
```
"#,
            project.build_command, project.test_command, project.lint_command
        )
    };

    let source_layout_section = if project.source_layout.is_empty() {
        String::new()
    } else {
        format!(
            r#"### Source Layout

```
{}
```
"#,
            project.source_layout
        )
    };

    format!(
        r#"# CLAUDE.md

## Project

{name} ({type_label}) — TODO: add a one-line project description here.

{build_section}
{source_layout_section}
### Directory Conventions

```
docs/active/tickets/    # Ticket files (markdown with YAML frontmatter)
docs/active/stories/    # Story files (same frontmatter pattern)
docs/active/work/       # Work artifacts, one subdirectory per ticket ID
```

---

The RDSPI workflow definition is in docs/rdspi-workflow.md and is injected into agent context by lisa automatically.
"#,
        name = project.name,
        type_label = type_label,
        build_section = build_section,
        source_layout_section = source_layout_section,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::ProjectType;

    #[test]
    fn test_rdspi_workflow_embedded() {
        assert!(RDSPI_WORKFLOW.contains("RDSPI Workflow"));
        assert!(RDSPI_WORKFLOW.contains("Research"));
        assert!(RDSPI_WORKFLOW.contains("Design"));
        assert!(RDSPI_WORKFLOW.contains("Structure"));
        assert!(RDSPI_WORKFLOW.contains("Plan"));
        assert!(RDSPI_WORKFLOW.contains("Implement"));
    }

    #[test]
    fn test_generate_claude_md_rust() {
        let project = DetectedProject {
            project_type: ProjectType::Rust,
            name: "my-app".to_string(),
            build_command: "cargo build".to_string(),
            test_command: "cargo test".to_string(),
            lint_command: "cargo clippy".to_string(),
            source_layout: "src:\n  lib.rs\n  main.rs".to_string(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("# CLAUDE.md"));
        assert!(result.contains("my-app"));
        assert!(result.contains("(Rust)"));
        assert!(result.contains("cargo build"));
        assert!(result.contains("cargo test"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("docs/active/tickets/"));
        assert!(result.contains("docs/rdspi-workflow.md"));
    }

    #[test]
    fn test_generate_claude_md_node() {
        let project = DetectedProject {
            project_type: ProjectType::Node,
            name: "my-node-app".to_string(),
            build_command: "npm run build".to_string(),
            test_command: "npm test".to_string(),
            lint_command: "npm run lint".to_string(),
            source_layout: "src:\n  index.ts".to_string(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("my-node-app"));
        assert!(result.contains("(Node.js)"));
        assert!(result.contains("npm run build"));
        assert!(result.contains("npm test"));
    }

    #[test]
    fn test_generate_claude_md_unknown() {
        let project = DetectedProject {
            project_type: ProjectType::Unknown,
            name: "mystery".to_string(),
            build_command: String::new(),
            test_command: String::new(),
            lint_command: String::new(),
            source_layout: String::new(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("mystery"));
        assert!(result.contains("(unknown type)"));
        // Should still have directory conventions
        assert!(result.contains("docs/active/tickets/"));
    }

    #[test]
    fn test_on_idle_hook_content() {
        assert!(ON_IDLE_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_IDLE_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_IDLE_HOOK.contains(".lisa/signals"));
        assert!(ON_IDLE_HOOK.contains(".idle"));
    }

    #[test]
    fn test_on_stop_hook_content() {
        assert!(ON_STOP_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_STOP_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_STOP_HOOK.contains(".lisa/signals"));
        assert!(ON_STOP_HOOK.contains(".stopped"));
    }

    #[test]
    fn test_on_clear_hook_content() {
        assert!(ON_CLEAR_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_CLEAR_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_CLEAR_HOOK.contains(".lisa/signals"));
        assert!(ON_CLEAR_HOOK.contains(".cleared"));
    }

    #[test]
    fn test_settings_local_json() {
        let json = settings_local_json();
        // All three hook types present
        assert!(json.contains("\"Stop\""));
        assert!(json.contains("\"SessionStart\""));
        assert!(json.contains("\"Notification\""));
        // Hook commands
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-idle.sh"));
        // Matchers
        assert!(json.contains("\"clear\""));
        assert!(json.contains("idle_prompt"));
        assert!(json.contains(r#""type": "command""#));
    }

    #[test]
    fn test_lisa_gitignore_content() {
        assert!(LISA_GITIGNORE.contains("signals/"));
    }

    #[test]
    fn test_merge_hooks_empty_object() {
        let result = merge_hooks("{}").unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("\"Notification\""));
        assert!(result.contains("idle_prompt"));
        assert!(result.contains("on-idle.sh"));
    }

    #[test]
    fn test_merge_hooks_with_existing_idle_only() {
        // Start with only idle_prompt — should add Stop + SessionStart
        let input = r#"{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        // idle_prompt should not be duplicated
        assert_eq!(result.matches("idle_prompt").count(), 1);
    }

    #[test]
    fn test_merge_hooks_already_complete() {
        let input = settings_local_json();
        let result = merge_hooks(&input).unwrap();
        // No duplicates
        assert_eq!(result.matches("on-stop.sh").count(), 1);
        assert_eq!(result.matches("on-clear.sh").count(), 1);
        assert_eq!(result.matches("idle_prompt").count(), 1);
    }

    #[test]
    fn test_merge_hooks_preserves_permissions() {
        let input = r#"{"permissions":{"allow":["Bash(cargo test:*)"]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("cargo test"));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("idle_prompt"));
    }

    #[test]
    fn test_merge_hooks_invalid_json() {
        let result = merge_hooks("not json");
        assert!(result.is_err());
    }
}
