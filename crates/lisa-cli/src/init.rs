use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::detect::{detect_project, DetectedProject};
use crate::templates;

/// An action that init will perform
#[derive(Debug, Clone)]
pub enum InitAction {
    CreateDir(PathBuf),
    CreateFile { path: PathBuf, content: String },
    Skip { path: PathBuf, reason: String },
}

impl fmt::Display for InitAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitAction::CreateDir(path) => write!(f, "  create  {}/", path.display()),
            InitAction::CreateFile { path, .. } => write!(f, "  create  {}", path.display()),
            InitAction::Skip { path, reason } => {
                write!(f, "  skip    {} ({})", path.display(), reason)
            }
        }
    }
}

/// Plan what init should do without executing anything
pub fn plan_init_actions(root: &Path, project: &DetectedProject) -> Vec<InitAction> {
    let mut actions = Vec::new();

    // Directories to create
    let dirs = [
        "docs/active/tickets",
        "docs/active/stories",
        "docs/active/work",
        "docs/archive/tickets",
        "docs/archive/stories",
        "docs/archive/work",
    ];

    for dir in &dirs {
        let path = root.join(dir);
        if path.exists() {
            actions.push(InitAction::Skip {
                path,
                reason: "already exists".to_string(),
            });
        } else {
            actions.push(InitAction::CreateDir(path));
        }
    }

    // CLAUDE.md
    let claude_md_path = root.join("CLAUDE.md");
    if claude_md_path.exists() {
        actions.push(InitAction::Skip {
            path: claude_md_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: claude_md_path,
            content: templates::generate_claude_md(project),
        });
    }

    // docs/rdspi-workflow.md
    let workflow_path = root.join("docs/rdspi-workflow.md");
    if workflow_path.exists() {
        actions.push(InitAction::Skip {
            path: workflow_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: workflow_path,
            content: templates::RDSPI_WORKFLOW.to_string(),
        });
    }

    // .lisa.toml
    let config_path = root.join(".lisa.toml");
    if config_path.exists() {
        actions.push(InitAction::Skip {
            path: config_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: config_path,
            content: config::default_config_toml().to_string(),
        });
    }

    actions
}

/// Execute the init command
pub fn run_init(root: &Path, dry_run: bool) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    // Step 1: Detect project type
    let project = detect_project(root);
    println!(
        "Detected project: {} ({})",
        project.name,
        match &project.project_type {
            crate::detect::ProjectType::Rust => "Rust",
            crate::detect::ProjectType::Node => "Node.js",
            crate::detect::ProjectType::Go => "Go",
            crate::detect::ProjectType::Python => "Python",
            crate::detect::ProjectType::Unknown => "unknown",
        }
    );
    println!();

    // Step 2: Plan actions
    let actions = plan_init_actions(root, &project);

    // Step 3: Print the plan
    println!("Planned actions:");
    for action in &actions {
        println!("{}", action);
    }
    println!();

    // Step 4: Dry run stops here
    if dry_run {
        println!("Dry run complete. No changes made.");
        return Ok(());
    }

    // Step 5: Execute
    for action in &actions {
        match action {
            InitAction::CreateDir(path) => {
                fs::create_dir_all(path).map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
            }
            InitAction::CreateFile { path, content } => {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
                fs::write(path, content)
                    .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
            }
            InitAction::Skip { .. } => {}
        }
    }

    println!("Initialization complete.");
    println!();

    // Step 6: Run validation
    run_validate(root)
}

