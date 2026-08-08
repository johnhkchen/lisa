use std::path::Path;

use crate::detect::{detect_project, ProjectType};

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
    let already_initialized =
        root.join(".lisa.toml").exists() && root.join("docs/active/tickets").exists();

    let body = if already_initialized {
        "Project is already initialized. Run `lisa init` again to update any stale files:\n\n\
         ```bash\n\
         lisa init\n\
         ```\n\n\
         This is safe to re-run — it only touches the files it created itself, and only \
         when they are out of date. Anything you wrote by hand it leaves alone."
            .to_string()
    } else {
        "Run `lisa init` from the project root to scaffold everything:\n\n\
         ```bash\n\
         lisa init\n\
         ```\n\n\
         This creates:\n\n\
         | Path | Purpose |\n\
         |------|--------|\n\
         | `.lisa.toml` | Lisa configuration (`max_threads`, etc.) |\n\
         | `docs/knowledge/rdspi-workflow.md` | RDSPI workflow definition (injected into agent sessions) |\n\
         | `docs/active/tickets/` | Ticket files (YAML frontmatter markdown) |\n\
         | `docs/active/stories/` | Story files (groups of related tickets) |\n\
         | `docs/active/work/` | Work artifacts, one subdirectory per ticket |\n\
         | `docs/archive/` | Completed tickets, stories, and work |\n\
         | `.lisa/hooks/` | Signal hooks (`on-idle.sh`, `on-stop.sh`, `on-clear.sh`, `on-heartbeat.sh`) |\n\
         | `.lisa/signals/` | Ephemeral signal files (gitignored) |\n\
         | `.claude/settings.local.json` | Claude Code hook integration |\n\
         | `.codex/hooks.json` | Codex hook integration |\n\n\
         `.lisa.toml` is the only file `lisa init` puts in the project root. Your agent \
         context file is not on this list, and Step 3 explains why."
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
         - `review_timeout_secs` — how long before a parked Review session gets a finish-up prompt\n\
         - `session_timeout_secs` — advisory budget for any single session (default: 3600s / 1h); \
         an over-budget session is flagged but never interrupted — reclamation requires \
         prolonged total silence (2x `stuck_threshold_secs`, i.e. no tool calls at all; \
         default bar: 40min, tolerating 30-minute silent test runs)\n\
         - `wind_down_secs` — how long a pane must be signal-silent before it can be \
         reused for a new ticket (default: 300s)\n\
         - `assignment_ack_timeout_secs` — positive deadline after submitting a tagged \
         recycled/recovery Codex prompt (default: 30s); timeout triggers one fresh-session \
         fallback, then an actionable terminal error if that fallback is not acknowledged\n\
         - `[scheduling.phase_timeouts]` — per-phase timeout overrides (e.g. `research = 300`)",
        default_content.trim()
    );

    GuideSection {
        title: "Configure .lisa.toml".to_string(),
        body,
    }
}

