# T-027-02 Review — Cost/Token Capture Per Adapter

Handoff for a human reviewer. What changed, how it was verified, what to watch.

## What the ticket asked, and how it's met

| Acceptance criterion | Status |
|---|---|
| Findings write-up (Q1–Q3) on the current Claude Code version | ✅ research.md §5, design.md Q1–Q3 |
| Claude runs populate ledger tokens where obtainable, else `null` (never fabricated) | ✅ `capture-usage` + generalized `read_usage`; null-safe paths verified |
| Codex usage capture verified end-to-end into the ledger | ◑ proven natively (test); live `codex exec` run not run this session (open item) |
| Ledger schema docs updated with per-provider fidelity caveats | ✅ provenance-ledger.md rewrite |
| Respect write-after; heartbeat stays trivial | ✅ Stop-hook capture (turn-boundary, not per-tool); heartbeat untouched (test-guarded) |

## Files changed

**New**
- `crates/lisa-cli/src/capture_usage.rs` — `lisa capture-usage`: reads the Stop
  payload from stdin, sums transcript `message.usage`, writes
  `.lisa/claude/<key>.usage.json`. Pure, defensive, 5 unit tests.

**Modified**
- `crates/lisa-cli/src/main.rs` — `mod capture_usage`, `Commands::CaptureUsage`,
  best-effort dispatch.
- `crates/lisa-cli/src/templates.rs` — `ON_STOP_HOOK` forwards stdin to
  `"${LISA_BIN:-lisa}" capture-usage`; 2 new hook-shape tests.
- `crates/lisa-plugin/src/lib.rs` — `claude_dir` field + `load()` init;
  `read_codex_usage` → `read_usage` (dir-by-client); `build_claude_command` gains
  `lisa_bin` → `LISA_BIN`; new Claude-usage test; fixed a pre-existing
  uncommitted-test defect (see below).
- `crates/lisa-plugin/src/adapter.rs` — `ClaudeCodeAdapter` carries `lisa_bin`;
  `adapter_for_route` forwards it; new launch-line test; signature updates.
- `crates/lisa-core/src/provenance.rs` — doc comments only (module + two fields).
- `docs/knowledge/provenance-ledger.md` — fidelity section + versioning note.

## Design decisions worth a reviewer's eye

1. **Stop hook, not a new hook, not in-plugin parsing.** The transcript lives at
   `~/.claude/projects/...`, outside the WASI `/host` mount, so the plugin cannot
   read it; POSIX-sh JSON summing is too fragile. A native subcommand invoked by
   the already-firing Stop hook mirrors the Codex `agent-exec` capture exactly and
   keeps the heartbeat trivial. (design.md Q1)
2. **`tokens_in` = fresh + cache-creation + cache-read.** Total input-side count,
   the quantity comparable to Codex's single `input_tokens` and the basis a
   downstream cost estimate needs. The per-class split is not persisted in schema
   v1 (additive-nullable later if a consumer needs it).
3. **`cost_usd` stays `null` for Claude.** No dependable dollar field in the
   transcript; a derived cost would bake a soon-stale number into append-only
   history. Cost is a reader concern (tokens × pricing). Never fabricated.
4. **No schema bump.** Record shape is unchanged; only previously-`null` Claude
   token fields become populated. `SCHEMA_VERSION` stays 1.
5. **Zero-regression launch line.** `LISA_BIN` is omitted when no `lisa_bin` is
   threaded, so the Claude launch line is byte-for-byte the pre-capture command;
   the hook then falls back to a PATH `lisa`.

## Test coverage

- **capture_usage** (unit, lisa-cli): multi-class summing; malformed/non-assistant
  lines skipped; empty/no-assistant → zero; missing fields → 0 not crash;
  artifact shape ↔ `provenance::extract_usage` (cross-crate contract).
- **templates** (unit): Stop hook still writes `.stopped` + now captures;
  heartbeat stays trivial (no stdin, no `capture-usage`).
- **plugin** (native): Claude usage artifact flows into a populated record;
  Claude-with-no-artifact → null; Codex path unchanged; launch line emits
  `LISA_BIN` when set and omits it otherwise.
- **e2e (manual, this session):** real transcript + Stop payload →
  `input_tokens: 15212, output_tokens: 740` (hand-verified); three
  never-fabricate paths → no artifact, exit 0.
- Full workspace: 595 tests green; WASM plugin builds.

## Open concerns / TODO for human attention

1. **Live Codex e2e not run.** AC asks for a real `codex exec` run into the
   ledger. The artifact→record plumbing is proven by native tests, but a live run
   (needs the `codex` CLI + a zellij loop) was not exercised here. Suggest running
   `docs/active/work/T-024-01/validate-codex-loop.sh` (or a loop) and confirming a
   Codex record carries tokens. Low risk — the read path is identical for both
   providers and unit-covered.
2. **Live Claude session e2e not run.** Same caveat: `capture-usage` is verified
   against a synthesized transcript matching the documented Claude Code JSONL
   shape, and against a live Stop payload structure, but not driven by an actual
   `claude` session in a pane. If Claude Code's transcript schema drifts, the
   summer degrades to an under-count/null (defensive), never a crash or a
   fabricated value — but a live smoke test before relying on the numbers is wise.
3. **`LISA_BIN` reachability.** When neither `LISA_BIN` nor a PATH `lisa` is
   present, capture silently no-ops (tokens null). This is intended (never fail a
   session), but means a misconfigured install yields null Claude tokens
   silently. `lisa doctor` could later assert `LISA_BIN`/PATH reachability.
4. **Pre-existing test fixed (not a regression I introduced).**
   `provenance_emitted_on_error_signal` (uncommitted T-027-01 work) asserted a
   `codex` route on a thread it built with the default Claude client — it was
   already failing in the working tree. Fixed by setting `thread.client = Codex`
   to match the real spawn path (lib.rs:687) and the sibling tests. Flagging in
   case the T-027-01 author expected different semantics.
5. **Commits deferred.** Per harness guidance (on `main`, with substantial
   unrelated uncommitted changes already in the tree) I did **not** create git
   commits. plan.md lists the intended atomic commit boundaries; a human should
   stage `crates/lisa-cli/src/{capture_usage.rs,main.rs,templates.rs}`,
   `crates/lisa-plugin/src/{lib.rs,adapter.rs}`,
   `crates/lisa-core/src/provenance.rs`, and the docs together (or per plan.md)
   on an appropriate branch.
