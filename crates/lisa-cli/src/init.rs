use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config;
use crate::detect::{detect_project, DetectedProject};
use crate::templates;

/// Update or insert the `version = "..."` line in a .lisa.toml string.
fn update_version_in_toml(existing: &str, new_version: &str) -> String {
    let version_line = format!("version = \"{}\"", new_version);
    // Try to replace an existing version line
    let mut found = false;
    let updated: Vec<String> = existing
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("version") && line.contains('=') {
                found = true;
                version_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect();

    if found {
        updated.join("\n") + if existing.ends_with('\n') { "\n" } else { "" }
    } else {
        // Insert version at the top, after any leading comment lines
        let mut result = String::new();
        let mut inserted = false;
        for line in existing.lines() {
            if !inserted && !line.starts_with('#') && !line.is_empty() {
                result.push_str(&version_line);
                result.push('\n');
                inserted = true;
            }
            result.push_str(line);
            result.push('\n');
        }
        if !inserted {
            // All lines were comments or empty — append at end
            result.push_str(&version_line);
            result.push('\n');
        }
        result
    }
}

/// Upsert missing keys from the default config template into an existing .lisa.toml.
/// New keys are appended as commented-out lines under the appropriate section.
fn upsert_missing_config_keys(existing: &str) -> String {
    let mut result = existing.to_string();

    // Parse existing to detect what's present (both active and commented keys)
    let has_key = |content: &str, section: &str, key: &str| -> bool {
        let mut in_section = section.is_empty(); // top-level keys: always "in section"
        for line in content.lines() {
            let trimmed = line.trim();
            // Track section headers (both active and commented)
            if trimmed.starts_with('[') || trimmed.starts_with("# [") {
                let cleaned = trimmed
                    .trim_start_matches('#')
                    .trim()
                    .trim_matches('[')
                    .trim_matches(']');
                if section.is_empty() {
                    // We left the top-level area
                    if !cleaned.is_empty() {
                        in_section = false;
                    }
                } else {
                    in_section = cleaned == section;
                }
                continue;
            }
            if in_section {
                // Check both active and commented forms: "key = " or "# key = "
                let without_comment = trimmed.trim_start_matches('#').trim();
                if without_comment.starts_with(key)
                    && without_comment[key.len()..].trim_start().starts_with('=')
                {
                    return true;
                }
            }
        }
        false
    };

    // Check if a section header exists (active or commented)
    let has_section = |content: &str, section: &str| -> bool {
        let active = format!("[{}]", section);
        let commented = format!("# [{}]", section);
        content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed == active || trimmed == commented
        })
    };

    // Find insertion point: the line index after the last line of a section
    let find_section_end = |content: &str, section: &str| -> Option<usize> {
        let mut in_section = false;
        let mut last_section_line = None;
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') || trimmed.starts_with("# [") {
                let cleaned = trimmed
                    .trim_start_matches('#')
                    .trim()
                    .trim_matches('[')
                    .trim_matches(']');
                if cleaned == section {
                    in_section = true;
                    last_section_line = Some(i);
                    continue;
                } else if in_section {
                    // Hit a different section — end of our section
                    return last_section_line.map(|l| l + 1);
                }
            }
            if in_section && !trimmed.is_empty() {
                last_section_line = Some(i);
            }
        }
        // Section goes to end of file
        last_section_line.map(|l| l + 1)
    };

    // Insert a line after a given line index
    let insert_after = |content: &str, after_line: usize, new_lines: &str| -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate() {
            out.push_str(line);
            out.push('\n');
            if i + 1 == after_line {
                out.push_str(new_lines);
            }
        }
        // If after_line is past the end, append
        if after_line >= lines.len() {
            out.push_str(new_lines);
        }
        out
    };

    // [scheduling] keys to upsert
    let scheduling_keys: &[(&str, &str)] = &[
        ("auto_advance", "# auto_advance = false"),
        ("review_timeout_secs", "# review_timeout_secs = 600"),
        ("session_timeout_secs", "# session_timeout_secs = 3600"),
        ("wind_down_secs", "# wind_down_secs = 300"),
        (
            "assignment_ack_timeout_secs",
            "# assignment_ack_timeout_secs = 30",
        ),
    ];

    for (key, commented_line) in scheduling_keys {
        if !has_key(&result, "scheduling", key) {
            if let Some(end) = find_section_end(&result, "scheduling") {
                result = insert_after(&result, end, &format!("{}\n", commented_line));
            }
        }
    }

    // [scheduling.phase_timeouts] section
    if !has_section(&result, "scheduling.phase_timeouts") {
        let phase_block = "\n# [scheduling.phase_timeouts]\n# research = 300\n# design = 300\n# implement = 1800\n";
        if let Some(end) = find_section_end(&result, "scheduling") {
            result = insert_after(&result, end, phase_block);
        } else {
            // No scheduling section at all — append
            result.push_str(phase_block);
        }
    }

    result
}

/// An action that init will perform
#[derive(Debug, Clone)]
pub enum InitAction {
    CreateDir(PathBuf),
    CreateFile { path: PathBuf, content: String },
    UpdateFile { path: PathBuf, content: String },
    NoOp { path: PathBuf, reason: String },
    SafetySkip { path: PathBuf, reason: String },
}

impl fmt::Display for InitAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitAction::CreateDir(path) => write!(f, "  create  {}/", path.display()),
            InitAction::CreateFile { path, .. } => write!(f, "  create  {}", path.display()),
            InitAction::UpdateFile { path, .. } => write!(f, "  update  {}", path.display()),
            InitAction::NoOp { path, reason } => {
                write!(f, "  no-op   {} ({})", path.display(), reason)
            }
            InitAction::SafetySkip { path, reason } => {
                write!(f, "  skip    {} ({})", path.display(), reason)
            }
        }
    }
}

/// Plan a whole-file template write without guessing that Lisa owns an existing
/// file. Exact current bytes are a no-op; only exact bytes from a bundled prior
/// template authorize replacement. Unknown or unreadable content is preserved.
fn plan_owned_template(path: PathBuf, current: &str, known_prior: &[&str]) -> InitAction {
    if !path.exists() {
        return InitAction::CreateFile {
            path,
            content: current.to_string(),
        };
    }

    match fs::read_to_string(&path) {
        Ok(existing) if existing == current => InitAction::NoOp {
            path,
            reason: "already up to date".to_string(),
        },
        Ok(existing) if known_prior.contains(&existing.as_str()) => InitAction::UpdateFile {
            path,
            content: current.to_string(),
        },
        Ok(_) => InitAction::SafetySkip {
            path,
            reason: "preserved: content is not a known Lisa template".to_string(),
        },
        Err(_) => InitAction::SafetySkip {
            path,
            reason: "preserved: existing file is unreadable".to_string(),
        },
    }
}

