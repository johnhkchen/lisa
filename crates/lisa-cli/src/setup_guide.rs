use std::path::Path;

use crate::config;
use crate::detect::{detect_project, DetectedProject, ProjectType};
use crate::templates;

struct GuideSection {
    title: String,
    body: String,
}

fn render_guide(header: &str, sections: Vec<GuideSection>) -> String {
    let mut out = String::new();
    out.push_str(header);
    out.push_str("\n\n");

    for (i, section) in sections.iter().enumerate() {
        out.push_str(&format!("## Step {}: {}\n\n", i + 1, section.title));
        out.push_str(&section.body);
        out.push_str("\n\n");
    }

    out
}

fn section_directories(root: &Path) -> GuideSection {
    let dirs = [
        "docs/active/tickets",
        "docs/active/stories",
        "docs/active/work",
        "docs/archive/tickets",
        "docs/archive/stories",
        "docs/archive/work",
    ];

    let already_exists = root.join("docs/active/tickets").exists();

    let body = if already_exists {
        "Directory structure already exists. Verify these directories are present:\n\n\
         ```\n\
         docs/active/tickets/\n\
         docs/active/stories/\n\
         docs/active/work/\n\
         docs/archive/tickets/\n\
         docs/archive/stories/\n\
         docs/archive/work/\n\
         ```"
        .to_string()
    } else {
        let cmds: Vec<String> = dirs.iter().map(|d| format!("mkdir -p {}", d)).collect();
        format!(
            "Create the lisa directory structure:\n\n\
             ```bash\n\
             {}\n\
             ```",
            cmds.join("\n")
        )
    };

    GuideSection {
        title: "Create directory structure".to_string(),
        body,
    }
}

fn section_config(root: &Path) -> GuideSection {
    let config_path = root.join(".lisa.toml");
    let default_content = config::default_config_toml();

    let body = if config_path.exists() {
        format!(
            "`.lisa.toml` already exists. Verify it contains valid configuration.\n\n\
             Default configuration for reference:\n\n\
             ```toml\n\
             {}\n\
             ```",
            default_content.trim()
        )
    } else {
        format!(
            "Create `.lisa.toml` in the project root with this content:\n\n\
             ```toml\n\
             {}\n\
             ```\n\n\
             - `max_threads` controls how many Claude Code sessions run concurrently\n\
             - `auto_advance` when true skips review pauses between RDSPI phases",
            default_content.trim()
        )
    };

    GuideSection {
        title: "Create .lisa.toml".to_string(),
        body,
    }
}

fn section_claude_md(root: &Path, project: &DetectedProject) -> GuideSection {
    let claude_md_path = root.join("CLAUDE.md");
    let template = templates::generate_claude_md(project);

    let body = if claude_md_path.exists() {
        "CLAUDE.md already exists. Review it and ensure it contains:\n\n\
         - Project description (one line)\n\
         - Build, test, and lint commands\n\
         - Source layout overview\n\
         - Directory conventions for docs/active/ and docs/archive/\n\n\
         The RDSPI workflow reference should point to `docs/rdspi-workflow.md`."
            .to_string()
    } else {
        format!(
            "Create `CLAUDE.md` in the project root. This file tells Claude Code about your project.\n\n\
             Use this template as a starting point — edit the TODO line and add any project-specific \
             context (architecture decisions, conventions, important modules):\n\n\
             ```markdown\n\
             {}\n\
             ```",
            template.trim()
        )
    };

    GuideSection {
        title: "Create CLAUDE.md".to_string(),
        body,
    }
}

fn section_rdspi_workflow() -> GuideSection {
    let body = format!(
        "Create `docs/rdspi-workflow.md` with the full RDSPI workflow definition below. \
         Lisa injects this into each agent session automatically.\n\n\
         ```markdown\n\
         {}\n\
         ```",
        templates::RDSPI_WORKFLOW.trim()
    );

    GuideSection {
        title: "Create RDSPI workflow file".to_string(),
        body,
    }
}

