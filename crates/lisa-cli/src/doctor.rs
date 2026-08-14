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
    Found {
        version: String,
    },
    NotFound {
        install_hint: String,
    },
    Unsupported {
        description: String,
        remedy: String,
    },
    /// The thing is here and out of date. Its own word, because "unsupported"
    /// is a different claim: a Lisa one release behind its channel still runs,
    /// and the row's job is to name the gap and the command that settles it.
    Behind {
        description: String,
        remedy: String,
    },
    Skipped {
        reason: String,
    },
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
            CheckResult::Behind {
                description,
                remedy,
            } => {
                write!(
                    f,
                    "  {:<12} behind\n    {}\n    Remedy: {}",
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
            remedy: "On Linux, set `[runtime] zellij = \"managed\"` and Lisa downloads its own managed runtime. Elsewhere, install Zellij yourself (macOS: `brew install zellij`) and set `[runtime] zellij = \"system\"`, or configure a compatible absolute path.".to_string(),
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
fn build_required_deps_checks(client: AgentClient, require_git: bool) -> Vec<DependencyCheck> {
    let (agent_name, agent_check): (&'static str, Box<dyn Fn() -> CheckResult>) = match client {
        AgentClient::Claude => ("claude", Box::new(check_claude)),
        AgentClient::Codex => ("codex", Box::new(check_codex)),
    };
    let mut checks = Vec::new();
    if require_git {
        checks.push(DependencyCheck {
            name: "git",
            required: true,
            check: Box::new(check_git),
        });
    }
    checks.push(DependencyCheck {
        name: agent_name,
        required: true,
        check: agent_check,
    });
    checks
}

/// The one-command remedy for Claude's one-time bypass confirmation. Lisa
/// never accepts this on the operator's behalf — Claude's own dialog calls
/// bypass mode highly unadvised, so the choice stays a conscious human one.
/// Lisa's job is only to say it plainly before a pane can freeze on it.
pub(crate) const CLAUDE_FIRST_RUN_REMEDY: &str =
    "Run `claude --dangerously-skip-permissions` once in your own terminal, \
read Claude's confirmation, and accept it if you're comfortable — then exit. \
Claude remembers that choice, and Lisa's sessions won't stop at a prompt no \
one can see. Lisa never accepts it for you.";

/// Whether this machine has completed Claude's one-time bypass-permissions
/// confirmation (`bypassPermissionsModeAccepted` in `$HOME/.claude.json`).
/// Missing file, missing key, or unreadable JSON all read as not-yet —
/// conservative, and never an error.
pub(crate) fn claude_bypass_accepted() -> bool {
    std::env::var_os("HOME")
        .map(|home| claude_bypass_accepted_in(Path::new(&home)))
        .unwrap_or(false)
}

pub(crate) fn claude_bypass_accepted_in(home: &Path) -> bool {
    // Claude Code has recorded this acceptance in two places across versions:
    // older builds set `bypassPermissionsModeAccepted` in `~/.claude.json`;
    // current builds set `skipDangerousModePermissionPrompt` in
    // `~/.claude/settings.json` (field observation, 2026-07-18: a real
    // operator acceptance wrote only the latter). Either signal counts.
    json_bool_flag(&home.join(".claude.json"), "bypassPermissionsModeAccepted")
        || json_bool_flag(
            &home.join(".claude/settings.json"),
            "skipDangerousModePermissionPrompt",
        )
}

fn json_bool_flag(path: &Path, key: &str) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|bytes| serde_json::from_str::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value.get(key).and_then(|flag| flag.as_bool()))
        .unwrap_or(false)
}

fn check_claude_first_run() -> CheckResult {
    if claude_bypass_accepted() {
        CheckResult::Found {
            version: "one-time confirmation accepted".to_string(),
        }
    } else {
        CheckResult::Skipped {
            reason: format!("pending — {CLAUDE_FIRST_RUN_REMEDY}"),
        }
    }
}

fn check_git_journal_optional() -> CheckResult {
    match get_command_version("git", &["--version"]) {
        Some(version) => CheckResult::Found { version },
        None => CheckResult::Skipped {
            reason: "not present — fine here: finished work is recorded in Lisa's journal. \
                     Install git if you want undoable history."
                .to_string(),
        },
    }
}

