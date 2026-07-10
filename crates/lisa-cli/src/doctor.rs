use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::client::AgentClient;

use crate::config;

/// Result of checking a single dependency.
enum CheckResult {
    Found { version: String },
    NotFound { install_hint: String },
    Skipped { reason: String },
}

/// A dependency check: name, whether it's required, and a closure that performs the check.
struct DependencyCheck {
    name: &'static str,
    required: bool,
    check: Box<dyn Fn() -> CheckResult>,
}

/// The result of running a single check.
struct CheckReport {
    name: &'static str,
    required: bool,
    result: CheckResult,
}

impl fmt::Display for CheckReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.result {
            CheckResult::Found { version } => {
                write!(f, "  {:<12} {:<14} OK", self.name, version)
            }
            CheckResult::NotFound { install_hint } => {
                write!(
                    f,
                    "  {:<12} not found\n    Install: {}",
                    self.name, install_hint
                )
            }
            CheckResult::Skipped { reason } => {
                write!(f, "  {:<12} skipped ({})", self.name, reason)
            }
        }
    }
}

/// Run a command and capture the first line of stdout as a version string.
fn get_command_version(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Some(stdout.lines().next().unwrap_or("").trim().to_string())
            } else {
                None
            }
        })
}

/// Check if a binary is available on PATH.
pub(crate) fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_zellij() -> CheckResult {
    match get_command_version("zellij", &["--version"]) {
        Some(version) => CheckResult::Found { version },
        None => CheckResult::NotFound {
            install_hint:
                "cargo install zellij\n    Or visit: https://zellij.dev/documentation/installation"
                    .to_string(),
        },
    }
}

fn check_claude() -> CheckResult {
    match get_command_version("claude", &["--version"]) {
        Some(version) => CheckResult::Found { version },
        None => CheckResult::NotFound {
            install_hint: "https://docs.anthropic.com/en/docs/claude-code".to_string(),
        },
    }
}

fn check_codex() -> CheckResult {
    match get_command_version("codex", &["--version"]) {
        Some(version) => CheckResult::Found { version },
        None => CheckResult::NotFound {
            install_hint:
                "npm i -g @openai/codex\n    Or visit: https://developers.openai.com/codex"
                    .to_string(),
        },
    }
}

fn check_wasm_target() -> CheckResult {
    if !which("rustup") {
        return CheckResult::Skipped {
            reason: "rustup not found".to_string(),
        };
    }

    match Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.lines().any(|l| l.trim() == "wasm32-wasip1") {
                CheckResult::Found {
                    version: "installed".to_string(),
                }
            } else {
                CheckResult::NotFound {
                    install_hint: "rustup target add wasm32-wasip1".to_string(),
                }
            }
        }
        _ => CheckResult::Skipped {
            reason: "rustup command failed".to_string(),
        },
    }
}

/// Build the list of dependency checks for the *selected* client.
///
/// zellij and the wasm target are client-independent; the agent binary checked
/// is exactly the one the loop will drive (`claude --version` or
/// `codex --version`), never both.
fn build_checks(client: AgentClient) -> Vec<DependencyCheck> {
    let (agent_name, agent_check): (&'static str, Box<dyn Fn() -> CheckResult>) = match client {
        AgentClient::Claude => ("claude", Box::new(check_claude)),
        AgentClient::Codex => ("codex", Box::new(check_codex)),
    };
    vec![
        DependencyCheck {
            name: "zellij",
            required: true,
            check: Box::new(check_zellij),
        },
        DependencyCheck {
            name: agent_name,
            required: true,
            check: agent_check,
        },
        DependencyCheck {
            name: "wasm target",
            required: false,
            check: Box::new(check_wasm_target),
        },
    ]
}

/// Execute all checks and collect reports.
fn run_checks(checks: Vec<DependencyCheck>) -> Vec<CheckReport> {
    checks
        .into_iter()
        .map(|c| CheckReport {
            name: c.name,
            required: c.required,
            result: (c.check)(),
        })
        .collect()
}

