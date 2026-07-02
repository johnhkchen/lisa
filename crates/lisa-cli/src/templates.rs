use crate::detect::DetectedProject;

/// The RDSPI workflow document, embedded at compile time
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");

/// The hooks setup guide, embedded at compile time. Printed by `lisa hooks-guide`.
pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");

/// The compiled WASM plugin, embedded at compile time via build.rs
pub const PLUGIN_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"));

/// The on-idle hook script, called by Claude Code's idle_prompt notification.
/// Writes a signal file so the plugin knows which session finished its work.
pub const ON_IDLE_HOOK: &str = r#"#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification.
# Writes a signal file so the plugin knows this session finished its work.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
fi
"#;

/// The on-stop hook script, called by Claude Code's Stop event.
/// Fires when Claude finishes responding (ready for input).
///
/// Beyond writing the `.stopped` signal it forwards the Stop payload (piped on
/// stdin, carrying `transcript_path`) to `lisa capture-usage`, which sums the
/// session's token usage into `.lisa/claude/<ticket>.usage.json` for the
/// provenance ledger (T-027-02). Stop fires per *turn*, not per tool call, so the
/// heartbeat hook stays trivial; the artifact is overwritten each turn with the
/// cumulative total and read by the plugin only write-after. Capture is
/// best-effort — `${LISA_BIN:-lisa}` degrades to a PATH lookup and any failure is
/// swallowed, leaving tokens null (never fabricated).
pub const ON_STOP_HOOK: &str = r#"#!/bin/sh
# Lisa stop signal hook — called by Claude Code when it finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input, and
# captures session token usage for the provenance ledger (T-027-02).

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. Best-effort: never fail the session if lisa is absent.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage 2>/dev/null || true
"#;

/// The on-clear hook script, called by Claude Code's SessionStart[clear] event.
/// Fires after /clear is processed (context cleared).
pub const ON_CLEAR_HOOK: &str = r#"#!/bin/sh
# Lisa clear signal hook — called by Claude Code after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#;

/// The heartbeat hook script, called by Claude Code's PostToolUse event.
/// Fires after every tool call, proving the session is actively working.
/// The plugin uses the absence of recent heartbeats — not stop/idle signals,
/// which fire before agents truly finish — to decide a pane is safe to reuse.
pub const ON_HEARTBEAT_HOOK: &str = r#"#!/bin/sh
# Lisa heartbeat signal hook — called by Claude Code after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
"#;

/// Gitignore content for the .lisa/ directory — ignores ephemeral signal files.
pub const LISA_GITIGNORE: &str = "signals/\n";

/// The `AGENTS.md` pointer file scaffolded by `lisa init`.
///
/// Codex auto-loads `AGENTS.md`; Claude Code auto-loads `CLAUDE.md` (codex-client
/// doc 06 §AGENTS.md). To make the two impossible to drift, `AGENTS.md` carries
/// no project body of its own — it points at `CLAUDE.md` as the single source of
/// truth and repeats only the RDSPI workflow reference. A Codex session, told by
/// its ticket prompt to read `AGENTS.md`, follows the pointer to `CLAUDE.md`.
pub const AGENTS_MD: &str = "# AGENTS.md\n\
\n\
This project's agent context lives in [CLAUDE.md](CLAUDE.md) — the single source \
of truth for every agent client (Claude Code reads `CLAUDE.md`; Codex reads this \
`AGENTS.md`). Read `CLAUDE.md` first.\n\
\n\
The RDSPI workflow definition is in docs/knowledge/rdspi-workflow.md and is \
injected into agent context by lisa automatically.\n";

