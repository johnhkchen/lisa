use crate::detect::DetectedProject;

/// The RDSPI workflow document, embedded at compile time
pub const RDSPI_WORKFLOW: &str = include_str!("../../../docs/knowledge/rdspi-workflow.md");

/// Generate a project-specific CLAUDE.md
pub fn generate_claude_md(project: &DetectedProject) -> String {
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

{name} — TODO: add a one-line project description here.

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
        assert!(result.contains("cargo build"));
        assert!(result.contains("cargo test"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("docs/active/tickets/"));
        assert!(result.contains("docs/rdspi-workflow.md"));
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
        // Should still have directory conventions
        assert!(result.contains("docs/active/tickets/"));
    }
}