/// Format check reports into a human-readable string.
fn format_report(reports: &[CheckReport]) -> String {
    let mut out = String::new();
    out.push_str("Checking dependencies...\n\n");

    for report in reports {
        out.push_str(&format!("{}\n", report));
    }

    out.push('\n');

    if has_failures(reports) {
        out.push_str("Some dependencies are missing. Lisa requires all of the above to run.");
    } else {
        out.push_str("All dependencies satisfied.");
    }

    out
}

/// Check if any required dependency is missing.
fn has_failures(reports: &[CheckReport]) -> bool {
    reports
        .iter()
        .any(|r| r.required && matches!(r.result, CheckResult::NotFound { .. }))
}

/// Check that all required runtime dependencies are present.
/// Returns Ok(()) if all found, Err with list of missing dep names otherwise.
pub(crate) fn check_required_deps(client: AgentClient) -> Result<(), Vec<String>> {
    check_required_deps_inner(build_checks(client))
}

fn check_required_deps_inner(checks: Vec<DependencyCheck>) -> Result<(), Vec<String>> {
    let reports = run_checks(checks);
    let missing: Vec<String> = reports
        .iter()
        .filter(|r| r.required && matches!(r.result, CheckResult::NotFound { .. }))
        .map(|r| r.name.to_string())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// Check the project version from .lisa.toml at the given path.
fn check_project_version(root: &Path) -> CheckReport {
    let config_path = root.join(".lisa.toml");
    if !config_path.exists() {
        return CheckReport {
            name: "project version",
            required: false,
            result: CheckResult::Skipped {
                reason: "no .lisa.toml in current directory".to_string(),
            },
        };
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            let parsed: Result<config::LisaConfig, _> = toml::from_str(&content);
            match parsed.ok().and_then(|c| c.version) {
                Some(v) => {
                    if config::version_is_stale(&v, config::LISA_VERSION) {
                        CheckReport {
                            name: "project version",
                            required: false,
                            result: CheckResult::NotFound {
                                install_hint: format!(
                                    "{} (current: {}). Run `lisa init` to update",
                                    v,
                                    config::LISA_VERSION
                                ),
                            },
                        }
                    } else {
                        CheckReport {
                            name: "project version",
                            required: false,
                            result: CheckResult::Found { version: v },
                        }
                    }
                }
                None => CheckReport {
                    name: "project version",
                    required: false,
                    result: CheckResult::NotFound {
                        install_hint: format!(
                            "no version field. Run `lisa init` to update (current: {})",
                            config::LISA_VERSION
                        ),
                    },
                },
            }
        }
        Err(_) => CheckReport {
            name: "project version",
            required: false,
            result: CheckResult::Skipped {
                reason: "could not read .lisa.toml".to_string(),
            },
        },
    }
}

/// Return the platform-specific Zellij plugin cache directory.
fn zellij_cache_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::Path::new(&home);

    if cfg!(target_os = "macos") {
        Some(home.join("Library/Caches/org.Zellij-Contributors.Zellij"))
    } else {
        // Linux / other Unix
        Some(home.join(".cache/zellij"))
    }
}

/// Recursively walk `cache_dir` and remove any directory or file whose name
/// contains `lisa-plugin`. Zellij nests cached plugins deep under
/// `session-uuid/file:/var/folders/.../T/lisa-plugin-*.wasm/`, so a shallow
/// walk is insufficient. Returns the number of entries removed.
pub(crate) fn clean_zellij_plugin_cache_in(cache_dir: &Path) -> usize {
    let mut removed = 0;
    let mut stack = vec![cache_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().contains("lisa-plugin") {
                let path = entry.path();
                let ok = if path.is_dir() {
                    std::fs::remove_dir_all(&path).is_ok()
                } else {
                    std::fs::remove_file(&path).is_ok()
                };
                if ok {
                    removed += 1;
                }
            } else if entry.path().is_dir() {
                stack.push(entry.path());
            }
        }
    }
    removed
}

