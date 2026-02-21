use std::fmt;
use std::path::Path;
use std::process::Command;

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

/// Build the list of dependency checks with real command execution.
fn build_checks() -> Vec<DependencyCheck> {
    vec![
        DependencyCheck {
            name: "zellij",
            required: true,
            check: Box::new(check_zellij),
        },
        DependencyCheck {
            name: "claude",
            required: true,
            check: Box::new(check_claude),
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
pub(crate) fn check_required_deps() -> Result<(), Vec<String>> {
    check_required_deps_inner(build_checks())
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

/// Run the doctor command: check all dependencies and report.
pub fn run_doctor(root: &Path) -> Result<(), String> {
    let checks = build_checks();
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
}
