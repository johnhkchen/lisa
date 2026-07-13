use crate::detect::DetectedProject;

/// The RDSPI workflow document, embedded at compile time
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");

/// Exact outgoing Lisa templates accepted as proof of an unmodified install.
/// Keep only byte-distinct generations; current content is handled separately.
pub(crate) const LEGACY_RDSPI_WORKFLOWS: &[&str] = &[
    include_str!("../data/legacy/rdspi-workflow-v0.2.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.md"),
];

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

pub(crate) const LEGACY_ON_IDLE_HOOKS: &[&str] = &[];

/// The on-stop hook script, called by the native client's Stop event.
/// Fires when Claude or Codex finishes responding (ready for input).
///
/// Beyond writing the `.stopped` signal it forwards the Stop payload (piped on
/// stdin, carrying `transcript_path`) to `lisa capture-usage`, which sums the
/// session's token usage into the provider-specific append-only capture ledger.
/// Stop fires per *turn*, not per tool call, so the heartbeat hook stays trivial.
/// Stops without observable transcript usage append a no-capture marker, while
/// malformed identity or persistence errors remain visible to the operator.
pub const ON_STOP_HOOK: &str = r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input, and
# captures session token usage for the provenance ledger (T-027-02).

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. No-capture markers and capture errors remain visible to operators.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage
"#;

/// Exact prior Stop-hook generations eligible for safe `lisa init` upgrades.
pub(crate) const LEGACY_ON_STOP_HOOKS: &[&str] = &[
    r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
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
"#,
    r#"#!/bin/sh
# Lisa stop signal hook — called by Claude Code when it finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi
"#,
];

/// The on-clear hook script, called by the native client's SessionStart[clear] event.
/// Fires after /clear is processed (context cleared).
pub const ON_CLEAR_HOOK: &str = r#"#!/bin/sh
# Lisa clear signal hook — called after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#;

pub(crate) const LEGACY_ON_CLEAR_HOOKS: &[&str] = &[r#"#!/bin/sh
# Lisa clear signal hook — called by Claude Code after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#];

/// The provider-neutral native process-start hook, called by the native
/// client's SessionStart[startup] event. It publishes the scheduler-owned
/// attempt lease only when the immutable launch identity still matches the
/// pane marker, so a stale predecessor cannot borrow a successor's identity.
pub const ON_START_HOOK: &str = r#"#!/bin/sh
# Lisa process-start signal hook — called when a native agent process starts.
# Publishes only an exact pane/ticket/attempt-scoped scheduler lease.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ] && [ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ]; then
    case "$LISA_ATTEMPT_ID" in
        *[!0-9]*) exit 0 ;;
    esac
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")
    actual=$(cat "$marker" 2>/dev/null) || exit 0
    [ "$actual" = "$expected" ] || exit 0

    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.started.tmp.$$"
    if cp "$marker" "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.started"
    else
        rm -f "$tmp"
    fi
fi
"#;

pub(crate) const LEGACY_ON_START_HOOKS: &[&str] = &[];

/// The heartbeat hook script, called by the native client's PostToolUse event.
/// Fires after every tool call, proving the session is actively working.
/// The plugin uses the absence of recent heartbeats — not stop/idle signals,
/// which fire before agents truly finish — to decide a pane is safe to reuse.
pub const ON_HEARTBEAT_HOOK: &str = r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Copies the scheduler-owned attempt lease into an atomic liveness signal.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat.tmp.$$"
    if [ -r "$marker" ] && cp "$marker" "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
    else
        rm -f "$tmp"
    fi
fi
"#;

pub(crate) const LEGACY_ON_HEARTBEAT_HOOKS: &[&str] = &[
    r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
"#,
    r#"#!/bin/sh
# Lisa heartbeat signal hook — called by Claude Code after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
"#,
];

/// The native provider `UserPromptSubmit` hook. It preserves the complete JSON
/// payload for the plugin's ticket/generation detector and publishes it with an
/// atomic rename so the polling scheduler never observes a partial document.
pub const ON_ACK_HOOK: &str = r#"#!/bin/sh
# Lisa assignment acknowledgment hook — called before a provider submits a user prompt.
# Writes the raw lifecycle payload for ticket/generation matching in the plugin.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
    if cat > "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
    else
        rm -f "$tmp"
    fi
fi
"#;

pub(crate) const LEGACY_ON_ACK_HOOKS: &[&str] = &[];

