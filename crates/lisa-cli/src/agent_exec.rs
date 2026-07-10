//! `lisa agent-exec` — the Codex-client signal producer.
//!
//! lisa types `LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec …` into a
//! fresh pane. This subcommand runs `codex exec --json …`, consumes the JSONL
//! event stream on stdout, translates events into lisa's signal files
//! (`.lisa/signals/pane-<id>.{heartbeat,stopped,error}`), renders a human-readable
//! conversation to its own stdout (which *is* the pane), persists the `thread_id`
//! for `resume`, and records `turn.completed.usage` for provenance.
//!
//! It is the host-side Rust analog of Claude Code's shell hooks (see
//! `templates.rs`), versioned atomically with the plugin. Design rationale and
//! the anchor rule live in `docs/active/work/T-023-01/`.
//!
//! The Codex JSON shape is `[PROVISIONAL]` (T-021-01 never ran against a live
//! `rust-v0.142.5`), so the parser is deliberately defensive: it keys on the
//! string `type` field with prefix matching and plucks fields best-effort,
//! tolerating renamed/missing keys and rendering anything unrecognised raw rather
//! than erroring.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Layer A — signal & event vocabulary (pure)
// ---------------------------------------------------------------------------

/// The three signals the Codex path ever produces. `.idle`/`.cleared`/`.awaiting`
/// do not occur on the autonomous `codex exec -a never` path (doc 05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalKind {
    Heartbeat,
    Stopped,
    Error,
}

impl SignalKind {
    fn suffix(self) -> &'static str {
        match self {
            SignalKind::Heartbeat => "heartbeat",
            SignalKind::Stopped => "stopped",
            SignalKind::Error => "error",
        }
    }
}

/// What the stream layer should do for a single observed event.
#[derive(Debug, Default, PartialEq, Eq)]
struct StreamEffect {
    /// Bump `pane-<id>.heartbeat` mtime (an `item.*` event = genuine progress).
    heartbeat: bool,
    /// A human-readable line to print to the pane (None => print nothing).
    render: Option<String>,
}

/// The terminal decision, applied once after the child process exits.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Success,
    Failure { message: String },
}

impl Outcome {
    /// The signal set to write. Failure writes `.error` (canonical, for the
    /// T-023-02 consumer + provenance) *and* `.stopped` (compat, so today's
    /// scheduler — which has no `.error` reader — still advances). See design
    /// Decision 4 / T-021-01 review Open-concern #1.
    fn signals(&self) -> &'static [SignalKind] {
        match self {
            Outcome::Success => &[SignalKind::Stopped],
            Outcome::Failure { .. } => &[SignalKind::Error, SignalKind::Stopped],
        }
    }
}

// ---------------------------------------------------------------------------
// Layer B — the translator (pure, the tested core)
// ---------------------------------------------------------------------------

/// Accumulates state across the event stream and applies the anchor rule.
///
/// Item statuses are best-effort heartbeat only (#14691); the authoritative
/// done/failed decision is `turn.completed`/`turn.failed` crossed with the
/// process exit code, taken in `finalize`.
#[derive(Debug, Default)]
struct Translator {
    thread_id: Option<String>,
    usage: Option<Value>,
    saw_turn_completed: bool,
    saw_turn_failed: bool,
    error_message: Option<String>,
}

impl Translator {
    /// Feed one parsed event. Updates captured state and returns what the stream
    /// layer should do. Never decides a terminal signal (that is `finalize`).
    fn observe(&mut self, event: &Value) -> StreamEffect {
        let ty = event_type(event).unwrap_or("");

        if ty == "thread.started" {
            if let Some(id) = extract_thread_id(event) {
                let render = Some(format!("⎇ thread {}", id));
                self.thread_id = Some(id);
                return StreamEffect {
                    heartbeat: false,
                    render,
                };
            }
            return StreamEffect::default();
        }

        if ty == "turn.completed" {
            self.saw_turn_completed = true;
            if let Some(u) = extract_usage(event) {
                self.usage = Some(u);
            }
            return StreamEffect {
                heartbeat: false,
                render: Some(format!("— turn complete{}", usage_summary(&self.usage))),
            };
        }

        if ty == "turn.failed" || ty == "error" {
            self.saw_turn_failed = true;
            let msg = extract_error_message(event)
                .unwrap_or_else(|| "codex reported a failure".to_string());
            let render = Some(format!("✗ {}", msg));
            self.error_message = Some(msg);
            return StreamEffect {
                heartbeat: false,
                render,
            };
        }

        if ty == "turn.started" {
            return StreamEffect {
                heartbeat: false,
                render: None,
            };
        }

        // `item.started` / `item.updated` / `item.completed` — best-effort
        // heartbeat, plus a rendered line when the item completes.
        if ty.starts_with("item.") {
            return StreamEffect {
                heartbeat: true,
                render: render_event(event),
            };
        }

        // Unknown / typeless line: nothing swallowed silently — render it raw,
        // but do not treat it as progress or a terminal event.
        StreamEffect {
            heartbeat: false,
            render: Some(compact_json(event)),
        }
    }