/// Resolve the Zellij cache dir and clean any cached lisa-plugin entries.
pub(crate) fn clean_zellij_plugin_cache() {
    if let Some(dir) = zellij_cache_dir() {
        clean_zellij_plugin_cache_in(&dir);
    }
}

/// Permissions the lisa plugin requests in its `request_permission` call
/// (`lisa-plugin` `load()`). The pre-granted set MUST match these exactly, or
/// Zellij will still prompt for the missing delta.
const PLUGIN_PERMISSIONS: &[&str] = &[
    "WriteToStdin",
    "ChangeApplicationState",
    "ReadApplicationState",
    "RunCommands",
];

/// Pre-grant the lisa plugin's permissions in Zellij's `permissions.kdl`.
///
/// Zellij withholds plugin events and rendering until the plugin's requested
/// permissions are granted — normally via an interactive prompt. That prompt does
/// not reliably complete in every environment (the plugin pane renders blank with
/// no confirmation). Because each `lisa loop` writes the plugin to a fresh
/// content-hashed path Zellij has never granted, we write the grant directly so the
/// plugin is authorized without depending on the prompt. Best-effort; on any IO
/// error the loop proceeds (and falls back to the prompt). Returns true if the entry
/// is present (already-granted or freshly written).
pub(crate) fn pregrant_plugin_permissions_in(cache_dir: &Path, wasm_path: &Path) -> bool {
    let perms_path = cache_dir.join("permissions.kdl");
    let key = format!("\"{}\"", wasm_path.display());

    let existing = std::fs::read_to_string(&perms_path).unwrap_or_default();
    // Entry lines look like `"<path>" {`; the closing quote in `key` prevents a
    // shorter hash path from prefix-matching a longer one.
    if existing.lines().any(|l| l.trim_start().starts_with(&key)) {
        return true; // already granted for this exact plugin path
    }

    let perms = PLUGIN_PERMISSIONS
        .iter()
        .map(|p| format!("    {p}\n"))
        .collect::<String>();
    let entry = format!("{key} {{\n{perms}}}\n");

    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&entry);

    if std::fs::create_dir_all(cache_dir).is_err() {
        return false;
    }
    std::fs::write(&perms_path, content).is_ok()
}

/// Resolve the Zellij cache dir and pre-grant the lisa plugin's permissions.
pub(crate) fn pregrant_plugin_permissions(wasm_path: &Path) {
    if let Some(dir) = zellij_cache_dir() {
        pregrant_plugin_permissions_in(&dir, wasm_path);
    }
}

/// Resolve Codex's config home: `$CODEX_HOME` if set, else `~/.codex`.
fn codex_home() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CODEX_HOME") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    let home = std::env::var("HOME").ok()?;
    Some(Path::new(&home).join(".codex"))
}

/// Pre-seed directory trust so unattended `codex exec` does not block on the
/// interactive trust prompt.
///
/// Writes a `[projects."<abs-working-tree>"] trust_level = "trusted"` block into
/// `<codex_home>/config.toml` (the user-level file — a repo-local
/// `.codex/config.toml` cannot carry trust). Best-effort and idempotent, modeled
/// on [`pregrant_plugin_permissions_in`]: if a `[projects."<path>"]` header for
/// this tree is already present the file is left untouched. Returns true if the
/// trust entry is present (already-seeded or freshly written).
///
/// Per Codex issue #14345 this trust behaviour is version-volatile — the doctor
/// surfaces the codex version alongside the seed rather than assuming it stable,
/// and `--dangerously-bypass-approvals-and-sandbox` remains the escape hatch.
pub(crate) fn pregrant_codex_trust_in(codex_home: &Path, work_tree: &Path) -> bool {
    let config_path = codex_home.join("config.toml");
    let header = format!("[projects.\"{}\"]", work_tree.display());

    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == header) {
        return true; // a projects block for this tree already exists
    }

    let entry = format!("{header}\ntrust_level = \"trusted\"\n");
    let mut content = existing;
    if !content.is_empty() {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n'); // blank line before the appended table
    }
    content.push_str(&entry);

    if std::fs::create_dir_all(codex_home).is_err() {
        return false;
    }
    std::fs::write(&config_path, content).is_ok()
}