fn build_checks(client: AgentClient, require_git: bool) -> Vec<DependencyCheck> {
    // Doctor also diagnoses packaging and optional developer-tool state. Loop
    // retains its adjacent empty-WASM guard so that failure has loop-specific
    // shell-installer guidance at the point before the bytes are consumed.
    // Git is a required dependency only when the resolved completion seal is
    // commit; on the journal tier it is informational — a machine without git
    // is a supported environment, never a doctor failure.
    let mut checks = build_required_deps_checks(client, require_git);
    if !require_git {
        checks.push(DependencyCheck {
            name: "git",
            required: false,
            check: Box::new(check_git_journal_optional),
        });
    }
    // Friendly reminder, never a failure: unattended Claude sessions need the
    // operator's one-time bypass confirmation. `lisa loop` enforces it with
    // the same remedy; doctor just gives the heads-up early.
    if client == AgentClient::Claude {
        checks.push(DependencyCheck {
            name: "claude unattended",
            required: false,
            check: Box::new(check_claude_first_run),
        });
    }
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
    } else if reports
        .iter()
        .any(|report| matches!(report.result, CheckResult::Behind { .. }))
    {
        // Nothing here stops a run, so the closing line still says so — and
        // still says the gap is there, rather than letting "satisfied" be the
        // last word an operator reads on a machine that is behind.
        out.push_str(
            "All dependencies satisfied. One check is behind, and its row names the command that settles it.",
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
pub(crate) fn check_required_deps(
    client: AgentClient,
    require_git: bool,
) -> Result<(), Vec<String>> {
    check_required_deps_inner(build_required_deps_checks(client, require_git))
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

/// How many stale-ticket findings are listed before the rest are counted.
///
/// A board can carry hundreds of tickets at a retired phase, and a page of
/// identical lines is a section an operator scrolls past. The count still says
/// how many there are.
const MAX_LISTED_PER_KIND: usize = 5;

/// Render the project-currency inventory as one more doctor section.
///
/// Doctor's shape is one line per check, a verdict, and — when something is
/// wrong — the command that fixes it. Project currency is the same shape asked
/// of the project rather than the machine. Every judgment here comes from
/// [`crate::currency::inventory`]; this function only decides how much of it a
/// person should read at once.
fn format_project_currency(currency: &crate::currency::ProjectCurrency) -> String {
    use crate::currency::CurrencyKind;

    let mut out = String::from("\n\nChecking project currency...\n\n");

    if currency.is_current() {
        out.push_str(&format!(
            "  This project is current with Lisa {}.\n",
            currency.current_version
        ));
        return out;
    }

    let mut listed_per_kind = std::collections::HashMap::new();
    let mut suppressed: Vec<(CurrencyKind, usize)> = Vec::new();

    for finding in &currency.findings {
        let listed = listed_per_kind.entry(finding.kind).or_insert(0usize);
        if *listed >= MAX_LISTED_PER_KIND {
            match suppressed
                .iter_mut()
                .find(|(kind, _)| *kind == finding.kind)
            {
                Some((_, count)) => *count += 1,
                None => suppressed.push((finding.kind, 1)),
            }
            continue;
        }
        *listed += 1;

        out.push_str(&format!(
            "  {:<8} {}\n    {}\n    Remedy: {}\n",
            finding.kind.label(),
            finding.subject,
            finding.detail,
            finding.remedy.line()
        ));
    }

    for (kind, count) in suppressed {
        out.push_str(&format!(
            "  {:<8} ... and {} more like the above\n",
            kind.label(),
            count
        ));
    }

    out
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
    "OpenTerminalsOrPlugins",
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

/// Ask this machine which Lisa it wants, and the published release list what
/// that channel resolves to right now.
///
/// Every failure on the way is a reportable state rather than an error: an
/// unreadable machine config, an unreachable release list, and a channel that
/// is holding this cycle are all things an operator needs said out loud, and
/// none of them is a reason for the rest of `doctor` not to run.
fn look_at_lisa() -> Result<crate::freshness::Freshness, String> {
    let installed = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("this build's own version is unreadable: {error}"))?;
    let machine = crate::channel::config_path().and_then(|path| crate::channel::load_from(&path));

    // Which of the three sources answers here is decided by how Lisa got onto
    // the machine: a package-managed box reads its channel off the package it
    // has, and every other box reads it out of the config file.
    let setting = installed_channel()
        .and_then(|derived| crate::freshness::package_setting(&derived, &machine))
        .unwrap_or_else(|| crate::freshness::machine_setting(&machine));
    let releases = crate::upgrade::fetch_releases_within(crate::upgrade::DOCTOR_LIST_TIMEOUT);

    Ok(crate::freshness::assess(
        installed,
        setting,
        &machine.unwrap_or_default(),
        releases.as_deref().map_err(|error| error.clone()),
        crate::channel::now_unix(),
    ))
}

/// What the package that installed this Lisa says about its channel, when a
/// package installed it. `None` on a box where `current_exe` cannot be read at
/// all, which is a state the rest of the report survives.
fn installed_channel() -> Option<crate::upgrade::install_channel::Derived> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = crate::upgrade::classify_install(&exe, home.as_deref());
    Some(crate::upgrade::install_channel::derive(&method, &exe))
}

/// How many lisas this machine could run, and which one it finds first.
///
/// A real state, and the one that made `installed_lisa()`'s bug: a box can
/// carry a package-manager Lisa *and* one the shell installer wrote into
/// `~/.local/bin`, and then PATH order decides which one runs — including which
/// one a nightly job upgrades. `lisa upgrade --tag` on a Homebrew box creates it
/// deliberately, because it is the only rollback Homebrew has. Reporting it is
/// the fix: nothing here removes anything.
fn shadowed_install_report() -> CheckReport {
    let name = "lisa install";
    let Ok(exe) = std::env::current_exe() else {
        return CheckReport {
            name,
            required: false,
            result: CheckResult::Skipped {
                reason: "cannot find the running lisa".to_string(),
            },
        };
    };
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = crate::upgrade::classify_install(&exe, home.as_deref());

    let installer_copy = home
        .as_deref()
        .map(crate::upgrade::installer_owned_path)
        .filter(|path| path.exists())
        .filter(|path| path.canonicalize().map(|it| it != exe).unwrap_or(true));

    let packaged = crate::upgrade::install_channel::package_lisa_path(&method, &exe);

    match (packaged, installer_copy) {
        (Some(packaged), Some(installer_copy)) => {
            let manager = if matches!(method, crate::upgrade::InstallMethod::Homebrew) {
                "Homebrew"
            } else {
                "apt"
            };
            let first = first_lisa_on_path();
            CheckResult::Unsupported {
                description: format!(
                    "this machine has two lisas: {manager}'s at {} and the shell installer's at \
                     {}. PATH order decides which one runs{}, and only the {manager} one \
                     follows this box's channel",
                    packaged.display(),
                    installer_copy.display(),
                    match &first {
                        Some(found) => format!(" — right now that is {}", found.display()),
                        None => String::new(),
                    }
                ),
                remedy: format!(
                    "Keep one. To go back to the packaged lisa:\n    rm {}\nTo keep the pinned \
                     one instead, leave it and remember that {manager} upgrades will not move it.",
                    installer_copy.display()
                ),
            }
        }
        _ => CheckResult::Found {
            version: format!("one lisa, at {}", exe.display()),
        },
    }
    .into_report(name)
}

/// The `lisa` the operator's shell actually finds.
fn first_lisa_on_path() -> Option<PathBuf> {
    let found = get_command_version("sh", &["-c", "command -v lisa"])?;
    if found.is_empty() {
        None
    } else {
        Some(PathBuf::from(found))
    }
}

impl CheckResult {
    /// Name a result, which is all a report is.
    fn into_report(self, name: &'static str) -> CheckReport {
        CheckReport {
            name,
            required: false,
            result: self,
        }
    }
}

/// The `lisa` row itself, in the shape every other row uses.
///
/// Informational on purpose. A box one release behind its channel still runs,
/// and a `doctor` that started failing on drift would fail on every machine in
/// the fleet at once — including, every time a release is cut, the desk that
/// built it. The row names the gap; moving is still a decision someone makes.
fn lisa_report(check: &Result<crate::freshness::Freshness, String>) -> CheckReport {
    use crate::freshness::State;

    let result = match check {
        Err(reason) => CheckResult::Skipped {
            reason: reason.clone(),
        },
        Ok(freshness) => match (&freshness.state, freshness.remedy()) {
            (State::Behind { .. }, Some(remedy)) => CheckResult::Behind {
                description: freshness.summary(),
                remedy,
            },
            (State::Unresolved { .. }, _) => CheckResult::Skipped {
                reason: freshness.summary(),
            },
            _ => CheckResult::Found {
                version: freshness.summary(),
            },
        },
    };

    CheckReport {
        name: "lisa",
        required: false,
        result,
    }
}

/// Everything `doctor` looked at, gathered once.
///
/// The prose report and the `--json` document are two renderings of this, so
/// the two cannot disagree about what was found — a fleet asked by script and a
/// fleet read on a terminal have to be the same fleet.
struct Findings {
    client: AgentClient,
    announcement: &'static str,
    completion: Result<crate::completion_seal::RunCompletionSeal, String>,
    lisa: Result<crate::freshness::Freshness, String>,
    /// The machine's rows: `lisa`, `zellij`, then the dependency checks.
    reports: Vec<CheckReport>,
    project: CheckReport,
    has_project: bool,
}

fn gather(root: &Path) -> Result<Findings, String> {
    let validation = config::load_config(root)?;
    let resolved_config = config::resolve_config(&validation.config, None, None);
    let client = resolved_config.client;
    let completion = crate::completion_seal::resolve_for_run(root, resolved_config.completion_mode);

    // Journal tier: git is informational, not required. An unresolvable
    // explicit-commit stance keeps git required — the operator chose it.
    let require_git = match &completion {
        Ok(resolved) => resolved.seal() == lisa_core::completion::CompletionSeal::Commit,
        Err(_) => true,
    };

    // Lisa itself comes first: the one version this report used to leave out is
    // the one that decides what every row below it even means.
    let lisa = look_at_lisa();
    let mut reports = vec![
        lisa_report(&lisa),
        shadowed_install_report(),
        CheckReport {
            name: "zellij",
            required: true,
            result: check_zellij_runtime(&resolved_config.zellij_runtime),
        },
    ];
    reports.extend(run_checks(build_checks(client, require_git)));

    let project = check_project_version(root);
    let has_project = !matches!(project.result, CheckResult::Skipped { .. });

    Ok(Findings {
        client,
        announcement: resolved_config.client_announcement(),
        completion,
        lisa,
        reports,
        project,
        has_project,
    })
}

/// One check, as a script reads it.
fn check_to_json(report: &CheckReport) -> serde_json::Value {
    let (status, detail, remedy) = match &report.result {
        CheckResult::Found { version } => ("ok", Some(version.clone()), None),
        CheckResult::NotFound { install_hint } => ("missing", None, Some(install_hint.clone())),
        CheckResult::Unsupported {
            description,
            remedy,
        } => (
            "unsupported",
            Some(description.clone()),
            Some(remedy.clone()),
        ),
        CheckResult::Behind {
            description,
            remedy,
        } => ("behind", Some(description.clone()), Some(remedy.clone())),
        CheckResult::Skipped { reason } => ("skipped", Some(reason.clone()), None),
    };

    serde_json::json!({
        "name": report.name,
        "required": report.required,
        "status": status,
        "detail": detail,
        "remedy": remedy,
    })
}

/// Run `lisa doctor --json`: the same findings, as one document.
///
/// Reporting only. The prose run cleans Zellij's plugin cache and seeds Codex
/// trust on its way past; a document a script collects from every box in the
/// fleet does neither.
/// Emit the document and return the exit status that goes with it. The status
/// keeps its existing meaning: `--json` adds a body, it does not replace the
/// verdict.
pub fn run_doctor_json(root: &Path) -> i32 {
    match doctor_document(root) {
        Ok((data, exit_code)) => crate::json_output::emit(
            "doctor",
            crate::json_output::Outcome::verdict(data, exit_code),
        ),
        Err(message) => {
            crate::json_output::emit("doctor", crate::json_output::Outcome::Failure(message))
        }
    }
}

fn doctor_document(root: &Path) -> Result<(serde_json::Value, i32), String> {
    let findings = gather(root)?;
    let mut reports = findings.reports;
    reports.push(findings.project);

    let failed = has_failures(&reports) || findings.completion.is_err();

    let data = serde_json::json!({
        "verdict": if failed { "failed" } else { "passed" },
        "client": findings.client.as_str(),
        "lisa": match &findings.lisa {
            Ok(freshness) => freshness.to_json(),
            Err(error) => serde_json::json!({ "error": error }),
        },
        "completion_seal": match &findings.completion {
            Ok(completion) => serde_json::json!(match completion.seal() {
                lisa_core::completion::CompletionSeal::Commit => "commit",
                lisa_core::completion::CompletionSeal::Journal => "journal",
            }),
            Err(_) => serde_json::Value::Null,
        },
        "completion_error": match &findings.completion {
            Ok(_) => None,
            Err(error) => Some(error.clone()),
        },
        "checks": reports.iter().map(check_to_json).collect::<Vec<_>>(),
    });

    Ok((data, i32::from(failed)))
}

/// Run the doctor command: check all dependencies and report.
pub fn run_doctor(root: &Path) -> Result<(), String> {
    let findings = gather(root)?;
    let client = findings.client;
    let completion = findings.completion;
    let mut reports = findings.reports;
    let project_report = findings.project;
    let has_project = findings.has_project;

    let mut output = format!("{}\n\n{}", findings.announcement, format_report(&reports));

    if has_project {
        output.push_str("\n\nChecking project...\n\n");
        output.push_str(&format!("{}\n", project_report));
        // Informational on purpose: a project three versions behind still runs,
        // and refusing to work is not doctor's job. Nothing below changes the
        // exit code.
        output.push_str(&format_project_currency(&crate::currency::inventory(root)));
    }
    append_completion_seal_report(&mut output, &completion);
    reports.push(project_report);

    output.push_str("\n\nChecking the environment agents will start in...\n\n");
    output.push_str(&format_inherited_session_markers(
        &lisa_core::session_env::inherited_markers(),
    ));

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

    if has_failures(&reports) || completion.is_err() {
        Err("Some required dependencies are unavailable or unsupported.".to_string())
    } else {
        Ok(())
    }
}

/// Report what this shell would hand down to an agent, and say plainly that
/// Lisa drops it rather than leaving the operator to run `env -u` themselves.
///
/// Informational, never a failure: an operator who runs `lisa doctor` from
/// inside their own agent session has done nothing wrong, and the run they
/// start next is already protected.
fn format_inherited_session_markers(markers: &[String]) -> String {
    if markers.is_empty() {
        return "  Nothing here marks another agent's session.\n".to_string();
    }
    format!(
        "  This shell is inside an agent session; {} marker{} would reach a new agent:\n    {}\n  \
         Lisa drops them when it starts a run, so an agent never inherits another one's identity.\n",
        markers.len(),
        if markers.len() == 1 { "" } else { "s" },
        markers.join("\n    "),
    )
}

fn append_completion_seal_line(output: &mut String, seal: lisa_core::completion::CompletionSeal) {
    output.push_str("  ");
    output.push_str(crate::completion_seal::visibility_line(seal));
    output.push('\n');
}

fn append_completion_seal_report(
    output: &mut String,
    completion: &Result<crate::completion_seal::RunCompletionSeal, String>,
) {
    output.push_str("\n\nChecking completion...\n\n");
    match completion {
        Ok(completion) => {
            append_completion_seal_line(output, completion.seal());
            if let Some(reason) = completion.commit_unavailable() {
                output.push_str("\n  Reason: ");
                output.push_str(&reason.to_string());
                output.push_str(".\n\n");
                output.push_str(
                    &completion
                        .commit_unavailable_remedy()
                        .expect("an unavailable reason always has a remedy"),
                );
                output.push('\n');
            }
        }
        Err(error) => {
            output.push_str("  ");
            output.push_str(error);
            output.push('\n');
        }
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

    /// The pre-granted set is the plugin's own `request_permission` list, and
    /// nothing checked that until it was wrong.
    ///
    /// A permission the plugin asks for and this list omits does not fail
    /// loudly: Zellij shows a prompt inside the plugin pane, the pane renders
    /// blank, and the loop waits forever for a keypress nobody knows to make.
    /// Measured against Zellij 0.44.3 while adding `OpenTerminalsOrPlugins` —
    /// the whole board simply never scheduled, and the only evidence was an
    /// empty pane.
    #[test]
    fn the_pre_granted_permissions_are_the_ones_the_plugin_asks_for() {
        let plugin_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../lisa-plugin/src/lib.rs"
        ));
        let call = plugin_source
            .split_once("request_permission(&[")
            .expect("the plugin still requests its permissions in one call")
            .1
            .split_once("]);")
            .expect("the request_permission call is still closed")
            .0;

        let requested: Vec<&str> = call
            .split(',')
            .filter_map(|entry| entry.trim().strip_prefix("PermissionType::"))
            .collect();

        assert!(!requested.is_empty(), "no permissions parsed from: {call}");
        let mut requested_sorted = requested.clone();
        requested_sorted.sort_unstable();
        let mut granted_sorted = PLUGIN_PERMISSIONS.to_vec();
        granted_sorted.sort_unstable();
        assert_eq!(
            requested_sorted, granted_sorted,
            "the plugin asks for {requested:?} but the pre-grant writes {PLUGIN_PERMISSIONS:?}; \
             Zellij prompts for the difference and the loop hangs on a blank pane"
        );
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

    /// The operator reading `lisa doctor` is the one person who can see the
    /// poisoned shell before a run starts, so this line has to name every
    /// marker it found — during the field failure the count alone would have
    /// been a shrug. It must also stay informational: nothing here is the
    /// operator's mistake, and the run they start next is already protected.
    #[test]
    fn the_environment_section_names_every_marker_and_says_lisa_drops_them() {
        let line = format_inherited_session_markers(&[
            "CLAUDECODE".to_string(),
            "CLAUDE_CODE_CHILD_SESSION".to_string(),
            "CLAUDE_CODE_SESSION_ID".to_string(),
        ]);
        assert!(line.contains("3 markers"), "{line}");
        for marker in [
            "CLAUDECODE",
            "CLAUDE_CODE_CHILD_SESSION",
            "CLAUDE_CODE_SESSION_ID",
        ] {
            assert!(line.contains(marker), "{marker} unnamed in: {line}");
        }
        assert!(line.contains("Lisa drops them"), "{line}");

        let single = format_inherited_session_markers(&["CLAUDECODE".to_string()]);
        assert!(single.contains("1 marker would reach"), "{single}");
    }

    #[test]
    fn a_clean_shell_gets_one_plain_sentence_and_no_warning() {
        let line = format_inherited_session_markers(&[]);
        assert_eq!(line, "  Nothing here marks another agent's session.\n");
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
    fn doctor_completion_fixtures_show_both_plain_language_tiers() {
        for (seal, expected) in [
            (
                lisa_core::completion::CompletionSeal::Commit,
                "completion seal: commit-sealed — finished work lands as history",
            ),
            (
                lisa_core::completion::CompletionSeal::Journal,
                "completion seal: journal-only — finished work is recorded but not undoable",
            ),
        ] {
            let mut output = String::new();
            append_completion_seal_line(&mut output, seal);
            assert!(output.contains(expected));
        }

        let mut journal = String::new();
        append_completion_seal_line(&mut journal, lisa_core::completion::CompletionSeal::Journal);
        assert!(!journal.to_ascii_lowercase().contains("git"));
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
        let names: Vec<&str> = build_checks(AgentClient::Claude, true)
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
        let names: Vec<&str> = build_checks(AgentClient::Codex, true)
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
    fn test_build_checks_journal_tier_keeps_git_informational_only() {
        let checks = build_checks(AgentClient::Claude, false);
        let git: Vec<_> = checks.iter().filter(|c| c.name == "git").collect();
        assert_eq!(git.len(), 1, "journal tier still shows one git line");
        assert!(!git[0].required, "journal tier never requires git");
    }

    #[test]
    fn test_claude_first_run_reminder_is_informational_and_claude_only() {
        let claude = build_checks(AgentClient::Claude, true);
        let reminder: Vec<_> = claude
            .iter()
            .filter(|c| c.name == "claude unattended")
            .collect();
        assert_eq!(reminder.len(), 1);
        assert!(
            !reminder[0].required,
            "the first-run reminder must never fail doctor"
        );
        assert!(build_checks(AgentClient::Codex, true)
            .iter()
            .all(|c| c.name != "claude unattended"));
    }

    #[test]
    fn test_claude_bypass_acceptance_reads_conservatively() {
        let dir = tempfile::tempdir().unwrap();
        // Missing file → not accepted.
        assert!(!claude_bypass_accepted_in(dir.path()));
        // Malformed JSON → not accepted, never an error.
        std::fs::write(dir.path().join(".claude.json"), "{not json").unwrap();
        assert!(!claude_bypass_accepted_in(dir.path()));
        // Key false → not accepted.
        std::fs::write(
            dir.path().join(".claude.json"),
            "{\"bypassPermissionsModeAccepted\": false}",
        )
        .unwrap();
        assert!(!claude_bypass_accepted_in(dir.path()));
        // Accepted, legacy location.
        std::fs::write(
            dir.path().join(".claude.json"),
            "{\"bypassPermissionsModeAccepted\": true}",
        )
        .unwrap();
        assert!(claude_bypass_accepted_in(dir.path()));
    }

    #[test]
    fn test_claude_bypass_acceptance_reads_current_settings_location() {
        // Field shape, 2026-07-18: Claude Code 2.1.211 records the acceptance
        // as skipDangerousModePermissionPrompt in ~/.claude/settings.json and
        // writes nothing to ~/.claude.json.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::write(
            dir.path().join(".claude/settings.json"),
            "{\"model\": \"sonnet\", \"skipDangerousModePermissionPrompt\": true}",
        )
        .unwrap();
        assert!(claude_bypass_accepted_in(dir.path()));

        std::fs::write(
            dir.path().join(".claude/settings.json"),
            "{\"skipDangerousModePermissionPrompt\": false}",
        )
        .unwrap();
        assert!(!claude_bypass_accepted_in(dir.path()));
    }

    #[test]
    fn test_loop_required_deps_include_git_but_leave_wasm_to_loop_guard() {
        let names: Vec<&str> = build_required_deps_checks(AgentClient::Claude, true)
            .iter()
            .map(|c| c.name)
            .collect();

        assert!(names.contains(&"git"));
        assert!(names.contains(&"claude"));
        assert!(!names.contains(&"embedded WASM"));
        assert!(!names.contains(&"wasm target"));
    }

    #[test]
    fn test_journal_loop_required_deps_exclude_git() {
        let names: Vec<&str> = build_required_deps_checks(AgentClient::Codex, false)
            .iter()
            .map(|c| c.name)
            .collect();

        assert_eq!(names, vec!["codex"]);
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

    // ---------------------------------------------------------------------
    // The `lisa` row. The rules it reports are unit-tested in
    // `src/freshness.rs`; what follows is how those states become a row an
    // operator reads, and what that row does to doctor's verdict.
    // ---------------------------------------------------------------------

    fn freshness_at(installed: &str, setting: crate::freshness::Setting) -> CheckReport {
        const NOW: i64 = 1_786_000_000;
        let releases = [
            crate::channel::Release::from_tag("v0.4.4", NOW - 26 * 86_400).unwrap(),
            crate::channel::Release::from_tag("v0.5.0-rc.2", NOW - 5 * 86_400).unwrap(),
        ];
        lisa_report(&Ok(crate::freshness::assess(
            semver::Version::parse(installed).unwrap(),
            setting,
            &crate::channel::MachineConfig::default(),
            Ok(&releases),
            NOW,
        )))
    }

    #[test]
    fn the_lisa_row_names_the_channel_the_version_and_what_the_channel_resolves_to() {
        let row = freshness_at(
            "0.4.4",
            crate::freshness::Setting::Chosen(crate::channel::Channel::Stable),
        )
        .to_string();

        assert!(
            row.contains("  lisa "),
            "the row is named like the rest: {row}"
        );
        assert!(row.contains("channel stable"), "{row}");
        assert!(row.contains("installed 0.4.4"), "{row}");
        assert!(row.contains("stable resolves to v0.4.4"), "{row}");
        assert!(row.contains("OK"), "a level box is OK and quiet: {row}");
    }

    #[test]
    fn a_lisa_behind_its_channel_is_a_named_row_that_carries_its_command() {
        let report = freshness_at(
            "0.4.3",
            crate::freshness::Setting::Chosen(crate::channel::Channel::Stable),
        );
        let row = report.to_string();

        assert!(row.contains("behind"), "{row}");
        assert!(!row.contains("OK"), "a gap is not an OK row: {row}");
        assert!(row.contains("Remedy: "), "{row}");
        assert!(row.contains("lisa upgrade"), "{row}");

        // Being behind is a thing to know, never a reason a run cannot start:
        // every machine in the fleet is behind the moment a release is cut.
        assert!(!has_failures(&[report]));
    }

    #[test]
    fn an_unset_channel_is_reported_as_unset_and_asks_for_one() {
        let row = freshness_at("0.4.3", crate::freshness::Setting::Unset).to_string();

        assert!(row.contains("channel unset (treated as stable)"), "{row}");
        assert!(row.contains("lisa upgrade --channel <name>"), "{row}");
    }

    /// Offline, the row says what it could not do and reports the installed
    /// version. It must not read as OK: nothing checked whether this box is
    /// level, and a green row would be a claim nobody made.
    #[test]
    fn with_no_release_list_the_row_says_so_instead_of_claiming_level() {
        let report = lisa_report(&Ok(crate::freshness::assess(
            semver::Version::parse("0.4.4").unwrap(),
            crate::freshness::Setting::Chosen(crate::channel::Channel::Nightly),
            &crate::channel::MachineConfig::default(),
            Err("cannot read the release list at https://api.github.com/…: dns error".to_string()),
            1_786_000_000,
        )));
        let row = report.to_string();

        assert!(row.contains("skipped"), "{row}");
        assert!(row.contains("installed 0.4.4"), "{row}");
        assert!(row.contains("could not be resolved"), "{row}");
        assert!(!row.contains("resolves to v"), "{row}");
        assert!(!has_failures(&[report]));
    }

    #[test]
    fn a_behind_row_still_says_every_dependency_is_satisfied_and_names_the_gap() {
        let reports = vec![
            freshness_at(
                "0.4.3",
                crate::freshness::Setting::Chosen(crate::channel::Channel::Stable),
            ),
            CheckReport {
                name: "git",
                required: true,
                result: CheckResult::Found {
                    version: "git version 2.39.5".to_string(),
                },
            },
        ];
        let output = format_report(&reports);

        assert!(output.contains("All dependencies satisfied."), "{output}");
        assert!(output.contains("One check is behind"), "{output}");
    }

    /// The tool section is the part of `lisa doctor` people already know, and
    /// project currency was added beside it rather than into it. Pinned whole,
    /// byte for byte, so a change to that section has to be deliberate.
    #[test]
    fn test_tool_section_output_is_byte_identical() {
        let reports = run_checks(vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("git", "git version 2.39.5"),
            mock_found("claude", "claude 1.2.3"),
            mock_skipped("wasm target", "rustup not found"),
            mock_not_found("codex", "https://github.com/openai/codex"),
            mock_unsupported(
                "zellij",
                "detected Zellij 0.40.1; supported range >= 0.43.0",
                ZELLIJ_INSTALL_REMEDY,
            ),
        ]);

        assert_eq!(
            format_report(&reports),
            "Checking dependencies...\n\
             \n\
             \x20 zellij       zellij 0.43.0  OK\n\
             \x20 git          git version 2.39.5 OK\n\
             \x20 claude       claude 1.2.3   OK\n\
             \x20 wasm target  skipped (rustup not found)\n\
             \x20 codex        not found\n\
             \x20   Install: https://github.com/openai/codex\n\
             \x20 zellij       unsupported\n\
             \x20   detected Zellij 0.40.1; supported range >= 0.43.0\n\
             \x20   Remedy: Use Zellij's prebuilt static binaries: https://github.com/zellij-org/zellij/releases\n\
             \n\
             Some required dependencies are unavailable or unsupported. Lisa requires supported versions of all required tools to run."
        );
    }

    #[test]
    fn test_fresh_init_project_gets_one_current_line() {
        let dir = tempfile::tempdir().unwrap();
        crate::init::run_init(dir.path(), false, crate::init::HistoryPreference::NoHistory)
            .expect("init must succeed in an empty directory");

        assert_eq!(
            format_project_currency(&crate::currency::inventory(dir.path())),
            format!(
                "\n\nChecking project currency...\n\n  This project is current with Lisa {}.\n",
                config::LISA_VERSION
            )
        );
    }

    #[test]
    fn test_every_rendered_finding_names_its_remedy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "version = \"0.4.0\"\n\n[scheduling]\nmax_threads = 2\nauto_advance = true\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            crate::templates::LEGACY_WORKFLOWS[2],
        )
        .unwrap();
        std::fs::write(
            dir.path().join("docs/active/tickets/T-024-01.md"),
            "---\nid: T-024-01\nphase: structure\n---\n\n## Context\n",
        )
        .unwrap();

        let section = format_project_currency(&crate::currency::inventory(dir.path()));

        assert!(section.starts_with("\n\nChecking project currency...\n\n"));
        assert!(!section.contains("This project is current"));
        // Doctor's rule: never a finding without the thing to do about it.
        let subject_lines = section
            .lines()
            .filter(|line| {
                ["behind", "retired", "ticket"]
                    .iter()
                    .any(|kind| line.starts_with(&format!("  {kind}")))
            })
            .count();
        assert_eq!(subject_lines, section.matches("    Remedy: ").count());
        assert!(subject_lines >= 3, "rendered section: {section}");
        assert!(section.contains("run `lisa init`"));
        assert!(section.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(section.contains("auto_advance"));
        assert!(section.contains("docs/active/tickets/T-024-01.md"));
    }

    /// A project too old to record a version is old, not broken. Doctor says so
    /// and still exits successfully — nothing in the currency inventory feeds
    /// [`has_failures`], and the project check itself is informational.
    #[test]
    fn test_pre_versioning_project_does_not_fail_the_run() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 2\n",
        )
        .unwrap();

        let mut reports = run_checks(vec![
            mock_found("zellij", "zellij 0.43.0"),
            mock_found("git", "git version 2.39.5"),
        ]);
        reports.push(check_project_version(dir.path()));
        assert!(!has_failures(&reports));

        let section = format_project_currency(&crate::currency::inventory(dir.path()));
        assert!(section.contains("Set up before Lisa recorded a version"));
        assert!(section.contains("run `lisa init`"));
    }
}
