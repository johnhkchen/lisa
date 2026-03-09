use std::path::Path;

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

fn section_init(root: &Path) -> GuideSection {
    let already_initialized = root.join("CLAUDE.md").exists()
        && root.join(".lisa.toml").exists()
        && root.join("docs/active/tickets").exists();

    let body = if already_initialized {
        "Project is already initialized. Run `lisa init` again to update any stale files:\n\n\
         ```bash\n\
         lisa init\n\
         ```\n\n\
         This is safe to re-run — it never overwrites CLAUDE.md and only updates files that are out of date."
            .to_string()
    } else {
        "Run `lisa init` from the project root to scaffold everything:\n\n\
         ```bash\n\
         lisa init\n\
         ```\n\n\
         This creates:\n\n\
         | Path | Purpose |\n\
         |------|--------|\n\
         | `CLAUDE.md` | Project context for Claude Code |\n\
         | `.lisa.toml` | Lisa configuration (`max_threads`, etc.) |\n\
         | `docs/knowledge/rdspi-workflow.md` | RDSPI workflow definition (injected into agent sessions) |\n\
         | `docs/active/tickets/` | Ticket files (YAML frontmatter markdown) |\n\
         | `docs/active/stories/` | Story files (groups of related tickets) |\n\
         | `docs/active/work/` | Work artifacts, one subdirectory per ticket |\n\
         | `docs/archive/` | Completed tickets, stories, and work |\n\
         | `.lisa/hooks/` | Signal hooks (`on-idle.sh`, `on-stop.sh`, `on-clear.sh`) |\n\
         | `.lisa/signals/` | Ephemeral signal files (gitignored) |\n\
         | `.claude/settings.local.json` | Claude Code hook integration |\n\n\
         After running, edit `CLAUDE.md` to add your project description, build commands, and source layout."
            .to_string()
    };

    GuideSection {
        title: "Initialize the project".to_string(),
        body,
    }
}

fn section_config() -> GuideSection {
    let default_content = crate::config::default_config_toml();

    let body = format!(
        "`lisa init` creates `.lisa.toml` with these defaults:\n\n\
         ```toml\n\
         {}\n\
         ```\n\n\
         - `max_threads` — how many Claude Code sessions run concurrently\n\
         - `auto_advance` — when true, skips review pauses between RDSPI phases\n\
         - `review_timeout_secs` — how long before a parked Review session gets a finish-up prompt\n\
         - `session_timeout_secs` — global timeout for any single session (default: 1800s / 30min)\n\
         - `[scheduling.phase_timeouts]` — per-phase timeout overrides (e.g. `research = 300`)",
        default_content.trim()
    );

    GuideSection {
        title: "Configure .lisa.toml".to_string(),
        body,
    }
}

fn section_claude_md(_root: &Path, project: &DetectedProject) -> GuideSection {
    let template = templates::generate_claude_md(project);

    let body = format!(
        "`lisa init` generates a CLAUDE.md template for your project type. \
         Edit it to include:\n\n\
         - Project description (one line)\n\
         - Build, test, and lint commands\n\
         - Source layout overview\n\
         - Any project-specific conventions or architecture decisions\n\n\
         Generated template:\n\n\
         ```markdown\n\
         {}\n\
         ```",
        template.trim()
    );

    GuideSection {
        title: "Edit CLAUDE.md".to_string(),
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
        type: task             # task | bug | feature | spike | chore\n\
        status: open           # open | in_progress | blocked | review | done | cancelled\n\
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
        - Lisa computes the reverse (`blocks`) automatically from `depends_on`\n\
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

fn section_validate() -> GuideSection {
    let body = "When you have created your tickets and stories, validate the setup:\n\n\
        ```bash\n\
        lisa validate\n\
        ```\n\n\
        This checks:\n\
        - CLAUDE.md and RDSPI workflow file exist\n\
        - .lisa.toml is valid (if present)\n\
        - Hook scripts and settings.local.json are configured correctly\n\
        - Ticket frontmatter parses correctly\n\
        - DAG has no cycles or missing dependencies\n\
        - At least one ticket is in `phase: ready`\n\n\
        Fix any errors, then launch with `lisa loop`."
        .to_string();

    GuideSection {
        title: "Validate and launch".to_string(),
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
        section_init(root),
        section_config(),
        section_claude_md(root, &project),
        section_ticket_format(),
        section_story_format(),
        section_dependencies(),
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
        assert!(guide.contains("lisa init"));
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
        fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 2\n",
        )
        .unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("already initialized"));
        // Should NOT contain mkdir commands when initialized
        assert!(!guide.contains("mkdir -p"));
    }

    #[test]
    fn test_guide_references_rdspi() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        // Guide should reference the RDSPI workflow file and phases
        assert!(guide.contains("rdspi-workflow.md"));
        assert!(guide.contains("research"));
        assert!(guide.contains("design"));
        assert!(guide.contains("implement"));
        assert!(guide.contains("review"));
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
        assert!(guide.contains("## Step 7:"));
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
        assert!(guide.contains("lisa validate"));
        let last_step = guide.rfind("## Step").unwrap();
        let last_section = &guide[last_step..];
        assert!(last_section.contains("Validate"));
    }

    #[test]
    fn test_guide_mentions_hooks() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains(".lisa/hooks/"));
        assert!(guide.contains("settings.local.json"));
    }

    #[test]
    fn test_guide_correct_type_values() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        // Should list the correct type enum values
        assert!(guide.contains("task | bug | feature | spike | chore"));
        // Should list correct status values
        assert!(guide.contains("open | in_progress | blocked | review | done | cancelled"));
    }
}
