# T-023-01 Structure — agent-exec wrapper

The blueprint: file-level changes, module boundaries, public interfaces, internal
organisation, ordering. Not code — the shape of the code.

## Files

| File | Change | Why |
|---|---|---|
| `crates/lisa-cli/src/agent_exec.rs` | **create** | the whole feature: CLI entry, translator core, IO shell, signal/artifact writers, unit tests |
| `crates/lisa-cli/src/main.rs` | **modify** | `mod agent_exec;`; add `AgentExec { … }` to `Commands`; add the match arm |
| `crates/lisa-cli/Cargo.toml` | **no change** | serde_json/serde/clap/tempfile already present |
| `crates/lisa-cli/tests/fixtures/*.jsonl` | **create** | recorded event streams for translator unit tests |

Nothing under `crates/lisa-plugin/` or `crates/lisa-core/` — the pane-launch
plumbing and the `.error` consumer are T-023-02.

## `agent_exec.rs` internal organisation

Four layers, top (pure) to bottom (IO). The pure layers carry the tests.

### Layer A — signal & event vocabulary (pure)

```rust
/// The three signals the Codex path ever produces.
enum SignalKind { Heartbeat, Stopped, Error }
impl SignalKind { fn suffix(&self) -> &'static str } // "heartbeat"|"stopped"|"error"

/// Per-line result during streaming.
struct StreamEffect { heartbeat: bool, render: Option<String> }

/// Final decision after process exit.
enum Outcome { Success, Failure { message: String } }
impl Outcome { fn signals(&self) -> &'static [SignalKind] }
// Success => &[Stopped]; Failure => &[Error, Stopped]  (Decision 4)
```

### Layer B — the translator (pure, the tested core)

```rust
#[derive(Default)]
struct Translator {
    thread_id: Option<String>,
    usage: Option<serde_json::Value>,
    saw_turn_completed: bool,
    saw_turn_failed: bool,
    error_message: Option<String>,
}

impl Translator {
    /// Feed one parsed event. Updates captured state; returns what the
    /// stream layer should do (bump heartbeat, print a line).
    fn observe(&mut self, event: &serde_json::Value) -> StreamEffect;

    /// After the child exits, apply the anchor rule.
    fn finalize(&self, exit_success: bool) -> Outcome;
}

/// Best-effort render of one event to a human line (None => print nothing).
fn render_event(event: &serde_json::Value) -> Option<String>;

/// Best-effort field pluck helpers, each tolerant of missing/renamed keys.
fn event_type(e: &Value) -> Option<&str>;
fn extract_thread_id(e: &Value) -> Option<String>;
fn extract_usage(e: &Value) -> Option<Value>;
fn item_of(e: &Value) -> Option<&Value>;
fn item_kind(item: &Value) -> Option<&str>;   // tries "item_type" then "type"
fn item_text(item: &Value) -> Option<String>; // tries "text"/"message"/"content"
```

`observe` logic (prefix-keyed on `event_type`):
- `thread.started` → `self.thread_id = extract_thread_id(e)`; render `⎇ thread <id>`.
- `turn.started` → no signal; render nothing (or dim `— turn started`).
- starts with `item.` → `heartbeat = true`; `render = render_event(e)`.
- `turn.completed` → `saw_turn_completed = true`; `usage = extract_usage(e)`;
  render `— turn complete`. **No terminal signal here** (Decision 3).
- `turn.failed` → `saw_turn_failed = true`; `error_message = …`; render `✗ …`.
- `error` (top-level) → `saw_turn_failed = true`; `error_message = …`; render `✗ …`.
- unknown `type` / no type → `heartbeat = false`; `render = compact-json(e)`
  (nothing silently swallowed).

`finalize`:
```
if saw_turn_completed && !saw_turn_failed && exit_success => Success
else => Failure { message: error_message.unwrap_or("codex exec failed (no terminal turn.completed / non-zero exit)") }
```

### Layer C — IO writers (thin, tempdir-testable)

```rust
struct SignalWriter { pane_id: Option<String>, signal_dir: PathBuf }
impl SignalWriter {
    /// Mirror the hook guard: no pane id => no-op.
    fn write(&self, kind: SignalKind) -> io::Result<()>;
    // mkdir -p signal_dir; write ISO-ish timestamp to pane-<id>.<suffix>
}

/// Persist thread_id + usage for resume/provenance (Decisions 6, 7).
fn persist_run_artifacts(
    codex_dir: &Path,          // .lisa/codex
    key: &str,                 // ticket id, else "pane-<id>", else "last"
    thread_id: Option<&str>,   // -> <key>.thread
    usage: Option<&Value>,     // -> <key>.usage.json  {ticket, thread_id, success, usage}
    success: bool,
) -> io::Result<()>;

/// Read a persisted thread id for --resume (Decision 6).
fn read_thread_id(codex_dir: &Path, key: &str) -> Option<String>;
```

Timestamp: `SystemTime::now()` → epoch seconds string (content is informational;
plugin reads only mtime/existence — see research.md). Avoids a date dependency.