fn section_agent_context() -> GuideSection {
    let body = "Lisa does not write this file. If you used Lisa 0.4, `lisa init` used to \
        generate a `CLAUDE.md` for you and no longer does — that is deliberate, not a \
        missing step.\n\n\
        An agent context file is where a project states its standing intentions to every \
        model that will ever read it. Everything in it gets believed. A file Lisa guessed \
        at from your `Cargo.toml` is a poor thing to have believed, so Lisa leaves the \
        whole document to you.\n\n\
        The clients read it from the project root under their own names — Claude Code \
        reads `CLAUDE.md`, Codex reads `AGENTS.md`. Write one, both, or neither; agents \
        run fine without one.\n\n\
        If you do write one, the things worth putting in it:\n\n\
        - What the project is, in one line\n\
        - How to build, test, and lint it — the exact commands\n\
        - Where the source lives\n\
        - Conventions and decisions an agent would otherwise have to guess at\n\n\
        You do not need to mention Lisa or the RDSPI workflow. Lisa injects the workflow \
        into every session itself."
        .to_string();

    GuideSection {
        title: "Write your own agent context file".to_string(),
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
        - .lisa.toml and the RDSPI workflow file exist\n\
        - .lisa.toml is valid (if present)\n\
        - Hook scripts and settings.local.json are configured correctly\n\
        - Ticket frontmatter parses correctly\n\
        - DAG has no cycles or missing dependencies\n\
        - At least one ticket is in `phase: ready`\n\n\
        Fix any errors until validate passes cleanly."
        .to_string();

    GuideSection {
        title: "Validate the board".to_string(),
        body,
    }
}

fn section_handoff() -> GuideSection {
    let body = "The board is built — setup is done, and so is the setup agent's job.\n\n\
        Two things an agent must NOT do from here:\n\n\
        - **Do not implement the tickets.** Lisa runs each ticket through its phases, \
        review, and sealed completion record; work done by hand outside the loop gets \
        none of that.\n\
        - **Do not run `lisa loop`.** That command belongs to the person you're working \
        with — they run it themselves in a separate terminal pane, window, or tab, where \
        they can watch the dashboard and answer anything waiting on them.\n\n\
        If you are an agent, finish by telling them: the board is ready, and running \
        `lisa loop` in another terminal starts the work."
        .to_string();

    GuideSection {
        title: "Hand off to the person running the loop".to_string(),
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
         Lisa runs coding agents through your ticket board, so you don't have to approve every step by hand.\n\n\
         Lisa keeps the trail reviewable: an append-only attempt ledger records each run, \
         the completion journal seals every finished ticket — to a commit where the project \
         keeps history, or to tamper-evident content hashes where it doesn't — and each \
         ticket keeps its work documents.\n\n\
         This guide is for the agent (or person) setting the project up. Setup means \
         building the board: the tickets themselves are implemented later by sessions \
         `lisa loop` starts — not by whoever follows this guide.\n\n\
         Follow these steps to set up this project for lisa-loop. \
         Each step is self-contained — complete them in order.",
        project.name, type_label
    );

    let sections = vec![
        section_init(root),
        section_config(),
        section_agent_context(),
        section_ticket_format(),
        section_story_format(),
        section_dependencies(),
        section_validate(),
        section_handoff(),
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
        assert!(guide.contains("lisa init"));
        // The guide addresses the setup agent and ends with the operator
        // handoff — never instructing the agent to run the loop itself.
        assert!(guide.contains("not by whoever follows this guide"));
        assert!(guide.contains("Do not run `lisa loop`."));
        assert!(guide.contains("Hand off to the person running the loop"));
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
    }

    /// The guide names every file `lisa init` creates and no file it does not.
    /// The project's own agent context file is the operator's to write, and the
    /// guide says so rather than leaving a 0.4 operator to wonder.
    #[test]
    fn test_guide_leaves_the_context_file_to_the_operator() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"context-owner\"\n",
        )
        .unwrap();

        let guide = build_guide(dir.path()).unwrap();

        // No table row claiming Lisa creates a context file.
        assert!(!guide.contains("| `CLAUDE.md` |"));
        assert!(!guide.contains("| `AGENTS.md` |"));
        // The one root file init writes is still named.
        assert!(guide.contains("| `.lisa.toml` |"));
        // And the guide says the second part on purpose.
        assert!(guide.contains("Write your own agent context file"));
        assert!(guide.contains("Lisa does not write this file."));
        // A removed config key must not survive in operator-facing copy.
        assert!(!guide.contains("auto_advance"));
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
    fn test_guide_names_the_existing_review_trail() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("append-only attempt ledger"));
        assert!(guide.contains("completion journal seals every finished ticket"));
        assert!(guide.contains("tamper-evident content hashes where it doesn't"));
        assert!(guide.contains("each ticket keeps its work documents"));
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
    fn test_guide_ends_with_operator_handoff() {
        let dir = tempfile::tempdir().unwrap();

        let guide = build_guide(dir.path()).unwrap();
        assert!(guide.contains("lisa validate"));
        let last_step = guide.rfind("## Step").unwrap();
        let last_section = &guide[last_step..];
        assert!(last_section.contains("Hand off to the person running the loop"));
        assert!(last_section.contains("Do not run `lisa loop`."));
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
