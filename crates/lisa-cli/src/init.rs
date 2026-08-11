use std::fmt;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::config;
use crate::currency;
use crate::detect::detect_project;
use crate::templates;

const HISTORY_OFFER: &str = "Bring project history along? Finished work can be undone, and you'll have a record of what the agents did. [Y/n] ";
const HISTORY_DECLINED: &str = "Continuing without project history: finished work will be recorded in Lisa's journal but won't be undoable.";
const HISTORY_KEPT: &str = "Keeping project history — finished work will be undoable.";
const HISTORY_NAME: &str = "Lisa (project history)";
const HISTORY_EMAIL: &str = "lisa@project";
const HISTORY_COMMIT_MESSAGE: &str = "Start project history";

/// The operator's requested project-history behavior for `lisa init`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPreference {
    Ask,
    WithHistory,
    NoHistory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryState {
    Unavailable { reason: String },
    Missing,
    Unborn { root: PathBuf },
    Born,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HistoryAction {
    None,
    CreateRepository,
    CreateInitialCommit { root: PathBuf },
    Decline,
}

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

    // Every fixed [scheduling] key comes from the shared config catalog. Active
    // and commented assignments both count as present, so customized values
    // and earlier stubs remain byte-for-byte untouched.
    for entry in config::CONFIG_KEYS
        .iter()
        .filter(|entry| entry.section == "scheduling")
    {
        if !has_key(&result, entry.section, entry.key) {
            if let Some(end) = find_section_end(&result, "scheduling") {
                result = insert_after(&result, end, &format!("{}\n", entry.commented_stub()));
            }
        }
    }

    // Append a complete inert block without trimming or normalizing any byte
    // that was already in the user's file.
    let append_commented_section = |content: &str, section: &str| -> String {
        let mut out = content.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }

        out.push_str(&format!("# [{section}]\n"));
        for entry in config::CONFIG_KEYS
            .iter()
            .filter(|entry| entry.section == section)
        {
            out.push_str(&entry.commented_stub());
            out.push('\n');
        }
        out
    };

    // These sections were added after the earliest Lisa project templates.
    // Only a genuinely absent section is appended; a user's active or
    // commented section is complete ownership evidence and is never repaired.
    for section in ["agent", "guards", "triage"] {
        if !has_section(&result, section) {
            result = append_commented_section(&result, section);
        }
    }

    result
}