/// The on-notify hook SAMPLE, scaffolded as `.lisa/hooks/on-notify.sample`.
/// User-owned attention/completion notification hook. It is deliberately a
/// non-executable `.sample` so the `test -x` guards stay inert until the user
/// opts in (`cp on-notify.sample on-notify && chmod +x on-notify`). lisa never
/// names a notification service outside the commented example below.
pub const ON_NOTIFY_HOOK: &str = r#"#!/bin/sh
# Lisa notify hook (SAMPLE) — copy to on-notify and `chmod +x` to enable.
#
# Contract:  on-notify <event> [detail]      ($1 mirrors $LISA_EVENT)
#
# Environment (all events):
#   LISA_EVENT    complete | attention
#   LISA_PROJECT  absolute project root (identifies which loop; you may `cd` to it)
# complete:
#   LISA_TICKETS_DONE   number of tickets completed
#   LISA_DURATION_SECS  loop duration in seconds
# attention:
#   LISA_REASON      question | permission | idle-without-artifact
#   LISA_PANE_ID     the originating pane
#   LISA_TICKET_ID   ticket the agent is working on, when known
#   LISA_QUESTION_HEADER  short label of the question (question reason only)
#
# Payload on STDIN: for the question/permission reasons, the full Claude Code
# hook JSON is piped to this script's stdin, so you can extract anything (e.g.
# every question + its options) with sed/jq:  payload=$(cat)
#
# Example dispatch (uncomment and customise):
# case "$1" in
#   complete)  msg="lisa [$LISA_PROJECT] done: $LISA_TICKETS_DONE tickets in ${LISA_DURATION_SECS}s" ;;
#   attention) msg="lisa [$LISA_PROJECT] ${LISA_TICKET_ID:-?} needs you (${LISA_REASON}): $2" ;;
# esac
# curl -s -d "$msg" ntfy.sh/your-topic-here

exit 0
"#;

/// Command for the catch-all (matcher-less) `Notification` hook that fires the
/// user-owned `on-notify` hook for permission/attention payloads. POSIX `sh`
/// only (no jq, no bashisms). It exits early when the user has not opted in,
/// reads the payload from stdin once, skips `idle_prompt` payloads (already
/// handled by on-idle.sh + the plugin), and otherwise invokes the user hook
/// with LISA_EVENT/LISA_REASON set inline.
const NOTIFY_ATTENTION_COMMAND: &str = "test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=permission LISA_PROJECT=\"$PWD\" .lisa/hooks/on-notify attention \"$in\" ;; esac";

/// Command for the `PreToolUse[AskUserQuestion]` hook. POSIX `sh` only (no jq,
/// no bashisms). It (1) **unconditionally** writes `pane-$LISA_PANE_ID.awaiting`
/// so the plugin can suppress injection while the agent is blocked on a question
/// (consumed in T-020-03; harmless unread file until then), and (2) best-effort
/// extracts the first question text and fires the opt-in `on-notify attention`
/// with `LISA_REASON=question`. Only the notify dispatch is `test -x`-gated — the
/// signal write must work even when the user never enabled `on-notify`. A question
/// containing an escaped `\"` truncates the greedy-free `[^"]*` capture; that
/// degrades to the generic detail, never a hard failure (design Q3).
const NOTIFY_QUESTION_COMMAND: &str = "mkdir -p .lisa/signals; [ -n \"$LISA_PANE_ID\" ] && date -u +%Y-%m-%dT%H:%M:%SZ > \".lisa/signals/pane-$LISA_PANE_ID.awaiting\"; in=$(cat); q=$(printf '%s' \"$in\" | sed -n 's/.*\"question\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); [ -z \"$q\" ] && q=\"agent is asking a question\"; hdr=$(printf '%s' \"$in\" | sed -n 's/.*\"header\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); test -x .lisa/hooks/on-notify && printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=question LISA_PROJECT=\"$PWD\" LISA_QUESTION_HEADER=\"$hdr\" .lisa/hooks/on-notify attention \"$q\"";

/// Generate .claude/settings.local.json with Stop, SessionStart, Notification
/// (idle_prompt + catch-all attention), PostToolUse heartbeat, and
/// PreToolUse[AskUserQuestion] hooks.
/// Hook commands use `test -x` guards so they succeed silently if the scripts
/// haven't been created yet (e.g. settings.local.json exists before `lisa init`).
pub fn settings_local_json() -> String {
    r#"{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "mkdir -p .lisa/signals; [ -n \"$LISA_PANE_ID\" ] && date -u +%Y-%m-%dT%H:%M:%SZ > \".lisa/signals/pane-$LISA_PANE_ID.awaiting\"; in=$(cat); q=$(printf '%s' \"$in\" | sed -n 's/.*\"question\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); [ -z \"$q\" ] && q=\"agent is asking a question\"; hdr=$(printf '%s' \"$in\" | sed -n 's/.*\"header\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); test -x .lisa/hooks/on-notify && printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=question LISA_PROJECT=\"$PWD\" LISA_QUESTION_HEADER=\"$hdr\" .lisa/hooks/on-notify attention \"$q\""
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "clear",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=permission LISA_PROJECT=\"$PWD\" .lisa/hooks/on-notify attention \"$in\" ;; esac"
          }
        ]
      }
    ]
  }
}
"#
    .to_string()
}