fn section_ticket_format() -> GuideSection {
    let body = "Tickets live in `docs/active/tickets/`. Each ticket is a markdown file with YAML frontmatter.\n\n\
        ### Required frontmatter fields\n\n\
        ```yaml\n\
        ---\n\
        id: T-001-01          # Unique ID. Convention: T-{story}-{sequence}\n\
        story: S-001           # Parent story ID\n\
        title: kebab-case-name # Short descriptive name\n\
        type: task             # task | bug | spike\n\
        status: open           # open | in-progress | review | done | blocked\n\
        priority: high         # critical | high | medium | low\n\
        phase: ready           # ready | research | design | structure | plan | implement | review | done\n\
        depends_on: []         # List of ticket IDs that must complete first\n\
        ---\n\
        ```\n\n\
        ### Body structure\n\n\
        ```markdown\n\
        ## Context\n\
        \n\
        Why this work matters and what it accomplishes.\n\
        \n\
        ## Acceptance Criteria\n\
        \n\
        - [ ] Concrete, verifiable condition 1\n\
        - [ ] Concrete, verifiable condition 2\n\
        ```\n\n\
        ### Rules\n\n\
        - Every ticket starts at `phase: ready` and `status: open`\n\
        - `depends_on` lists ticket IDs that must reach `phase: done` before this ticket starts\n\
        - `blocks` is optional — lisa computes it automatically from `depends_on`\n\
        - Ticket filenames should match the ID: `T-001-01.md`\n\
        - Each ticket gets a work directory: `docs/active/work/{ticket-id}/`"
        .to_string();

    GuideSection {
        title: "Write tickets".to_string(),
        body,
    }
}

fn section_story_format() -> GuideSection {
    let body = "Stories live in `docs/active/stories/`. A story groups related tickets into a coherent unit of work.\n\n\
        ### Frontmatter\n\n\
        ```yaml\n\
        ---\n\
        id: S-001\n\
        title: descriptive-story-name\n\
        type: story\n\
        status: open\n\
        priority: high\n\
        tickets: [T-001-01, T-001-02, T-001-03]\n\
        ---\n\
        ```\n\n\
        ### Body structure\n\n\
        - Narrative description of the goal\n\
        - Track breakdown (group tickets by parallel execution tracks)\n\
        - DAG visualization showing which tickets can run concurrently\n\n\
        ### When to use stories\n\n\
        - Every ticket belongs to a story\n\
        - A story typically contains 2-6 tickets\n\
        - Stories represent a sprint or feature — something that ships as a unit\n\
        - The `tickets` list in the story frontmatter should match the ticket files"
        .to_string();

    GuideSection {
        title: "Write stories".to_string(),
        body,
    }
}

fn section_dependencies() -> GuideSection {
    let body = "Lisa computes a DAG from ticket `depends_on` fields and schedules work concurrently.\n\n\
        ### Key rules\n\n\
        - Tickets with no dependencies (or all dependencies done) are scheduled immediately\n\
        - Multiple tickets run in parallel up to `max_threads`\n\
        - Commit serialization is handled via file locking — agents don't coordinate with each other\n\n\
        ### The critical rule\n\n\
        **If two tickets modify the same files, add a dependency edge between them.**\n\n\
        The commit lock is a safety net, not a substitute for correct dependency modeling. \
        If ticket A and ticket B both modify `src/lib.rs`, one must `depends_on` the other. \
        Otherwise they will produce conflicting changes.\n\n\
        ### Modeling tips\n\n\
        - Foundational work (types, interfaces, shared modules) should be early tickets that others depend on\n\
        - Independent features that touch different files can run in parallel\n\
        - Use the DAG to maximize concurrency: more independent tickets = faster execution"
        .to_string();

    GuideSection {
        title: "Model dependencies".to_string(),
        body,
    }
}

fn section_archiving() -> GuideSection {
    let body = "When a ticket or story is complete (`phase: done`, `status: done`):\n\n\
        ```bash\n\
        # Archive a completed ticket\n\
        mv docs/active/tickets/T-001-01.md docs/archive/tickets/\n\
        mv docs/active/work/T-001-01/ docs/archive/work/\n\
        \n\
        # Archive a completed story (when all its tickets are done)\n\
        mv docs/active/stories/S-001.md docs/archive/stories/\n\
        ```\n\n\
        This keeps the active directory clean. Lisa only scans `docs/active/tickets/` for scheduling."
        .to_string();

    GuideSection {
        title: "Archiving completed work".to_string(),
        body,
    }
}

