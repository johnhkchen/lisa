# T-027-02 Structure — File-Level Blueprint

The shape of the code implementing design.md. No logic bodies — interfaces,
boundaries, and ordering.

## New files

### `crates/lisa-cli/src/capture_usage.rs`

The native Claude usage capture, the analogue of `agent_exec.rs`'s
`persist_run_artifacts`. Pure, host-side, testable.

Public entry:
```rust
/// Read the Stop-hook payload from `stdin` (JSON with `transcript_path`), sum the
/// transcript's per-message usage, and write `.lisa/claude/<key>.usage.json`.
/// Best-effort: any missing input exits Ok(()) writing nothing (never fabricates).
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()>;
```

Internal, unit-tested seams (kept free of I/O so tests pass strings):
```rust
/// The Stop-hook stdin shape we consume. Only `transcript_path` is required.
#[derive(Deserialize)] struct StopPayload { transcript_path: Option<String>, .. }

/// Summed, provider-native totals in the shape `extract_usage` already reads.
struct ClaudeUsage { input_tokens: u64, output_tokens: u64 }

/// Sum `message.usage` across every assistant line of a transcript JSONL.
/// tokens_in = input + cache_creation + cache_read; tokens_out = output.
fn sum_transcript_usage(jsonl: &str) -> ClaudeUsage;

/// Build the artifact JSON `{ key, usage: { input_tokens, output_tokens } }`
/// — same nested-`usage` shape the plugin's reader expects.
fn usage_artifact(key: &str, u: &ClaudeUsage) -> serde_json::Value;
```

Key derivation mirrors `agent_exec.rs:493-496`:
`key = LISA_TICKET_ID → pane-<LISA_PANE_ID> → "last"`. Artifact path:
`cwd.join(".lisa/claude/<key>.usage.json")`.

Absence rules (all → write nothing, exit Ok):
- no `transcript_path`, unreadable/absent transcript, malformed JSON lines
  (skip the line, keep summing), zero assistant messages.

## Modified files

### `crates/lisa-cli/src/main.rs`
- Add `mod capture_usage;` (with the other `mod` lines).
- Add a `Commands::CaptureUsage { cwd }` variant (near `AgentExec`, ~line 77):
  ```rust
  /// Capture Claude session token usage from a Stop-hook payload on stdin.
  CaptureUsage {
      #[arg(long, default_value = ".")]
      cwd: PathBuf,
  },
  ```
- Dispatch in `main()` (~line 129): `Commands::CaptureUsage { cwd } =>
  { let _ = capture_usage::run_capture_usage(&cwd); }` — best-effort, errors
  swallowed so a hook never fails the session.

### `crates/lisa-cli/src/templates.rs`
- Extend `ON_STOP_HOOK` (28-38): after writing `.stopped`, read stdin once and
  forward to capture. New body (still POSIX `sh`, still trivial):
  ```sh
  if [ -n "$LISA_PANE_ID" ]; then
      echo "$(date -u ...)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
  fi
  in=$(cat)
  printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage 2>/dev/null || true
  ```
  Note: Stop fires per turn, not per tool call — the heartbeat hook is untouched.
- No change to `settings_local_json()` (the Stop hook entry already points at
  `on-stop.sh`; only the script body changes).

### `crates/lisa-plugin/src/lib.rs`
- **New field** `claude_dir: PathBuf` on the plugin struct (beside `codex_dir`,
  ~line 240), set in `load()` to `host.join(".lisa/claude")` (beside line 2827).
  Default empty in the struct initializer / tests.
- **`build_claude_command`** (66-86): add a `lisa_bin: Option<&str>` param and
  prepend `LISA_BIN=<bin>` to the env prefix (empty/None → omit, preserving the
  byte-for-byte pre-routing line when no bin is threaded). Mirrors the Codex
  adapter's `unwrap_or("lisa")` fallback.
- **`ClaudeCodeAdapter`** (adapter.rs:155-211): carry `lisa_bin: Option<String>`
  (like `CodexAdapter`), forward it into `build_claude_command`. `new(model,
  lisa_bin)`; `adapter_for_route` (adapter.rs:327-333) passes `lisa_bin` to the
  Claude arm too.
- **Generalize the reader**: rename `read_codex_usage` →
  `read_usage(client, ticket_id)` (or keep the name, drop the Codex guard). Pick
  the dir by client:
  ```rust
  let dir = match client {
      AgentClient::Codex => &self.codex_dir,
      AgentClient::Claude => &self.claude_dir,
  };
  ```
  The rest (read file, parse, `extract_usage(usage)`) is unchanged — both
  providers write the same `{ ..., usage: {...} }` shape. `emit_provenance`
  (1447) calls it unchanged.

### `crates/lisa-core/src/provenance.rs`
- No code change required — `extract_usage` already reads `input_tokens` /
  `output_tokens`. Update the module doc comment (26-31 / 95) to note Claude now
  populates via `.lisa/claude/<ticket>.usage.json`.

### `docs/knowledge/provenance-ledger.md`
- Rewrite the "Nullability & fidelity" section: Claude now populates
  tokens; add per-provider fidelity caveats (Claude `tokens_in` includes
  cache-read + cache-creation; `cost_usd` stays `null` for Claude, derive from
  tokens + pricing downstream; Codex counts are provider-native and provisional;
  the two are not commensurable at the token level — segment by `actual.method`).

## Ordering of changes

1. `capture_usage.rs` + main.rs wiring (self-contained, unit-testable first).
2. `templates.rs` Stop-hook body (depends on the subcommand existing).
3. Plugin: `claude_dir` field + `load()` + generalized reader.
4. Plugin/adapter: `LISA_BIN` threading through `build_claude_command`.
5. Docs (`provenance-ledger.md`, provenance.rs doc comment).

Steps 1 and 3 are independently committable and testable; 2 and 4 are the
wiring that makes the end-to-end path live. Each has native tests (plan.md).

## Interfaces held stable

- `ProvenanceRecord` schema and `SCHEMA_VERSION` (still 1) — unchanged.
- `provenance::append_record` / `extract_usage` signatures — unchanged.
- The Codex path — unchanged (its artifact + reader spine are reused verbatim).
- The Claude launch line when no `lisa_bin` is threaded — byte-for-byte identical
  (LISA_BIN omitted), preserving the zero-regression anchor-leg guarantee.