/// Run validation on the project setup
pub fn run_validate(root: &Path) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check CLAUDE.md exists
    if !root.join("CLAUDE.md").exists() {
        errors.push("CLAUDE.md not found. Run `lisa init` to create it.".to_string());
    }

    // Check workflow file exists
    if !root.join("docs/rdspi-workflow.md").exists() {
        warnings.push("docs/rdspi-workflow.md not found. Run `lisa init` to create it.".to_string());
    }

    // Validate .lisa.toml if present
    let config_path = root.join(".lisa.toml");
    if config_path.exists() {
        match config::load_config(root) {
            Ok(_) => {
                println!(".lisa.toml: valid");
            }
            Err(e) => {
                errors.push(format!(".lisa.toml: {}", e));
            }
        }
    }

    // Check directory structure
    let required_dirs = ["docs/active/tickets", "docs/active/stories", "docs/active/work"];
    for dir in &required_dirs {
        if !root.join(dir).exists() {
            warnings.push(format!("{} directory not found. Run `lisa init` to create it.", dir));
        }
    }

    // Scan and validate tickets if the directory exists
    let ticket_dir = root.join("docs/active/tickets");
    if ticket_dir.exists() {
        match lisa_core::ticket::scan_tickets(&ticket_dir) {
            Ok(tickets) => {
                if tickets.is_empty() {
                    println!("No tickets found in docs/active/tickets/");
                } else {
                    println!("Found {} ticket(s)", tickets.len());

                    // Check for acceptance criteria
                    for ticket in &tickets {
                        if !ticket.content.contains("Acceptance Criteria")
                            && !ticket.content.contains("acceptance criteria")
                        {
                            warnings.push(format!(
                                "Ticket {} is missing acceptance criteria",
                                ticket.id
                            ));
                        }
                    }

                    // Build DAG and check for issues
                    match lisa_core::dag::Dag::from_tickets(tickets) {
                        Ok(dag) => {
                            // Check for cycles
                            match dag.detect_cycles() {
                                lisa_core::dag::CycleDetectionResult::NoCycle => {
                                    println!("DAG validation: no cycles detected");
                                }
                                lisa_core::dag::CycleDetectionResult::Cycle(nodes) => {
                                    errors.push(format!(
                                        "Cycle detected involving tickets: {}",
                                        nodes.join(", ")
                                    ));
                                }
                            }

                            // Show DAG stats
                            let stats = dag.stats();
                            println!(
                                "DAG stats: {} total, {} ready, {} in progress, {} done",
                                stats.total_tickets,
                                stats.ready_tickets,
                                stats.in_progress_tickets,
                                stats.done_tickets,
                            );
                            if stats.critical_path_length > 0 {
                                println!("Critical path length: {}", stats.critical_path_length);
                            }
                        }
                        Err(lisa_core::dag::DagError::MissingDependency {
                            ticket_id,
                            missing_dep,
                        }) => {
                            errors.push(format!(
                                "Ticket {} depends on {} which does not exist",
                                ticket_id, missing_dep
                            ));
                        }
                        Err(lisa_core::dag::DagError::CycleDetected(nodes)) => {
                            errors.push(format!(
                                "Cycle detected involving tickets: {}",
                                nodes.join(", ")
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("Could not scan tickets: {}", e));
            }
        }
    }

    // Print results
    if !warnings.is_empty() {
        println!();
        println!("Warnings:");
        for w in &warnings {
            println!("  ! {}", w);
        }
    }

    if !errors.is_empty() {
        println!();
        println!("Errors:");
        for e in &errors {
            println!("  x {}", e);
        }
        return Err(format!("{} error(s) found", errors.len()));
    }

    println!();
    println!("Validation passed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_plan_init_actions_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // Should plan to create 6 directories + 3 files (CLAUDE.md, rdspi-workflow.md, .lisa.toml)
        let creates: Vec<_> = actions
            .iter()
            .filter(|a| !matches!(a, InitAction::Skip { .. }))
            .collect();
        assert_eq!(creates.len(), 9);
    }

    #[test]
    fn test_plan_init_actions_existing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "existing").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // CLAUDE.md should be skipped
        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::Skip { path, .. } if path.ends_with("CLAUDE.md")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_plan_init_actions_existing_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "[scheduling]\nmax_threads = 4\n").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // .lisa.toml should be skipped
        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::Skip { path, .. } if path.ends_with(".lisa.toml")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_run_init_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        let result = run_init(dir.path(), true);
        assert!(result.is_ok());

        // Dry run should not create any files
        assert!(!dir.path().join("CLAUDE.md").exists());
        assert!(!dir.path().join("docs/active/tickets").exists());
        assert!(!dir.path().join(".lisa.toml").exists());
    }

    #[test]
    fn test_run_init_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // Should create all directories and files
        assert!(dir.path().join("CLAUDE.md").exists());
        assert!(dir.path().join("docs/rdspi-workflow.md").exists());
        assert!(dir.path().join(".lisa.toml").exists());
        assert!(dir.path().join("docs/active/tickets").exists());
        assert!(dir.path().join("docs/active/stories").exists());
        assert!(dir.path().join("docs/active/work").exists());
        assert!(dir.path().join("docs/archive/tickets").exists());
        assert!(dir.path().join("docs/archive/stories").exists());
        assert!(dir.path().join("docs/archive/work").exists());

        // Check CLAUDE.md content
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("test-project"));
        assert!(claude_md.contains("cargo build"));

        // Check .lisa.toml content
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(lisa_toml.contains("max_threads"));
        assert!(lisa_toml.contains("docs/active/tickets"));
    }

    #[test]
    fn test_run_init_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // Create CLAUDE.md with custom content
        fs::write(dir.path().join("CLAUDE.md"), "my custom content").unwrap();

        // Create .lisa.toml with custom content
        fs::write(dir.path().join(".lisa.toml"), "# my config").unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // Original CLAUDE.md should be preserved
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(claude_md, "my custom content");

        // Original .lisa.toml should be preserved
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert_eq!(lisa_toml, "# my config");
    }

    #[test]
    fn test_validate_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_validate(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_setup() {
        let dir = tempfile::tempdir().unwrap();

        // Create minimal valid setup
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();

        let result = run_validate(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 4\n",
        )
        .unwrap();

        let result = run_validate(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        fs::write(dir.path().join(".lisa.toml"), "not valid toml {{{").unwrap();

        let result = run_validate(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_with_tickets() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();

        // Create a valid ticket
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            r#"---
id: T-001
title: test-ticket
type: task
status: open
priority: medium
phase: ready
---

## Context

Test ticket.

## Acceptance Criteria

- It works
"#,
        )
        .unwrap();

        let result = run_validate(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_detects_missing_dependency() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();

        // Create ticket with missing dependency
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            r#"---
id: T-001
title: test-ticket
type: task
status: open
priority: medium
phase: ready
depends_on: [T-999]
---

## Acceptance Criteria

- It works
"#,
        )
        .unwrap();

        let result = run_validate(dir.path());
        assert!(result.is_err());
    }
}
