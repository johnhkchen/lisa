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

    // Hook infrastructure directories
    let hook_dirs = [".lisa/hooks", ".lisa/signals"];
    for dir in &hook_dirs {
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

    // Hook script
    let hook_path = root.join(".lisa/hooks/on-idle.sh");
    if hook_path.exists() {
        actions.push(InitAction::Skip {
            path: hook_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: hook_path,
            content: templates::ON_IDLE_HOOK.to_string(),
        });
    }

    // .lisa/.gitignore (ignores ephemeral signal files)
    let lisa_gitignore_path = root.join(".lisa/.gitignore");
    if lisa_gitignore_path.exists() {
        actions.push(InitAction::Skip {
            path: lisa_gitignore_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: lisa_gitignore_path,
            content: templates::LISA_GITIGNORE.to_string(),
        });
    }

    // .claude/settings.local.json (idle_prompt notification hook)
    let settings_path = root.join(".claude/settings.local.json");
    if settings_path.exists() {
        actions.push(InitAction::Skip {
            path: settings_path,
            reason: "already exists — verify hooks config".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: settings_path,
            content: templates::settings_local_json(),
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

    // Make hook scripts executable on Unix
    #[cfg(unix)]
    {
        let hook_path = root.join(".lisa/hooks/on-idle.sh");
        if hook_path.exists() {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&hook_path, perms)
                .map_err(|e| format!("Failed to set permissions on {}: {}", hook_path.display(), e))?;
        }
    }

    println!("Initialization complete.");
    println!();
    println!("Next steps:");
    println!("  1. Create tickets in docs/active/tickets/");
    println!("  2. Run `lisa validate` to check readiness");
    println!("  3. Run `lisa loop` to start scheduling");

    Ok(())
}

/// Severity of a validation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

/// A single validation finding with structured path, category, and message.
#[derive(Debug, Clone)]
struct ValidationDiagnostic {
    /// Relative file path or logical location
    path: String,
    /// Category tag: frontmatter, dependency, structure, config, readiness
    category: &'static str,
    /// Human-readable description of the problem
    message: String,
    /// Whether this blocks readiness
    severity: Severity,
}

impl fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.severity {
            Severity::Error => write!(f, "{}: {}: {}", self.path, self.category, self.message),
            Severity::Warning => {
                write!(
                    f,
                    "{}: {} (warning): {}",
                    self.path, self.category, self.message
                )
            }
        }
    }
}

/// Result of validation, structured for both display and testing.
struct ValidationResult {
    diagnostics: Vec<ValidationDiagnostic>,
    ticket_count: usize,
    ready_count: usize,
}

impl ValidationResult {
    fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
}