    /// Apply the anchor rule after the child exits.
    fn finalize(&self, exit_success: bool) -> Outcome {
        if self.saw_turn_completed && !self.saw_turn_failed && exit_success {
            Outcome::Success
        } else {
            let message = self.error_message.clone().unwrap_or_else(|| {
                "codex exec failed (no terminal turn.completed, or non-zero exit)".to_string()
            });
            Outcome::Failure { message }
        }
    }
}

// ---------------------------------------------------------------------------
// Field-pluck helpers — each tolerant of missing/renamed keys ([PROVISIONAL]).
// ---------------------------------------------------------------------------

fn event_type(e: &Value) -> Option<&str> {
    e.get("type").and_then(Value::as_str)
}

fn extract_thread_id(e: &Value) -> Option<String> {
    // Candidate shapes: {thread_id}, {thread:{id}}, {id}.
    for key in ["thread_id", "threadId"] {
        if let Some(s) = e.get(key).and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    if let Some(s) = e
        .get("thread")
        .and_then(|t| t.get("id"))
        .and_then(Value::as_str)
    {
        return Some(s.to_string());
    }
    e.get("id").and_then(Value::as_str).map(str::to_string)
}

fn extract_usage(e: &Value) -> Option<Value> {
    // `usage` may ride on the event directly or nested under `turn`.
    if let Some(u) = e.get("usage") {
        return Some(u.clone());
    }
    e.get("turn").and_then(|t| t.get("usage")).cloned()
}

fn extract_error_message(e: &Value) -> Option<String> {
    // {error:{message}} | {error:"…"} | {message}
    if let Some(m) = e
        .get("error")
        .and_then(|x| x.get("message"))
        .and_then(Value::as_str)
    {
        return Some(m.to_string());
    }
    if let Some(s) = e.get("error").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    e.get("message").and_then(Value::as_str).map(str::to_string)
}

/// The `item` payload of an `item.*` event, if present.
fn item_of(e: &Value) -> Option<&Value> {
    e.get("item").or_else(|| e.get("payload"))
}

/// The item's kind discriminator (`command_execution`, `agent_message`, …).
fn item_kind(item: &Value) -> Option<&str> {
    for key in ["item_type", "type", "kind"] {
        if let Some(s) = item.get(key).and_then(Value::as_str) {
            return Some(s);
        }
    }
    None
}

/// Best-effort human text out of an item (assistant messages, reasoning summary).
fn item_text(item: &Value) -> Option<String> {
    for key in ["text", "message", "content", "summary"] {
        if let Some(s) = item.get(key).and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    None
}

/// One-line human render of an event (None => print nothing). Only completed
/// items are rendered by default so the pane view stays chunked and readable.
fn render_event(event: &Value) -> Option<String> {
    let ty = event_type(event).unwrap_or("");
    // Render items on completion (and started for commands, so the pane shows the
    // command *before* it runs). updated events are heartbeat-only.
    let item = item_of(event)?;
    let kind = item_kind(item).unwrap_or("item");

    match kind {
        "agent_message" | "assistant_message" | "message" => {
            if ty == "item.completed" {
                item_text(item)
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
            } else {
                None
            }
        }
        "command_execution" | "command" => {
            if ty == "item.started" {
                item.get("command")
                    .and_then(Value::as_str)
                    .map(|c| format!("$ {}", c))
            } else {
                None
            }
        }
        "file_change" | "file_edit" | "patch" => {
            if ty == "item.completed" {
                Some(render_file_change(item))
            } else {
                None
            }
        }
        "reasoning" | "thinking" => {
            if ty == "item.completed" {
                item_text(item).map(|t| format!("· {}", first_line(&t)))
            } else {
                None
            }
        }
        "mcp_tool_call" | "tool_call" => {
            if ty == "item.completed" {
                let server = item.get("server").and_then(Value::as_str).unwrap_or("mcp");
                let tool = item
                    .get("tool")
                    .or_else(|| item.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("tool");
                Some(format!("→ {}.{}", server, tool))
            } else {
                None
            }
        }
        // Unknown item kind on completion — surface it compactly, never silent.
        _ => {
            if ty == "item.completed" {
                Some(compact_json(item))
            } else {
                None
            }
        }
    }
}

fn render_file_change(item: &Value) -> String {
    // {changes:[{path}]} | {files:[…]} | {path}
    let paths: Vec<String> = item
        .get("changes")
        .or_else(|| item.get("files"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    c.get("path")
                        .and_then(Value::as_str)
                        .or_else(|| c.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();

    if paths.is_empty() {
        if let Some(p) = item.get("path").and_then(Value::as_str) {
            format!("~ {}", p)
        } else {
            "~ (file change)".to_string()
        }
    } else {
        format!("~ {}", paths.join(", "))
    }
}

fn usage_summary(usage: &Option<Value>) -> String {
    match usage {
        Some(u) => {
            // Try common token fields without hardcoding the full schema.
            let input = u
                .get("input_tokens")
                .or_else(|| u.get("input"))
                .and_then(Value::as_u64);
            let output = u
                .get("output_tokens")
                .or_else(|| u.get("output"))
                .and_then(Value::as_u64);
            match (input, output) {
                (Some(i), Some(o)) => format!(" ({} in / {} out tokens)", i, o),
                _ => String::new(),
            }
        }
        None => String::new(),
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

fn compact_json(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "<unrenderable event>".to_string())
}

// ---------------------------------------------------------------------------
// Layer C — signal & artifact writers (thin, tempdir-testable)
// ---------------------------------------------------------------------------

/// Writes `pane-<id>.<suffix>` signal files. Mirrors the hook guard: with no
/// pane id, every write is a no-op (degrade-safely contract).
struct SignalWriter {
    pane_id: Option<String>,
    signal_dir: PathBuf,
}

impl SignalWriter {
    fn write(&self, kind: SignalKind) -> std::io::Result<()> {
        let pane_id = match &self.pane_id {
            Some(id) => id,
            None => return Ok(()), // no LISA_PANE_ID => no signals, still ran
        };
        fs::create_dir_all(&self.signal_dir)?;
        let path = self
            .signal_dir
            .join(format!("pane-{}.{}", pane_id, kind.suffix()));
        // Content is informational only — the plugin reads mtime/existence, not
        // bytes (see research.md). A timestamp keeps parity with the hooks.
        fs::write(path, format!("{}\n", now_epoch_secs()))
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Persist `thread_id` (for `--resume`) and `usage` (for provenance) keyed by
/// ticket. A separate artifact survives the plugin deleting `.stopped` on read.
fn persist_run_artifacts(
    codex_dir: &Path,
    key: &str,
    thread_id: Option<&str>,
    usage: Option<&Value>,
    success: bool,
) -> std::io::Result<()> {
    if thread_id.is_none() && usage.is_none() {
        return Ok(());
    }
    fs::create_dir_all(codex_dir)?;
    if let Some(id) = thread_id {
        fs::write(
            codex_dir.join(format!("{}.thread", key)),
            format!("{}\n", id),
        )?;
    }
    let record = serde_json::json!({
        "key": key,
        "thread_id": thread_id,
        "success": success,
        "usage": usage.cloned().unwrap_or(Value::Null),
    });
    fs::write(
        codex_dir.join(format!("{}.usage.json", key)),
        serde_json::to_string_pretty(&record).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Read a persisted thread id for `--resume`.
fn read_thread_id(codex_dir: &Path, key: &str) -> Option<String> {
    let raw = fs::read_to_string(codex_dir.join(format!("{}.thread", key))).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ---------------------------------------------------------------------------
// Layer D — the command shell
// ---------------------------------------------------------------------------

/// Parsed CLI arguments for `lisa agent-exec` (built from clap in `main.rs`).
pub struct AgentExecArgs {
    pub prompt: String,
    pub resume: bool,
    pub codex_bin: String,
    pub cwd: PathBuf,
    pub bypass_sandbox: bool,
    pub codex_args: Vec<String>,
    pub signal_dir: PathBuf,
}

/// Build the argv passed to the codex binary. Pure, so it is unit-tested without
/// spawning. `resolved_thread` is `Some(id)` for `--resume` with a known thread,
/// `None` for a fresh run or a `--resume` that must fall back to `--last`.
///
/// `-a/--ask-for-approval` is a top-level codex option on 0.144.0: accepted
/// before the `exec` subcommand, rejected after it (observed live 2026-07-09;
/// the pinned research target 0.142.5 tolerated both positions). It must lead
/// the argv.
fn build_codex_argv(args: &AgentExecArgs, resolved_thread: Option<&str>) -> Vec<String> {
    let mut argv: Vec<String> = Vec::new();

    if !args.bypass_sandbox {
        argv.push("-a".to_string());
        argv.push("never".to_string());
    }
    argv.push("exec".to_string());

    if args.resume {
        argv.push("resume".to_string());
        match resolved_thread {
            Some(id) => argv.push(id.to_string()),
            None => argv.push("--last".to_string()),
        }
    }

    argv.push("--json".to_string());
    argv.push("--skip-git-repo-check".to_string());
    argv.push("-C".to_string());
    argv.push(args.cwd.display().to_string());

    if args.bypass_sandbox {
        argv.push("--dangerously-bypass-approvals-and-sandbox".to_string());
    } else {
        argv.push("-s".to_string());
        argv.push("workspace-write".to_string());
    }

    argv.extend(args.codex_args.iter().cloned());
    argv.push(args.prompt.clone());
    argv
}

/// Run codex under the wrapper: stream events → signals + rendered pane view.
///
/// Returns `Ok(())` even when codex *fails* — the outcome travels to the plugin
/// via the signal files, not this process's exit code. Only an outright
/// spawn/IO failure (e.g. codex not on PATH) is an `Err`.
pub fn run_agent_exec(args: AgentExecArgs) -> Result<(), String> {
    let pane_id = std::env::var("LISA_PANE_ID").ok().filter(|s| !s.is_empty());
    let ticket_id = std::env::var("LISA_TICKET_ID")
        .ok()
        .filter(|s| !s.is_empty());

    let codex_dir = args.cwd.join(".lisa").join("codex");
    let key = ticket_id
        .clone()
        .or_else(|| pane_id.clone().map(|p| format!("pane-{}", p)))
        .unwrap_or_else(|| "last".to_string());

    // Resolve a thread id for --resume before building argv.
    let resolved_thread = if args.resume {
        read_thread_id(&codex_dir, &key)
    } else {
        None
    };
    if args.resume && resolved_thread.is_none() && ticket_id.is_none() && pane_id.is_none() {
        return Err(
            "--resume set but no thread could be located (no LISA_TICKET_ID/LISA_PANE_ID and no \
             persisted thread; codex would fall back to --last with no key)"
                .to_string(),
        );
    }

    let argv = build_codex_argv(&args, resolved_thread.as_deref());

    let mut child = Command::new(&args.codex_bin)
        .args(&argv)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn `{}`: {}", args.codex_bin, e))?;

    let signal_writer = SignalWriter {
        pane_id: pane_id.clone(),
        signal_dir: args.signal_dir.clone(),
    };
    let mut translator = Translator::default();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "codex child produced no stdout pipe".to_string())?;

    let out = std::io::stdout();
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // pipe closed / read error — proceed to wait()
        };
        if line.trim().is_empty() {
            continue;
        }

        let effect = match serde_json::from_str::<Value>(&line) {
            Ok(event) => translator.observe(&event),
            // Non-JSON on stdout (shouldn't happen under --json): render raw.
            Err(_) => StreamEffect {
                heartbeat: false,
                render: Some(line.clone()),
            },
        };

        if effect.heartbeat {
            let _ = signal_writer.write(SignalKind::Heartbeat);
        }
        if let Some(text) = effect.render {
            let mut handle = out.lock();
            let _ = writeln!(handle, "{}", text);
            let _ = handle.flush(); // chunked, per-line pane view
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("failed waiting on codex: {}", e))?;

    let outcome = translator.finalize(status.success());
    for kind in outcome.signals() {
        let _ = signal_writer.write(*kind);
    }

    let success = matches!(outcome, Outcome::Success);
    let _ = persist_run_artifacts(
        &codex_dir,
        &key,
        translator.thread_id.as_deref(),
        translator.usage.as_ref(),
        success,
    );

    match &outcome {
        Outcome::Success => {
            let mut handle = out.lock();
            let _ = writeln!(handle, "— agent-exec complete");
        }
        Outcome::Failure { message } => {
            eprintln!("agent-exec: codex failed: {}", message);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Layer E — tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- observe / finalize -------------------------------------------------

    #[test]
    fn thread_started_captures_id() {
        let mut t = Translator::default();
        let eff = t.observe(&json!({"type": "thread.started", "thread_id": "th_abc"}));
        assert!(!eff.heartbeat);
        assert_eq!(t.thread_id.as_deref(), Some("th_abc"));
    }

    #[test]
    fn thread_started_tolerates_nested_id() {
        let mut t = Translator::default();
        t.observe(&json!({"type": "thread.started", "thread": {"id": "th_nested"}}));
        assert_eq!(t.thread_id.as_deref(), Some("th_nested"));
    }

    #[test]
    fn item_events_are_heartbeats() {
        let mut t = Translator::default();
        for ty in ["item.started", "item.updated", "item.completed"] {
            let eff = t.observe(
                &json!({"type": ty, "item": {"item_type": "command_execution", "command": "ls"}}),
            );
            assert!(eff.heartbeat, "{} should heartbeat", ty);
        }
    }

    #[test]
    fn turn_completed_captures_usage_but_no_terminal_signal() {
        let mut t = Translator::default();
        let eff = t.observe(&json!({
            "type": "turn.completed",
            "usage": {"input_tokens": 10, "output_tokens": 20}
        }));
        assert!(!eff.heartbeat);
        assert!(t.saw_turn_completed);
        assert!(t.usage.is_some());
    }

    #[test]
    fn anchor_rule_success_requires_completed_and_exit_ok() {
        let mut t = Translator::default();
        t.observe(&json!({"type": "turn.completed"}));
        assert_eq!(t.finalize(true), Outcome::Success);
        // Non-zero exit overrides a completed turn.
        assert!(matches!(t.finalize(false), Outcome::Failure { .. }));
    }

    #[test]
    fn missing_turn_completed_is_failure_even_on_exit_ok() {
        let t = Translator::default();
        assert!(matches!(t.finalize(true), Outcome::Failure { .. }));
    }

    #[test]
    fn turn_failed_is_failure_even_on_exit_ok() {
        let mut t = Translator::default();
        t.observe(&json!({"type": "turn.completed"}));
        t.observe(&json!({"type": "turn.failed", "error": {"message": "boom"}}));
        match t.finalize(true) {
            Outcome::Failure { message } => assert!(message.contains("boom")),
            other => panic!("expected failure, got {:?}", other),
        }
    }

    #[test]
    fn top_level_error_is_failure() {
        let mut t = Translator::default();
        t.observe(&json!({"type": "error", "message": "top-level explosion"}));
        assert!(matches!(t.finalize(true), Outcome::Failure { .. }));
    }

    #[test]
    fn unknown_line_no_heartbeat_no_panic() {
        let mut t = Translator::default();
        let eff = t.observe(&json!({"type": "something.new", "data": 1}));
        assert!(!eff.heartbeat);
        assert!(eff.render.is_some()); // rendered raw, nothing swallowed
    }

    #[test]
    fn outcome_signal_sets() {
        assert_eq!(Outcome::Success.signals(), &[SignalKind::Stopped]);
        assert_eq!(
            Outcome::Failure {
                message: "x".into()
            }
            .signals(),
            &[SignalKind::Error, SignalKind::Stopped]
        );
    }

    // --- full recorded streams (AC: no live codex) --------------------------

    const SUCCESS_STREAM: &str = include_str!("../tests/fixtures/codex-success.jsonl");
    const FAILED_STREAM: &str = include_str!("../tests/fixtures/codex-turn-failed.jsonl");

    fn run_stream(raw: &str) -> (Translator, usize) {
        let mut t = Translator::default();
        let mut heartbeats = 0;
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: Value = serde_json::from_str(line).expect("fixture line must be JSON");
            if t.observe(&event).heartbeat {
                heartbeats += 1;
            }
        }
        (t, heartbeats)
    }

    #[test]
    fn success_fixture_maps_to_stopped() {
        let (t, heartbeats) = run_stream(SUCCESS_STREAM);
        assert!(heartbeats >= 1, "expected item heartbeats");
        assert_eq!(t.thread_id.as_deref(), Some("th_success_1"));
        assert!(t.usage.is_some(), "usage should be captured");
        assert_eq!(t.finalize(true), Outcome::Success);
        assert_eq!(t.finalize(true).signals(), &[SignalKind::Stopped]);
    }

    #[test]
    fn success_fixture_with_nonzero_exit_is_failure() {
        let (t, _) = run_stream(SUCCESS_STREAM);
        assert!(matches!(t.finalize(false), Outcome::Failure { .. }));
        assert_eq!(
            t.finalize(false).signals(),
            &[SignalKind::Error, SignalKind::Stopped]
        );
    }

    #[test]
    fn failed_fixture_maps_to_error_plus_stopped() {
        let (t, _) = run_stream(FAILED_STREAM);
        let outcome = t.finalize(true);
        assert!(matches!(outcome, Outcome::Failure { .. }));
        assert_eq!(outcome.signals(), &[SignalKind::Error, SignalKind::Stopped]);
    }

    // --- writers (tempdir) --------------------------------------------------

    #[test]
    fn signal_writer_writes_expected_filename() {
        let dir = tempfile::tempdir().unwrap();
        let w = SignalWriter {
            pane_id: Some("7".to_string()),
            signal_dir: dir.path().to_path_buf(),
        };
        w.write(SignalKind::Heartbeat).unwrap();
        assert!(dir.path().join("pane-7.heartbeat").exists());
        w.write(SignalKind::Stopped).unwrap();
        assert!(dir.path().join("pane-7.stopped").exists());
        w.write(SignalKind::Error).unwrap();
        assert!(dir.path().join("pane-7.error").exists());
    }

    #[test]
    fn signal_writer_no_pane_id_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let w = SignalWriter {
            pane_id: None,
            signal_dir: dir.path().join("signals"),
        };
        w.write(SignalKind::Stopped).unwrap();
        // Nothing created, including the directory.
        assert!(!dir.path().join("signals").exists());
    }

    #[test]
    fn artifacts_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".lisa/codex");
        let usage = json!({"input_tokens": 5, "output_tokens": 6});
        persist_run_artifacts(&codex_dir, "T-999-01", Some("th_xyz"), Some(&usage), true).unwrap();

        assert_eq!(
            read_thread_id(&codex_dir, "T-999-01").as_deref(),
            Some("th_xyz")
        );
        let recorded: Value = serde_json::from_str(
            &fs::read_to_string(codex_dir.join("T-999-01.usage.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(recorded["usage"]["output_tokens"], 6);
        assert_eq!(recorded["success"], true);
    }

    #[test]
    fn read_thread_id_absent_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_thread_id(dir.path(), "nope").is_none());
    }

    // --- argv builder -------------------------------------------------------

    fn base_args() -> AgentExecArgs {
        AgentExecArgs {
            prompt: "do the thing".to_string(),
            resume: false,
            codex_bin: "codex".to_string(),
            cwd: PathBuf::from("/work"),
            bypass_sandbox: false,
            codex_args: vec![],
            signal_dir: PathBuf::from(".lisa/signals"),
        }
    }

    #[test]
    fn argv_default_flags() {
        let argv = build_codex_argv(&base_args(), None);
        // Full positional shape — `-a never` precedes `exec` (top-level option
        // on codex 0.144.0; rejected after the subcommand).
        assert_eq!(
            argv,
            vec![
                "-a",
                "never",
                "exec",
                "--json",
                "--skip-git-repo-check",
                "-C",
                "/work",
                "-s",
                "workspace-write",
                "do the thing",
            ]
        );
    }

    #[test]
    fn argv_bypass_sandbox() {
        let mut a = base_args();
        a.bypass_sandbox = true;
        let argv = build_codex_argv(&a, None);
        assert_eq!(argv[0], "exec");
        assert!(argv.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
        assert!(!argv.contains(&"never".to_string()));
    }

    #[test]
    fn argv_resume_with_thread() {
        let mut a = base_args();
        a.resume = true;
        let argv = build_codex_argv(&a, Some("th_prev"));
        assert_eq!(argv[..5], ["-a", "never", "exec", "resume", "th_prev"]);
    }

    #[test]
    fn argv_resume_falls_back_to_last() {
        let mut a = base_args();
        a.resume = true;
        let argv = build_codex_argv(&a, None);
        assert!(argv.windows(2).any(|w| w == ["resume", "--last"]));
    }

    #[test]
    fn argv_passes_extra_codex_args() {
        let mut a = base_args();
        a.codex_args = vec!["--model".to_string(), "o4".to_string()];
        let argv = build_codex_argv(&a, None);
        assert!(argv.windows(2).any(|w| w == ["--model", "o4"]));
    }
}
