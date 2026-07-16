use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use directories::ProjectDirs;
use lisa_core::client::AgentClient;
use lisa_core::version::SUPPORTED_ZELLIJ_RANGE;
#[cfg(test)]
use lisa_core::version::{classify_zellij_version_output, ZellijVersionVerdict};

use crate::config;
use crate::runtime::{ResolvedZellijRuntime, ZellijRuntimeRequest};
use crate::templates::PLUGIN_WASM;

pub(crate) const LISA_SHELL_INSTALL_COMMAND: &str = "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh";

#[cfg(test)]
const ZELLIJ_INSTALL_REMEDY: &str =
    "Use Zellij's prebuilt static binaries: https://github.com/zellij-org/zellij/releases";

/// Result of checking a single dependency.
enum CheckResult {
    Found { version: String },
    NotFound { install_hint: String },
    Unsupported { description: String, remedy: String },
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
            CheckResult::Unsupported {
                description,
                remedy,
            } => {
                write!(
                    f,
                    "  {:<12} unsupported\n    {}\n    Remedy: {}",
                    self.name, description, remedy
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

#[cfg(test)]
fn check_zellij_version_output(output: &str) -> CheckResult {
    match classify_zellij_version_output(output) {
        ZellijVersionVerdict::InRange(version) => CheckResult::Found {
            version: format!("detected {version}, supported {SUPPORTED_ZELLIJ_RANGE}"),
        },
        ZellijVersionVerdict::BelowFloor(version) => CheckResult::Unsupported {
            description: format!(
                "detected Zellij {version}; supported range {SUPPORTED_ZELLIJ_RANGE}"
            ),
            remedy: ZELLIJ_INSTALL_REMEDY.to_string(),
        },
        ZellijVersionVerdict::Unparseable => CheckResult::Unsupported {
            description: format!(
                "unparseable Zellij version output {:?}; supported range {SUPPORTED_ZELLIJ_RANGE}",
                output.trim()
            ),
            remedy: ZELLIJ_INSTALL_REMEDY.to_string(),
        },
    }
}

fn resolved_zellij_check(runtime: &ResolvedZellijRuntime) -> CheckResult {
    CheckResult::Found {
        version: format!(
            "mode {}, version {}, supported {}, path {}",
            runtime.mode,
            runtime.version,
            SUPPORTED_ZELLIJ_RANGE,
            runtime.path.display()
        ),
    }
}

fn check_zellij_runtime(request: &ZellijRuntimeRequest) -> CheckResult {
    match crate::runtime::resolve_zellij_runtime(request) {
        Ok(runtime) => resolved_zellij_check(&runtime),
        Err(description) => CheckResult::Unsupported {
            description,
            remedy: "Set `[runtime] zellij = \"managed\"` to use Lisa's managed runtime, or configure a compatible absolute path.".to_string(),
        },
    }
}

fn check_git() -> CheckResult {
    match get_command_version("git", &["--version"]) {
        Some(version) => CheckResult::Found { version },
        None => CheckResult::NotFound {
            install_hint: "sudo apt install git".to_string(),
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

fn check_embedded_wasm_bytes(wasm: &[u8]) -> CheckResult {
    if wasm.is_empty() {
        CheckResult::Unsupported {
            description: "this Lisa binary contains an empty embedded WASM plugin placeholder"
                .to_string(),
            remedy: format!(
                "Reinstall Lisa with the shell installer:\n    {LISA_SHELL_INSTALL_COMMAND}"
            ),
        }
    } else {
        CheckResult::Found {
            version: "plugin embedded".to_string(),
        }
    }
}

fn check_embedded_wasm() -> CheckResult {
    check_embedded_wasm_bytes(PLUGIN_WASM)
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

/// Build the external dependency checks shared by doctor and loop preflight.
///
/// The Zellij runtime is resolved independently so its configured mode and path
/// are preserved. The agent binary checked is exactly the one the loop drives
/// (`claude --version` or `codex --version`), never both.
fn build_required_deps_checks(client: AgentClient) -> Vec<DependencyCheck> {
    let (agent_name, agent_check): (&'static str, Box<dyn Fn() -> CheckResult>) = match client {
        AgentClient::Claude => ("claude", Box::new(check_claude)),
        AgentClient::Codex => ("codex", Box::new(check_codex)),
    };
    vec![
        DependencyCheck {
            name: "git",
            required: true,
            check: Box::new(check_git),
        },
        DependencyCheck {
            name: agent_name,
            required: true,
            check: agent_check,
        },
    ]
}

fn build_checks(client: AgentClient) -> Vec<DependencyCheck> {
    // Doctor also diagnoses packaging and optional developer-tool state. Loop
    // retains its adjacent empty-WASM guard so that failure has loop-specific
    // shell-installer guidance at the point before the bytes are consumed.
    let mut checks = build_required_deps_checks(client);
    checks.extend([
        DependencyCheck {
            name: "embedded WASM",
            required: true,
            check: Box::new(check_embedded_wasm),
        },
        DependencyCheck {
            name: "wasm target",
            required: false,
            check: Box::new(check_wasm_target),
        },
    ]);
    checks
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
        out.push_str(
            "Some required dependencies are unavailable or unsupported. Lisa requires supported versions of all required tools to run.",
        );
    } else {
        out.push_str("All dependencies satisfied.");
    }

    out
}

/// Check if any required dependency is unavailable or unsupported.
fn has_failures(reports: &[CheckReport]) -> bool {
    reports.iter().any(|r| {
        r.required
            && matches!(
                r.result,
                CheckResult::NotFound { .. } | CheckResult::Unsupported { .. }
            )
    })
}

/// Check that all non-Zellij runtime dependencies are present.
/// Returns Ok(()) if all are found and supported, or rendered failure details
/// for unavailable and unsupported dependencies otherwise.
pub(crate) fn check_required_deps(client: AgentClient) -> Result<(), Vec<String>> {
    check_required_deps_inner(build_required_deps_checks(client))
}

fn check_required_deps_inner(checks: Vec<DependencyCheck>) -> Result<(), Vec<String>> {
    let reports = run_checks(checks);
    let failures: Vec<String> = reports
        .iter()
        .filter(|r| {
            r.required
                && matches!(
                    r.result,
                    CheckResult::NotFound { .. } | CheckResult::Unsupported { .. }
                )
        })
        .map(ToString::to_string)
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
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

/// Return the Zellij plugin cache directory using Zellij 0.43's project identity.
fn zellij_cache_dir() -> Option<PathBuf> {
    ProjectDirs::from("org", "Zellij Contributors", "Zellij")
        .map(|project_dirs| project_dirs.cache_dir().to_path_buf())
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
/// Existing work trees are canonicalized before the header is built so the
/// trusted path matches Codex's resolved cwd (notably macOS's `/var` ->
/// `/private/var` temp-directory alias). If canonicalization fails, the supplied
/// path is retained under the existing best-effort policy.
///
/// Per Codex issue #14345 this trust behaviour is version-volatile — the doctor
/// surfaces the codex version alongside the seed rather than assuming it stable,
/// and `--dangerously-bypass-approvals-and-sandbox` remains the escape hatch.
pub(crate) fn pregrant_codex_trust_in(codex_home: &Path, work_tree: &Path) -> bool {
    let config_path = codex_home.join("config.toml");
    let work_tree = work_tree
        .canonicalize()
        .unwrap_or_else(|_| work_tree.to_path_buf());
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
    let validation = config::load_config(root)?;
    let resolved_config = config::resolve_config(&validation.config, None, None);
    let client = resolved_config.client;

    let checks = build_checks(client);
    let mut reports = vec![CheckReport {
        name: "zellij",
        required: true,
        result: check_zellij_runtime(&resolved_config.zellij_runtime),
    }];
    reports.extend(run_checks(checks));

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
        Err("Some required dependencies are unavailable or unsupported.".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use std::ffi::OsString;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use std::sync::Mutex;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    static ZELLIJ_CACHE_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    struct ScopedZellijCacheEnv {
        home: Option<OsString>,
        xdg_cache_home: Option<OsString>,
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    impl ScopedZellijCacheEnv {
        fn set(home: &Path, xdg_cache_home: Option<&Path>) -> Self {
            let previous = Self {
                home: std::env::var_os("HOME"),
                xdg_cache_home: std::env::var_os("XDG_CACHE_HOME"),
            };
            std::env::set_var("HOME", home);
            match xdg_cache_home {
                Some(path) => std::env::set_var("XDG_CACHE_HOME", path),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
            previous
        }

        fn restore(name: &str, value: Option<&OsString>) {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    impl Drop for ScopedZellijCacheEnv {
        fn drop(&mut self) {
            Self::restore("HOME", self.home.as_ref());
            Self::restore("XDG_CACHE_HOME", self.xdg_cache_home.as_ref());
        }
    }

    #[cfg(target_os = "linux")]
    fn expected_zellij_cache_dir(home: &Path, xdg_cache_home: Option<&Path>) -> PathBuf {
        xdg_cache_home
            .map(|path| path.join("zellij"))
            .unwrap_or_else(|| home.join(".cache/zellij"))
    }

    #[cfg(target_os = "macos")]
    fn expected_zellij_cache_dir(home: &Path, _xdg_cache_home: Option<&Path>) -> PathBuf {
        home.join("Library/Caches/org.Zellij-Contributors.Zellij")
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn assert_zellij_cache_routing(expected: &Path) {
        assert_eq!(zellij_cache_dir().as_deref(), Some(expected));

        let cached_plugin = expected
            .join("session-abc/file:/tmp")
            .join("lisa-plugin-deadbeef.wasm");
        std::fs::create_dir_all(&cached_plugin).unwrap();
        clean_zellij_plugin_cache();
        assert!(!cached_plugin.exists());

        let wasm = Path::new("/tmp/lisa-plugin-deadbeef.wasm");
        pregrant_plugin_permissions(wasm);
        let content = std::fs::read_to_string(expected.join("permissions.kdl")).unwrap();
        assert!(content.contains("\"/tmp/lisa-plugin-deadbeef.wasm\" {"));
        for permission in PLUGIN_PERMISSIONS {
            assert!(
                content.contains(permission),
                "missing permission {permission}"
            );
        }
    }

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

    fn mock_unsupported(name: &'static str, description: &str, remedy: &str) -> DependencyCheck {
        let description = description.to_string();
        let remedy = remedy.to_string();
        DependencyCheck {
            name,
            required: true,
            check: Box::new(move || CheckResult::Unsupported {
                description: description.clone(),
                remedy: remedy.clone(),
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

    fn format_zellij_report(output: &str) -> String {
        CheckReport {
            name: "zellij",
            required: true,
            result: check_zellij_version_output(output),
        }
        .to_string()
    }

    fn format_resolved_runtime_report(
        mode: crate::runtime::ZellijRuntimeMode,
        version: &str,
        path: &str,
    ) -> String {
        let runtime = ResolvedZellijRuntime {
            mode,
            version: lisa_core::version::ZellijVersion::parse_command_output(&format!(
                "zellij {version}"
            ))
            .unwrap(),
            path: PathBuf::from(path),
        };
        CheckReport {
            name: "zellij",
            required: true,
            result: resolved_zellij_check(&runtime),
        }
        .to_string()
    }

    #[test]
    fn test_runtime_report_names_mode_version_and_path_for_every_mode() {
        for (mode, label, path) in [
            (
                crate::runtime::ZellijRuntimeMode::Packaged,
                "packaged",
                "/usr/libexec/lisa/zellij",
            ),
            (
                crate::runtime::ZellijRuntimeMode::Managed,
                "managed",
                "/data/lisa/runtime/zellij-0.43.1/zellij",
            ),
            (
                crate::runtime::ZellijRuntimeMode::System,
                "system",
                "/usr/local/bin/zellij",
            ),
            (
                crate::runtime::ZellijRuntimeMode::Pinned,
                "pinned",
                "/opt/zellij/bin/zellij",
            ),
        ] {
            let report = format_resolved_runtime_report(mode, "0.43.1", path);
            assert!(report.contains(&format!("mode {label}")));
            assert!(report.contains("version 0.43.1"));
            assert!(report.contains("supported >= 0.43.0"));
            assert!(report.contains(&format!("path {path}")));
            assert!(report.contains("OK"));
        }
    }

    #[test]
    fn test_zellij_043_reports_detected_version_and_supported_range() {
        let report = format_zellij_report("zellij 0.43.7\n");

        assert!(report.contains("detected 0.43.7"));
        assert!(report.contains(&format!("supported {SUPPORTED_ZELLIJ_RANGE}")));
        assert!(report.contains("OK"));
    }

    #[test]
    fn test_zellij_044_reports_detected_version_and_supported_range() {
        let report = format_zellij_report("zellij 0.44.2");

        assert!(report.contains("detected 0.44.2"));
        assert!(report.contains(&format!("supported {SUPPORTED_ZELLIJ_RANGE}")));
        assert!(report.contains("OK"));
    }

    #[test]
    fn test_zellij_below_floor_is_unsupported_with_static_binary_remedy() {
        let report = format_zellij_report("zellij 0.40.1");

        assert!(report.contains("unsupported"));
        assert!(report.contains("detected Zellij 0.40.1"));
        assert!(report.contains(&format!("supported range {SUPPORTED_ZELLIJ_RANGE}")));
        assert!(report.contains("prebuilt static binaries"));
        assert!(report.contains("https://github.com/zellij-org/zellij/releases"));
    }

    #[test]
    fn test_unparseable_zellij_output_is_named_unsupported() {
        let report = format_zellij_report("zellij definitely-not-a-version");

        assert!(report.contains("unsupported"));
        assert!(report
            .contains("unparseable Zellij version output \"zellij definitely-not-a-version\""));
        assert!(report.contains(&format!("supported range {SUPPORTED_ZELLIJ_RANGE}")));
        assert!(report.contains("prebuilt static binaries"));
    }

    #[test]
    fn test_zellij_runtime_failure_names_managed_runtime_remedy() {
        let report = CheckReport {
            name: "zellij",
            required: true,
            result: check_zellij_runtime(&ZellijRuntimeRequest::Pinned(PathBuf::from(
                "/definitely/missing/zellij",
            ))),
        }
        .to_string();

        assert!(report.contains("unsupported"));
        assert!(report.contains("managed runtime"));
        assert!(report.contains("[runtime] zellij = \"managed\""));
    }

    #[test]
    fn test_embedded_wasm_check_accepts_nonempty_plugin() {
        assert!(matches!(
            check_embedded_wasm_bytes(b"not-empty"),
            CheckResult::Found { .. }
        ));
    }

    #[test]
    fn test_embedded_wasm_check_names_placeholder_and_shell_installer() {
        let report = CheckReport {
            name: "embedded WASM",
            required: true,
            result: check_embedded_wasm_bytes(b""),
        }
        .to_string();

        assert!(report.contains("embedded WASM"));
        assert!(report.contains("unsupported"));
        assert!(report.contains("empty embedded WASM plugin placeholder"));
        assert!(report.contains("lisa-cli-installer.sh"));
        assert!(report.contains(LISA_SHELL_INSTALL_COMMAND));
        assert!(!report.contains("git clone"));
        assert!(!report.contains("just release"));
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
        let checks = vec![mock_not_found("git", "sudo apt install git")];
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
            mock_not_found("git", "sudo apt install git"),
            mock_found("claude", "claude 1.2.3"),
        ];
        let reports = run_checks(checks);
        let output = format_report(&reports);

        assert!(output.contains("not found"));
        assert!(output.contains("Install: sudo apt install git"));
        assert!(output.contains("Some required dependencies are unavailable or unsupported."));
    }

    #[test]
    fn test_format_report_with_unsupported_dependency() {
        let checks = vec![
            mock_unsupported(
                "zellij",
                "detected Zellij 0.40.1; supported range >= 0.43.0",
                ZELLIJ_INSTALL_REMEDY,
            ),
            mock_found("claude", "claude 1.2.3"),
        ];
        let reports = run_checks(checks);
        let output = format_report(&reports);

        assert!(output.contains("unsupported"));
        assert!(output.contains("detected Zellij 0.40.1"));
        assert!(output.contains("Some required dependencies are unavailable or unsupported."));
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
    fn test_has_failures_required_unsupported() {
        let checks = vec![mock_unsupported(
            "zellij",
            "detected Zellij 0.40.1; supported range >= 0.43.0",
            ZELLIJ_INSTALL_REMEDY,
        )];
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
            name: "git",
            required: true,
            result: CheckResult::NotFound {
                install_hint: "sudo apt install git".to_string(),
            },
        };
        let s = format!("{}", report);
        assert!(s.contains("not found"));
        assert!(s.contains("Install: sudo apt install git"));
    }

    #[test]
    fn test_report_display_unsupported() {
        let report = CheckReport {
            name: "zellij",
            required: true,
            result: CheckResult::Unsupported {
                description: "detected Zellij 0.40.1; supported range >= 0.43.0".to_string(),
                remedy: ZELLIJ_INSTALL_REMEDY.to_string(),
            },
        };
        let output = report.to_string();

        assert!(output.contains("zellij"));
        assert!(output.contains("unsupported"));
        assert!(output.contains("detected Zellij 0.40.1"));
        assert!(output.contains("Remedy:"));
        assert!(output.contains("prebuilt static binaries"));
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
            mock_not_found("git", "sudo apt install git"),
            mock_found("claude", "claude 1.2.3"),
        ];
        let result = check_required_deps_inner(checks);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("git"));
        assert!(missing[0].contains("not found"));
        assert!(missing[0].contains("sudo apt install git"));
    }

    #[test]
    fn test_check_required_deps_all_missing() {
        let checks = vec![
            mock_not_found("git", "sudo apt install git"),
            mock_not_found("claude", "install claude"),
        ];
        let result = check_required_deps_inner(checks);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing.len(), 2);
        assert!(missing[0].contains("git"));
        assert!(missing[0].contains("sudo apt install git"));
        assert!(missing[1].contains("claude"));
        assert!(missing[1].contains("install claude"));
    }

    #[test]
    fn test_check_required_deps_preserves_unsupported_details() {
        let checks = vec![
            mock_unsupported(
                "zellij",
                "detected Zellij 0.40.1; supported range >= 0.43.0",
                ZELLIJ_INSTALL_REMEDY,
            ),
            mock_found("claude", "claude 1.2.3"),
        ];
        let failures = check_required_deps_inner(checks).unwrap_err();

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("zellij"));
        assert!(failures[0].contains("detected Zellij 0.40.1"));
        assert!(failures[0].contains("supported range >= 0.43.0"));
        assert!(failures[0].contains("prebuilt static binaries"));
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
        assert!(names.contains(&"git"));
        assert!(names.contains(&"embedded WASM"));
        assert!(!names.contains(&"codex"));
        assert!(!names.contains(&"zellij"));
    }

    #[test]
    fn test_build_checks_codex_selects_codex() {
        let names: Vec<&str> = build_checks(AgentClient::Codex)
            .iter()
            .map(|c| c.name)
            .collect();
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"git"));
        assert!(names.contains(&"embedded WASM"));
        assert!(!names.contains(&"claude"));
        assert!(!names.contains(&"zellij"));
    }

    #[test]
    fn test_loop_required_deps_include_git_but_leave_wasm_to_loop_guard() {
        let names: Vec<&str> = build_required_deps_checks(AgentClient::Claude)
            .iter()
            .map(|c| c.name)
            .collect();

        assert!(names.contains(&"git"));
        assert!(names.contains(&"claude"));
        assert!(!names.contains(&"embedded WASM"));
        assert!(!names.contains(&"wasm target"));
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

    #[cfg(unix)]
    #[test]
    fn test_pregrant_codex_trust_matches_canonicalized_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        let project_link = dir.path().join("project-link");
        let codex_home = dir.path().join("codex-home");
        std::fs::create_dir(&project).unwrap();
        std::os::unix::fs::symlink(&project, &project_link).unwrap();

        let codex_cwd = project_link.canonicalize().unwrap();
        assert_ne!(project_link, codex_cwd, "fixture must resolve a symlink");
        assert!(pregrant_codex_trust_in(&codex_home, &project_link));

        let content = std::fs::read_to_string(codex_home.join("config.toml")).unwrap();
        let canonical_entry = format!(
            "[projects.\"{}\"]\ntrust_level = \"trusted\"",
            codex_cwd.display()
        );
        let alias_header = format!("[projects.\"{}\"]", project_link.display());
        assert!(content.contains(&canonical_entry));
        assert!(!content.contains(&alias_header));
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_zellij_cache_wrappers_honor_configured_environment() {
        let _lock = ZELLIJ_CACHE_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg_cache_home = dir.path().join("xdg-cache");
        let _env = ScopedZellijCacheEnv::set(&home, Some(&xdg_cache_home));
        let expected = expected_zellij_cache_dir(&home, Some(&xdg_cache_home));

        assert_zellij_cache_routing(&expected);

        #[cfg(target_os = "linux")]
        let non_selected = home.join(".cache/zellij");
        #[cfg(target_os = "macos")]
        let non_selected = xdg_cache_home.join("zellij");
        assert!(!non_selected.join("permissions.kdl").exists());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_zellij_cache_wrappers_honor_unconfigured_environment() {
        let _lock = ZELLIJ_CACHE_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let _env = ScopedZellijCacheEnv::set(&home, None);
        let expected = expected_zellij_cache_dir(&home, None);

        assert_zellij_cache_routing(&expected);
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