/// Plan an append-only update to Lisa's nested gitignore. Existing bytes are
/// retained as an immutable prefix; only required rules that are absent after
/// trimming harmless surrounding whitespace are appended.
fn plan_append_only_gitignore(path: PathBuf, required: &str) -> InitAction {
    if !path.exists() {
        return InitAction::CreateFile {
            path,
            content: required.to_string(),
        };
    }

    match fs::read_to_string(&path) {
        Ok(existing) => {
            let existing_rules: Vec<&str> = existing.lines().map(str::trim).collect();
            let missing: Vec<&str> = required
                .lines()
                .map(str::trim)
                .filter(|rule| !rule.is_empty() && !existing_rules.contains(rule))
                .collect();

            if missing.is_empty() {
                return InitAction::NoOp {
                    path,
                    reason: "already up to date".to_string(),
                };
            }

            let mut content = existing;
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            for rule in missing {
                content.push_str(rule);
                content.push('\n');
            }

            InitAction::UpdateFile { path, content }
        }
        Err(_) => InitAction::SafetySkip {
            path,
            reason: "preserved: existing file is unreadable".to_string(),
        },
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
            actions.push(InitAction::NoOp {
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
        actions.push(InitAction::NoOp {
            path: claude_md_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: claude_md_path,
            content: templates::generate_claude_md(project),
        });
    }

    // AGENTS.md — Codex's native context file. A pointer to CLAUDE.md (single
    // source of truth), scaffolded like CLAUDE.md: skip if present, never
    // overwrite. Emitted unconditionally so switching to the Codex client is a
    // one-line .lisa.toml edit with no re-scaffold; inert for Claude-only projects.
    let agents_md_path = root.join("AGENTS.md");
    if agents_md_path.exists() {
        actions.push(InitAction::NoOp {
            path: agents_md_path,
            reason: "already exists".to_string(),
        });
    } else {
        actions.push(InitAction::CreateFile {
            path: agents_md_path,
            content: templates::AGENTS_MD.to_string(),
        });
    }

    // docs/knowledge/rdspi-workflow.md
    let workflow_path = root.join("docs/knowledge/rdspi-workflow.md");
    actions.push(plan_owned_template(
        workflow_path,
        templates::RDSPI_WORKFLOW,
        templates::LEGACY_RDSPI_WORKFLOWS,
    ));

    // .lisa.toml
    let config_path = root.join(".lisa.toml");
    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(existing) => {
                let parsed: Result<config::LisaConfig, _> = toml::from_str(&existing);
                let project_version = parsed.ok().and_then(|c| c.version);
                let version_current = matches!(&project_version, Some(v) if !config::version_is_stale(v, config::LISA_VERSION));

                // Always upsert missing keys, even if version is current
                let with_version = if version_current {
                    existing.clone()
                } else {
                    update_version_in_toml(&existing, config::LISA_VERSION)
                };
                let updated = upsert_missing_config_keys(&with_version);

                if updated == existing {
                    actions.push(InitAction::NoOp {
                        path: config_path,
                        reason: "already up to date".to_string(),
                    });
                } else {
                    actions.push(InitAction::UpdateFile {
                        path: config_path.clone(),
                        content: updated,
                    });
                }
            }
            Err(_) => {
                actions.push(InitAction::SafetySkip {
                    path: config_path,
                    reason: "exists but unreadable".to_string(),
                });
            }
        }
    } else {
        actions.push(InitAction::CreateFile {
            path: config_path,
            content: config::default_config_toml(),
        });
    }

    // Hook infrastructure directories
    let hook_dirs = [".lisa/hooks", ".lisa/signals"];
    for dir in &hook_dirs {
        let path = root.join(dir);
        if path.exists() {
            actions.push(InitAction::NoOp {
                path,
                reason: "already exists".to_string(),
            });
        } else {
            actions.push(InitAction::CreateDir(path));
        }
    }

    // Hook scripts
    let hook_scripts: &[(&str, &str, &[&str])] = &[
        (
            "on-idle.sh",
            templates::ON_IDLE_HOOK,
            templates::LEGACY_ON_IDLE_HOOKS,
        ),
        (
            "on-stop.sh",
            templates::ON_STOP_HOOK,
            templates::LEGACY_ON_STOP_HOOKS,
        ),
        (
            "on-clear.sh",
            templates::ON_CLEAR_HOOK,
            templates::LEGACY_ON_CLEAR_HOOKS,
        ),
        (
            "on-heartbeat.sh",
            templates::ON_HEARTBEAT_HOOK,
            templates::LEGACY_ON_HEARTBEAT_HOOKS,
        ),
        (
            "on-ack.sh",
            templates::ON_ACK_HOOK,
            templates::LEGACY_ON_ACK_HOOKS,
        ),
        // Scaffolded as a non-executable `.sample`: the user opts in by copying
        // it to `on-notify` and `chmod +x`. Deliberately excluded from the chmod
        // loop below so the catch-all Notification hook's `test -x` guard stays
        // inert until then.
        (
            "on-notify.sample",
            templates::ON_NOTIFY_HOOK,
            templates::LEGACY_ON_NOTIFY_HOOKS,
        ),
    ];
    for (name, content, known_prior) in hook_scripts {
        let hook_path = root.join(format!(".lisa/hooks/{}", name));
        actions.push(plan_owned_template(hook_path, content, known_prior));
    }

    // .lisa/.gitignore (ignores ephemeral signal/session/usage files)
    let lisa_gitignore_path = root.join(".lisa/.gitignore");
    actions.push(plan_append_only_gitignore(
        lisa_gitignore_path,
        templates::LISA_GITIGNORE,
    ));

    // .claude/settings.local.json (Stop, SessionStart, Notification hooks)
    // Always run merge_hooks on existing files to upgrade old bare-path commands.
    let settings_path = root.join(".claude/settings.local.json");
    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                match templates::merge_hooks(&content) {
                    Ok(merged) => {
                        // Compare parsed JSON to avoid false updates from whitespace changes
                        let old: Option<serde_json::Value> = serde_json::from_str(&content).ok();
                        let new: Option<serde_json::Value> = serde_json::from_str(&merged).ok();
                        if old == new {
                            actions.push(InitAction::NoOp {
                                path: settings_path,
                                reason: "already up to date".to_string(),
                            });
                        } else {
                            actions.push(InitAction::UpdateFile {
                                path: settings_path,
                                content: merged,
                            });
                        }
                    }
                    Err(_) => {
                        actions.push(InitAction::SafetySkip {
                            path: settings_path,
                            reason: "exists but JSON is malformed — add hooks manually".to_string(),
                        });
                    }
                }
            }
            Err(_) => {
                actions.push(InitAction::SafetySkip {
                    path: settings_path,
                    reason: "exists but unreadable — check permissions".to_string(),
                });
            }
        }
    } else {
        actions.push(InitAction::CreateFile {
            path: settings_path,
            content: templates::settings_local_json(),
        });
    }

    // .codex/hooks.json — native Codex TUI lifecycle signals. Keep this separate
    // from `.claude/settings.local.json`: both clients load their own native
    // configuration while sharing the versioned `.lisa/hooks/*.sh` scripts.
    let codex_hooks_path = root.join(".codex/hooks.json");
    if codex_hooks_path.exists() {
        match fs::read_to_string(&codex_hooks_path) {
            Ok(content) => match templates::merge_codex_hooks(&content) {
                Ok(merged) => {
                    let old: Option<serde_json::Value> = serde_json::from_str(&content).ok();
                    let new: Option<serde_json::Value> = serde_json::from_str(&merged).ok();
                    if old == new {
                        actions.push(InitAction::NoOp {
                            path: codex_hooks_path,
                            reason: "already up to date".to_string(),
                        });
                    } else {
                        actions.push(InitAction::UpdateFile {
                            path: codex_hooks_path,
                            content: merged,
                        });
                    }
                }
                Err(_) => actions.push(InitAction::SafetySkip {
                    path: codex_hooks_path,
                    reason: "exists but JSON is malformed — add hooks manually".to_string(),
                }),
            },
            Err(_) => actions.push(InitAction::SafetySkip {
                path: codex_hooks_path,
                reason: "exists but unreadable — check permissions".to_string(),
            }),
        }
    } else {
        actions.push(InitAction::CreateFile {
            path: codex_hooks_path,
            content: templates::codex_hooks_json(),
        });
    }

    actions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileMutationKind {
    Created,
    Updated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileMutation {
    kind: FileMutationKind,
    path: PathBuf,
}

fn write_init_line(out: &mut impl Write, args: fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}").map_err(|e| format!("Failed to write init output: {e}"))
}