/// Resolve `$CODEX_HOME` and pre-seed directory trust for `work_tree`.
/// Returns the config.toml path on success (for reporting), None on failure.
pub(crate) fn pregrant_codex_trust(work_tree: &Path) -> Option<PathBuf> {
    let home = codex_home()?;
    if pregrant_codex_trust_in(&home, work_tree) {
        Some(home.join("config.toml"))
    } else {
        None
    }
}

/// Run the doctor command: check all dependencies and report.
pub fn run_doctor(root: &Path) -> Result<(), String> {
    // Determine the selected client from .lisa.toml (defaults to Claude, so a
    // project with no opt-in produces exactly today's output).
    let client = config::load_config(root)
        .map(|v| config::resolve_config(&v.config, None, None).client)
        .unwrap_or_default();

    let checks = build_checks(client);
    let mut reports = run_checks(checks);

    // Add project version check
    let project_report = check_project_version(root);
    let has_project = !matches!(project_report.result, CheckResult::Skipped { .. });

    let mut output = format_report(&reports);

    if has_project {
        output.push_str("\n\nChecking project...\n\n");
        output.push_str(&format!("{}\n", project_report));
    }
    reports.push(project_report);

    // Clean stale Zellij plugin cache
    output.push_str("\n\nChecking Zellij plugin cache...\n\n");
    if let Some(dir) = zellij_cache_dir() {
        let removed = clean_zellij_plugin_cache_in(&dir);
        if removed > 0 {
            output.push_str(&format!(
                "  Cleaned {} cached lisa-plugin entr{}\n",
                removed,
                if removed == 1 { "y" } else { "ies" }
            ));
        } else {
            output.push_str("  No stale cache entries found.\n");
        }
    } else {
        output.push_str("  Could not determine Zellij cache directory.\n");
    }

    // Codex-only: pre-seed directory trust so unattended `codex exec` doesn't
    // block on the interactive trust prompt. Best-effort; never a hard failure
    // (the bypass flag is a valid fallback, and #14345 makes trust behaviour
    // version-specific — hence the version note).
    if client == AgentClient::Codex {
        output.push_str("\n\nChecking Codex trust...\n\n");
        match pregrant_codex_trust(root) {
            Some(config_path) => {
                output.push_str(&format!(
                    "  Seeded trust_level=\"trusted\" for {}\n    in {}\n",
                    root.display(),
                    config_path.display()
                ));
                output.push_str(
                    "    Note: Codex trust behaviour is version-specific (#14345); \
                     re-run `lisa doctor` after `codex update`.\n",
                );
            }
            None => {
                output.push_str(
                    "  Could not seed Codex directory trust (set CODEX_HOME/HOME, or run\n    \
                     `codex exec` with --dangerously-bypass-approvals-and-sandbox).\n",
                );
            }
        }
    }

    println!("{}", output);

    if has_failures(&reports) {
        Err("Some dependencies are missing.".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_found(name: &'static str, version: &str) -> DependencyCheck {
        let version = version.to_string();
        DependencyCheck {
            name,
            required: true,
            check: Box::new(move || CheckResult::Found {
                version: version.clone(),
            }),
        }
    }

    fn mock_not_found(name: &'static str, hint: &str) -> DependencyCheck {
        let hint = hint.to_string();
        DependencyCheck {
            name,
            required: true,
            check: Box::new(move || CheckResult::NotFound {
                install_hint: hint.clone(),
            }),
        }
    }

    fn mock_skipped(name: &'static str, reason: &str) -> DependencyCheck {
        let reason = reason.to_string();
        DependencyCheck {
            name,
            required: false,
            check: Box::new(move || CheckResult::Skipped {
                reason: reason.clone(),
            }),
        }
    }

    #[test]
    fn test_run_checks_found() {
        let checks = vec![mock_found("zellij", "zellij 0.43.0")];
        let reports = run_checks(checks);
        assert_eq!(reports.len(), 1);
        assert!(matches!(reports[0].result, CheckResult::Found { .. }));
    }

    #[test]
    fn test_run_checks_not_found() {
        let checks = vec![mock_not_found("zellij", "cargo install zellij")];
        let reports = run_checks(checks);
        assert_eq!(reports.len(), 1);
        assert!(matches!(reports[0].result, CheckResult::NotFound { .. }));
    }

    #[test]
    fn test_run_checks_skipped() {
        let checks = vec![mock_skipped("wasm target", "rustup not found")];
        let reports = run_checks(checks);
        assert_eq!(reports.len(), 1);
        assert!(matches!(reports[0].result, CheckResult::Skipped { .. }));
    }

    #[test]
    fn test_format_report_all_ok() {
        let checks = vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("claude", "claude 1.2.3"),
        ];
        let reports = run_checks(checks);
        let output = format_report(&reports);

        assert!(output.contains("Checking dependencies..."));
        assert!(output.contains("zellij"));
        assert!(output.contains("zellij 0.43.0"));
        assert!(output.contains("OK"));
        assert!(output.contains("claude"));
        assert!(output.contains("All dependencies satisfied."));
        assert!(!output.contains("missing"));
    }

    #[test]
    fn test_format_report_with_failure() {
        let checks = vec![
            mock_not_found("zellij", "cargo install zellij"),
            mock_found("claude", "claude 1.2.3"),
        ];
        let reports = run_checks(checks);
        let output = format_report(&reports);

        assert!(output.contains("not found"));
        assert!(output.contains("Install: cargo install zellij"));
        assert!(output.contains("Some dependencies are missing."));
    }

    #[test]
    fn test_format_report_with_skipped() {
        let checks = vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("claude", "claude 1.2.3"),
            mock_skipped("wasm target", "rustup not found"),
        ];
        let reports = run_checks(checks);
        let output = format_report(&reports);

        assert!(output.contains("skipped (rustup not found)"));
        assert!(output.contains("All dependencies satisfied."));
    }

    #[test]
    fn test_has_failures_all_found() {
        let checks = vec![
            mock_found("zellij", "v0.43.0"),
            mock_found("claude", "v1.2.3"),
        ];
        let reports = run_checks(checks);
        assert!(!has_failures(&reports));
    }

    #[test]
    fn test_has_failures_required_missing() {
        let checks = vec![
            mock_not_found("zellij", "install it"),
            mock_found("claude", "v1.2.3"),
        ];
        let reports = run_checks(checks);
        assert!(has_failures(&reports));
    }

    #[test]
    fn test_has_failures_optional_skipped_not_failure() {
        let checks = vec![
            mock_found("zellij", "v0.43.0"),
            mock_found("claude", "v1.2.3"),
            mock_skipped("wasm target", "rustup not found"),
        ];
        let reports = run_checks(checks);
        assert!(!has_failures(&reports));
    }

    #[test]
    fn test_report_display_found() {
        let report = CheckReport {
            name: "zellij",
            required: true,
            result: CheckResult::Found {
                version: "zellij 0.43.0".to_string(),
            },
        };
        let s = format!("{}", report);
        assert!(s.contains("zellij"));
        assert!(s.contains("zellij 0.43.0"));
        assert!(s.contains("OK"));
    }

    #[test]
    fn test_report_display_not_found() {
        let report = CheckReport {
            name: "zellij",
            required: true,
            result: CheckResult::NotFound {
                install_hint: "cargo install zellij".to_string(),
            },
        };
        let s = format!("{}", report);
        assert!(s.contains("not found"));
        assert!(s.contains("Install: cargo install zellij"));
    }

    #[test]
    fn test_report_display_skipped() {
        let report = CheckReport {
            name: "wasm target",
            required: false,
            result: CheckResult::Skipped {
                reason: "rustup not found".to_string(),
            },
        };
        let s = format!("{}", report);
        assert!(s.contains("skipped"));
        assert!(s.contains("rustup not found"));
    }

    #[test]
    fn test_check_required_deps_all_found() {
        let checks = vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("claude", "claude 1.2.3"),
        ];
        assert!(check_required_deps_inner(checks).is_ok());
    }

    #[test]
    fn test_check_required_deps_one_missing() {
        let checks = vec![
            mock_not_found("zellij", "cargo install zellij"),
            mock_found("claude", "claude 1.2.3"),
        ];
        let result = check_required_deps_inner(checks);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing, vec!["zellij"]);
    }

    #[test]
    fn test_check_required_deps_all_missing() {
        let checks = vec![
            mock_not_found("zellij", "cargo install zellij"),
            mock_not_found("claude", "install claude"),
        ];
        let result = check_required_deps_inner(checks);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing, vec!["zellij", "claude"]);
    }

    #[test]
    fn test_check_required_deps_optional_skipped_is_ok() {
        let checks = vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("claude", "claude 1.2.3"),
            mock_skipped("wasm target", "rustup not found"),
        ];
        assert!(check_required_deps_inner(checks).is_ok());
    }

    #[test]
    fn test_build_checks_claude_selects_claude() {
        let names: Vec<&str> = build_checks(AgentClient::Claude)
            .iter()
            .map(|c| c.name)
            .collect();
        assert!(names.contains(&"claude"));
        assert!(!names.contains(&"codex"));
        assert!(names.contains(&"zellij"));
    }

    #[test]
    fn test_build_checks_codex_selects_codex() {
        let names: Vec<&str> = build_checks(AgentClient::Codex)
            .iter()
            .map(|c| c.name)
            .collect();
        assert!(names.contains(&"codex"));
        assert!(!names.contains(&"claude"));
        assert!(names.contains(&"zellij"));
    }

    #[test]
    fn test_pregrant_codex_trust_writes_block() {
        let dir = tempfile::tempdir().unwrap();
        let work = Path::new("/work/tree");
        assert!(pregrant_codex_trust_in(dir.path(), work));

        let content = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(content.contains("[projects.\"/work/tree\"]"));
        assert!(content.contains("trust_level = \"trusted\""));
    }

    #[test]
    fn test_pregrant_codex_trust_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let work = Path::new("/work/tree");
        pregrant_codex_trust_in(dir.path(), work);
        pregrant_codex_trust_in(dir.path(), work);

        let content = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        let count = content.matches("[projects.\"/work/tree\"]").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_pregrant_codex_trust_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").unwrap();

        assert!(pregrant_codex_trust_in(dir.path(), Path::new("/work/tree")));

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("model = \"gpt-5\""));
        assert!(content.contains("[projects.\"/work/tree\"]"));
    }

    #[test]
    fn test_codex_home_honors_env() {
        // Serialized implicitly: each assertion sets/removes the var in-scope.
        std::env::set_var("CODEX_HOME", "/custom/codex");
        assert_eq!(codex_home(), Some(PathBuf::from("/custom/codex")));
        std::env::remove_var("CODEX_HOME");
    }

    #[test]
    fn test_project_version_check_current() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            format!("version = \"{}\"\n", config::LISA_VERSION),
        )
        .unwrap();

        let report = check_project_version(dir.path());
        assert!(matches!(report.result, CheckResult::Found { .. }));
        if let CheckResult::Found { version } = &report.result {
            assert_eq!(version, config::LISA_VERSION);
        }
    }

    #[test]
    fn test_project_version_check_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "version = \"0.1.0\"\n").unwrap();

        let report = check_project_version(dir.path());
        assert!(matches!(report.result, CheckResult::NotFound { .. }));
        if let CheckResult::NotFound { install_hint } = &report.result {
            assert!(install_hint.contains("0.1.0"));
            assert!(install_hint.contains(config::LISA_VERSION));
            assert!(install_hint.contains("lisa init"));
        }
    }

    #[test]
    fn test_project_version_check_missing_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 2\n",
        )
        .unwrap();

        let report = check_project_version(dir.path());
        assert!(matches!(report.result, CheckResult::NotFound { .. }));
        if let CheckResult::NotFound { install_hint } = &report.result {
            assert!(install_hint.contains("no version field"));
            assert!(install_hint.contains("lisa init"));
        }
    }

    #[test]
    fn test_project_version_check_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let report = check_project_version(dir.path());
        assert!(matches!(report.result, CheckResult::Skipped { .. }));
    }

    #[test]
    fn test_clean_cache_no_entries() {
        let dir = tempfile::tempdir().unwrap();
        // Mimic Zellij structure: session/file:/path/to/other-plugin
        let nested = dir.path().join("session-abc/file:/var/folders/xx/T");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(nested.join("some-other-plugin")).unwrap();

        let removed = clean_zellij_plugin_cache_in(dir.path());
        assert_eq!(removed, 0);
        assert!(nested.join("some-other-plugin").exists());
    }

    #[test]
    fn test_clean_cache_removes_lisa_entries() {
        let dir = tempfile::tempdir().unwrap();
        // Mimic real Zellij cache: session/file:/var/folders/.../T/lisa-plugin-hash.wasm
        let nested = dir.path().join("session-abc/file:/var/folders/xx/T");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(nested.join("lisa-plugin-abc123.wasm")).unwrap();
        std::fs::create_dir_all(nested.join("other-plugin")).unwrap();

        let removed = clean_zellij_plugin_cache_in(dir.path());
        assert_eq!(removed, 1);
        assert!(!nested.join("lisa-plugin-abc123.wasm").exists());
        assert!(nested.join("other-plugin").exists());
    }

    #[test]
    fn test_clean_cache_nonexistent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("does-not-exist");
        let removed = clean_zellij_plugin_cache_in(&nonexistent);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_pregrant_writes_all_requested_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = Path::new("/tmp/lisa-plugin-deadbeef.wasm");

        assert!(pregrant_plugin_permissions_in(dir.path(), wasm));

        let content = std::fs::read_to_string(dir.path().join("permissions.kdl")).unwrap();
        assert!(content.contains("\"/tmp/lisa-plugin-deadbeef.wasm\" {"));
        // The granted set must match exactly what the plugin requests.
        for perm in PLUGIN_PERMISSIONS {
            assert!(content.contains(perm), "missing permission {perm}");
        }
    }

    #[test]
    fn test_pregrant_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = Path::new("/tmp/lisa-plugin-deadbeef.wasm");

        pregrant_plugin_permissions_in(dir.path(), wasm);
        pregrant_plugin_permissions_in(dir.path(), wasm);

        let content = std::fs::read_to_string(dir.path().join("permissions.kdl")).unwrap();
        // Exactly one entry for this path, no duplicate block.
        let count = content
            .matches("\"/tmp/lisa-plugin-deadbeef.wasm\" {")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_pregrant_preserves_existing_entries() {
        let dir = tempfile::tempdir().unwrap();
        let perms_path = dir.path().join("permissions.kdl");
        std::fs::write(
            &perms_path,
            "\"/tmp/lisa-plugin-old.wasm\" {\n    WriteToStdin\n}\n",
        )
        .unwrap();

        let wasm = Path::new("/tmp/lisa-plugin-new.wasm");
        assert!(pregrant_plugin_permissions_in(dir.path(), wasm));

        let content = std::fs::read_to_string(&perms_path).unwrap();
        assert!(content.contains("lisa-plugin-old.wasm"));
        assert!(content.contains("lisa-plugin-new.wasm"));
    }
}