/// An action that init will perform
#[derive(Debug, Clone)]
pub enum InitAction {
    CreateDir(PathBuf),
    CreateFile {
        path: PathBuf,
        content: String,
    },
    UpdateFile {
        path: PathBuf,
        content: String,
    },
    NoOp {
        path: PathBuf,
        reason: String,
    },
    SafetySkip {
        path: PathBuf,
        reason: String,
    },
    /// Delete a file Lisa installed at a path it no longer installs to.
    ///
    /// The only action that destroys a file, and it is reachable only through a
    /// [`crate::currency::Disposition::RemoveFile`], which requires exact bytes
    /// from a bundled generation. It is an action rather than a side effect so
    /// `--dry-run` shows it before it happens.
    RemoveFile {
        path: PathBuf,
        reason: String,
    },
    /// A `.lisa.toml` key Lisa no longer reads, named so `--dry-run` shows it.
    ///
    /// Reporting only. The bytes ride in the `.lisa.toml` [`InitAction::UpdateFile`]
    /// planned above — one file, one write — so the execute loop does nothing
    /// for this action. It exists because `update  .lisa.toml` does not tell an
    /// operator which key is about to disappear, and the preview is what they
    /// read before letting init touch a repository they have work in.
    RetireConfigKey {
        path: PathBuf,
        section: &'static str,
        key: &'static str,
        reason: String,
    },
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
            InitAction::RemoveFile { path, reason } => {
                write!(f, "  remove  {} ({})", path.display(), reason)
            }
            InitAction::RetireConfigKey {
                path,
                section,
                key,
                reason,
            } => write!(
                f,
                "  remove  {} [{section}] {key} ({reason})",
                path.display()
            ),
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

/// Turn the shared retirement detection into plan lines.
///
/// One verb — `remove` — for the destructive class, and the existing `skip`
/// for everything Lisa recognises and will not touch. Every preserved reason
/// arrives carrying the `preserved:` prefix the rest of the plan uses, so a
/// preserved ticket cannot be misread as a ticket Lisa declined to schedule.
///
/// `config_key_dropped` says whether the `.lisa.toml` content this plan will
/// actually write came back without the key. Init never removes the key on its
/// own judgment and never reports a removal it did not make: the two answers
/// are the same answer.
///
/// Retired-phase tickets are the one unbounded group, and a board with two
/// hundred of them would push the removals off the top of the preview. Five are
/// listed, then one aggregate line. The cap lives here rather than in the
/// detector because the inventory has to stay complete for `lisa doctor`; this
/// is a decision about a preview, and it belongs to the thing rendering one.
fn plan_retirements(retired: &[currency::Retirement], config_key_dropped: bool) -> Vec<InitAction> {
    const TICKET_PREVIEW_LIMIT: usize = 5;

    let mut actions = Vec::new();
    let mut tickets_seen = 0usize;
    let mut tickets_hidden = 0usize;
    let mut ticket_dir = None;

    for retirement in retired {
        if matches!(
            retirement.kind,
            currency::RetirementKind::TicketPhase { .. }
        ) {
            tickets_seen += 1;
            ticket_dir = retirement
                .disposition
                .path()
                .parent()
                .map(Path::to_path_buf)
                .or(ticket_dir);
            if tickets_seen > TICKET_PREVIEW_LIMIT {
                tickets_hidden += 1;
                continue;
            }
        }

        actions.push(match &retirement.disposition {
            currency::Disposition::RemoveFile { path, reason } => InitAction::RemoveFile {
                path: path.clone(),
                reason: reason.clone(),
            },
            currency::Disposition::DropConfigKey {
                path,
                section,
                key,
                reason,
            } if config_key_dropped => InitAction::RetireConfigKey {
                path: path.clone(),
                section,
                key,
                reason: reason.clone(),
            },
            // The removal was authorized from the file on disk but the content
            // this run would write kept the key — a `.lisa.toml` that changed
            // under us, and not a removal to announce.
            currency::Disposition::DropConfigKey {
                path, section, key, ..
            } => InitAction::SafetySkip {
                path: path.clone(),
                reason: format!(
                    "preserved: [{section}] {key} is inert since 0.5.0, but this run could not \
                         rewrite the file"
                ),
            },
            currency::Disposition::Preserve { path, reason } => InitAction::SafetySkip {
                path: path.clone(),
                reason: reason.clone(),
            },
        });
    }

    if let (true, Some(dir)) = (tickets_hidden > 0, ticket_dir) {
        actions.push(InitAction::SafetySkip {
            path: dir,
            reason: format!(
                "preserved: {tickets_hidden} more tickets record a retired phase; \
                 `lisa doctor` lists them"
            ),
        });
    }

    actions
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
pub fn plan_init_actions(root: &Path) -> Vec<InitAction> {
    let mut actions = Vec::new();

    // Everything Lisa used to write here and no longer does, detected once.
    // Init consumes this; it never re-derives staleness of its own, which is
    // what keeps `lisa doctor`'s diagnosis and init's fix the same answer.
    let retired = currency::retirements(root);

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

    // A project's agent context file — CLAUDE.md, AGENTS.md, or whatever the
    // next client reads — is where the project states its standing intentions
    // to every model that will ever read it. Lisa does not write one, does not
    // report on one, and does not name those paths at all. The only file init
    // creates in the repository root is .lisa.toml.

    // docs/knowledge/lisa-workflow.md
    const WORKFLOW_DOCUMENT: &str = "docs/knowledge/lisa-workflow.md";
    actions.push(plan_owned_template(
        root.join(WORKFLOW_DOCUMENT),
        templates::LISA_WORKFLOW.as_str(),
        templates::LEGACY_WORKFLOWS,
    ));

    // .lisa.toml
    let config_path = root.join(".lisa.toml");
    // Removing a dead key is authorized by the retirement, never by init's own
    // reading of the file. That gate is what makes "a key disappeared with no
    // line in the preview naming it" impossible rather than merely unlikely.
    let config_key_authorized = retired.iter().any(|retirement| {
        matches!(&retirement.disposition,
            currency::Disposition::DropConfigKey { path, .. } if path == &config_path)
    });
    let mut config_key_dropped = false;
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
                let updated = match config_key_authorized
                    .then(|| config::remove_retired_scheduling_key(&updated))
                {
                    Some(config::RetiredKeyRemoval::Removed(without_key)) => {
                        config_key_dropped = true;
                        without_key
                    }
                    _ => updated,
                };

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
            "on-start.sh",
            templates::ON_START_HOOK,
            templates::LEGACY_ON_START_HOOKS,
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

    // Retirements last, so the preview closes on what is about to be destroyed
    // rather than burying it under twenty no-ops. `--dry-run` is the load-bearing
    // flag now: it is what an operator reads before letting init touch a
    // repository they have work in.
    actions.extend(plan_retirements(&retired, config_key_dropped));

    actions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileMutationKind {
    Created,
    Updated,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileMutation {
    kind: FileMutationKind,
    path: PathBuf,
}

fn repository_state(root: &Path) -> Result<RepositoryState, String> {
    let repository = ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-toplevel"])
        .env("LC_ALL", "C")
        .output();
    let repository = match repository {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(RepositoryState::Unavailable {
                reason: "Git is not available on this machine".to_string(),
            });
        }
        Err(error) => {
            return Ok(RepositoryState::Unavailable {
                reason: format!("Could not inspect project history: {error}"),
            });
        }
    };
    if !repository.status.success() {
        let stderr = String::from_utf8_lossy(&repository.stderr);
        if stderr.to_ascii_lowercase().contains("not a git repository") {
            return Ok(RepositoryState::Missing);
        }
        return Ok(RepositoryState::Unavailable {
            reason: history_command_failure("inspect existing project history", repository),
        });
    }

    let raw_root = String::from_utf8_lossy(&repository.stdout)
        .trim()
        .to_string();
    if raw_root.is_empty() {
        return Ok(RepositoryState::Unavailable {
            reason: "Could not inspect project history: command returned no result".to_string(),
        });
    }
    let repository_root = PathBuf::from(raw_root);
    let head = ProcessCommand::new("git")
        .arg("-C")
        .arg(&repository_root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output();
    let head = match head {
        Ok(output) => output,
        Err(error) => {
            return Ok(RepositoryState::Unavailable {
                reason: format!("Could not inspect project history: {error}"),
            });
        }
    };
    if head.status.success() {
        return Ok(RepositoryState::Born);
    }

    let symbolic_head = ProcessCommand::new("git")
        .arg("-C")
        .arg(&repository_root)
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .output();
    let symbolic_head = match symbolic_head {
        Ok(output) => output,
        Err(error) => {
            return Ok(RepositoryState::Unavailable {
                reason: format!("Could not inspect project history: {error}"),
            });
        }
    };
    if symbolic_head.status.success() && !symbolic_head.stdout.is_empty() {
        Ok(RepositoryState::Unborn {
            root: repository_root,
        })
    } else {
        Ok(RepositoryState::Unavailable {
            reason: history_command_failure("inspect the existing project-history branch", head),
        })
    }
}

fn history_command_failure(action: &str, output: std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("Could not {action}: command exited with {}", output.status)
    } else {
        format!("Could not {action}: {stderr}")
    }
}

fn run_history_command(command: &mut ProcessCommand, action: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("Could not {action}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(history_command_failure(action, output))
    }
}

fn history_command_stdout(command: &mut ProcessCommand, action: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("Could not {action}: {error}"))?;
    if !output.status.success() {
        return Err(history_command_failure(action, output));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Err(format!("Could not {action}: command returned no result"))
    } else {
        Ok(value)
    }
}

fn set_history_identity(command: &mut ProcessCommand) {
    command
        .env("GIT_AUTHOR_NAME", HISTORY_NAME)
        .env("GIT_AUTHOR_EMAIL", HISTORY_EMAIL)
        .env("GIT_COMMITTER_NAME", HISTORY_NAME)
        .env("GIT_COMMITTER_EMAIL", HISTORY_EMAIL);
}

fn create_initial_history_commit(repository_root: &Path) -> Result<(), String> {
    let mut empty_tree = ProcessCommand::new("git");
    empty_tree.arg("-C").arg(repository_root).args(["mktree"]);
    let empty_tree = history_command_stdout(&mut empty_tree, "prepare empty project history")?;

    let mut commit = ProcessCommand::new("git");
    commit.arg("-C").arg(repository_root).args([
        "commit-tree",
        &empty_tree,
        "-m",
        HISTORY_COMMIT_MESSAGE,
    ]);
    set_history_identity(&mut commit);
    let commit = history_command_stdout(&mut commit, "start project history")?;

    let mut advance_head = ProcessCommand::new("git");
    let missing_head = "0".repeat(commit.len());
    advance_head.arg("-C").arg(repository_root).args([
        "update-ref",
        "HEAD",
        &commit,
        &missing_head,
    ]);
    run_history_command(&mut advance_head, "make project history ready")
}

fn configure_project_history_identity(root: &Path) -> Result<(), String> {
    let mut name = ProcessCommand::new("git");
    name.arg("-C")
        .arg(root)
        .args(["config", "--local", "user.name", HISTORY_NAME]);
    run_history_command(&mut name, "set the project-history name")?;

    let mut email = ProcessCommand::new("git");
    email
        .arg("-C")
        .arg(root)
        .args(["config", "--local", "user.email", HISTORY_EMAIL]);
    run_history_command(&mut email, "set the project-history email")
}

fn initialize_project_history(root: &Path) -> Result<(), String> {
    let mut initialize = ProcessCommand::new("git");
    initialize.args(["init", "--quiet"]).arg(root);
    run_history_command(&mut initialize, "prepare project history")?;

    configure_project_history_identity(root)?;
    create_initial_history_commit(root)
}

fn prompt_for_history(input: &mut impl BufRead, out: &mut impl Write) -> Result<bool, String> {
    loop {
        write!(out, "{HISTORY_OFFER}")
            .map_err(|error| format!("Failed to write the project-history offer: {error}"))?;
        out.flush()
            .map_err(|error| format!("Failed to show the project-history offer: {error}"))?;

        let mut answer = String::new();
        let bytes = input
            .read_line(&mut answer)
            .map_err(|error| format!("Failed to read the project-history choice: {error}"))?;
        if bytes == 0 {
            return Err("No project-history choice was received".to_string());
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                writeln!(out, "Please answer yes or no.")
                    .map_err(|error| format!("Failed to write init output: {error}"))?;
            }
        }
    }
}

fn resolve_history_action(
    state: RepositoryState,
    preference: HistoryPreference,
    dry_run: bool,
    interactive: bool,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<HistoryAction, String> {
    if state == RepositoryState::Born {
        return Ok(HistoryAction::None);
    }

    let accepted = match preference {
        HistoryPreference::WithHistory => true,
        HistoryPreference::NoHistory => false,
        HistoryPreference::Ask if interactive && !dry_run => prompt_for_history(input, out)?,
        HistoryPreference::Ask => !matches!(&state, RepositoryState::Unavailable { .. }),
    };

    if !accepted {
        return Ok(HistoryAction::Decline);
    }

    Ok(match state {
        RepositoryState::Unavailable { reason } => {
            if preference == HistoryPreference::WithHistory {
                return Err(format!(
                    "Project history was requested, but Lisa cannot keep it: {reason}. Install or repair Git, then rerun `lisa init --with-history`; or run `lisa init --no-history` to use Lisa's journal."
                ));
            }
            HistoryAction::Decline
        }
        RepositoryState::Missing => HistoryAction::CreateRepository,
        RepositoryState::Unborn { root } => HistoryAction::CreateInitialCommit { root },
        RepositoryState::Born => HistoryAction::None,
    })
}

fn write_init_line(out: &mut impl Write, args: fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}").map_err(|e| format!("Failed to write init output: {e}"))
}

/// Execute the init command, writing user-facing output to stdout.
pub fn run_init(
    root: &Path,
    dry_run: bool,
    history_preference: HistoryPreference,
) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let interactive = stdin.is_terminal();
    let mut input = stdin.lock();
    let mut out = stdout.lock();
    run_init_with_io(
        root,
        dry_run,
        history_preference,
        interactive,
        &mut input,
        &mut out,
    )
}

/// Internal init entry point with injectable output for end-to-end reporting
/// tests. Planning remains complete before any filesystem mutation.
#[cfg(test)]
fn run_init_with_writer(root: &Path, dry_run: bool, out: &mut impl Write) -> Result<(), String> {
    let mut input = io::Cursor::new(Vec::<u8>::new());
    run_init_with_io(
        root,
        dry_run,
        HistoryPreference::NoHistory,
        false,
        &mut input,
        out,
    )
}