/// Gitignore content for `.lisa/` runtime state. Signal files and per-provider
/// usage/session artifacts are machine-owned and must never enter the project DAG.
pub const LISA_GITIGNORE: &str = "signals/\nattempts/\nclaude/\ncodex/\n";

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

pub(crate) const LEGACY_ON_NOTIFY_HOOKS: &[&str] = &[];

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

/// Generate .claude/settings.local.json with Stop, SessionStart, UserPromptSubmit, Notification
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
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh"
          }
        ]
      },
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
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
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

/// Generate `.codex/hooks.json` for the native interactive Codex adapter.
///
/// Codex has no `idle_prompt` or `AskUserQuestion` hook equivalent, so the TUI
/// installs only lifecycle signals it can state truthfully: prompt submission,
/// tool progress, turn completion, and `/clear` completion. Shared scripts
/// attribute events through `LISA_PANE_ID`; the ack script preserves raw JSON.
pub fn codex_hooks_json() -> String {
    r#"{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh"
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
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh"
          }
        ]
      },
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
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
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
            // Entry exists — upgrade Lisa's old bare-path command in place. A
            // user hook may share the same matcher; keep it and append Lisa's
            // command instead of treating the matcher alone as a duplicate.
            if let Some(hooks_arr) = arr[idx].get_mut("hooks").and_then(|h| h.as_array_mut()) {
                let mut lisa_hook_found = false;
                for hook in hooks_arr.iter_mut() {
                    if let Some(cmd_val) = hook.get_mut("command") {
                        if let Some(existing) = cmd_val.as_str() {
                            if existing.contains(script_path) {
                                lisa_hook_found = true;
                                if existing != command {
                                    *cmd_val = serde_json::json!(command);
                                }
                            }
                        }
                    }
                }
                if !lisa_hook_found {
                    hooks_arr.push(serde_json::json!({
                        "type": "command",
                        "command": command
                    }));
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
        Some("startup"),
        "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh",
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
    ensure_hook(
        hooks_obj,
        "UserPromptSubmit",
        None,
        "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh",
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

/// Merge the native Codex lifecycle hooks into an existing `.codex/hooks.json`
/// without disturbing user-owned hook groups.
pub fn merge_codex_hooks(existing_json: &str) -> Result<String, String> {
    let mut root: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| format!("invalid JSON in hooks.json: {}", e))?;
    let obj = root
        .as_object_mut()
        .ok_or("hooks.json root is not an object")?;
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or("hooks.json 'hooks' is not an object")?;

    ensure_hook(
        hooks_obj,
        "Stop",
        None,
        "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("startup"),
        "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("clear"),
        "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh",
    );
    ensure_hook(
        hooks_obj,
        "PostToolUse",
        Some(".*"),
        "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh",
    );
    ensure_hook(
        hooks_obj,
        "UserPromptSubmit",
        None,
        "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh",
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
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_rdspi_workflow_embedded() {
        assert!(RDSPI_WORKFLOW.contains("RDSPI Workflow"));
        assert!(RDSPI_WORKFLOW.contains("Research"));
        assert!(RDSPI_WORKFLOW.contains("Design"));
        assert!(RDSPI_WORKFLOW.contains("Structure"));
        assert!(RDSPI_WORKFLOW.contains("Plan"));
        assert!(RDSPI_WORKFLOW.contains("Implement"));
        assert!(RDSPI_WORKFLOW.contains("Review"));
    }

    #[test]
    fn test_review_disposition_contract_is_injected() {
        let project = DetectedProject {
            project_type: ProjectType::Rust,
            name: "review-contract".to_string(),
            build_command: "cargo build".to_string(),
            test_command: "cargo test".to_string(),
            lint_command: "cargo clippy".to_string(),
            source_layout: "src:\n  main.rs".to_string(),
        };

        let claude_md = generate_claude_md(&project);
        assert!(claude_md.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(claude_md.contains("injected into agent context"));
        assert!(RDSPI_WORKFLOW.contains("review-disposition.json"));
        assert!(RDSPI_WORKFLOW.contains(r#"{"disposition":"pass","reason":null}"#));
        assert!(RDSPI_WORKFLOW
            .contains(r#"{"disposition":"block","reason":"<non-empty actionable reason>"}"#));
        assert!(RDSPI_WORKFLOW
            .contains("A pass with a reason, or a block without a non-empty reason, is invalid."));
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
    fn test_on_start_hook_content() {
        assert!(ON_START_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_START_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_START_HOOK.contains("LISA_TICKET_ID"));
        assert!(ON_START_HOOK.contains("LISA_ATTEMPT_ID"));
        assert!(ON_START_HOOK.contains(".lease"));
        assert!(ON_START_HOOK.contains(".started"));
        assert!(ON_START_HOOK.contains("mv \"$tmp\""));
    }

    fn run_start_hook(marker: Option<&str>, ticket_id: &str, attempt_id: &str) -> bool {
        let root = tempfile::tempdir().unwrap();
        let signals = root.path().join(".lisa/signals");
        fs::create_dir_all(&signals).unwrap();
        let script = root.path().join("on-start.sh");
        fs::write(&script, ON_START_HOOK).unwrap();
        if let Some(body) = marker {
            fs::write(signals.join("pane-7.lease"), body).unwrap();
        }
        let status = Command::new("/bin/sh")
            .arg(&script)
            .current_dir(root.path())
            .env("LISA_PANE_ID", "7")
            .env("LISA_TICKET_ID", ticket_id)
            .env("LISA_ATTEMPT_ID", attempt_id)
            .status()
            .unwrap();
        assert!(status.success());
        let started = signals.join("pane-7.started");
        if started.exists() {
            assert_eq!(fs::read_to_string(&started).unwrap(), marker.unwrap());
            true
        } else {
            assert!(fs::read_dir(&signals)
                .unwrap()
                .flatten()
                .all(|entry| !entry.file_name().to_string_lossy().contains("started.tmp")));
            false
        }
    }

    #[test]
    fn test_start_hook_fixture_accepts_only_matching_attempt() {
        let matching = r#"{"ticket_id":"T-035-01-01","attempt_id":2}"#;
        assert!(run_start_hook(Some(matching), "T-035-01-01", "2"));
        assert!(!run_start_hook(Some(matching), "T-035-01-01", "1"));
        assert!(!run_start_hook(Some(matching), "T-OTHER", "2"));
        assert!(!run_start_hook(None, "T-035-01-01", "2"));
        assert!(!run_start_hook(Some(matching), "T-035-01-01", "bad"));
    }

    #[test]
    fn test_no_provider_start_produces_no_signal() {
        let root = tempfile::tempdir().unwrap();
        let signals = root.path().join(".lisa/signals");
        fs::create_dir_all(&signals).unwrap();
        fs::write(
            signals.join("pane-7.lease"),
            r#"{"ticket_id":"T-035-01-01","attempt_id":2}"#,
        )
        .unwrap();
        assert!(!signals.join("pane-7.started").exists());
    }

    #[test]
    fn test_on_heartbeat_hook_content() {
        assert!(ON_HEARTBEAT_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_HEARTBEAT_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_HEARTBEAT_HOOK.contains(".lisa/signals"));
        assert!(ON_HEARTBEAT_HOOK.contains(".heartbeat"));
        assert!(ON_HEARTBEAT_HOOK.contains(".lease"));
        assert!(ON_HEARTBEAT_HOOK.contains("mv \"$tmp\""));
    }

    #[test]
    fn test_on_ack_hook_preserves_payload_atomically() {
        assert!(ON_ACK_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_ACK_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_ACK_HOOK.contains("cat > \"$tmp\""));
        assert!(ON_ACK_HOOK.contains("mv \"$tmp\""));
        assert!(ON_ACK_HOOK.contains("pane-$LISA_PANE_ID.ack"));
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
        // Lifecycle and interaction hook types present.
        assert!(json.contains("\"Stop\""));
        assert!(json.contains("\"SessionStart\""));
        assert!(json.contains("\"Notification\""));
        assert!(json.contains("\"PostToolUse\""));
        assert!(json.contains("\"UserPromptSubmit\""));
        // Hook commands
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-start.sh"));
        assert!(json.contains("on-idle.sh"));
        assert!(json.contains("on-heartbeat.sh"));
        assert!(json.contains("on-ack.sh"));
        // Matchers
        assert!(json.contains("\"clear\""));
        assert!(json.contains("idle_prompt"));
        assert!(json.contains(r#""type": "command""#));
        // Catch-all attention Notification binding (alongside idle_prompt).
        assert!(json.contains("on-notify"));
        // The generated JSON must embed the exact catch-all command and parse cleanly.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
            "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
        );
        assert_eq!(parsed["hooks"]["SessionStart"][0]["matcher"], "startup");
        assert_eq!(parsed["hooks"]["SessionStart"][1]["matcher"], "clear");
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
    fn test_codex_hooks_json_contains_native_tui_signals() {
        let json = codex_hooks_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["hooks"]["Stop"].is_array());
        assert!(parsed["hooks"]["PostToolUse"].is_array());
        assert!(parsed["hooks"]["UserPromptSubmit"].is_array());
        assert_eq!(parsed["hooks"]["PostToolUse"][0]["matcher"], ".*");
        assert_eq!(parsed["hooks"]["SessionStart"][0]["matcher"], "startup");
        assert_eq!(parsed["hooks"]["SessionStart"][1]["matcher"], "clear");
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-start.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-heartbeat.sh"));
        assert!(json.contains("on-ack.sh"));
        assert!(!json.contains("idle_prompt"));
        assert!(!json.contains("AskUserQuestion"));
    }

    #[test]
    fn test_merge_codex_hooks_preserves_user_hooks_and_is_idempotent() {
        let input = r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"./mine.sh"}]}],"PostToolUse":[{"matcher":".*","hooks":[{"type":"command","command":"./my-heartbeat.sh"}]}]}}"#;
        let merged = merge_codex_hooks(input).unwrap();
        assert!(merged.contains("./mine.sh"));
        assert!(merged.contains("./my-heartbeat.sh"));
        assert!(merged.contains("on-stop.sh"));
        assert!(merged.contains("on-start.sh"));
        assert!(merged.contains("on-clear.sh"));
        assert!(merged.contains("on-heartbeat.sh"));
        assert!(merged.contains("on-ack.sh"));

        let again = merge_codex_hooks(&merged).unwrap();
        assert_eq!(again.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(again.matches("test -x .lisa/hooks/on-start.sh").count(), 1);
        assert_eq!(again.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(
            again.matches("test -x .lisa/hooks/on-heartbeat.sh").count(),
            1
        );
        assert_eq!(again.matches("test -x .lisa/hooks/on-ack.sh").count(), 1);
    }

    #[test]
    fn test_lisa_gitignore_content() {
        assert!(LISA_GITIGNORE.contains("signals/"));
        assert!(LISA_GITIGNORE.contains("claude/"));
        assert!(LISA_GITIGNORE.contains("codex/"));
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
        assert!(result.contains("\"UserPromptSubmit\""));
        assert!(result.contains("on-ack.sh"));
        // Catch-all attention binding added too.
        assert!(result.contains("on-notify"));
        // PreToolUse[AskUserQuestion] question binding added.
        assert!(result.contains("\"PreToolUse\""));
        assert!(result.contains("AskUserQuestion"));
        assert_eq!(count_question_commands(&result), 1);
        let again = merge_hooks(&result).unwrap();
        assert_eq!(again.matches("test -x .lisa/hooks/on-ack.sh").count(), 1);
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
    fn stop_hook_writes_stopped_and_keeps_capture_outcomes_visible() {
        // T-027-02: the Stop hook keeps writing the `.stopped` signal and now
        // forwards its stdin payload to `lisa capture-usage`.
        assert!(ON_STOP_HOOK.contains("pane-$LISA_PANE_ID.stopped"));
        assert!(ON_STOP_HOOK.contains("capture-usage"));
        assert!(ON_STOP_HOOK.contains("${LISA_BIN:-lisa}"));
        // Reads stdin once (the Stop payload carries transcript_path).
        assert!(ON_STOP_HOOK.contains("in=$(cat)"));
        assert!(!ON_STOP_HOOK.contains("2>/dev/null"));
        assert!(!ON_STOP_HOOK.contains("|| true"));

        let live_hook = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.lisa/hooks/on-stop.sh"),
        )
        .unwrap();
        assert_eq!(live_hook, ON_STOP_HOOK);
    }

    #[test]
    fn heartbeat_hook_stays_trivial() {
        // The ticket's constraint: PostToolUse capture must not grow. The
        // heartbeat hook must not read stdin or invoke lisa.
        assert!(!ON_HEARTBEAT_HOOK.contains("capture-usage"));
        assert!(!ON_HEARTBEAT_HOOK.contains("$(cat)"));
        assert!(ON_HEARTBEAT_HOOK.contains("pane-$LISA_PANE_ID.heartbeat"));
        assert!(ON_HEARTBEAT_HOOK.contains("pane-$LISA_PANE_ID.lease"));
    }
}