/// Ensure a single hook entry exists in the hooks object with the correct command.
/// For hooks with a matcher (SessionStart, Notification), deduplication checks the matcher value.
/// For hooks without a matcher (Stop), deduplication checks the command path.
/// If the hook exists but uses an old bare-path command, it is upgraded in place.
fn ensure_hook(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event_type: &str,
    matcher: Option<&str>,
    command: &str,
) {
    let entries = hooks_obj
        .entry(event_type)
        .or_insert_with(|| serde_json::json!([]));
    let arr = match entries.as_array_mut() {
        Some(a) => a,
        None => return,
    };

    // Extract the script path from the command for dedup matching.
    // Commands may be bare paths (".lisa/hooks/on-stop.sh") or guarded
    // ("test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh").
    // Match on the script filename to handle both forms.
    let script_path = command.rsplit("&& ").next().unwrap_or(command).trim();

    // Find the matching entry index (if any)
    let found_idx = match matcher {
        Some(m) => arr
            .iter()
            .position(|entry| entry.get("matcher").and_then(|v| v.as_str()) == Some(m)),
        None => arr.iter().position(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|hooks| {
                    hooks.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .is_some_and(|c| c.contains(script_path))
                    })
                })
        }),
    };

    match found_idx {
        Some(idx) => {
            // Entry exists — upgrade the command if it uses the old bare-path form
            if let Some(hooks_arr) = arr[idx].get_mut("hooks").and_then(|h| h.as_array_mut()) {
                for hook in hooks_arr.iter_mut() {
                    if let Some(cmd_val) = hook.get_mut("command") {
                        if let Some(existing) = cmd_val.as_str() {
                            if existing.contains(script_path) && existing != command {
                                *cmd_val = serde_json::json!(command);
                            }
                        }
                    }
                }
            }
        }
        None => {
            // Entry doesn't exist — create it
            let mut entry = serde_json::Map::new();
            if let Some(m) = matcher {
                entry.insert("matcher".to_string(), serde_json::json!(m));
            }
            entry.insert(
                "hooks".to_string(),
                serde_json::json!([{
                    "type": "command",
                    "command": command
                }]),
            );
            arr.push(serde_json::Value::Object(entry));
        }
    }
}

/// Merge all Lisa hooks (Stop, SessionStart[clear], Notification[idle_prompt],
/// PostToolUse heartbeat, the catch-all Notification[attention] binding, and the
/// PreToolUse[AskUserQuestion] question binding) into an existing
/// settings.local.json. Returns the updated JSON string, or an error if the JSON
/// is malformed.
pub fn merge_hooks(existing_json: &str) -> Result<String, String> {
    let mut root: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| format!("invalid JSON in settings.local.json: {}", e))?;

    let obj = root
        .as_object_mut()
        .ok_or("settings.local.json root is not an object")?;

    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or("settings.local.json 'hooks' is not an object")?;

    ensure_hook(
        hooks_obj,
        "Stop",
        None,
        "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("clear"),
        "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh",
    );
    ensure_hook(
        hooks_obj,
        "Notification",
        Some("idle_prompt"),
        "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh",
    );
    ensure_hook(
        hooks_obj,
        "PostToolUse",
        None,
        "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh",
    );
    // Catch-all (matcher-less) Notification entry for permission/attention payloads.
    // Distinct from the idle_prompt entry above: ensure_hook dedups a matcher-less
    // entry by its command substring, which references on-notify (not on-idle.sh),
    // so the two coexist and re-runs stay idempotent.
    ensure_hook(hooks_obj, "Notification", None, NOTIFY_ATTENTION_COMMAND);
    // PreToolUse[AskUserQuestion]: fires the on-notify attention path with
    // LISA_REASON=question and writes the pane-<id>.awaiting signal. It carries a
    // matcher, so ensure_hook dedups by matcher value (idempotent, coexists with
    // the matcher-less PostToolUse heartbeat — a different event key entirely).
    ensure_hook(
        hooks_obj,
        "PreToolUse",
        Some("AskUserQuestion"),
        NOTIFY_QUESTION_COMMAND,
    );

    serde_json::to_string_pretty(&root).map_err(|e| format!("failed to serialize JSON: {}", e))
}

