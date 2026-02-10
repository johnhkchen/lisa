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

if [ -n "$LISA_TICKET_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/$LISA_TICKET_ID.idle"
fi
"#;

/// Gitignore content for the .lisa/ directory — ignores ephemeral signal files.
pub const LISA_GITIGNORE: &str = "signals/\n";

/// Generate .claude/settings.local.json with the idle_prompt notification hook.
pub fn settings_local_json() -> String {
    r#"{
  "hooks": {
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
        assert!(ON_IDLE_HOOK.contains("LISA_TICKET_ID"));
        assert!(ON_IDLE_HOOK.contains(".lisa/signals"));
        assert!(ON_IDLE_HOOK.contains(".idle"));
    }

    #[test]
    fn test_settings_local_json() {
        let json = settings_local_json();
        assert!(json.contains("idle_prompt"));
        assert!(json.contains("on-idle.sh"));
        assert!(json.contains("Notification"));
        assert!(json.contains(r#""type": "command""#));
    }

    #[test]
    fn test_lisa_gitignore_content() {
        assert!(LISA_GITIGNORE.contains("signals/"));
    }
}