fn section_validate() -> GuideSection {
    let body = "When you have created your directories, config, CLAUDE.md, stories, and tickets, run:\n\n\
        ```bash\n\
        lisa validate\n\
        ```\n\n\
        This checks:\n\
        - CLAUDE.md exists\n\
        - RDSPI workflow file exists\n\
        - .lisa.toml is valid (if present)\n\
        - Required directories exist\n\
        - Ticket frontmatter parses correctly\n\
        - DAG has no cycles or missing dependencies\n\n\
        Fix any errors before running `lisa loop`."
        .to_string();

    GuideSection {
        title: "Validate your setup".to_string(),
        body,
    }
}

fn project_type_label(pt: &ProjectType) -> &'static str {
    match pt {
        ProjectType::Rust => "Rust",
        ProjectType::Node => "Node.js",
        ProjectType::Go => "Go",
        ProjectType::Python => "Python",
        ProjectType::Unknown => "unknown",
    }
}

fn build_guide(root: &Path) -> Result<String, String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let project = detect_project(root);
    let type_label = project_type_label(&project.project_type);

    let header = format!(
        "# Lisa Setup Guide for {} ({})\n\n\
         Follow these steps to set up this project for lisa-loop. \
         Each step is self-contained — complete them in order.",
        project.name, type_label
    );

    let sections = vec![
        section_directories(root),
        section_config(root),
        section_claude_md(root, &project),
        section_rdspi_workflow(),
        section_ticket_format(),
        section_story_format(),
        section_dependencies(),
        section_archiving(),
        section_validate(),
    ];

    Ok(render_guide(&header, sections))
}

pub fn run_setup_guide(root: &Path) -> Result<(), String> {
    let guide = build_guide(root)?;
    print!("{}", guide);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_guide_rust_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-rust-app\"\n",
        )
        .unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("my-rust-app"));
        assert!(guide.contains("Rust"));
        assert!(guide.contains("cargo build"));
        assert!(guide.contains("cargo test"));
        assert!(guide.contains("mkdir -p"));
    }

    #[test]
    fn test_guide_node_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"my-node-app\"\n}\n",
        )
        .unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("my-node-app"));
        assert!(guide.contains("Node.js"));
        assert!(guide.contains("npm run build"));
        assert!(guide.contains("npm test"));
    }

    #[test]
    fn test_guide_unknown_project() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("unknown"));
        // Should still have ticket format, workflow, etc.
        assert!(guide.contains("depends_on"));
        assert!(guide.contains("RDSPI"));
    }

    #[test]
    fn test_guide_already_initialized() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\n",
        )
        .unwrap();

        // Simulate `lisa init` having run
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# existing").unwrap();
        fs::write(dir.path().join(".lisa.toml"), "[scheduling]\nmax_threads = 2\n").unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("already exists"));
        // Should NOT contain mkdir commands when dirs exist
        assert!(!guide.contains("mkdir -p"));
    }

    #[test]
    fn test_guide_contains_rdspi_workflow() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("Research"));
        assert!(guide.contains("Design"));
        assert!(guide.contains("Structure"));
        assert!(guide.contains("Plan"));
        assert!(guide.contains("Implement"));
        assert!(guide.contains("~200 lines"));
    }

    #[test]
    fn test_guide_contains_ticket_format() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("depends_on"));
        assert!(guide.contains("phase: ready"));
        assert!(guide.contains("status: open"));
        assert!(guide.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_guide_step_numbering() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("## Step 1:"));
        assert!(guide.contains("## Step 2:"));
        assert!(guide.contains("## Step 3:"));
        assert!(guide.contains("## Step 9:"));
    }

    #[test]
    fn test_guide_nonexistent_path() {
        let result = build_guide(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn test_guide_ends_with_validate() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        // The last step should be about validation
        assert!(guide.contains("lisa validate"));
        // Find the last "## Step" and verify it's about validation
        let last_step = guide.rfind("## Step").unwrap();
        let last_section = &guide[last_step..];
        assert!(last_section.contains("Validate"));
    }
}