fn run_init_with_io(
    root: &Path,
    dry_run: bool,
    history_preference: HistoryPreference,
    interactive: bool,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let state = repository_state(root)?;
    run_init_with_history_state(
        root,
        dry_run,
        history_preference,
        interactive,
        input,
        out,
        state,
    )
}

fn run_init_with_history_state(
    root: &Path,
    dry_run: bool,
    history_preference: HistoryPreference,
    interactive: bool,
    input: &mut impl BufRead,
    out: &mut impl Write,
    state: RepositoryState,
) -> Result<(), String> {
    let history_action =
        resolve_history_action(state, history_preference, dry_run, interactive, input, out)?;

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
    let actions = plan_init_actions(root);

    // Step 3: Print the plan
    write_init_line(out, format_args!("Planned actions:"))?;
    for action in &actions {
        write_init_line(out, format_args!("{action}"))?;
    }
    write_init_line(out, format_args!(""))?;

    // Step 4: Dry run stops here
    if dry_run {
        match history_action {
            HistoryAction::CreateRepository | HistoryAction::CreateInitialCommit { .. } => {
                write_init_line(out, format_args!("Project history would be kept."))?;
            }
            HistoryAction::Decline => {
                write_init_line(out, format_args!("{HISTORY_DECLINED}"))?;
            }
            HistoryAction::None => {}
        }
        write_init_line(out, format_args!("Dry run complete. No changes made."))?;
        return Ok(());
    }

    // Step 5: Establish the requested history boundary before writing scaffold
    // files. The initial commit always has an explicitly empty tree.
    match history_action {
        HistoryAction::CreateRepository => {
            initialize_project_history(root)?;
            write_init_line(out, format_args!("{HISTORY_KEPT}"))?;
            write_init_line(out, format_args!(""))?;
        }
        HistoryAction::CreateInitialCommit { root } => {
            configure_project_history_identity(&root)?;
            create_initial_history_commit(&root)?;
            write_init_line(out, format_args!("{HISTORY_KEPT}"))?;
            write_init_line(out, format_args!(""))?;
        }
        HistoryAction::Decline => {
            write_init_line(out, format_args!("{HISTORY_DECLINED}"))?;
            write_init_line(out, format_args!(""))?;
        }
        HistoryAction::None => {}
    }

    // Step 6: Execute the scaffold plan.
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
            InitAction::RemoveFile { path, .. } => {
                fs::remove_file(path)
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
                mutations.push(FileMutation {
                    kind: FileMutationKind::Removed,
                    path: path.clone(),
                });
            }
            // The retired key's bytes ride in the `.lisa.toml` UpdateFile above.
            // One file, one write: this action exists to be read, not to run.
            InitAction::NoOp { .. }
            | InitAction::SafetySkip { .. }
            | InitAction::RetireConfigKey { .. } => {}
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
            "on-start.sh",
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
                FileMutationKind::Removed => "removed",
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

    // 2. .lisa.toml exists — the file `lisa init` actually creates, and the
    //    honest answer to "has this project been initialised?". The project's
    //    own agent context file is the operator's to write, so its absence is
    //    not Lisa's to report.
    if !root.join(".lisa.toml").exists() {
        diagnostics.push(ValidationDiagnostic {
            path: ".lisa.toml".to_string(),
            category: "structure",
            message: "not found. Run `lisa init` to create it.".to_string(),
            severity: Severity::Error,
        });
    }

    // 3. docs/knowledge/lisa-workflow.md exists (error, not warning)
    if !root.join("docs/knowledge/lisa-workflow.md").exists() {
        diagnostics.push(ValidationDiagnostic {
            path: "docs/knowledge/lisa-workflow.md".to_string(),
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
                    ("on-start.sh", "SessionStart[startup]"),
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
        "on-start.sh",
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

    // 9. A clean empty board is valid, but has no schedulable work yet.
    if scan.tickets.is_empty() {
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
    if result.ticket_count == 0 {
        println!(
            "No tickets yet. A ticket is a Markdown file that tells Lisa what work to schedule; put one in {}/, then run `lisa validate` again.",
            resolved.ticket_dir.trim_end_matches('/')
        );
        return Ok(());
    }
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

/// Run validation and print one JSON document. Returns the exit code.
///
/// The exit code is exactly what the human path returns: `0` when validation
/// passed, `1` when it found errors. That code is what a caller already treats
/// as authoritative for "could a run start here", so the body is an addition to
/// it and never a replacement. Finding problems is an *answer*, not a failure
/// to answer — the problems ride in the body where a consumer can read them.
pub fn run_validate_json(root: &Path, check_tools: bool) -> i32 {
    let result = validate(root, check_tools);
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let exit_code = i32::from(result.has_errors());
    crate::json_output::emit(
        "validate",
        crate::json_output::Outcome::verdict(validate_payload(&result, &resolved), exit_code),
    )
}

fn validate_payload(
    result: &ValidationResult,
    resolved: &config::ResolvedConfig,
) -> serde_json::Value {
    let problems: Vec<crate::json_output::ProblemView> = result
        .diagnostics
        .iter()
        .map(|diagnostic| crate::json_output::ProblemView {
            path: diagnostic.path.clone(),
            category: diagnostic.category.to_string(),
            severity: match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            message: diagnostic.message.clone(),
        })
        .collect();
    let warning_count = problems.len() - result.error_count();

    let mut phase_timeouts: std::collections::BTreeMap<String, u64> =
        std::collections::BTreeMap::new();
    phase_timeouts.extend(
        resolved
            .phase_timeouts
            .iter()
            .map(|(phase, seconds)| (phase.clone(), *seconds)),
    );

    serde_json::json!({
        "verdict": if result.has_errors() { "failed" } else { "passed" },
        "ticket_count": result.ticket_count,
        "ready_count": result.ready_count,
        "error_count": result.error_count(),
        "warning_count": warning_count,
        "problems": problems,
        "config": crate::json_output::ConfigView {
            max_threads: resolved.max_threads,
            session_timeout_secs: resolved.session_timeout_secs,
            phase_timeouts,
        },
    })
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
        if result.ticket_count > 0 {
            println!(
                "All checks passed. {} tickets, {} ready, DAG valid. Run `lisa loop` to start.",
                result.ticket_count, result.ready_count
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn project_history_copy_names_benefits_without_mechanism_jargon() {
        let offer = HISTORY_OFFER.to_ascii_lowercase();
        assert!(offer.contains("undone"));
        assert!(offer.contains("record of what the agents did"));
        assert!(!offer.contains("git"));
        assert!(!HISTORY_DECLINED.to_ascii_lowercase().contains("git"));
        assert!(HISTORY_DECLINED
            .contains("finished work will be recorded in Lisa's journal but won't be undoable"));
        assert_eq!(
            HISTORY_KEPT,
            "Keeping project history — finished work will be undoable."
        );
    }

    #[test]
    fn project_history_prompt_accepts_defaults_and_retries_invalid_answers() {
        for answer in ["\n", "y\n", "YES\n"] {
            let mut input = io::Cursor::new(answer.as_bytes());
            let mut output = Vec::new();
            assert!(prompt_for_history(&mut input, &mut output).unwrap());
            assert_eq!(String::from_utf8(output).unwrap(), HISTORY_OFFER);
        }

        let mut input = io::Cursor::new(b"later\nno\n");
        let mut output = Vec::new();
        assert!(!prompt_for_history(&mut input, &mut output).unwrap());
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches(HISTORY_OFFER).count(), 2);
        assert!(output.contains("Please answer yes or no."));
    }

    #[test]
    fn project_history_prompt_rejects_end_of_input() {
        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        let error = prompt_for_history(&mut input, &mut output).unwrap_err();
        assert_eq!(error, "No project-history choice was received");
    }

    #[test]
    fn noninteractive_init_keeps_history_by_default_when_available() {
        let dir = tempfile::tempdir().unwrap();
        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            dir.path(),
            false,
            HistoryPreference::Ask,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();
        assert!(dir.path().join(".git").exists());
        assert!(dir.path().join(".lisa.toml").exists());
        assert!(String::from_utf8(output).unwrap().contains(HISTORY_KEPT));
    }

    #[test]
    fn unavailable_history_falls_back_unless_explicitly_required() {
        let unavailable = || RepositoryState::Unavailable {
            reason: "Git is not available on this machine".to_string(),
        };

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        assert_eq!(
            resolve_history_action(
                unavailable(),
                HistoryPreference::Ask,
                false,
                false,
                &mut input,
                &mut output,
            )
            .unwrap(),
            HistoryAction::Decline
        );
        assert!(output.is_empty());

        let mut input = io::Cursor::new(b"yes\n");
        let mut output = Vec::new();
        assert_eq!(
            resolve_history_action(
                unavailable(),
                HistoryPreference::Ask,
                false,
                true,
                &mut input,
                &mut output,
            )
            .unwrap(),
            HistoryAction::Decline
        );
        assert_eq!(String::from_utf8(output).unwrap(), HISTORY_OFFER);

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        let error = resolve_history_action(
            unavailable(),
            HistoryPreference::WithHistory,
            false,
            false,
            &mut input,
            &mut output,
        )
        .unwrap_err();
        assert!(error.contains("Git is not available"));
        assert!(error.contains("Install or repair Git"));
        assert!(error.contains("--with-history"));
        assert!(error.contains("--no-history"));

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        assert_eq!(
            resolve_history_action(
                unavailable(),
                HistoryPreference::NoHistory,
                false,
                false,
                &mut input,
                &mut output,
            )
            .unwrap(),
            HistoryAction::Decline
        );
    }

    #[test]
    fn interactive_accept_without_git_completes_with_journal_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let mut input = io::Cursor::new(b"\n");
        let mut output = Vec::new();
        run_init_with_history_state(
            dir.path(),
            false,
            HistoryPreference::Ask,
            true,
            &mut input,
            &mut output,
            RepositoryState::Unavailable {
                reason: "Git is not available on this machine".to_string(),
            },
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with(HISTORY_OFFER));
        assert!(output.contains(HISTORY_DECLINED));
        assert!(output.contains("Initialization complete."));
        assert!(!dir.path().join(".git").exists());
        assert!(dir.path().join(".lisa.toml").exists());
    }

    #[test]
    fn dry_run_describes_history_choice_without_mutating_or_prompting() {
        let dir = tempfile::tempdir().unwrap();
        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            dir.path(),
            true,
            HistoryPreference::WithHistory,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Project history would be kept."));
        assert!(output.contains("Dry run complete. No changes made."));
        assert!(!output.contains(HISTORY_OFFER));
        assert!(!dir.path().join(".git").exists());

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            dir.path(),
            true,
            HistoryPreference::Ask,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Project history would be kept."));
        assert!(!output.contains(HISTORY_OFFER));
        assert!(!output.contains("choose --with-history or --no-history"));
        assert!(!dir.path().join(".git").exists());
    }

    #[test]
    fn test_plan_init_actions_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let actions = plan_init_actions(dir.path());

        // Should plan to create:
        //   8 directories (6 docs + .lisa/hooks + .lisa/signals)
        //   12 files (the workflow document, .lisa.toml, seven shared hook files,
        //   .lisa/.gitignore, Claude settings, and Codex hooks.json)
        let creates: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::CreateDir(_) | InitAction::CreateFile { .. }))
            .collect();
        assert_eq!(creates.len(), 20);
    }

    /// `.lisa.toml` is the only file init writes to the repository root.
    #[test]
    fn test_plan_init_writes_nothing_else_to_the_repository_root() {
        let dir = tempfile::tempdir().unwrap();
        let actions = plan_init_actions(dir.path());

        let root_files: Vec<_> = actions
            .iter()
            .filter_map(|a| match a {
                InitAction::CreateFile { path, .. } | InitAction::UpdateFile { path, .. } => {
                    Some(path)
                }
                _ => None,
            })
            .filter(|path| path.parent() == Some(dir.path()))
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert_eq!(root_files, vec![".lisa.toml".to_string()]);
    }

    #[test]
    fn test_plan_init_creates_on_notify_sample() {
        let dir = tempfile::tempdir().unwrap();
        let actions = plan_init_actions(dir.path());

        let created: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::CreateFile { path, .. } if path.ends_with("on-notify.sample")))
            .collect();
        assert_eq!(created.len(), 1, "on-notify.sample should be scaffolded");
    }

    /// A hand-written context file is not Lisa's to touch, and not Lisa's to
    /// comment on either — init plans no action of any kind against it.
    #[test]
    fn test_plan_init_ignores_existing_context_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "existing").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "existing").unwrap();

        let actions = plan_init_actions(dir.path());

        for name in ["CLAUDE.md", "AGENTS.md"] {
            let mentions: Vec<_> = actions
                .iter()
                .filter(|a| a.to_string().contains(name))
                .collect();
            assert!(
                mentions.is_empty(),
                "init planned an action naming {name}: {mentions:?}"
            );
        }
    }

    #[test]
    fn test_plan_init_actions_existing_lisa_toml_no_version() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 4\n",
        )
        .unwrap();

        let actions = plan_init_actions(dir.path());

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

        let result = run_init(dir.path(), true, HistoryPreference::NoHistory);
        assert!(result.is_ok());

        // Dry run should not create any files
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

        let result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(result.is_ok());

        // Should create all directories and files
        assert!(dir.path().join("docs/knowledge/lisa-workflow.md").exists());
        assert!(dir.path().join(".lisa.toml").exists());
        assert!(dir.path().join("docs/active/tickets").exists());
        assert!(dir.path().join("docs/active/stories").exists());
        assert!(dir.path().join("docs/active/work").exists());
        assert!(dir.path().join("docs/archive/tickets").exists());
        assert!(dir.path().join("docs/archive/stories").exists());
        assert!(dir.path().join("docs/archive/work").exists());

        // The project's agent context file is the operator's to write. Init
        // creates neither one, in a fresh project or any other.
        assert!(!dir.path().join("CLAUDE.md").exists());
        assert!(!dir.path().join("AGENTS.md").exists());

        // Check .lisa.toml content
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(lisa_toml.contains("max_threads"));
        assert!(lisa_toml.contains("docs/active/tickets"));

        // Check hook infrastructure
        assert!(dir.path().join(".lisa/hooks/on-idle.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-stop.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-clear.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-start.sh").exists());
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
            ("on-start.sh", ".started"),
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
        assert!(codex_hooks.contains("on-start.sh"));
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

        let result = run_init(dir.path(), false, HistoryPreference::NoHistory);
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

        // A user-authored AGENTS.md must be preserved.
        fs::write(dir.path().join("AGENTS.md"), "my custom agents content").unwrap();

        let result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(result.is_ok());

        let agents_md = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert_eq!(agents_md, "my custom agents content");
    }

    /// The common upgrade path: a project that already has a hand-written
    /// context file. Init leaves it byte-identical and never mentions it —
    /// not as a creation, not as a skip.
    #[test]
    fn test_run_init_reports_no_action_for_hand_written_context_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test-project\"\n",
        )
        .unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "hand written").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "hand written too").unwrap();

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            dir.path(),
            false,
            HistoryPreference::NoHistory,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();

        let reported = String::from_utf8(output).unwrap();
        assert!(
            !reported.contains("CLAUDE.md"),
            "init reported on CLAUDE.md:\n{reported}"
        );
        assert!(
            !reported.contains("AGENTS.md"),
            "init reported on AGENTS.md:\n{reported}"
        );

        assert_eq!(
            fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap(),
            "hand written"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("AGENTS.md")).unwrap(),
            "hand written too"
        );
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

        let result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(result.is_ok());

        // .lisa.toml should now have version, but preserve original content
        let lisa_toml = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(lisa_toml.contains(&format!("version = \"{}\"", config::LISA_VERSION)));
        assert!(lisa_toml.contains("max_threads = 4"));
    }

    #[test]
    fn test_validate_missing_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_validate(dir.path(), false);
        assert!(result.is_err());

        let diagnostics = validate(dir.path(), false).diagnostics;
        assert!(
            diagnostics
                .iter()
                .any(|d| d.path == ".lisa.toml" && matches!(d.severity, Severity::Error)),
            "expected a .lisa.toml error, got: {diagnostics:?}"
        );
        assert!(
            !diagnostics.iter().any(|d| d.path == "CLAUDE.md"),
            "validate must not report on the operator's context file"
        );
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
            ("on-start.sh", templates::ON_START_HOOK),
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
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
        )
        .unwrap();
        write_hook_infrastructure(dir.path());
        write_ready_ticket(dir.path());

        let result = run_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_accepts_both_context_files() {
        // Hand-written context files validate clean: neither is required, and
        // neither is rejected. Validate has no opinion about them at all.
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# hand written\n").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "# hand written\n").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
        )
        .unwrap();
        fs::write(dir.path().join(".lisa.toml"), "not valid toml {{{").unwrap();

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_with_tickets() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
    fn test_validate_missing_workflow_document() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        write_ready_ticket(dir.path());
        // No docs/knowledge/lisa-workflow.md

        let result = run_validate(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error"));
    }

    #[test]
    fn test_validate_empty_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        let result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(result.is_ok());

        // The unknown hook must remain byte-for-byte unchanged.
        let hook = fs::read_to_string(dir.path().join(".lisa/hooks/on-idle.sh")).unwrap();
        assert_eq!(hook, "old hook content");
        // New hook scripts should be created
        assert!(dir.path().join(".lisa/hooks/on-stop.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-clear.sh").exists());
        assert!(dir.path().join(".lisa/hooks/on-start.sh").exists());
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

        let actions = plan_init_actions(dir.path());

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

        let actions = plan_init_actions(dir.path());

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

        let actions = plan_init_actions(dir.path());

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

        let actions = plan_init_actions(dir.path());

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

        // Verify the updated content has guarded, project-addressed commands:
        // the guard keeps an absent script silent, and the script is reached
        // through the leased project rather than the agent's working directory.
        if let InitAction::UpdateFile { content, .. } = &update_settings[0] {
            let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
            // Find each upgraded command by the script it names: merge appends
            // new matchers, so positions are not stable across events.
            let command_for = |script: &str| -> String {
                parsed["hooks"]
                    .as_object()
                    .unwrap()
                    .values()
                    .flat_map(|entries| entries.as_array().unwrap())
                    .flat_map(|entry| entry["hooks"].as_array().unwrap())
                    .filter_map(|hook| hook["command"].as_str())
                    .find(|command| command.contains(script))
                    .unwrap_or_else(|| panic!("{script} binding is present"))
                    .to_string()
            };
            for script in ["on-stop.sh", "on-clear.sh", "on-idle.sh"] {
                let command = command_for(script);
                let command = command.as_str();
                assert!(
                    command.contains("test -x"),
                    "{script} should stay guarded: {command}"
                );
                assert!(
                    command.contains("${LISA_PROJECT:-.}/.lisa/hooks/"),
                    "{script} should be reached through the leased project, not the \
                     directory the agent is standing in: {command}"
                );
                assert_eq!(
                    command.matches(script).count(),
                    1,
                    "{script} is named once in its command: {command}"
                );
            }
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

        let actions = plan_init_actions(dir.path());

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

        let actions = plan_init_actions(dir.path());

        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with(".lisa.toml")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_plan_init_upserts_missing_config_keys() {
        let dir = tempfile::tempdir().unwrap();
        let existing = format!(
            "# Keep this project note exactly here.\nversion = \"{}\"\n\n\
[dirs]\ntickets = \"custom/tickets\"\nstories = \"custom/stories\"\nwork = \"custom/work\"\n\n\
[scheduling]\n# Keep this timeout.\nmax_threads = 4\nsession_timeout_secs = 900",
            config::LISA_VERSION
        );
        fs::write(dir.path().join(".lisa.toml"), &existing).unwrap();

        let actions = plan_init_actions(dir.path());

        let updated: Vec<_> = actions
            .iter()
            .filter(
                |a| matches!(a, InitAction::UpdateFile { path, .. } if path.ends_with(".lisa.toml")),
            )
            .collect();
        assert_eq!(updated.len(), 1, "should update to add missing keys");

        if let InitAction::UpdateFile { content, .. } = &updated[0] {
            assert!(
                content.starts_with(&existing),
                "every legacy byte must remain an exact prefix"
            );
            assert_eq!(content.matches("session_timeout_secs").count(), 1);
            assert_eq!(
                content
                    .matches("# Keep this project note exactly here.")
                    .count(),
                1
            );
            assert_eq!(content.matches("# Keep this timeout.").count(), 1);

            for section in ["agent", "guards", "triage"] {
                assert_eq!(
                    content.matches(&format!("# [{section}]")).count(),
                    1,
                    "missing section {section} must be appended once"
                );
                for entry in config::CONFIG_KEYS
                    .iter()
                    .filter(|entry| entry.section == section)
                {
                    assert!(
                        content.contains(&entry.commented_stub()),
                        "missing appended stub for {}",
                        entry.path
                    );
                }
            }

            for entry in config::CONFIG_KEYS
                .iter()
                .filter(|entry| entry.section == "scheduling")
            {
                assert!(
                    content.contains(&format!("{} = ", entry.key)),
                    "missing scheduling setting {}",
                    entry.path
                );
            }

            let before: config::LisaConfig = toml::from_str(&existing).unwrap();
            let after: config::LisaConfig = toml::from_str(content).unwrap();
            assert_eq!(
                after, before,
                "commented stubs must not change parsed configuration"
            );
            assert_eq!(
                upsert_missing_config_keys(content),
                *content,
                "a second upsert must be byte-identical"
            );
        }
    }

    #[test]
    fn test_upsert_preserves_custom_values_comments_and_current_order() {
        let customized = config::default_config_toml()
            .replace(
                "[agent]\n",
                "[agent]\n# Keep Codex selected for this project.\n",
            )
            .replace("# client = \"claude\"", "client = \"codex\"")
            .replace("# completion = \"auto\"", "completion = \"journal\"")
            .replace("# enabled = true", "enabled = false")
            .replace("# timeout_secs = 120", "timeout_secs = 45")
            .replace("max_threads = 2", "max_threads = 7");

        let result = upsert_missing_config_keys(&customized);
        assert_eq!(
            result, customized,
            "a customized current file must remain byte-identical"
        );
        assert_eq!(result.matches("client = \"codex\"").count(), 1);
        assert_eq!(result.matches("completion = \"journal\"").count(), 1);
        assert_eq!(
            result
                .matches("# Keep Codex selected for this project.")
                .count(),
            1
        );
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
    fn test_plan_init_preserves_unknown_workflow() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Old workflow content",
        )
        .unwrap();

        let actions = plan_init_actions(dir.path());

        let preserved: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::SafetySkip { path, reason } if path.ends_with("lisa-workflow.md") && reason == "preserved: content is not a known Lisa template"))
            .collect();
        assert_eq!(preserved.len(), 1);
    }

    #[test]
    fn test_plan_init_skips_current_workflow() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            templates::LISA_WORKFLOW.as_str(),
        )
        .unwrap();

        let actions = plan_init_actions(dir.path());

        let skipped: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, InitAction::NoOp { path, .. } if path.ends_with("lisa-workflow.md")))
            .collect();
        assert_eq!(skipped.len(), 1);
    }

    /// A project holding a byte-exact 0.4.4 `rdspi-workflow.md` is migrated:
    /// the document appears under its new name and the old file is removed.
    ///
    /// The removal is the whole point of the rename being a migration rather
    /// than an addition. Without it the project keeps two contract documents
    /// that disagree, and the stale one is the one every 0.4-era prompt points
    /// at.
    #[test]
    fn an_unmodified_prior_workflow_is_migrated_to_the_new_name() {
        for generation in templates::LEGACY_WORKFLOWS {
            let dir = tempfile::tempdir().unwrap();
            let retired = dir.path().join("docs/knowledge/rdspi-workflow.md");
            let current = dir.path().join("docs/knowledge/lisa-workflow.md");
            fs::create_dir_all(retired.parent().unwrap()).unwrap();
            fs::write(&retired, generation).unwrap();

            let actions = plan_init_actions(dir.path());
            assert_eq!(
                actions
                    .iter()
                    .filter(|a| matches!(a, InitAction::CreateFile { path, content }
                        if path == &current && content == templates::LISA_WORKFLOW.as_str()))
                    .count(),
                1,
                "the document must be created under its new name"
            );
            assert_eq!(
                actions
                    .iter()
                    .filter(|a| matches!(a, InitAction::RemoveFile { path, reason }
                        if path == &retired && reason == "superseded by docs/knowledge/lisa-workflow.md"))
                    .count(),
                1,
                "an unmodified prior generation must be removed"
            );

            let mut input = io::Cursor::new(Vec::<u8>::new());
            let mut output = Vec::new();
            run_init_with_io(
                dir.path(),
                false,
                HistoryPreference::NoHistory,
                false,
                &mut input,
                &mut output,
            )
            .unwrap();

            assert!(!retired.exists(), "the old file must be gone from disk");
            assert_eq!(
                fs::read_to_string(&current).unwrap(),
                templates::LISA_WORKFLOW.as_str()
            );
            let reported = String::from_utf8(output).unwrap();
            assert!(
                reported.contains("removed  ") && reported.contains("rdspi-workflow.md"),
                "the run must say what it deleted:\n{reported}"
            );

            // Idempotent: a second pass has nothing left to migrate.
            assert!(plan_init_actions(dir.path())
                .iter()
                .all(|a| !matches!(a, InitAction::RemoveFile { .. })));
        }
    }

    /// A project whose `rdspi-workflow.md` has been edited keeps it.
    ///
    /// Deleting it would be the first thing Lisa ever destroyed that a person
    /// wrote. The skip names the rename, so an operator finding two documents
    /// knows which one is live and why theirs survived.
    #[test]
    fn a_modified_workflow_is_left_where_the_operator_put_it() {
        let dir = tempfile::tempdir().unwrap();
        let retired = dir.path().join("docs/knowledge/rdspi-workflow.md");
        let current = dir.path().join("docs/knowledge/lisa-workflow.md");
        fs::create_dir_all(retired.parent().unwrap()).unwrap();
        let edited = format!(
            "{}\n\nOur team's amendment.\n",
            templates::LEGACY_WORKFLOWS[1]
        );
        fs::write(&retired, &edited).unwrap();

        let actions = plan_init_actions(dir.path());
        assert_eq!(
            actions
                .iter()
                .filter(|a| matches!(a, InitAction::CreateFile { path, .. } if path == &current))
                .count(),
            1,
            "the document must still be created under its new name"
        );
        let skips: Vec<_> = actions
            .iter()
            .filter_map(|a| match a {
                InitAction::SafetySkip { path, reason } if path == &retired => Some(reason),
                _ => None,
            })
            .collect();
        assert_eq!(skips.len(), 1, "the edited file must be reported as a skip");
        assert_eq!(
            skips[0],
            "preserved: content is not a known Lisa template; superseded by docs/knowledge/lisa-workflow.md",
            "the skip reason must name the rename"
        );

        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            dir.path(),
            false,
            HistoryPreference::NoHistory,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(&retired).unwrap(),
            edited,
            "the operator's file must survive byte-for-byte"
        );
        assert_eq!(
            fs::read_to_string(&current).unwrap(),
            templates::LISA_WORKFLOW.as_str()
        );
    }

    /// A project that never had the old file hears nothing about it.
    #[test]
    fn a_project_without_the_retired_document_gets_no_line_about_it() {
        let dir = tempfile::tempdir().unwrap();
        assert!(plan_init_actions(dir.path())
            .iter()
            .all(|a| !format!("{a}").contains("rdspi-workflow.md")));
    }

    #[test]
    fn test_plan_init_updates_known_prior_plain_text_templates() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            templates::LEGACY_WORKFLOWS[0],
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

        let actions = plan_init_actions(dir.path());

        for name in &[
            "lisa-workflow.md",
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
    fn test_plan_init_updates_every_known_workflow_template() {
        assert!(
            templates::LEGACY_WORKFLOWS
                .iter()
                .all(|legacy| *legacy != templates::LISA_WORKFLOW.as_str()),
            "legacy workflow fixtures must be byte-distinct from current content"
        );

        for legacy in templates::LEGACY_WORKFLOWS {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
            fs::write(dir.path().join("docs/knowledge/lisa-workflow.md"), legacy).unwrap();

            let actions = plan_init_actions(dir.path());

            assert!(
                actions.iter().any(
                    |action| matches!(action, InitAction::UpdateFile { path, content }
                        if path.ends_with("lisa-workflow.md")
                            && content == templates::LISA_WORKFLOW.as_str())
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
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            templates::LISA_WORKFLOW.as_str(),
        )
        .unwrap();
        for (name, content) in [
            ("on-idle.sh", templates::ON_IDLE_HOOK),
            ("on-stop.sh", templates::ON_STOP_HOOK),
            ("on-clear.sh", templates::ON_CLEAR_HOOK),
            ("on-start.sh", templates::ON_START_HOOK),
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

        let actions = plan_init_actions(dir.path());

        for name in &[
            "lisa-workflow.md",
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-start.sh",
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
        assert_eq!(
            merged,
            "signals/\nattempts/\nclaude/\ncodex/\nrun-events.jsonl\nrun-baseline.json\nscheduler.alive\n"
        );
        assert!(merged.starts_with("signals/"));

        fs::write(&path, &merged).unwrap();
        assert!(matches!(
            plan_append_only_gitignore(path.clone(), templates::LISA_GITIGNORE),
            InitAction::NoOp { reason, .. } if reason == "already up to date"
        ));

        let spaced = "  signals/  \n attempts/ \n\tclaude/\t\ncodex/\n run-events.jsonl \n\trun-baseline.json\t\n scheduler.alive ";
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

        let recreated_path = dir.path().join(".lisa/hooks/on-stop.sh");
        let gitignore_path = dir.path().join(".lisa/.gitignore");
        let workflow_path = dir.path().join("docs/knowledge/lisa-workflow.md");
        let skipped_hook_path = dir.path().join(".lisa/hooks/on-idle.sh");
        fs::remove_file(&recreated_path).unwrap();
        fs::write(&gitignore_path, "signals/\nhooks/ntfy-topic\n").unwrap();
        fs::write(&skipped_hook_path, "#!/bin/sh\n# project-owned\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&skipped_hook_path, fs::Permissions::from_mode(0o640)).unwrap();
        }

        let actions = plan_init_actions(dir.path());
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
        assert!(!recreated_path.exists());
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
            vec![recreated_path.clone(), gitignore_path.clone()]
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
                recreated_path.display(),
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
            "signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\nrun-events.jsonl\nrun-baseline.json\nscheduler.alive\n"
        );
    }

    #[test]
    fn test_init_preserves_vend_customizations_and_secret_ignore_rule() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::create_dir_all(dir.path().join(".lisa/hooks")).unwrap();

        let workflow = format!(
            "{}\n## Story Layer\n\nRead the parent story before every ticket.\n",
            templates::LISA_WORKFLOW.as_str()
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
            ("docs/knowledge/lisa-workflow.md", workflow.as_bytes()),
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

        let actions = plan_init_actions(dir.path());
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
            Some("signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\nrun-events.jsonl\nrun-baseline.json\nscheduler.alive\n")
        );
        assert_eq!(
            fs::read(dir.path().join("docs/knowledge/lisa-workflow.md")).unwrap(),
            workflow.as_bytes()
        );

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();

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
            "signals/\nhooks/ntfy-topic\nattempts/\nclaude/\ncodex/\nrun-events.jsonl\nrun-baseline.json\nscheduler.alive\n"
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
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            [0xff, 0xfe],
        )
        .unwrap();
        fs::write(dir.path().join(".lisa/hooks/on-stop.sh"), [0xff, 0xfe]).unwrap();

        let actions = plan_init_actions(dir.path());

        for name in &["lisa-workflow.md", "on-stop.sh"] {
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

        let actions = plan_init_actions(dir.path());

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

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();

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
            "on-start.sh",
            "on-heartbeat.sh",
            "on-notify.sample",
        ] {
            fs::write(
                dir.path().join(format!(".lisa/hooks/{name}")),
                format!("project-owned {name}\n"),
            )
            .unwrap();
        }

        let actions = plan_init_actions(dir.path());

        for name in &[
            "on-idle.sh",
            "on-stop.sh",
            "on-clear.sh",
            "on-start.sh",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
        let init_result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(init_result.is_ok());

        // Add a ready ticket
        write_ready_ticket(dir.path());

        // Validate should pass
        let validate_result = run_validate(dir.path(), false);
        assert!(validate_result.is_ok());

        // Init wrote no context file for either client to read.
        assert!(!dir.path().join("CLAUDE.md").exists());
        assert!(!dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_init_then_validate_roundtrip_codex_hooks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"codex-project\"\n",
        )
        .unwrap();
        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
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
        assert!(hooks.contains("on-start.sh"));
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
        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
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
        let init_result = run_init(dir.path(), false, HistoryPreference::NoHistory);
        assert!(init_result.is_ok());

        // Add a ready ticket
        write_ready_ticket(dir.path());

        // Validate should pass
        let validate_result = run_validate(dir.path(), false);
        assert!(validate_result.is_ok());

        // Init wrote no context file for either client to read.
        assert!(!dir.path().join("CLAUDE.md").exists());
        assert!(!dir.path().join("AGENTS.md").exists());
    }

    // --- Structured diagnostic tests (call validate() directly) ---

    #[test]
    fn test_diagnostics_clean_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
    fn test_diagnostics_missing_lisa_toml() {
        let dir = tempfile::tempdir().unwrap();

        let result = validate(dir.path(), false);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.path == ".lisa.toml")
            .collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].category, "structure");
    }

    #[test]
    fn test_diagnostics_ticket_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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
                        || d.path.contains("on-clear.sh")
                        || d.path.contains("on-start.sh"))
            })
            .collect();
        // 1 settings.local.json missing + 4 selected hook scripts missing = 5
        assert_eq!(hook_errors.len(), 5);
    }

    #[test]
    fn test_diagnostics_success_counts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join("docs/knowledge/lisa-workflow.md"),
            "# Workflow",
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

    // ---- Retiring what init once wrote (T-057-02-02) ----------------------

    /// The `.lisa.toml` a 0.4.4 project actually has: comments the operator
    /// wrote, a custom value, a section Lisa has never heard of, and the dead
    /// key sitting among them.
    const FIXTURE_CONFIG: &str = r#"# Two agents is all this laptop can take.