/// Generate a project-specific CLAUDE.md
pub fn generate_claude_md(project: &DetectedProject) -> String {
    use crate::detect::ProjectType;

    let type_label = match project.project_type {
        ProjectType::Rust => "Rust",
        ProjectType::Node => "Node.js",
        ProjectType::Go => "Go",
        ProjectType::Python => "Python",
        ProjectType::Unknown => "unknown type",
    };

    let build_section = if project.build_command.is_empty() {
        String::new()
    } else {
        format!(
            r#"### Build and Test

```bash
# Build
{}

# Run tests
{}

# Lint
{}
```
"#,
            project.build_command, project.test_command, project.lint_command
        )
    };

    let source_layout_section = if project.source_layout.is_empty() {
        String::new()
    } else {
        format!(
            r#"### Source Layout

```
{}
```
"#,
            project.source_layout
        )
    };

    format!(
        r#"# CLAUDE.md

## Project

{name} ({type_label}) — TODO: add a one-line project description here.

{build_section}
{source_layout_section}
### Directory Conventions

```
docs/active/tickets/    # Ticket files (markdown with YAML frontmatter)
docs/active/stories/    # Story files (same frontmatter pattern)
docs/active/work/       # Work artifacts, one subdirectory per ticket ID
```

---

The RDSPI workflow definition is in docs/knowledge/rdspi-workflow.md and is injected into agent context by lisa automatically.
"#,
        name = project.name,
        type_label = type_label,
        build_section = build_section,
        source_layout_section = source_layout_section,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::ProjectType;

    #[test]
    fn test_rdspi_workflow_embedded() {
        assert!(RDSPI_WORKFLOW.contains("RDSPI Workflow"));
        assert!(RDSPI_WORKFLOW.contains("Research"));
        assert!(RDSPI_WORKFLOW.contains("Design"));
        assert!(RDSPI_WORKFLOW.contains("Structure"));
        assert!(RDSPI_WORKFLOW.contains("Plan"));
        assert!(RDSPI_WORKFLOW.contains("Implement"));
    }

    #[test]
    fn test_hooks_guide_embedded() {
        assert!(HOOKS_GUIDE.contains("on-notify"));
        assert!(HOOKS_GUIDE.contains("LISA_EVENT"));
    }

    #[test]
    fn test_generate_claude_md_rust() {
        let project = DetectedProject {
            project_type: ProjectType::Rust,
            name: "my-app".to_string(),
            build_command: "cargo build".to_string(),
            test_command: "cargo test".to_string(),
            lint_command: "cargo clippy".to_string(),
            source_layout: "src:\n  lib.rs\n  main.rs".to_string(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("# CLAUDE.md"));
        assert!(result.contains("my-app"));
        assert!(result.contains("(Rust)"));
        assert!(result.contains("cargo build"));
        assert!(result.contains("cargo test"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("docs/active/tickets/"));
        assert!(result.contains("docs/knowledge/rdspi-workflow.md"));
    }

    #[test]
    fn test_generate_claude_md_node() {
        let project = DetectedProject {
            project_type: ProjectType::Node,
            name: "my-node-app".to_string(),
            build_command: "npm run build".to_string(),
            test_command: "npm test".to_string(),
            lint_command: "npm run lint".to_string(),
            source_layout: "src:\n  index.ts".to_string(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("my-node-app"));
        assert!(result.contains("(Node.js)"));
        assert!(result.contains("npm run build"));
        assert!(result.contains("npm test"));
    }

    #[test]
    fn test_generate_claude_md_unknown() {
        let project = DetectedProject {
            project_type: ProjectType::Unknown,
            name: "mystery".to_string(),
            build_command: String::new(),
            test_command: String::new(),
            lint_command: String::new(),
            source_layout: String::new(),
        };

        let result = generate_claude_md(&project);
        assert!(result.contains("mystery"));
        assert!(result.contains("(unknown type)"));
        // Should still have directory conventions
        assert!(result.contains("docs/active/tickets/"));
    }

    #[test]
    fn test_on_idle_hook_content() {
        assert!(ON_IDLE_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_IDLE_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_IDLE_HOOK.contains(".lisa/signals"));
        assert!(ON_IDLE_HOOK.contains(".idle"));
    }

    #[test]
    fn test_on_stop_hook_content() {
        assert!(ON_STOP_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_STOP_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_STOP_HOOK.contains(".lisa/signals"));
        assert!(ON_STOP_HOOK.contains(".stopped"));
    }

    #[test]
    fn test_on_clear_hook_content() {
        assert!(ON_CLEAR_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_CLEAR_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_CLEAR_HOOK.contains(".lisa/signals"));
        assert!(ON_CLEAR_HOOK.contains(".cleared"));
    }

    #[test]
    fn test_on_heartbeat_hook_content() {
        assert!(ON_HEARTBEAT_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_HEARTBEAT_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_HEARTBEAT_HOOK.contains(".lisa/signals"));
        assert!(ON_HEARTBEAT_HOOK.contains(".heartbeat"));
    }

    #[test]
    fn test_on_notify_hook_content() {
        assert!(ON_NOTIFY_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_NOTIFY_HOOK.contains("on-notify"));
        assert!(ON_NOTIFY_HOOK.contains("LISA_EVENT"));
        assert!(ON_NOTIFY_HOOK.contains("complete"));
        assert!(ON_NOTIFY_HOOK.contains("attention"));
        assert!(ON_NOTIFY_HOOK.contains("LISA_REASON"));
        // ntfy may only appear as a commented example — never active.
        for line in ON_NOTIFY_HOOK.lines() {
            if line.contains("ntfy") {
                assert!(
                    line.trim_start().starts_with('#'),
                    "ntfy must only appear in comments: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_settings_local_json() {
        let json = settings_local_json();
        // All four hook types present
        assert!(json.contains("\"Stop\""));
        assert!(json.contains("\"SessionStart\""));
        assert!(json.contains("\"Notification\""));
        assert!(json.contains("\"PostToolUse\""));
        // Hook commands
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-idle.sh"));
        assert!(json.contains("on-heartbeat.sh"));
        // Matchers
        assert!(json.contains("\"clear\""));
        assert!(json.contains("idle_prompt"));
        assert!(json.contains(r#""type": "command""#));
        // Catch-all attention Notification binding (alongside idle_prompt).
        assert!(json.contains("on-notify"));
        // The generated JSON must embed the exact catch-all command and parse cleanly.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let notifications = parsed["hooks"]["Notification"].as_array().unwrap();
        assert_eq!(notifications.len(), 2, "idle_prompt + catch-all attention");
        let cmd = notifications[1]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(cmd, NOTIFY_ATTENTION_COMMAND);
        // PreToolUse[AskUserQuestion] question binding present and in sync with the const.
        assert!(json.contains("AskUserQuestion"));
        let pretool = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pretool.len(), 1, "single AskUserQuestion entry");
        assert_eq!(
            pretool[0]["matcher"].as_str().unwrap(),
            "AskUserQuestion",
            "the entry carries the AskUserQuestion matcher"
        );
        let qcmd = pretool[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(
            qcmd, NOTIFY_QUESTION_COMMAND,
            "embedded JSON command must match the const (no drift)"
        );
    }

    #[test]
    fn test_lisa_gitignore_content() {
        assert!(LISA_GITIGNORE.contains("signals/"));
    }

    #[test]
    fn test_agents_md_points_to_claude() {
        // The pointer names CLAUDE.md as the source of truth and keeps the RDSPI
        // reference, but carries no duplicated project body (so it cannot drift).
        assert!(AGENTS_MD.contains("# AGENTS.md"));
        assert!(AGENTS_MD.contains("CLAUDE.md"));
        assert!(AGENTS_MD.contains("docs/knowledge/rdspi-workflow.md"));
        // No build/source-layout sections copied from CLAUDE.md.
        assert!(!AGENTS_MD.contains("Build and Test"));
        assert!(!AGENTS_MD.contains("Source Layout"));
    }

    #[test]
    fn test_merge_hooks_empty_object() {
        let result = merge_hooks("{}").unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("\"Notification\""));
        assert!(result.contains("idle_prompt"));
        assert!(result.contains("on-idle.sh"));
        assert!(result.contains("\"PostToolUse\""));
        assert!(result.contains("on-heartbeat.sh"));
        // Catch-all attention binding added too.
        assert!(result.contains("on-notify"));
        // PreToolUse[AskUserQuestion] question binding added.
        assert!(result.contains("\"PreToolUse\""));
        assert!(result.contains("AskUserQuestion"));
        assert_eq!(count_question_commands(&result), 1);
    }

    #[test]
    fn test_merge_hooks_adds_attention_to_existing_idle() {
        // Settings that already has the idle_prompt Notification hook.
        let input = r#"{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}"#;
        let result = merge_hooks(input).unwrap();
        // Both entries present: idle_prompt preserved (not duplicated) + catch-all added.
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
        assert!(result.contains("on-notify"));
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed["hooks"]["Notification"].as_array().unwrap().len(),
            2,
            "idle_prompt entry and catch-all attention entry coexist"
        );
        assert_eq!(count_attention_commands(&result), 1);
        // Idempotent: re-merging does not collapse or duplicate either entry.
        let again = merge_hooks(&result).unwrap();
        assert_eq!(again.matches("\"idle_prompt\"").count(), 1);
        assert_eq!(count_attention_commands(&again), 1);
        let reparsed: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(
            reparsed["hooks"]["Notification"].as_array().unwrap().len(),
            2
        );
    }

    /// Count how many Notification hook commands exactly equal the catch-all
    /// attention command. Parses JSON so escaped quotes don't break a substring
    /// match (the command embeds `"` characters).
    fn count_attention_commands(json: &str) -> usize {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let Some(entries) = v["hooks"]["Notification"].as_array() else {
            return 0;
        };
        entries
            .iter()
            .filter_map(|e| e["hooks"].as_array())
            .flatten()
            .filter_map(|h| h["command"].as_str())
            .filter(|c| *c == NOTIFY_ATTENTION_COMMAND)
            .count()
    }

    /// Count how many PreToolUse hook commands exactly equal the question command.
    /// Parses JSON so the escaped quotes/backslashes in the command don't break a
    /// substring match.
    fn count_question_commands(json: &str) -> usize {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let Some(entries) = v["hooks"]["PreToolUse"].as_array() else {
            return 0;
        };
        entries
            .iter()
            .filter_map(|e| e["hooks"].as_array())
            .flatten()
            .filter_map(|h| h["command"].as_str())
            .filter(|c| *c == NOTIFY_QUESTION_COMMAND)
            .count()
    }

    #[test]
    fn test_merge_hooks_adds_pretooluse_question() {
        // Start from settings that already has the five legacy bindings but no PreToolUse.
        let input = r#"{
  "hooks": {
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh" }] }],
    "Notification": [{ "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh" }] }]
  }
}"#;
        let result = merge_hooks(input).unwrap();
        // The question binding is added exactly once, with the right matcher.
        assert_eq!(count_question_commands(&result), 1);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let pretool = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pretool.len(), 1);
        assert_eq!(pretool[0]["matcher"].as_str().unwrap(), "AskUserQuestion");
        // The pre-existing five bindings survive untouched.
        assert!(result.contains("on-heartbeat.sh"));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("idle_prompt"));
        assert_eq!(count_attention_commands(&result), 1);
        // Idempotent: re-merging neither duplicates nor drops the question entry.
        let again = merge_hooks(&result).unwrap();
        assert_eq!(count_question_commands(&again), 1);
        let reparsed: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(reparsed["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }

    /// Replicate the hook's POSIX `sed` extraction so the contract is tested end to
    /// end against a real payload, not a reimplementation. `sed` is unix-only.
    #[cfg(unix)]
    fn extract_question_via_sed(payload: &str) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("sed")
            .args(["-n", r#"s/.*"question":[ ]*"\([^"]*\)".*/\1/p"#])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn sed");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    #[cfg(unix)]
    #[test]
    fn test_notify_question_command_extracts_question() {
        // The const must literally embed the documented sed expression.
        assert!(
            NOTIFY_QUESTION_COMMAND.contains(r#"sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p'"#)
        );
        // It writes the awaiting signal unconditionally and only test-x-gates the notify.
        assert!(NOTIFY_QUESTION_COMMAND.contains("pane-$LISA_PANE_ID.awaiting"));
        assert!(NOTIFY_QUESTION_COMMAND.contains("LISA_REASON=question"));
        assert!(NOTIFY_QUESTION_COMMAND.contains("test -x .lisa/hooks/on-notify"));

        // (i) Happy path: the real captured single-line payload shape.
        let payload = r#"{"tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Which approach should I use to build the feature?","header":"Approach","options":[]}]}}"#;
        assert_eq!(
            extract_question_via_sed(payload),
            "Which approach should I use to build the feature?"
        );

        // (ii) Escaped-quote variant degrades gracefully (truncates, never panics).
        let escaped = r#"{"questions":[{"question":"He said \"hi\" to me","header":"X"}]}"#;
        let got = extract_question_via_sed(escaped);
        // The greedy-free [^"]* stops at the embedded quote; result is a (possibly
        // empty/partial) string, and the hook's `[ -z "$q" ]` fallback covers empties.
        assert!(
            !got.contains("to me"),
            "extraction stops at the escaped quote"
        );

        // (iii) No question key at all -> empty extraction -> hook falls back to generic.
        let none = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert_eq!(extract_question_via_sed(none), "");
    }

    #[test]
    fn test_merge_hooks_with_existing_idle_only() {
        // Start with only idle_prompt — should add Stop + SessionStart
        let input = r#"{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        // idle_prompt should not be duplicated
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
    }

    #[test]
    fn test_merge_hooks_already_complete() {
        let input = settings_local_json();
        let result = merge_hooks(&input).unwrap();
        // No duplicate hook entries (each command string contains the script name twice
        // due to the test -x guard, so count the full command instead)
        assert_eq!(result.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(result.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
        // The catch-all attention command appears exactly once.
        assert_eq!(count_attention_commands(&result), 1);
    }

    #[test]
    fn test_merge_hooks_upgrades_bare_path_commands() {
        // Old-style settings with bare-path hook commands
        let input = r#"{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": ".lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-clear.sh" }] }],
    "Notification": [{ "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-idle.sh" }] }]
  }
}"#;
        let result = merge_hooks(input).unwrap();
        // Should upgrade to guarded commands
        assert!(result.contains("test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh"));
        assert!(result.contains("test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh"));
        assert!(result.contains("test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh"));
        // No duplicates — each hook entry appears once
        assert_eq!(result.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(result.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
    }

    #[test]
    fn test_merge_hooks_preserves_permissions() {
        let input = r#"{"permissions":{"allow":["Bash(cargo test:*)"]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("cargo test"));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("idle_prompt"));
    }

    #[test]
    fn test_merge_hooks_invalid_json() {
        let result = merge_hooks("not json");
        assert!(result.is_err());
    }

    #[test]
    fn stop_hook_still_writes_stopped_and_captures_usage() {
        // T-027-02: the Stop hook keeps writing the `.stopped` signal and now
        // forwards its stdin payload to `lisa capture-usage`.
        assert!(ON_STOP_HOOK.contains("pane-$LISA_PANE_ID.stopped"));
        assert!(ON_STOP_HOOK.contains("capture-usage"));
        assert!(ON_STOP_HOOK.contains("${LISA_BIN:-lisa}"));
        // Reads stdin once (the Stop payload carries transcript_path).
        assert!(ON_STOP_HOOK.contains("in=$(cat)"));
    }

    #[test]
    fn heartbeat_hook_stays_trivial() {
        // The ticket's constraint: PostToolUse capture must not grow. The
        // heartbeat hook must not read stdin or invoke lisa.
        assert!(!ON_HEARTBEAT_HOOK.contains("capture-usage"));
        assert!(!ON_HEARTBEAT_HOOK.contains("$(cat)"));
        assert!(ON_HEARTBEAT_HOOK.contains("pane-$LISA_PANE_ID.heartbeat"));
    }
}