/// Collect all validation diagnostics without printing.
fn validate(root: &Path, check_tools: bool) -> ValidationResult {
    let mut diagnostics: Vec<ValidationDiagnostic> = Vec::new();
    let mut ticket_count: usize = 0;
    let mut ready_count: usize = 0;

    // 1. Tool checks (optional)
    if check_tools {
        if !crate::loop_cmd::which("zellij") {
            diagnostics.push(ValidationDiagnostic {
                path: "(tools)".to_string(),
                category: "config",
                message: "`zellij` not found on PATH. Install: https://zellij.dev/documentation/installation".to_string(),
                severity: Severity::Error,
            });
        }
        if !crate::loop_cmd::which("claude") {
            diagnostics.push(ValidationDiagnostic {
                path: "(tools)".to_string(),
                category: "config",
                message: "`claude` not found on PATH. Install: https://docs.anthropic.com/en/docs/claude-code".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // 2. CLAUDE.md exists
    if !root.join("CLAUDE.md").exists() {
        diagnostics.push(ValidationDiagnostic {
            path: "CLAUDE.md".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create it.".to_string(),
            severity: Severity::Error,
        });
    }

    // 3. docs/rdspi-workflow.md exists (error, not warning)
    if !root.join("docs/rdspi-workflow.md").exists() {
        diagnostics.push(ValidationDiagnostic {
            path: "docs/rdspi-workflow.md".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create it.".to_string(),
            severity: Severity::Error,
        });
    }

    // 4. Validate .lisa.toml if present
    let ticket_dir_rel = match config::load_config(root) {
        Ok(validation) => {
            for w in &validation.warnings {
                diagnostics.push(ValidationDiagnostic {
                    path: ".lisa.toml".to_string(),
                    category: "config",
                    message: w.clone(),
                    severity: Severity::Warning,
                });
            }
            validation
                .config
                .dirs
                .tickets
                .unwrap_or_else(|| "docs/active/tickets".to_string())
        }
        Err(e) => {
            diagnostics.push(ValidationDiagnostic {
                path: ".lisa.toml".to_string(),
                category: "config",
                message: e,
                severity: Severity::Error,
            });
            "docs/active/tickets".to_string()
        }
    };

    // 5. Hook infrastructure
    let settings_path = root.join(".claude/settings.local.json");
    if !settings_path.exists() {
        diagnostics.push(ValidationDiagnostic {
            path: ".claude/settings.local.json".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create idle signal hooks.".to_string(),
            severity: Severity::Error,
        });
    } else {
        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                if !content.contains("idle_prompt") {
                    diagnostics.push(ValidationDiagnostic {
                        path: ".claude/settings.local.json".to_string(),
                        category: "config",
                        message: "missing idle_prompt hook configuration".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
            Err(e) => {
                diagnostics.push(ValidationDiagnostic {
                    path: ".claude/settings.local.json".to_string(),
                    category: "config",
                    message: format!("could not read file: {}", e),
                    severity: Severity::Error,
                });
            }
        }
    }

    let hook_path = root.join(".lisa/hooks/on-idle.sh");
    if !hook_path.exists() {
        diagnostics.push(ValidationDiagnostic {
            path: ".lisa/hooks/on-idle.sh".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create idle signal hooks.".to_string(),
            severity: Severity::Error,
        });
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&hook_path) {
                if meta.permissions().mode() & 0o111 == 0 {
                    diagnostics.push(ValidationDiagnostic {
                        path: ".lisa/hooks/on-idle.sh".to_string(),
                        category: "structure",
                        message: "not executable. Run: chmod +x .lisa/hooks/on-idle.sh".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    // 6. Check directory structure
    let optional_dirs = ["docs/active/stories", "docs/active/work"];
    for dir in &optional_dirs {
        if !root.join(dir).exists() {
            diagnostics.push(ValidationDiagnostic {
                path: dir.to_string(),
                category: "structure",
                message: "directory not found. Run `lisa init` to create it.".to_string(),
                severity: Severity::Warning,
            });
        }
    }

    // 7. Ticket directory must exist
    let ticket_dir = root.join(&ticket_dir_rel);
    if !ticket_dir.exists() {
        diagnostics.push(ValidationDiagnostic {
            path: ticket_dir_rel.clone(),
            category: "structure",
            message: "directory not found. Run `lisa init` to create it.".to_string(),
            severity: Severity::Error,
        });
        return ValidationResult {
            diagnostics,
            ticket_count,
            ready_count,
        };
    }

    // 8. Scan tickets with diagnostics
    let scan = match lisa_core::ticket::scan_tickets_with_diagnostics(&ticket_dir) {
        Ok(scan) => scan,
        Err(e) => {
            diagnostics.push(ValidationDiagnostic {
                path: ticket_dir_rel.clone(),
                category: "structure",
                message: format!("could not scan tickets: {}", e),
                severity: Severity::Error,
            });
            return ValidationResult {
                diagnostics,
                ticket_count,
                ready_count,
            };
        }
    };

    // Surface per-file parse errors
    for (path, err) in &scan.errors {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        diagnostics.push(ValidationDiagnostic {
            path: rel_path,
            category: "frontmatter",
            message: err.to_string(),
            severity: Severity::Error,
        });
    }

    // 9. Must have at least one ticket
    if scan.tickets.is_empty() {
        diagnostics.push(ValidationDiagnostic {
            path: format!("{}/", ticket_dir_rel),
            category: "readiness",
            message: "no tickets found. Create at least one ticket file.".to_string(),
            severity: Severity::Error,
        });
        return ValidationResult {
            diagnostics,
            ticket_count,
            ready_count,
        };
    }

    // 10. Acceptance criteria (warning)
    for ticket in &scan.tickets {
        if !ticket.content.contains("Acceptance Criteria")
            && !ticket.content.contains("acceptance criteria")
        {
            let rel_path = ticket
                .file_path
                .strip_prefix(root)
                .unwrap_or(&ticket.file_path)
                .display()
                .to_string();
            diagnostics.push(ValidationDiagnostic {
                path: rel_path,
                category: "frontmatter",
                message: "missing Acceptance Criteria section".to_string(),
                severity: Severity::Warning,
            });
        }
    }

    ticket_count = scan.tickets.len();

    // 11. Build DAG
    match lisa_core::dag::Dag::from_tickets(scan.tickets) {
        Ok(dag) => {
            // Check for cycles
            if let lisa_core::dag::CycleDetectionResult::Cycle(nodes) = dag.detect_cycles() {
                diagnostics.push(ValidationDiagnostic {
                    path: format!("{}/", ticket_dir_rel),
                    category: "dependency",
                    message: format!("cycle detected involving tickets: {}", nodes.join(", ")),
                    severity: Severity::Error,
                });
            }

            let ready = dag.get_ready_tickets();
            ready_count = ready.len();

            // 12. At least one ready ticket
            if ready.is_empty() {
                diagnostics.push(ValidationDiagnostic {
                    path: format!("{}/", ticket_dir_rel),
                    category: "readiness",
                    message: "no tickets with phase 'ready' and all dependencies satisfied".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        Err(lisa_core::dag::DagError::MissingDependency {
            ticket_id,
            missing_dep,
        }) => {
            diagnostics.push(ValidationDiagnostic {
                path: format!("{}/", ticket_dir_rel),
                category: "dependency",
                message: format!(
                    "ticket {} depends on {} which does not exist",
                    ticket_id, missing_dep
                ),
                severity: Severity::Error,
            });
        }
        Err(lisa_core::dag::DagError::CycleDetected(nodes)) => {
            diagnostics.push(ValidationDiagnostic {
                path: format!("{}/", ticket_dir_rel),
                category: "dependency",
                message: format!("cycle detected involving tickets: {}", nodes.join(", ")),
                severity: Severity::Error,
            });
        }
    }

    ValidationResult {
        diagnostics,
        ticket_count,
        ready_count,
    }
}

/// Run validation on the project setup.
///
/// When `check_tools` is true, also verifies that `zellij` and `claude` are on PATH.
pub fn run_validate(root: &Path, check_tools: bool) -> Result<(), String> {
    let result = validate(root, check_tools);
    print_diagnostics(&result)
}

/// Print structured validation diagnostics and return appropriate Result.
fn print_diagnostics(result: &ValidationResult) -> Result<(), String> {
    // Print errors first, then warnings
    for d in &result.diagnostics {
        if d.severity == Severity::Error {
            println!("{}", d);
        }
    }
    for d in &result.diagnostics {
        if d.severity == Severity::Warning {
            println!("{}", d);
        }
    }

    if result.has_errors() {
        let count = result.error_count();
        println!(
            "\n{} error(s) found. Fix and re-run `lisa validate`.",
            count
        );
        Err(format!("{} error(s) found. Fix and re-run `lisa validate`.", count))
    } else {
        println!(
            "All checks passed. {} tickets, {} ready, DAG valid. Run `lisa loop` to start.",
            result.ticket_count, result.ready_count
        );
        Ok(())
    }
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

        // Should plan to create:
        //   8 directories (6 docs + .lisa/hooks + .lisa/signals)
        //   6 files (CLAUDE.md, rdspi-workflow.md, .lisa.toml, on-idle.sh, .lisa/.gitignore, settings.local.json)
        let creates: Vec<_> = actions
            .iter()
            .filter(|a| !matches!(a, InitAction::Skip { .. }))
            .collect();
        assert_eq!(creates.len(), 14);
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

        // Check hook infrastructure
        assert!(dir.path().join(".lisa/hooks/on-idle.sh").exists());
        assert!(dir.path().join(".lisa/signals").exists());
        assert!(dir.path().join(".lisa/.gitignore").exists());
        assert!(dir.path().join(".claude/settings.local.json").exists());

        // Check hook script content
        let hook = fs::read_to_string(dir.path().join(".lisa/hooks/on-idle.sh")).unwrap();
        assert!(hook.starts_with("#!/bin/sh"));
        assert!(hook.contains("LISA_TICKET_ID"));

        // Check hook script is executable on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::metadata(dir.path().join(".lisa/hooks/on-idle.sh"))
                .unwrap()
                .permissions();
            assert!(perms.mode() & 0o111 != 0, "hook script should be executable");
        }

        // Check settings.local.json content
        let settings = fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
        assert!(settings.contains("idle_prompt"));

        // Check .lisa/.gitignore content
        let gitignore = fs::read_to_string(dir.path().join(".lisa/.gitignore")).unwrap();
        assert!(gitignore.contains("signals/"));
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
        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    /// Helper to create hook infrastructure required by validate.
    fn write_hook_infrastructure(root: &Path) {
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(
            root.join(".claude/settings.local.json"),
            templates::settings_local_json(),
        )
        .unwrap();
        fs::create_dir_all(root.join(".lisa/hooks")).unwrap();
        fs::write(
            root.join(".lisa/hooks/on-idle.sh"),
            templates::ON_IDLE_HOOK,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(root.join(".lisa/hooks/on-idle.sh"), perms).unwrap();
        }
    }

    /// Helper to create a minimal ready ticket in the given project root.
    fn write_ready_ticket(root: &Path) {
        fs::write(
            root.join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: medium\nphase: ready\n---\n\n## Acceptance Criteria\n\n- It works\n",
        ).unwrap();
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
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
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
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
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

        let result = run_validate(dir.path(), false);
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
        write_hook_infrastructure(dir.path());

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

        let result = run_validate(dir.path(), false);
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

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_rdspi_workflow() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        write_ready_ticket(dir.path());
        // No docs/rdspi-workflow.md

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error"));
    }

    #[test]
    fn test_validate_empty_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // No ticket files

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error"));
    }

    #[test]
    fn test_validate_no_ready_tickets() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();

        // All tickets are done — no ready tickets
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: done-ticket\ntype: task\nstatus: done\npriority: medium\nphase: done\n---\n\n## Acceptance Criteria\n\n- Done\n",
        ).unwrap();

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error"));
    }

    #[test]
    fn test_validate_ticket_parse_error() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();

        // Malformed ticket (missing required fields)
        fs::write(
            dir.path().join("docs/active/tickets/T-BAD.md"),
            "---\nid: T-BAD\ntitle: bad\n---\nNo type/status/priority/phase\n",
        )
        .unwrap();

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_acceptance_criteria_warning() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // Ticket without Acceptance Criteria section
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: no-ac\ntype: task\nstatus: open\npriority: medium\nphase: ready\n---\n\nNo AC section here.\n",
        ).unwrap();

        // Should still pass (warning, not error) because there's a ready ticket
        let result = run_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_tools_false() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        // check_tools=false should not fail even if tools are missing
        let result = run_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_no_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // No docs/active/tickets directory

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_init_never_overwrites_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // Pre-create hook files with custom content
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "custom hook").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            "{}",
        )
        .unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // Original files should be preserved
        let hook = fs::read_to_string(dir.path().join(".lisa/hooks/on-idle.sh")).unwrap();
        assert_eq!(hook, "custom hook");
        let settings = fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
        assert_eq!(settings, "{}");
    }

    #[test]
    fn test_plan_init_actions_existing_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "existing").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(dir.path().join(".claude/settings.local.json"), "{}").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // Hook-related files should be skipped
        let skipped_hook: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::Skip { path, .. } if path.ends_with("on-idle.sh")))
            .collect();
        assert_eq!(skipped_hook.len(), 1);

        let skipped_settings: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::Skip { path, .. } if path.ends_with("settings.local.json")))
            .collect();
        assert_eq!(skipped_settings.len(), 1);
    }

    #[test]
    fn test_validate_missing_settings_json() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // Create on-idle.sh but NOT settings.local.json
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                dir.path().join(".lisa/hooks/on-idle.sh"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_settings_json_without_idle_hook() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // settings.local.json exists but without idle_prompt
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(dir.path().join(".claude/settings.local.json"), "{}").unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                dir.path().join(".lisa/hooks/on-idle.sh"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_idle_hook_script() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // settings.local.json exists with idle_prompt, but NO on-idle.sh
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            templates::settings_local_json(),
        )
        .unwrap();
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_idle_hook_not_executable() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            templates::settings_local_json(),
        )
        .unwrap();
        // on-idle.sh exists but NOT executable
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "#!/bin/sh\n").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                dir.path().join(".lisa/hooks/on-idle.sh"),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap();
        }
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_ticket_type_value() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // Ticket with invalid type: "ticket" instead of task/bug/feature/spike/chore
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: bad-type\ntype: ticket\nstatus: open\npriority: medium\nphase: ready\n---\n\n## Acceptance Criteria\n\n- It works\n",
        ).unwrap();

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_phase_value() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // Ticket with invalid phase: "coding" instead of valid values
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: bad-phase\ntype: task\nstatus: open\npriority: medium\nphase: coding\n---\n\n## Acceptance Criteria\n\n- It works\n",
        ).unwrap();

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_init_then_validate_roundtrip_rust() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-rust-project\"\n",
        )
        .unwrap();

        // Run init
        let init_result = run_init(dir.path(), false);
        assert!(init_result.is_ok());

        // Add a ready ticket
        write_ready_ticket(dir.path());

        // Validate should pass
        let validate_result = run_validate(dir.path(), false);
        assert!(validate_result.is_ok());

        // Verify CLAUDE.md contains project type
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("my-rust-project"));
        assert!(claude_md.contains("(Rust)"));
        assert!(claude_md.contains("cargo build"));
    }

    #[test]
    fn test_init_then_validate_roundtrip_node() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"my-node-project\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();

        // Run init
        let init_result = run_init(dir.path(), false);
        assert!(init_result.is_ok());

        // Add a ready ticket
        write_ready_ticket(dir.path());

        // Validate should pass
        let validate_result = run_validate(dir.path(), false);
        assert!(validate_result.is_ok());

        // Verify CLAUDE.md contains project type
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("my-node-project"));
        assert!(claude_md.contains("(Node.js)"));
        assert!(claude_md.contains("npm"));
    }

    // --- Structured diagnostic tests (call validate() directly) ---

    #[test]
    fn test_diagnostics_clean_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        assert!(!result.has_errors());
        assert_eq!(result.ticket_count, 1);
        assert_eq!(result.ready_count, 1);
    }

    #[test]
    fn test_diagnostics_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();

        let result = validate(dir.path(), false);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.path == "CLAUDE.md")
            .collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].category, "structure");
    }

    #[test]
    fn test_diagnostics_ticket_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // Malformed ticket
        fs::write(
            dir.path().join("docs/active/tickets/T-BAD.md"),
            "---\nid: T-BAD\ntitle: bad\n---\nNo type\n",
        )
        .unwrap();

        let result = validate(dir.path(), false);
        let frontmatter_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.category == "frontmatter")
            .collect();
        assert_eq!(frontmatter_errors.len(), 1);
        assert!(frontmatter_errors[0].path.contains("T-BAD.md"));
    }

    #[test]
    fn test_diagnostics_missing_dependency() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: medium\nphase: ready\ndepends_on: [T-999]\n---\n\n## Acceptance Criteria\n\n- It works\n",
        ).unwrap();

        let result = validate(dir.path(), false);
        let dep_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.category == "dependency")
            .collect();
        assert_eq!(dep_errors.len(), 1);
        assert!(dep_errors[0].message.contains("T-999"));
    }

    #[test]
    fn test_diagnostics_no_ready_tickets() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // All done
        fs::write(
            dir.path().join("docs/active/tickets/T-001.md"),
            "---\nid: T-001\ntitle: done\ntype: task\nstatus: done\npriority: medium\nphase: done\n---\n\n## Acceptance Criteria\n\n- Done\n",
        ).unwrap();

        let result = validate(dir.path(), false);
        let readiness_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.category == "readiness")
            .collect();
        assert_eq!(readiness_errors.len(), 1);
    }

    #[test]
    fn test_diagnostics_format_error() {
        let d = ValidationDiagnostic {
            path: "docs/active/tickets/T-001.md".to_string(),
            category: "frontmatter",
            message: "missing required field 'phase'".to_string(),
            severity: Severity::Error,
        };
        assert_eq!(
            d.to_string(),
            "docs/active/tickets/T-001.md: frontmatter: missing required field 'phase'"
        );
    }

    #[test]
    fn test_diagnostics_format_warning() {
        let d = ValidationDiagnostic {
            path: "docs/active/stories".to_string(),
            category: "structure",
            message: "directory not found".to_string(),
            severity: Severity::Warning,
        };
        assert_eq!(
            d.to_string(),
            "docs/active/stories: structure (warning): directory not found"
        );
    }

    #[test]
    fn test_diagnostics_hook_structure_errors() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        // No hook infrastructure at all
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        let hook_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| {
                d.severity == Severity::Error
                    && (d.path.contains("settings.local.json")
                        || d.path.contains("on-idle.sh"))
            })
            .collect();
        assert_eq!(hook_errors.len(), 2);
    }

    #[test]
    fn test_diagnostics_success_counts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::write(dir.path().join("docs/rdspi-workflow.md"), "# RDSPI").unwrap();
        write_hook_infrastructure(dir.path());

        // Two ready tickets, one done
        write_ready_ticket(dir.path());
        fs::write(
            dir.path().join("docs/active/tickets/T-002.md"),
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: medium\nphase: ready\n---\n\n## Acceptance Criteria\n\n- Done\n",
        ).unwrap();
        fs::write(
            dir.path().join("docs/active/tickets/T-003.md"),
            "---\nid: T-003\ntitle: third\ntype: task\nstatus: done\npriority: medium\nphase: done\n---\n\n## Acceptance Criteria\n\n- Done\n",
        ).unwrap();

        let result = validate(dir.path(), false);
        assert!(!result.has_errors());
        assert_eq!(result.ticket_count, 3);
        assert_eq!(result.ready_count, 2);
    }
}