/// Execute the init command, writing user-facing output to stdout.
pub fn run_init(root: &Path, dry_run: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    run_init_with_writer(root, dry_run, &mut out)
}

/// Internal init entry point with injectable output for end-to-end reporting
/// tests. Planning remains complete before any filesystem mutation.
fn run_init_with_writer(root: &Path, dry_run: bool, out: &mut impl Write) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    // Step 1: Detect project type
    let project = detect_project(root);
    write_init_line(
        out,
        format_args!(
            "Detected project: {} ({})",
            project.name,
            match &project.project_type {
                crate::detect::ProjectType::Rust => "Rust",
                crate::detect::ProjectType::Node => "Node.js",
                crate::detect::ProjectType::Go => "Go",
                crate::detect::ProjectType::Python => "Python",
                crate::detect::ProjectType::Unknown => "unknown",
            }
        ),
    )?;
    write_init_line(out, format_args!(""))?;

    // Step 2: Plan actions
    let actions = plan_init_actions(root, &project);

    // Step 3: Print the plan
    write_init_line(out, format_args!("Planned actions:"))?;
    for action in &actions {
        write_init_line(out, format_args!("{action}"))?;
    }
    write_init_line(out, format_args!(""))?;

    // Step 4: Dry run stops here
    if dry_run {
        write_init_line(out, format_args!("Dry run complete. No changes made."))?;
        return Ok(());
    }

    // Step 5: Execute
    let mut mutations = Vec::new();
    for action in &actions {
        match action {
            InitAction::CreateDir(path) => {
                fs::create_dir_all(path)
                    .map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
            }
            InitAction::CreateFile { path, content } => {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
                fs::write(path, content)
                    .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
                mutations.push(FileMutation {
                    kind: FileMutationKind::Created,
                    path: path.clone(),
                });
            }
            InitAction::UpdateFile { path, content } => {
                fs::write(path, content)
                    .map_err(|e| format!("Failed to update {}: {}", path.display(), e))?;
                mutations.push(FileMutation {
                    kind: FileMutationKind::Updated,
                    path: path.clone(),
                });
            }
            InitAction::NoOp { .. } | InitAction::SafetySkip { .. } => {}
        }
    }

    // Make only active hook scripts written by this run executable. A no-op or
    // safety-skipped project hook is left completely untouched.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for script in &[
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-heartbeat.sh",
            "on-ack.sh",
        ] {
            let hook_path = root.join(format!(".lisa/hooks/{}", script));
            if mutations.iter().any(|mutation| mutation.path == hook_path) {
                let perms = fs::Permissions::from_mode(0o755);
                fs::set_permissions(&hook_path, perms).map_err(|e| {
                    format!(
                        "Failed to set permissions on {}: {}",
                        hook_path.display(),
                        e
                    )
                })?;
            }
        }
    }

    write_init_line(out, format_args!("Initialization complete."))?;
    write_init_line(out, format_args!(""))?;
    write_init_line(out, format_args!("Files changed:"))?;
    if mutations.is_empty() {
        write_init_line(out, format_args!("  none"))?;
    } else {
        for mutation in &mutations {
            let label = match mutation.kind {
                FileMutationKind::Created => "created",
                FileMutationKind::Updated => "updated",
            };
            write_init_line(
                out,
                format_args!("  {label:<8} {}", mutation.path.display()),
            )?;
        }
    }
    write_init_line(out, format_args!(""))?;
    write_init_line(out, format_args!("Next steps:"))?;
    write_init_line(
        out,
        format_args!("  1. Inspect the files reported above before your next commit"),
    )?;
    write_init_line(
        out,
        format_args!("  2. Create tickets in docs/active/tickets/"),
    )?;
    write_init_line(
        out,
        format_args!("  3. Run `lisa validate` to check readiness"),
    )?;
    write_init_line(
        out,
        format_args!("  4. Run `lisa loop` to start scheduling"),
    )?;

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
    let selected_client = config::load_config(root)
        .map(|v| config::resolve_config(&v.config, None, None).client)
        .unwrap_or_default();

    // 1. Tool checks (optional)
    if check_tools {
        if !crate::doctor::which("zellij") {
            diagnostics.push(ValidationDiagnostic {
                path: "(tools)".to_string(),
                category: "config",
                message: "`zellij` not found on PATH. Install: https://zellij.dev/documentation/installation".to_string(),
                severity: Severity::Error,
            });
        }
        let (agent, install) = match selected_client {
            lisa_core::client::AgentClient::Claude => {
                ("claude", "https://docs.anthropic.com/en/docs/claude-code")
            }
            lisa_core::client::AgentClient::Codex => ("codex", "npm i -g @openai/codex"),
        };
        if !crate::doctor::which(agent) {
            diagnostics.push(ValidationDiagnostic {
                path: "(tools)".to_string(),
                category: "config",
                message: format!("`{agent}` not found on PATH. Install: {install}"),
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

    // 3. docs/knowledge/rdspi-workflow.md exists (error, not warning)
    if !root.join("docs/knowledge/rdspi-workflow.md").exists() {
        diagnostics.push(ValidationDiagnostic {
            path: "docs/knowledge/rdspi-workflow.md".to_string(),
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

    // 5. Hook infrastructure — settings.local.json
    let settings_path = root.join(".claude/settings.local.json");
    if !settings_path.exists() {
        diagnostics.push(ValidationDiagnostic {
            path: ".claude/settings.local.json".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create hooks.".to_string(),
            severity: Severity::Error,
        });
    } else {
        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                for (key, label) in [
                    ("idle_prompt", "Notification[idle_prompt]"),
                    ("on-notify", "Notification[attention]"),
                    ("\"Stop\"", "Stop"),
                    ("\"SessionStart\"", "SessionStart[clear]"),
                    ("\"PostToolUse\"", "PostToolUse[heartbeat]"),
                    ("AskUserQuestion", "PreToolUse[AskUserQuestion]"),
                ] {
                    if !content.contains(key) {
                        diagnostics.push(ValidationDiagnostic {
                            path: ".claude/settings.local.json".to_string(),
                            category: "config",
                            message: format!("missing {} hook configuration", label),
                            severity: Severity::Error,
                        });
                    }
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

    // Native Codex TUI lifecycle hooks are required only when Codex is the
    // configured loop client. Per-ticket Codex routing is checked by loop
    // preflight after the DAG is loaded.
    if selected_client == lisa_core::client::AgentClient::Codex {
        let codex_hooks_path = root.join(".codex/hooks.json");
        if !codex_hooks_path.exists() {
            diagnostics.push(ValidationDiagnostic {
                path: ".codex/hooks.json".to_string(),
                category: "structure",
                message: "not found. Run `lisa init` to create Codex hooks.".to_string(),
                severity: Severity::Error,
            });
        } else {
            match fs::read_to_string(&codex_hooks_path) {
                Ok(content) => match templates::merge_codex_hooks(&content) {
                    Ok(expected) => {
                        let current: Option<serde_json::Value> =
                            serde_json::from_str(&content).ok();
                        let expected: Option<serde_json::Value> =
                            serde_json::from_str(&expected).ok();
                        if current != expected {
                            diagnostics.push(ValidationDiagnostic {
                                path: ".codex/hooks.json".to_string(),
                                category: "config",
                                message: "missing or stale Lisa lifecycle hooks. Run `lisa init`."
                                    .to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                    Err(e) => diagnostics.push(ValidationDiagnostic {
                        path: ".codex/hooks.json".to_string(),
                        category: "config",
                        message: e,
                        severity: Severity::Error,
                    }),
                },
                Err(e) => diagnostics.push(ValidationDiagnostic {
                    path: ".codex/hooks.json".to_string(),
                    category: "config",
                    message: format!("could not read file: {e}"),
                    severity: Severity::Error,
                }),
            }
        }
    }

    // Hook scripts — active lifecycle hooks plus the opt-in notification sample.
    // The `.sample` is scaffolded non-executable (opt-in), so it is checked for
    // existence but exempt from the executable-bit check.
    for script in &[
        "on-idle.sh",
        "on-stop.sh",
        "on-clear.sh",
        "on-heartbeat.sh",
        "on-ack.sh",
        "on-notify.sample",
    ] {
        let hook_path = root.join(format!(".lisa/hooks/{}", script));
        if !hook_path.exists() {
            diagnostics.push(ValidationDiagnostic {
                path: format!(".lisa/hooks/{}", script),
                category: "structure",
                message: "not found. Run `lisa init` to create hooks.".to_string(),
                severity: Severity::Error,
            });
        } else if !script.ends_with(".sample") {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&hook_path) {
                    if meta.permissions().mode() & 0o111 == 0 {
                        diagnostics.push(ValidationDiagnostic {
                            path: format!(".lisa/hooks/{}", script),
                            category: "structure",
                            message: format!(
                                "not executable. Run: chmod +x .lisa/hooks/{}",
                                script
                            ),
                            severity: Severity::Error,
                        });
                    }
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
                    message: "no tickets with phase 'ready' and all dependencies satisfied"
                        .to_string(),
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
    print_diagnostics(&result)?;

    // On success, print config summary including timeout
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let timeout_str = if resolved.session_timeout_secs == 0 {
        "disabled".to_string()
    } else {
        format!("{}s", resolved.session_timeout_secs)
    };
    println!(
        "Config: max_threads={}, session_timeout={}",
        resolved.max_threads, timeout_str
    );
    if !resolved.phase_timeouts.is_empty() {
        let mut entries: Vec<_> = resolved.phase_timeouts.iter().collect();
        entries.sort_by_key(|(k, _)| (*k).clone());
        let parts: Vec<String> = entries
            .iter()
            .map(|(k, v)| format!("{}={}s", k, v))
            .collect();
        println!("  phase_timeouts: {}", parts.join(" "));
    }
    Ok(())
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
        Err(format!(
            "{} error(s) found. Fix and re-run `lisa validate`.",
            count
        ))
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
    use std::process::Command;

    #[test]
    fn test_plan_init_actions_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // Should plan to create:
        //   8 directories (6 docs + .lisa/hooks + .lisa/signals)
        //   13 files (the project/context/config files, six shared hook files,
        //   .lisa/.gitignore, Claude settings, and Codex hooks.json)
        let creates: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::CreateDir(_) | InitAction::CreateFile { .. }))
            .collect();
        assert_eq!(creates.len(), 21);
    }

    #[test]
    fn test_plan_init_creates_on_notify_sample() {
        let dir = tempfile::tempdir().unwrap();
        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let created: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::CreateFile { path, .. } if path.ends_with("on-notify.sample")))
            .collect();
        assert_eq!(created.len(), 1, "on-notify.sample should be scaffolded");
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
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with("CLAUDE.md")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_plan_init_actions_existing_lisa_toml_no_version() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 4\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // .lisa.toml without version should be updated
        let updated: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(".lisa.toml")))
            .collect();
        assert_eq!(updated.len(), 1);

        // Updated content should have version line
        if let InitAction::UpdateFile { content, .. } = &updated[0] {
            assert!(content.contains(&format!("version = \"{}\"", config::LISA_VERSION)));
            // Original content should be preserved
            assert!(content.contains("max_threads = 4"));
        }
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
        assert!(dir.path().join("docs/knowledge/rdspi-workflow.md").exists());
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

        // AGENTS.md is scaffolded as a pointer to CLAUDE.md.
        assert!(dir.path().join("AGENTS.md").exists());
        let agents_md = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(agents_md.contains("CLAUDE.md"));
        assert!(agents_md.contains("docs/knowledge/rdspi-workflow.md"));

        // Check .lisa.toml content
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(lisa_toml.contains("max_threads"));
        assert!(lisa_toml.contains("docs/active/tickets"));

        // Check hook infrastructure
        assert!(dir.path().join(".lisa/hooks/on-idle.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-stop.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-clear.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-notify.sample").exists());
        assert!(dir.path().join(".lisa/signals").exists());
        assert!(dir.path().join(".lisa/.gitignore").exists());
        assert!(dir.path().join(".claude/settings.local.json").exists());
        assert!(dir.path().join(".codex/hooks.json").exists());

        // Check hook script content
        for (name, ext) in &[
            ("on-idle.sh", ".idle"),
            ("on-stop.sh", ".stopped"),
            ("on-clear.sh", ".cleared"),
        ] {
            let hook =
                fs::read_to_string(dir.path().join(format!(".lisa/hooks/{}", name))).unwrap();
            assert!(
                hook.starts_with("#!/bin/sh"),
                "{} should start with shebang",
                name
            );
            assert!(
                hook.contains("LISA_PANE_ID"),
                "{} should reference LISA_PANE_ID",
                name
            );
            assert!(hook.contains(ext), "{} should write {} signal", name, ext);

            // Check hook script is executable on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::metadata(dir.path().join(format!(".lisa/hooks/{}", name)))
                    .unwrap()
                    .permissions();
                assert!(perms.mode() & 0o111 != 0, "{} should be executable", name);
            }
        }

        // Check settings.local.json content
        let settings = fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
        assert!(settings.contains("idle_prompt"));
        assert!(settings.contains("\"Stop\""));
        assert!(settings.contains("\"SessionStart\""));

        let codex_hooks = fs::read_to_string(dir.path().join(".codex/hooks.json")).unwrap();
        assert!(codex_hooks.contains("\"Stop\""));
        assert!(codex_hooks.contains("\"SessionStart\""));
        assert!(codex_hooks.contains("\"PostToolUse\""));

        // Check .lisa/.gitignore content
        let gitignore = fs::read_to_string(dir.path().join(".lisa/.gitignore")).unwrap();
        assert!(gitignore.contains("signals/"));

        // on-notify.sample is scaffolded but NOT executable (opt-in).
        let sample = fs::read_to_string(dir.path().join(".lisa/hooks/on-notify.sample")).unwrap();
        assert!(sample.starts_with("#!/bin/sh"));
        assert!(sample.contains("on-notify"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::metadata(dir.path().join(".lisa/hooks/on-notify.sample"))
                .unwrap()
                .permissions();
            assert_eq!(
                perms.mode() & 0o111,
                0,
                "on-notify.sample must not be executable"
            );
        }
    }

    #[test]
    fn test_run_init_never_overwrites_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // Create CLAUDE.md with custom content
        fs::write(dir.path().join("CLAUDE.md"), "my custom content").unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // Original CLAUDE.md should be preserved
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(claude_md, "my custom content");
    }

    #[test]
    fn test_run_init_never_overwrites_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // A user-authored AGENTS.md must be preserved (skip-if-exists, like CLAUDE.md).
        fs::write(dir.path().join("AGENTS.md"), "my custom agents content").unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        let agents_md = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert_eq!(agents_md, "my custom agents content");
    }

    #[test]
    fn test_run_init_updates_stale_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // Create .lisa.toml without version
        fs::write(
            dir.path().join(".lisa.toml"),
            "# my config\n[scheduling]\nmax_threads = 4\n",
        )
        .unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // .lisa.toml should now have version, but preserve original content
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(lisa_toml.contains(&format!("version = \"{}\"", config::LISA_VERSION)));
        assert!(lisa_toml.contains("max_threads = 4"));
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
        let hooks: &[(&str, &str)] = &[
            ("on-idle.sh", templates::ON_IDLE_HOOK),
            ("on-stop.sh", templates::ON_STOP_HOOK),
            ("on-clear.sh", templates::ON_CLEAR_HOOK),
            ("on-heartbeat.sh", templates::ON_HEARTBEAT_HOOK),
            ("on-ack.sh", templates::ON_ACK_HOOK),
        ];
        for (name, content) in hooks {
            fs::write(root.join(format!(".lisa/hooks/{}", name)), content).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o755);
                fs::set_permissions(root.join(format!(".lisa/hooks/{}", name)), perms).unwrap();
            }
        }
        // on-notify.sample is required by validate but is non-executable (opt-in).
        fs::write(
            root.join(".lisa/hooks/on-notify.sample"),
            templates::ON_NOTIFY_HOOK,
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_accepts_both_context_files() {
        // A project carrying both CLAUDE.md and AGENTS.md (as `lisa init` now
        // scaffolds) validates clean — AGENTS.md is neither required nor rejected.
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::write(dir.path().join("AGENTS.md"), templates::AGENTS_MD).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();

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
        // No docs/knowledge/rdspi-workflow.md

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error"));
    }

    #[test]
    fn test_validate_empty_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();

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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();

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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        // No docs/active/tickets directory

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_init_preserves_unknown_hook_content() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();

        // Pre-create a locally modified hook that Lisa cannot prove it owns.
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join(".lisa/hooks/on-idle.sh"),
            "old hook content",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(dir.path().join(".claude/settings.local.json"), "{}").unwrap();

        let result = run_init(dir.path(), false);
        assert!(result.is_ok());

        // The unknown hook must remain byte-for-byte unchanged.
        let hook = fs::read_to_string(dir.path().join(".lisa/hooks/on-idle.sh")).unwrap();
        assert_eq!(hook, "old hook content");
        // New hook scripts should be created
        assert!(dir.path().join(".lisa/hooks/on-stop.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-clear.sh").exists());
        // settings.local.json should be updated to include all hooks
        let settings = fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
        assert!(settings.contains("idle_prompt"));
        assert!(settings.contains("\"Stop\""));
        assert!(settings.contains("\"SessionStart\""));
    }

    #[test]
    fn test_plan_init_actions_preserves_unknown_hook() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "old content").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        // settings.local.json without idle_prompt → should plan UpdateFile
        fs::write(dir.path().join(".claude/settings.local.json"), "{}").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // An arbitrary difference is not evidence that this is a Lisa template.
        let preserved_hook: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with("on-idle.sh") && reason == "preserved: content is not a known Lisa template"))
            .collect();
        assert_eq!(preserved_hook.len(), 1);

        // settings.local.json should be updated (not skipped) since it lacks idle_prompt
        let updated_settings: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with("settings.local.json")))
            .collect();
        assert_eq!(updated_settings.len(), 1);
    }

    #[test]
    fn test_plan_init_actions_existing_hooks_current() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        // Write the current template content — should be skipped
        fs::write(
            dir.path().join(".lisa/hooks/on-idle.sh"),
            templates::ON_IDLE_HOOK,
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            templates::settings_local_json(),
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // on-idle.sh should be skipped (already up to date)
        let skipped_hook: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with("on-idle.sh")))
            .collect();
        assert_eq!(skipped_hook.len(), 1);
    }

    #[test]
    fn test_plan_init_actions_settings_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join(".lisa/hooks/on-idle.sh"),
            templates::ON_IDLE_HOOK,
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        // settings.local.json WITH all hooks → should skip
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            templates::settings_local_json(),
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let skipped_settings: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with("settings.local.json")))
            .collect();
        assert_eq!(skipped_settings.len(), 1);
    }

    #[test]
    fn test_plan_init_upgrades_old_bare_path_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-idle.sh"), "existing").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        // Old-style settings.local.json with bare-path hook commands
        let old_settings = r#"{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": ".lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-clear.sh" }] }],
    "Notification": [{ "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-idle.sh" }] }]
  }
}"#;
        fs::write(dir.path().join(".claude/settings.local.json"), old_settings).unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        // Should plan an UpdateFile (not Skip) to upgrade to guarded commands
        let update_settings: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with("settings.local.json")))
            .collect();
        assert_eq!(
            update_settings.len(),
            1,
            "Should update settings.local.json to upgrade hooks"
        );

        // Verify the updated content has guarded commands
        if let InitAction::UpdateFile { content, .. } = &update_settings[0] {
            assert!(
                content.contains("test -x .lisa/hooks/on-stop.sh"),
                "Stop hook should be guarded"
            );
            assert!(
                content.contains("test -x .lisa/hooks/on-clear.sh"),
                "Clear hook should be guarded"
            );
            assert!(
                content.contains("test -x .lisa/hooks/on-idle.sh"),
                "Idle hook should be guarded"
            );
            // No duplicate entries
            assert_eq!(
                content.matches("on-stop.sh").count(),
                2,
                "on-stop.sh should appear twice (guard + path)"
            );
        }
    }

    #[test]
    fn test_plan_init_updates_stale_version() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "version = \"0.1.0\"\n\n[scheduling]\nmax_threads = 4\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let updated: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(".lisa.toml")))
            .collect();
        assert_eq!(updated.len(), 1);

        if let InitAction::UpdateFile { content, .. } = &updated[0] {
            assert!(content.contains(&format!("version = \"{}\"", config::LISA_VERSION)));
            assert!(content.contains("max_threads = 4"));
        }
    }

    #[test]
    fn test_plan_init_skips_current_version() {
        let dir = tempfile::tempdir().unwrap();
        // Include all known keys so upsert has nothing to add
        fs::write(dir.path().join(".lisa.toml"), config::default_config_toml()).unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with(".lisa.toml")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_plan_init_upserts_missing_config_keys() {
        let dir = tempfile::tempdir().unwrap();
        // Current version but missing new keys
        fs::write(
            dir.path().join(".lisa.toml"),
            &format!(
                "version = \"{}\"\n\n[scheduling]\nmax_threads = 4\n",
                config::LISA_VERSION
            ),
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let updated: Vec<_> = actions
            .iter()
            .filter(
                |a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(".lisa.toml")),
            )
            .collect();
        assert_eq!(updated.len(), 1, "should update to add missing keys");

        // Verify the content has the new keys
        if let InitAction::UpdateFile { content, .. } = &updated[0] {
            assert!(content.contains("session_timeout_secs"));
            assert!(content.contains("phase_timeouts"));
            assert!(content.contains("review_timeout_secs"));
            assert!(content.contains("assignment_ack_timeout_secs"));
            // Original content preserved
            assert!(content.contains("max_threads = 4"));
        }
    }

    #[test]
    fn test_upsert_missing_config_keys_preserves_active_values() {
        let existing = "[scheduling]\nmax_threads = 4\nsession_timeout_secs = 900\n";
        let result = upsert_missing_config_keys(existing);
        // Should not duplicate session_timeout_secs
        assert_eq!(
            result.matches("session_timeout_secs").count(),
            1,
            "should not duplicate existing key"
        );
        // Should add missing keys
        assert!(result.contains("review_timeout_secs"));
        assert!(result.contains("phase_timeouts"));
        assert!(result.contains("assignment_ack_timeout_secs"));
    }

    #[test]
    fn test_upsert_missing_config_keys_preserves_commented_values() {
        let existing = "[scheduling]\nmax_threads = 4\n# session_timeout_secs = 3600\n";
        let result = upsert_missing_config_keys(existing);
        // Should not duplicate — commented key counts as present
        assert_eq!(
            result.matches("session_timeout_secs").count(),
            1,
            "should not duplicate commented key"
        );
    }

    #[test]
    fn test_upsert_noop_when_complete() {
        let complete = config::default_config_toml();
        let result = upsert_missing_config_keys(&complete);
        assert_eq!(result, complete, "should be no-op when all keys present");
    }

    #[test]
    fn test_plan_init_preserves_unknown_rdspi() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# Old RDSPI content",
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let preserved: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with("rdspi-workflow.md") && reason == "preserved: content is not a known Lisa template"))
            .collect();
        assert_eq!(preserved.len(), 1);
    }

    #[test]
    fn test_plan_init_skips_current_rdspi() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            templates::RDSPI_WORKFLOW,
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with("rdspi-workflow.md")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_plan_init_updates_known_prior_plain_text_templates() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            templates::LEGACY_RDSPI_WORKFLOWS[0],
        )
        .unwrap();
        for (name, content) in [
            ("on-stop.sh", templates::LEGACY_ON_STOP_HOOKS[0]),
            ("on-clear.sh", templates::LEGACY_ON_CLEAR_HOOKS[0]),
            ("on-heartbeat.sh", templates::LEGACY_ON_HEARTBEAT_HOOKS[0]),
        ] {
            fs::write(dir.path().join(format!(".lisa/hooks/{name}")), content).unwrap();
        }
        fs::write(dir.path().join(".lisa/.gitignore"), "signals/\n").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        for name in &[
            "rdspi-workflow.md",
            "on-stop.sh",
            "on-clear.sh",
            "on-heartbeat.sh",
            ".gitignore",
        ] {
            assert!(
                actions.iter().any(
                    |a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(name))
                ),
                "known prior {name} should update"
            );
        }
    }

    #[test]
    fn test_plan_init_updates_every_known_rdspi_template() {
        assert!(
            templates::LEGACY_RDSPI_WORKFLOWS
                .iter()
                .all(|legacy| *legacy != templates::RDSPI_WORKFLOW),
            "legacy workflow fixtures must be byte-distinct from current content"
        );

        for legacy in templates::LEGACY_RDSPI_WORKFLOWS {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
            fs::write(dir.path().join("docs/knowledge/rdspi-workflow.md"), legacy).unwrap();

            let project = detect_project(dir.path());
            let actions = plan_init_actions(dir.path(), &project);

            assert!(
                actions.iter().any(
                    |action| matches!(action, InitAction::UpdateFile { path, content }
                        if path.ends_with("rdspi-workflow.md")
                            && content == templates::RDSPI_WORKFLOW)
                ),
                "every exact prior Lisa workflow must upgrade to the current template"
            );
        }
    }

    #[test]
    fn test_plan_init_skips_all_current_plain_text_templates() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            templates::RDSPI_WORKFLOW,
        )
        .unwrap();
        for (name, content) in [
            ("on-idle.sh", templates::ON_IDLE_HOOK),
            ("on-stop.sh", templates::ON_STOP_HOOK),
            ("on-clear.sh", templates::ON_CLEAR_HOOK),
            ("on-heartbeat.sh", templates::ON_HEARTBEAT_HOOK),
            ("on-notify.sample", templates::ON_NOTIFY_HOOK),
        ] {
            fs::write(dir.path().join(format!(".lisa/hooks/{name}")), content).unwrap();
        }
        fs::write(
            dir.path().join(".lisa/.gitignore"),
            templates::LISA_GITIGNORE,
        )
        .unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        for name in &[
            "rdspi-workflow.md",
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-heartbeat.sh",
            "on-notify.sample",
            ".gitignore",
        ] {
            assert!(
                actions.iter().any(|a| matches!(a, InitAction::NoOp { path, reason } if path.ends_with(name) && reason == "already up to date")),
                "current {name} should be a no-op"
            );
        }
    }

    #[test]
    fn test_append_only_gitignore_handles_spacing_newlines_and_idempotence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".gitignore");

        fs::write(&path, "signals/").unwrap();
        let action = plan_append_only_gitignore(path.clone(), templates::LISA_GITIGNORE);
        let merged = match action {
            InitAction::UpdateFile { content, .. } => content,
            other => panic!("expected append-only update, got {other:?}"),
        };
        assert_eq!(merged, "signals/\nattempts/\nclaude/\ncodex/\n");
        assert!(merged.starts_with("signals/"));

        fs::write(&path, &merged).unwrap();
        assert!(matches!(
            plan_append_only_gitignore(path.clone(), templates::LISA_GITIGNORE),
            InitAction::NoOp { reason, .. } if reason == "already up to date"
        ));

        let spaced = "  signals/  \n attempts/ \n\tclaude/\t\ncodex/";
        fs::write(&path, spaced).unwrap();
        assert!(matches!(
            plan_append_only_gitignore(path, templates::LISA_GITIGNORE),
            InitAction::NoOp { .. }
        ));
        assert_eq!(
            fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            spaced
        );
    }

    #[test]
    fn test_append_only_gitignore_preserves_unreadable_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".gitignore");
        let original = [0xff, 0xfe, b'\n'];
        fs::write(&path, original).unwrap();

        assert!(matches!(
            plan_append_only_gitignore(path.clone(), templates::LISA_GITIGNORE),
            InitAction::SafetySkip { reason, .. }
                if reason == "preserved: existing file is unreadable"
        ));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn test_init_output_categories_and_mutation_report_match_write_set() {
        let dir = tempfile::tempdir().unwrap();
        let mut initial_output = Vec::new();
        run_init_with_writer(dir.path(), false, &mut initial_output).unwrap();

        let agents_path = dir.path().join("AGENTS.md");
        let gitignore_path = dir.path().join(".lisa/.gitignore");
        let workflow_path = dir.path().join("docs/knowledge/rdspi-workflow.md");
        let skipped_hook_path = dir.path().join(".lisa/hooks/on-idle.sh");
        fs::remove_file(&agents_path).unwrap();
        fs::write(&gitignore_path, "signals/\nhooks/ntfy-topic\n").unwrap();
        fs::write(&skipped_hook_path, "#!/bin/sh\n# project-owned\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&skipped_hook_path, fs::Permissions::from_mode(0o640)).unwrap();
        }

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);
        let file_paths: Vec<PathBuf> = actions
            .iter()
            .filter_map(|action| match action {
                InitAction::CreateFile { path, .. }
                | InitAction::UpdateFile { path, .. }
                | InitAction::NoOp { path, .. }
                | InitAction::SafetySkip { path, .. }
                    if !path.is_dir() =>
                {
                    Some(path.clone())
                }
                _ => None,
            })
            .collect();
        let before: Vec<(PathBuf, Option<Vec<u8>>)> = file_paths
            .iter()
            .map(|path| (path.clone(), fs::read(path).ok()))
            .collect();

        let mut dry_output = Vec::new();
        run_init_with_writer(dir.path(), true, &mut dry_output).unwrap();
        let dry_output = String::from_utf8(dry_output).unwrap();
        assert!(dry_output.contains("  create  "));
        assert!(dry_output.contains("  update  "));
        assert!(dry_output.contains("  no-op   "));
        assert!(dry_output.contains("  skip    "));
        assert!(dry_output.contains("Dry run complete. No changes made."));
        assert!(!agents_path.exists());
        assert_eq!(
            fs::read_to_string(&gitignore_path).unwrap(),
            "signals/\nhooks/ntfy-topic\n"
        );

        let mut real_output = Vec::new();
        run_init_with_writer(dir.path(), false, &mut real_output).unwrap();
        let real_output = String::from_utf8(real_output).unwrap();
        let actual_changed: Vec<PathBuf> = before
            .iter()
            .filter(|(path, old)| fs::read(path).ok() != *old)
            .map(|(path, _)| path.clone())
            .collect();
        assert_eq!(
            actual_changed,
            vec![agents_path.clone(), gitignore_path.clone()]
        );

        let report = real_output
            .split_once("Files changed:\n")
            .unwrap()
            .1
            .split_once("\nNext steps:")
            .unwrap()
            .0;
        assert_eq!(
            report,
            format!(
                "  created  {}\n  updated  {}\n",
                agents_path.display(),
                gitignore_path.display()
            )
        );
        assert!(!report.contains(&workflow_path.display().to_string()));
        assert!(!report.contains(&skipped_hook_path.display().to_string()));
        assert!(
            real_output.contains("  1. Inspect the files reported above before your next commit")
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&skipped_hook_path)
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o640, "safety-skipped hook mode changed");
        }

        let mut second_output = Vec::new();
        run_init_with_writer(dir.path(), false, &mut second_output).unwrap();
        let second_output = String::from_utf8(second_output).unwrap();
        assert!(second_output.contains("Files changed:\n  none\n"));
        assert_eq!(
            fs::read_to_string(gitignore_path).unwrap(),
            "signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\n"
        );
    }

    #[test]
    fn test_init_preserves_vend_customizations_and_secret_ignore_rule() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();

        let workflow = format!(
            "{}\n## Story Layer\n\nRead the parent story before every ticket.\n",
            templates::RDSPI_WORKFLOW
        );
        let stop_hook = format!(
            "{}\n# Project addition: notify the local supervisor.\n",
            templates::LEGACY_ON_STOP_HOOKS[0]
        );
        let notify_sample = format!(
            "{}\n# Project addition: custom notification notes.\n",
            templates::ON_NOTIFY_HOOK
        );
        let gitignore = "signals/\nhooks/ntfy-topic\n";
        let preserved_fixtures = [
            ("docs/knowledge/rdspi-workflow.md", workflow.as_bytes()),
            (".lisa/hooks/on-stop.sh", stop_hook.as_bytes()),
            (".lisa/hooks/on-notify.sample", notify_sample.as_bytes()),
        ];
        for (path, content) in preserved_fixtures {
            fs::write(dir.path().join(path), content).unwrap();
        }
        fs::write(dir.path().join(".lisa/.gitignore"), gitignore).unwrap();
        fs::write(dir.path().join(".lisa/hooks/ntfy-topic"), "secret-topic").unwrap();

        let git_init = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert!(git_init.success());

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);
        for path in preserved_fixtures.map(|(path, _)| path) {
            assert!(actions.iter().any(|a| matches!(a, InitAction::SafetySkip { path: action_path, reason } if action_path == &dir.path().join(path) && reason == "preserved: content is not a known Lisa template")));
        }
        let planned_gitignore = actions.iter().find_map(|action| match action {
            InitAction::UpdateFile { path, content }
                if path == &dir.path().join(".lisa/.gitignore") =>
            {
                Some(content)
            }
            _ => None,
        });
        assert_eq!(
            planned_gitignore.map(String::as_str),
            Some("signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\n")
        );
        assert_eq!(
            fs::read(dir.path().join("docs/knowledge/rdspi-workflow.md")).unwrap(),
            workflow.as_bytes()
        );

        run_init(dir.path(), false).unwrap();

        for (path, content) in preserved_fixtures {
            assert_eq!(
                fs::read(dir.path().join(path)).unwrap(),
                content,
                "{path} changed during real init"
            );
        }
        let upgraded_gitignore = fs::read_to_string(dir.path().join(".lisa/.gitignore")).unwrap();
        assert_eq!(
            upgraded_gitignore,
            "signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\n"
        );

        let ignored = Command::new("git")
            .args(["check-ignore", ".lisa/hooks/ntfy-topic"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            ignored.status.success(),
            "notification secret should remain ignored: {}",
            String::from_utf8_lossy(&ignored.stderr)
        );
        assert_eq!(
            String::from_utf8(ignored.stdout).unwrap().trim(),
            ".lisa/hooks/ntfy-topic"
        );
    }

    #[test]
    fn test_plan_init_preserves_non_utf8_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            [0xff, 0xfe],
        )
        .unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-stop.sh"), [0xff, 0xfe]).unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        for name in &["rdspi-workflow.md", "on-stop.sh"] {
            assert!(actions.iter().any(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with(name) && reason == "preserved: existing file is unreadable")));
            assert!(!actions
                .iter()
                .any(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(name))));
        }
    }

    #[test]
    fn test_plan_init_never_replaces_malformed_structured_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        let malformed_toml = "project_setting = [\n# keep this project content\n";
        fs::write(dir.path().join(".lisa.toml"), malformed_toml).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            "{ not valid json",
        )
        .unwrap();
        fs::write(dir.path().join(".codex/hooks.json"), "[ not valid json").unwrap();

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        for name in &["settings.local.json", "hooks.json"] {
            assert!(actions.iter().any(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with(name) && reason.contains("JSON is malformed"))));
            assert!(!actions
                .iter()
                .any(|a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(name))));
        }

        let config_update = actions.iter().find_map(|action| match action {
            InitAction::UpdateFile { path, content } if path.ends_with(".lisa.toml") => {
                Some(content)
            }
            _ => None,
        });
        assert!(
            config_update.is_some_and(|content| content.contains(malformed_toml)),
            "the textual TOML merge must retain malformed project content instead of falling back to defaults"
        );
    }

    #[test]
    fn test_run_init_upgrades_known_prior_hook() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join(".lisa/hooks/on-stop.sh"),
            templates::LEGACY_ON_STOP_HOOKS[0],
        )
        .unwrap();

        run_init(dir.path(), false).unwrap();

        assert_eq!(
            fs::read_to_string(dir.path().join(".lisa/hooks/on-stop.sh")).unwrap(),
            templates::ON_STOP_HOOK
        );
    }

    #[test]
    fn test_plan_init_preserves_unknown_plain_text_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        for name in &[
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-heartbeat.sh",
            "on-notify.sample",
        ] {
            fs::write(
                dir.path().join(format!(".lisa/hooks/{name}")),
                format!("project-owned {name}\n"),
            )
            .unwrap();
        }

        let project = detect_project(dir.path());
        let actions = plan_init_actions(dir.path(), &project);

        for name in &[
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-heartbeat.sh",
            "on-notify.sample",
        ] {
            let preserved: Vec<_> = actions
                .iter()
                .filter(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with(name) && reason == "preserved: content is not a known Lisa template"))
                .collect();
            assert_eq!(preserved.len(), 1, "{} should be preserved", name);
        }
    }

    #[test]
    fn test_validate_missing_settings_json() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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

    #[test]
    fn test_validate_missing_stop_hook() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        // Full hook infra except on-stop.sh
        write_hook_infrastructure(dir.path());
        fs::remove_file(dir.path().join(".lisa/hooks/on-stop.sh")).unwrap();
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        let stop_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.path.contains("on-stop.sh"))
            .collect();
        assert_eq!(stop_errors.len(), 1);
    }

    #[test]
    fn test_validate_missing_clear_hook() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        // Full hook infra except on-clear.sh
        write_hook_infrastructure(dir.path());
        fs::remove_file(dir.path().join(".lisa/hooks/on-clear.sh")).unwrap();
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        let clear_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.path.contains("on-clear.sh"))
            .collect();
        assert_eq!(clear_errors.len(), 1);
    }

    #[test]
    fn test_validate_missing_pretooluse_binding() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        // Full hook infra, then overwrite settings with the five legacy bindings
        // only (no PreToolUse[AskUserQuestion]).
        write_hook_infrastructure(dir.path());
        let legacy_settings = r#"{
  "hooks": {
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh" }] }],
    "Notification": [
      { "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh" }] },
      { "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-notify || exit 0" }] }
    ]
  }
}"#;
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            legacy_settings,
        )
        .unwrap();
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        let pretool_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| {
                d.severity == Severity::Error && d.message.contains("PreToolUse[AskUserQuestion]")
            })
            .collect();
        assert_eq!(
            pretool_errors.len(),
            1,
            "missing AskUserQuestion binding should flag exactly one error"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_idle_hook_not_executable() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
    fn test_init_then_validate_roundtrip_codex_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"codex-project\"\n",
        )
        .unwrap();
        run_init(dir.path(), false).unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            format!(
                "version = \"{}\"\n\n[agent]\nclient = \"codex\"\n",
                config::LISA_VERSION
            ),
        )
        .unwrap();
        write_ready_ticket(dir.path());

        assert!(run_validate(dir.path(), false).is_ok());
        let hooks = fs::read_to_string(dir.path().join(".codex/hooks.json")).unwrap();
        assert!(hooks.contains("on-stop.sh"));
        assert!(hooks.contains("on-clear.sh"));
        assert!(hooks.contains("on-heartbeat.sh"));
    }

    #[test]
    fn test_validate_codex_rejects_unrelated_hooks_with_same_event() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"codex-project\"\n",
        )
        .unwrap();
        run_init(dir.path(), false).unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            format!(
                "version = \"{}\"\n\n[agent]\nclient = \"codex\"\n",
                config::LISA_VERSION
            ),
        )
        .unwrap();
        write_ready_ticket(dir.path());
        fs::write(
            dir.path().join(".codex/hooks.json"),
            r#"{"hooks":{"PostToolUse":[{"matcher":".*","hooks":[{"type":"command","command":"./mine.sh"}]}]}}"#,
        )
        .unwrap();

        assert!(run_validate(dir.path(), false).is_err());
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
        // No hook infrastructure at all
        write_ready_ticket(dir.path());

        let result = validate(dir.path(), false);
        let hook_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| {
                d.severity == Severity::Error
                    && (d.path.contains("settings.local.json")
                        || d.path.contains("on-idle.sh")
                        || d.path.contains("on-stop.sh")
                        || d.path.contains("on-clear.sh"))
            })
            .collect();
        // 1 settings.local.json missing + 3 hook scripts missing = 4
        assert_eq!(hook_errors.len(), 4);
    }

    #[test]
    fn test_diagnostics_success_counts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            "# RDSPI",
        )
        .unwrap();
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
