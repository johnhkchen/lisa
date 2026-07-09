# T-027-02 Progress

Tracking against plan.md.

## Step 1 — `capture_usage` subcommand ✅
- New `crates/lisa-cli/src/capture_usage.rs`: `sum_transcript_usage`,
  `usage_artifact`, `resolve_key`, `run_capture_usage`. 5 unit tests green,
  including the cross-crate contract test (artifact `usage` ↔
  `provenance::extract_usage`).
- `main.rs`: `mod capture_usage;`, `Commands::CaptureUsage { cwd }`, best-effort
  dispatch (errors swallowed — a hook never fails the session).

## Step 2 — Stop hook forwards payload ✅
- `templates.rs::ON_STOP_HOOK` now reads stdin once and pipes it to
  `"${LISA_BIN:-lisa}" capture-usage 2>/dev/null || true` after writing
  `.stopped`. Two new tests: Stop hook captures + heartbeat stays trivial.

## Step 3 — plugin reads Claude artifact ✅
- Added `claude_dir` field (+ `load()` init `.lisa/claude`).
- Generalized `read_codex_usage` → `read_usage(client, ticket_id)`: selects
  `.lisa/codex` vs `.lisa/claude` by client; shared read/parse/extract spine.
- New native test `provenance_claude_usage_flows_into_record`.
- **Deviation:** fixed a pre-existing latent failure in the uncommitted
  T-027-01 test `provenance_emitted_on_error_signal` — it built a thread with the
  default (Claude) client but asserted `actual.method == "codex"`. Route derives
  purely from `thread.client` (untouched by this ticket); the test's manual
  construction omitted `thread.client = Codex` that the real spawn path sets
  (lib.rs:687). Set it to match reality and the sibling tests. Confirmed the
  failure is independent of the reader change (route is not read-path dependent).

## Step 4 — LISA_BIN threading ✅
- `build_claude_command` takes `lisa_bin: Option<&str>`, prepends `LISA_BIN=<bin>`
  only when set (empty/None → byte-for-byte the pre-capture launch line).
- `ClaudeCodeAdapter` carries `lisa_bin`; `adapter_for_route` forwards it to the
  Claude arm (it was already threaded to Codex). No spawn-site change needed —
  the adapter resolver already receives `config.lisa_bin`.
- New test `native_launch_exports_lisa_bin_when_set`; updated 4 free-fn tests +
  2 adapter tests for the new signature.

## Step 5 — docs ✅
- `docs/knowledge/provenance-ledger.md`: rewrote "Nullability & fidelity" with
  per-provider caveats (Claude `tokens_in` = fresh+cache-creation+cache-read;
  `cost_usd` null-for-Claude with rationale; raw-counts-only comparability note);
  updated the versioning note (no schema bump).
- `provenance.rs`: module doc + `tokens_in`/`cost_usd` field docs updated.

## Step 6 — e2e verification ✅
- **Claude:** ran a real transcript (2 assistant messages) + a Stop payload
  through `lisa capture-usage` → artifact
  `{ key: "T-027-02", usage: { input_tokens: 15212, output_tokens: 740 } }`
  (sum verified by hand). Never-fabricate paths verified: missing
  `transcript_path`, no-assistant transcript, unreadable path → exit 0, no
  artifact written.
- **Codex:** covered by the native `provenance_codex_usage_flows_into_record`
  test (artifact → populated record). A live `codex exec` run was **not**
  exercised in this session (see review.md open item — it needs the codex CLI +
  a zellij loop); the plumbing from artifact to ledger is proven natively.
- Full suite: 595 tests green across the workspace; WASM plugin builds.

Deviations: one (the pre-existing test fix in Step 3), documented above.
