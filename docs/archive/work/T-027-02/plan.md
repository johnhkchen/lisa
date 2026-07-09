# T-027-02 Plan — Implementation Steps

Ordered, independently-verifiable steps. Each ends at a green
`cargo test --workspace` and is committable atomically.

## Step 1 — `capture_usage` subcommand (native, unit-tested)

Files: new `crates/lisa-cli/src/capture_usage.rs`; `main.rs` (`mod` + `Commands`
variant + dispatch).

Work:
- `sum_transcript_usage(jsonl)` — iterate lines, `serde_json::from_str` each,
  skip non-parseable/non-assistant lines, read `message.usage`, accumulate
  `input_tokens + cache_creation_input_tokens + cache_read_input_tokens` into
  `input_tokens`, `output_tokens` into `output_tokens`.
- `usage_artifact(key, u)` → `{ key, usage: { input_tokens, output_tokens } }`.
- `run_capture_usage(cwd)` — read stdin, parse `StopPayload`, resolve key from
  env, read transcript, sum, `fs::write` the artifact under `.lisa/claude/`.
  Every absence path returns `Ok(())` writing nothing.

Tests (in-module):
- `sum_transcript_usage` over a fixture with 3 assistant messages sums all input
  classes + output correctly.
- Non-assistant / malformed lines are skipped, not fatal.
- Empty / no-assistant transcript → zeros.
- `usage_artifact` shape matches what `provenance::extract_usage` reads
  (round-trip: build artifact → `extract_usage(artifact["usage"])` → expected).

Verify: `cargo test -p lisa-cli capture_usage`.
Commit: "T-027-02: add lisa capture-usage (Claude transcript token capture)".

## Step 2 — Wire the Stop hook to capture

Files: `templates.rs` (`ON_STOP_HOOK`).

Work: append the `in=$(cat); printf … | "${LISA_BIN:-lisa}" capture-usage` line
after the `.stopped` write. Keep it `2>/dev/null || true` so a missing binary is
inert.

Tests: extend the existing templates test (if any asserts hook contents) to
confirm the Stop hook still writes `.stopped` and now references
`capture-usage`. Assert the heartbeat hook is unchanged (still no stdin read).

Verify: `cargo test -p lisa-cli`.
Commit: "T-027-02: forward Stop-hook payload to capture-usage".

## Step 3 — Plugin reader: `claude_dir` + generalized `read_usage`

Files: `lib.rs`.

Work:
- Add `claude_dir: PathBuf` field; init in `load()`
  (`host.join(".lisa/claude")`); add to struct initializers / test constructors
  (search for existing `codex_dir:` initializers, add a sibling).
- Drop the `client != Codex` guard in `read_codex_usage`; select dir by client;
  rename to `read_usage`. Update the one call site in `emit_provenance`.

Tests (native, in lib.rs test module — mirror the existing Codex usage test):
- A Claude run with a `.lisa/claude/<ticket>.usage.json` present →
  record carries the tokens.
- A Claude run with no artifact → tokens `null` (never fabricated).
- Codex path still reads `.lisa/codex/...` (no regression).

Verify: `cargo test -p lisa-plugin`.
Commit: "T-027-02: plugin reads Claude usage artifact into the ledger".

## Step 4 — Thread `LISA_BIN` into the Claude launch line

Files: `lib.rs` (`build_claude_command`), `adapter.rs` (`ClaudeCodeAdapter`,
`adapter_for_route`).

Work:
- `build_claude_command(..., lisa_bin: Option<&str>)` prepends
  `LISA_BIN=<bin> ` only when `Some(non-empty)`.
- `ClaudeCodeAdapter { model, lisa_bin }`; `new(model, lisa_bin)`; pass through
  in `launch_command`. `adapter_for_route` forwards `lisa_bin` to the Claude arm.
- Update existing adapter tests that call `ClaudeCodeAdapter::new`/`default` and
  those asserting the exact launch line (native_launch_matches_free_fn etc.) to
  the new signature; add a test that `Some("/abs/lisa")` yields a `LISA_BIN=`
  prefix and `None` yields the byte-for-byte pre-routing line.

Verify: `cargo test -p lisa-plugin` + `cargo build -p lisa-plugin --target
wasm32-wasip1 --release` (WASM still compiles).
Commit: "T-027-02: export LISA_BIN to Claude sessions for capture-usage".

## Step 5 — Docs + module comments

Files: `docs/knowledge/provenance-ledger.md`, `provenance.rs` doc comment,
`agent_exec.rs`/`lib.rs` doc mentions of "Claude deferred to T-027-02".

Work: rewrite "Nullability & fidelity" with per-provider caveats (design Q3);
note Claude tokens now populate; keep the `cost_usd` null-for-Claude rationale.

Verify: `cargo test --workspace` (doc-comment code, if any).
Commit: "T-027-02: document per-provider cost/token fidelity".

## Step 6 — End-to-end verification (AC: Codex + Claude real-run)

Not a code change — a validation pass recorded in progress.md / review.md.

- **Codex**: synthesize a `.lisa/codex/<t>.usage.json` (or reuse a wrapper run)
  and confirm `emit_provenance` writes tokens into `provenance.jsonl`. Covered by
  the Step 3 native test at minimum; note whether a real `codex exec` run was
  exercised or simulated.
- **Claude**: feed a captured Stop payload + a real transcript JSONL through
  `lisa capture-usage`, confirm the artifact, then confirm the plugin reader
  turns it into a populated record. Covered by Step 1 + Step 3 tests; note the
  fidelity of the check (unit vs. live session) honestly in review.md.

## Testing strategy summary

| Surface | Test type | Where |
|---|---|---|
| transcript summing | unit | capture_usage.rs |
| artifact ↔ extract_usage shape | unit | capture_usage.rs |
| Stop hook body | string assertion | templates test |
| plugin reader (Claude present/absent) | native | lib.rs tests |
| launch line LISA_BIN on/off | native | adapter.rs tests |
| WASM still compiles | build | `just check` |

Risk notes: the transcript JSON shape is external (Claude Code) and could drift;
`sum_transcript_usage` reads defensively (missing fields → 0, bad line → skip) so
drift degrades to under-count/null, never a crash or a fabricated number.