version = "0.4.0"

[dirs]
tickets = "docs/active/tickets"   # unchanged, but we said so out loud

[scheduling]
max_threads = 2
# Left over from 0.4 — nobody remembers turning it on.
auto_advance = true
review_timeout_secs = 900

[experimental]
my_own_setting = "keep me"
"#;

    const TICKET_AT_RETIRED_PHASE: &str = "---\nid: T-024-01\nstory: S-024\ntitle: migrate-climate-calls\ntype: task\nstatus: open\npriority: high\nphase: structure\ndepends_on: []\n---\n\n## Context\n\nWork.\n";

    /// A `CLAUDE.md` exactly as a 0.4.4 `generate_claude_md` wrote it.
    fn generated_claude_md() -> String {
        format!(
            "{}my-app (Rust) — TODO: add a one-line project description here.\n\n### Build and Test\n\n```bash\n# Build\ncargo build\n\n# Run tests\ncargo test\n\n# Lint\ncargo clippy\n```\n\n### Source Layout\n\n```\nsrc:\n  main.rs\n```\n\n{}",
            include_str!("../data/legacy/claude-md-header-v0.4.4.md"),
            include_str!("../data/legacy/claude-md-tail.md"),
        )
    }

    /// A project shaped like one 0.4.4 left behind, carrying every subject this
    /// ticket retires at once.
    fn upgrade_fixture(root: &Path) {
        fs::create_dir_all(root.join("docs/knowledge")).unwrap();
        fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
        fs::create_dir_all(root.join(".lisa/hooks")).unwrap();
        fs::write(root.join(".lisa.toml"), FIXTURE_CONFIG).unwrap();
        fs::write(
            root.join("docs/knowledge/rdspi-workflow.md"),
            templates::LEGACY_WORKFLOWS[2],
        )
        .unwrap();
        fs::write(
            root.join(".lisa/hooks/on-stop.sh"),
            templates::LEGACY_ON_STOP_HOOKS[0],
        )
        .unwrap();
        fs::write(root.join("CLAUDE.md"), generated_claude_md()).unwrap();
        fs::write(
            root.join("AGENTS.md"),
            crate::legacy_context::LEGACY_AGENTS_CONTEXTS[1],
        )
        .unwrap();
        fs::write(
            root.join("docs/active/tickets/T-024-01.md"),
            TICKET_AT_RETIRED_PHASE,
        )
        .unwrap();
    }

    /// Every file under `root`, by relative path and exact bytes.
    fn tree_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    out.push((
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        fs::read(&path).unwrap(),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    fn init_output(root: &Path, dry_run: bool) -> String {
        let mut input = io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_init_with_io(
            root,
            dry_run,
            HistoryPreference::NoHistory,
            false,
            &mut input,
            &mut output,
        )
        .unwrap();
        String::from_utf8(output).unwrap()
    }

    fn mutating(plan: &[InitAction]) -> Vec<&InitAction> {
        plan.iter()
            .filter(|action| {
                matches!(
                    action,
                    InitAction::CreateDir(_)
                        | InitAction::CreateFile { .. }
                        | InitAction::UpdateFile { .. }
                        | InitAction::RemoveFile { .. }
                        | InitAction::RetireConfigKey { .. }
                )
            })
            .collect()
    }

    /// `--dry-run` is the load-bearing flag now that init can remove things:
    /// it is what an operator reads before letting an upgrade touch a
    /// repository they have work in. Every retirement has to be in it, named,
    /// with its reason — and nothing on disk may move.
    #[test]
    fn dry_run_names_every_retirement_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());
        let before = tree_snapshot(dir.path());

        let preview = init_output(dir.path(), true);

        for subject in [
            "rdspi-workflow.md",
            "CLAUDE.md",
            "AGENTS.md",
            "[scheduling] auto_advance",
            "T-024-01.md",
        ] {
            assert!(
                preview.contains(subject),
                "the preview must name {subject}:\n{preview}"
            );
        }
        for reason in [
            "superseded by docs/knowledge/lisa-workflow.md",
            "generated by Lisa and unedited since",
            "Lisa stopped reading this setting in 0.5.0",
            "your board is not Lisa's to rewrite",
        ] {
            assert!(
                preview.contains(reason),
                "every retirement must carry its reason; missing {reason:?}:\n{preview}"
            );
        }
        assert!(preview.contains("Dry run complete. No changes made."));

        assert_eq!(
            tree_snapshot(dir.path()),
            before,
            "--dry-run must leave the tree byte-identical"
        );
    }

    /// End to end: a project 0.4.4 set up becomes a current one through a
    /// single `lisa init`, with no hand edits.
    #[test]
    fn one_init_brings_a_0_4_4_project_current() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();

        assert!(!dir.path().join("docs/knowledge/rdspi-workflow.md").exists());
        assert!(!dir.path().join("CLAUDE.md").exists());
        assert!(!dir.path().join("AGENTS.md").exists());
        assert!(dir.path().join("docs/knowledge/lisa-workflow.md").exists());

        let currency = crate::currency::inventory(dir.path());
        let carried: Vec<_> = currency
            .findings
            .iter()
            .filter(|finding| finding.kind != crate::currency::CurrencyKind::StaleContent)
            .collect();
        assert!(
            carried.is_empty(),
            "nothing behind or retired survives one init: {carried:#?}"
        );
        // What is left is the board, which no Lisa command rewrites — the
        // operator's own edit, and the one thing T-057-02-03 will not take
        // either.
        assert!(currency
            .findings
            .iter()
            .all(|finding| matches!(finding.remedy, crate::currency::Remedy::Operator(_))));
        assert_eq!(
            currency.recorded_version,
            crate::currency::RecordedVersion::Current {
                recorded: config::LISA_VERSION.to_string()
            }
        );

        // The board row is the one thing left, and the ticket is explicit that
        // it is not init's to fix: a retired phase still loads as `implement`,
        // and Lisa rewrites the row itself the next time it moves the ticket.
        // Settle it the way its owner would and the project is current outright
        // — one `lisa init`, no hand edits to anything Lisa wrote.
        fs::write(
            dir.path().join("docs/active/tickets/T-024-01.md"),
            TICKET_AT_RETIRED_PHASE.replace("phase: structure", "phase: implement"),
        )
        .unwrap();
        assert!(crate::currency::inventory(dir.path()).is_current());
    }

    /// No retirement fires twice.
    #[test]
    fn a_second_consecutive_run_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());
        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        let after_first = tree_snapshot(dir.path());

        let plan = plan_init_actions(dir.path());
        let still_to_do = mutating(&plan);
        assert!(
            still_to_do.is_empty(),
            "a second run has nothing left to do: {still_to_do:#?}"
        );

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        assert_eq!(tree_snapshot(dir.path()), after_first);
    }

    /// And on a project this binary itself created, the plan is nothing but
    /// no-ops — not a skip, not a report, nothing to read at all.
    #[test]
    fn init_on_an_already_current_project_is_all_no_op() {
        let dir = tempfile::tempdir().unwrap();
        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        for action in plan_init_actions(dir.path()) {
            assert!(matches!(action, InitAction::NoOp { .. }), "{action}");
        }
    }

    /// `docs/active/tickets/` is the operator's board, not a Lisa-owned
    /// template. A retired phase value still loads — it reads as `implement` —
    /// and init rewriting frontmatter in bulk would be a far larger claim on
    /// the repository than anything init does today.
    #[test]
    fn a_ticket_at_a_retired_phase_is_reported_and_never_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());
        let ticket = dir.path().join("docs/active/tickets/T-024-01.md");

        let reported = plan_init_actions(dir.path())
            .into_iter()
            .filter(|action| {
                matches!(action, InitAction::SafetySkip { path, reason }
                    if path == &ticket && reason.contains("phase: structure"))
            })
            .count();
        assert_eq!(reported, 1, "the ticket is reported, once");

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        assert_eq!(
            fs::read_to_string(&ticket).unwrap(),
            TICKET_AT_RETIRED_PHASE,
            "init must not touch a byte of the board"
        );
    }

    /// A board with two hundred retired phases would push the removals off the
    /// top of the preview, which is the part an operator is reading it for.
    #[test]
    fn a_board_full_of_retired_phases_previews_a_few_and_counts_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());
        for index in 2..=9 {
            fs::write(
                dir.path()
                    .join(format!("docs/active/tickets/T-024-{index:02}.md")),
                TICKET_AT_RETIRED_PHASE.replace("T-024-01", &format!("T-024-{index:02}")),
            )
            .unwrap();
        }

        let preview = init_output(dir.path(), true);
        let listed = preview.matches("records `phase: structure`").count();
        assert_eq!(listed, 5, "five listed, then a count:\n{preview}");
        assert!(
            preview.contains("4 more tickets record a retired phase; `lisa doctor` lists them"),
            "the rest are counted, not dropped silently:\n{preview}"
        );

        // The cap is a preview decision. Doctor still gets all nine.
        assert_eq!(
            crate::currency::inventory(dir.path())
                .findings
                .iter()
                .filter(|finding| finding.kind == crate::currency::CurrencyKind::StaleContent)
                .count(),
            9
        );
    }

    /// `.lisa.toml` is a file the operator edits. A rewrite that strips their
    /// comments is a worse outcome than the dead key sitting inert, so the
    /// removal is one line lifted out and nothing else.
    #[test]
    fn the_dead_config_key_goes_and_every_other_byte_stays() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        let updated = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();

        assert!(!updated.contains("auto_advance"), "{updated}");
        for kept in [
            "# Two agents is all this laptop can take.",
            "tickets = \"docs/active/tickets\"   # unchanged, but we said so out loud",
            "max_threads = 2",
            "# Left over from 0.4 — nobody remembers turning it on.",
            "review_timeout_secs = 900",
            "[experimental]",
            "my_own_setting = \"keep me\"",
        ] {
            assert!(updated.contains(kept), "lost {kept:?} from:\n{updated}");
        }

        // Key order, not just key survival: every surviving original line is
        // still in its original position relative to the others.
        let mut cursor = 0;
        for line in FIXTURE_CONFIG
            .lines()
            .filter(|line| !line.contains("auto_advance") && !line.starts_with("version"))
            .filter(|line| !line.trim().is_empty())
        {
            let at = updated[cursor..]
                .find(line)
                .unwrap_or_else(|| panic!("{line:?} moved or vanished from:\n{updated}"));
            cursor += at + line.len();
        }

        // And it is still a config Lisa can load.
        let validation = config::load_config(dir.path()).expect("the rewritten file must load");
        assert_eq!(validation.config.scheduling.max_threads, Some(2));
        assert!(
            validation
                .warnings
                .iter()
                .all(|warning| !warning.contains("auto_advance")),
            "the warning goes with the key: {:?}",
            validation.warnings
        );
    }

    /// When the key cannot be lifted out without reformatting the operator's
    /// file, init does not do it. It says so and leaves the bytes alone.
    #[test]
    fn a_config_that_cannot_be_edited_surgically_is_left_alone_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        upgrade_fixture(dir.path());
        // An inline table: the setting has no line of its own.
        let inline = "version = \"0.4.0\"\nscheduling = { auto_advance = true, max_threads = 2 }\n";
        fs::write(dir.path().join(".lisa.toml"), inline).unwrap();

        let preview = init_output(dir.path(), true);
        assert!(
            preview.contains("preserved: [scheduling] auto_advance is inert since 0.5.0, but"),
            "the refusal must be reported, with its reason:\n{preview}"
        );
        assert!(
            !preview.contains("] auto_advance (Lisa stopped reading"),
            "no removal may be announced:\n{preview}"
        );

        run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
        let after = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        assert!(
            after.contains("scheduling = { auto_advance = true, max_threads = 2 }"),
            "the operator's line survives byte-identical:\n{after}"
        );
    }

    /// The consent rule this ticket turns on, both directions, both files.
    #[test]
    fn a_generated_context_file_goes_and_an_edited_one_stays() {
        for (name, generated, edited) in [
            (
                "CLAUDE.md",
                generated_claude_md(),
                generated_claude_md().replace(
                    "TODO: add a one-line project description here.",
                    "A scheduler for climate model runs.",
                ),
            ),
            (
                "AGENTS.md",
                crate::legacy_context::LEGACY_AGENTS_CONTEXTS[1].to_string(),
                format!(
                    "{}\nAnd read the runbook before touching the scheduler.\n",
                    crate::legacy_context::LEGACY_AGENTS_CONTEXTS[1]
                ),
            ),
        ] {
            // Byte-identical to a generation Lisa shipped: Lisa's litter, and
            // Lisa's to retire.
            let dir = tempfile::tempdir().unwrap();
            upgrade_fixture(dir.path());
            fs::write(dir.path().join(name), &generated).unwrap();
            run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
            assert!(
                !dir.path().join(name).exists(),
                "a proven, unedited {name} is removed"
            );

            // One line different: the operator's file now, kept exactly, and
            // reported so they know Lisa looked and declined.
            let dir = tempfile::tempdir().unwrap();
            upgrade_fixture(dir.path());
            fs::write(dir.path().join(name), &edited).unwrap();
            // Keep the other half of the pair out of it — the pointer rule has
            // its own test.
            let other = if name == "CLAUDE.md" {
                "AGENTS.md"
            } else {
                "CLAUDE.md"
            };
            fs::remove_file(dir.path().join(other)).unwrap();

            let preview = init_output(dir.path(), true);
            assert!(
                preview.contains(&format!(
                    "{} (preserved: edited since Lisa generated it, so it is yours now)",
                    dir.path().join(name).display()
                )),
                "{name} must be reported as preserved, with a reason:\n{preview}"
            );

            run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();
            assert_eq!(
                fs::read_to_string(dir.path().join(name)).unwrap(),
                edited,
                "an edited {name} survives byte-identical"
            );
        }
    }

    /// The mixed case. Both frozen `AGENTS.md` generations end by pointing at
    /// `CLAUDE.md`, so the two files are not symmetric: removing the pointer
    /// is harmless, removing its target while something still points at it is
    /// the dangling reference the ticket forbids.
    #[test]
    fn a_pointer_target_is_retired_only_when_nothing_points_at_it() {
        let generated_claude = generated_claude_md();
        let generated_agents = crate::legacy_context::LEGACY_AGENTS_CONTEXTS[1];
        let hand_written_agents = "# AGENTS.md\n\nRead CLAUDE.md first, then the README.\n";
        let unrelated_agents = "# AGENTS.md\n\nStart with the README.\n";

        // (CLAUDE.md, AGENTS.md) → (does CLAUDE.md survive, does AGENTS.md)
        let cases: [(Option<&str>, Option<&str>, bool, bool); 5] = [
            // The pair goes together.
            (
                Some(&generated_claude),
                Some(generated_agents),
                false,
                false,
            ),
            // Something is left pointing at it, so the target stays.
            (
                Some(&generated_claude),
                Some(hand_written_agents),
                true,
                true,
            ),
            // Nothing points at it: the pointer was the operator's and says
            // nothing about CLAUDE.md.
            (Some(&generated_claude), Some(unrelated_agents), false, true),
            // The pointer alone is Lisa's to remove.
            (
                Some("# CLAUDE.md\n\nOur own house rules.\n"),
                Some(generated_agents),
                true,
                false,
            ),
            (None, Some(generated_agents), false, false),
        ];

        for (claude, agents, claude_survives, agents_survives) in cases {
            let dir = tempfile::tempdir().unwrap();
            upgrade_fixture(dir.path());
            match claude {
                Some(content) => fs::write(dir.path().join("CLAUDE.md"), content).unwrap(),
                None => fs::remove_file(dir.path().join("CLAUDE.md")).unwrap(),
            }
            fs::write(dir.path().join("AGENTS.md"), agents.unwrap()).unwrap();

            run_init(dir.path(), false, HistoryPreference::NoHistory).unwrap();

            let claude_left = dir.path().join("CLAUDE.md").exists();
            let agents_left = dir.path().join("AGENTS.md").exists();
            assert_eq!(claude_left, claude_survives, "CLAUDE.md, given {agents:?}");
            assert_eq!(agents_left, agents_survives, "AGENTS.md, given {claude:?}");

            // The invariant, checked on the tree rather than on the plan: no
            // run leaves an AGENTS.md pointing at a CLAUDE.md that is gone.
            if agents_left {
                let left = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
                assert!(
                    !left.contains("CLAUDE.md") || claude_left,
                    "left a dangling pointer:\n{left}"
                );
            }
        }
    }
}