### Layer D — the command shell (the only untested-in-CI part)

```rust
pub struct AgentExecArgs {           // built from clap in main.rs
    pub prompt: String,
    pub resume: bool,
    pub codex_bin: String,
    pub cwd: PathBuf,
    pub bypass_sandbox: bool,
    pub codex_args: Vec<String>,
    pub signal_dir: PathBuf,
}

pub fn run_agent_exec(args: AgentExecArgs) -> Result<(), String>;
```

`run_agent_exec` sequence:
1. Read env: `pane_id = env::var("LISA_PANE_ID").ok()`,
   `ticket_id = env::var("LISA_TICKET_ID").ok()`.
2. `codex_dir = cwd/.lisa/codex`; `key = ticket_id | pane-<id> | "last"`.
3. Build the codex argv:
   `exec` (+ `resume <id|--last>` when `--resume`) `--json --skip-git-repo-check
   -C <cwd>` + sandbox flags (`-a never -s workspace-write` or
   `--dangerously-bypass-approvals-and-sandbox`) + `codex_args` + `<prompt>`.
   `--resume` resolves the thread id via `read_thread_id` (else `--last`).
4. `Command::new(codex_bin).args(argv).stdout(Stdio::piped()).stderr(inherit).spawn()`.
   Spawn failure → `Err` (e.g. codex not on PATH) with a clear message.
5. `BufReader::new(stdout).lines()`: for each line — `serde_json::from_str::<Value>`;
   on Ok call `translator.observe`; on Err render the raw line. Write `.heartbeat`
   via `SignalWriter` when `effect.heartbeat`. Print `effect.render` (or raw) to
   stdout, flushing per line (chunked pane view).
6. `status = child.wait()`. `outcome = translator.finalize(status.success())`.
7. For each `kind` in `outcome.signals()` → `signal_writer.write(kind)`.
8. `persist_run_artifacts(codex_dir, key, translator.thread_id, translator.usage,
   outcome is Success)`.
9. Print a final summary line; return `Ok(())` even on codex failure (the signal
   files, not the exit code, carry the outcome to the plugin) — **but** map an
   outright spawn/IO failure to `Err`.

### Layer E — tests (`#[cfg(test)] mod tests`, plus fixtures)

Pure-core tests (no codex, no spawn):
- `observe` on each event type → correct `StreamEffect` + captured state.
- A full recorded stream (fixture) fed line-by-line → N heartbeats, thread_id
  captured, usage captured; `finalize(true)` → `Success` → `[Stopped]`.
- Same stream, `finalize(false)` → `Failure` → `[Error, Stopped]`.
- A `turn.failed` fixture → `Failure` even with `exit_success = true`.
- Unknown/garbage line → no heartbeat, no panic, rendered raw.
- Missing `turn.completed` + success exit → `Failure` (anchor rule).

Writer tests (tempdir):
- `SignalWriter{pane_id: Some("7")}.write(Heartbeat)` → `pane-7.heartbeat` exists.
- `pane_id: None` → nothing written (degrade-safely guard).
- `persist_run_artifacts` → `<ticket>.thread` + `<ticket>.usage.json` exist and
  round-trip through `read_thread_id`.

Fixtures live in `crates/lisa-cli/tests/fixtures/`:
- `codex-success.jsonl` — thread.started, several item.*, turn.completed(+usage).
- `codex-turn-failed.jsonl` — thread.started, item.*, turn.failed.
- Loaded via `include_str!` so they're compiled in (CI needs no external files).

## Public interface delta (`main.rs`)

```rust
mod agent_exec;
// in enum Commands:
/// Run codex under lisa's signal/rendering wrapper (Codex client path)
AgentExec {
    prompt: String,
    #[arg(long)] resume: bool,
    #[arg(long, default_value = "codex")] codex_bin: String,
    #[arg(long, default_value = ".")] cwd: PathBuf,
    #[arg(long)] bypass_sandbox: bool,
    #[arg(long = "codex-arg")] codex_args: Vec<String>,
    #[arg(long, default_value = ".lisa/signals")] signal_dir: PathBuf,
}
// arm: build AgentExecArgs, call run_agent_exec, eprintln!+exit(1) on Err.
```

## Ordering of changes (feeds plan.md)

1. Layer A + B + tests (pure core) — the AC's tested heart, no IO risk.
2. Layer C writers + tempdir tests.
3. Layer D shell + `main.rs` wiring.
4. Fixtures + full-stream tests.
5. `cargo test --workspace`, `cargo build -p lisa-cli`, `just check`.

## Boundaries honoured

- No plugin/core changes (T-023-02 owns the pane launch + `.error` consumer).
- No new crate dependency.
- Signal-file format byte-compatible with the hooks (`pane-<id>.<kind>`,
  mtime/existence semantics).
- Failure path resolves T-021-01 review Open-concern #1 (`.error` + compat
  `.stopped`), documented inline.
</content>
