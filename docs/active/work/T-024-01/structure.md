# T-024-01 Structure — Codex loop parity validation

The blueprint: exact files, test names, and shapes. No production `src/**` changes.

## Files

| File | Change | Purpose |
|---|---|---|
| `crates/lisa-plugin/src/lib.rs` | **modified** (tests only) | New `// --- T-024-01: Codex loop parity ---` section in `#[cfg(test)] mod tests`; 7 tests. No non-test lines. |
| `docs/active/work/T-024-01/validate-codex-loop.sh` | **created** | One-command live-codex scaffold + PASS/FAIL runbook. |
| `docs/active/work/T-024-01/checklist.md` | **created** | Live-run PASS/FAIL table for after-the-fact triage. |
| `docs/active/work/T-024-01/*.md` | **created** | RDSPI artifacts (this set). |

No `lisa-core` / `ui.rs` / adapter / wrapper changes. No new dependencies
(`tempfile`, `serde_json` already dev-present). Test additions are append-only at
the end of the existing `mod tests`, disjoint from sibling threads' edits.

## Test module additions (`lib.rs`)

All follow the house idiom: build `State` with a `PluginConfig { client:
AgentClient::Codex, .. }`, tempdir `signal_dir` / `ticket_dir`, push `AgentSlot`
literals, insert `Thread`s, drive one consumer, assert on `threads` /
`agent_slots` / `activity_log` / `error_alerts` / `finish_up_sent` / the ticket
file on disk. Helper reuse: `scan_tickets` + `Dag::from_tickets` for real tickets.

### 1. `test_codex_dag_advances_all_phases_via_artifacts`
- Tickets on disk: `T-CDX-01` (phase `research`, no deps), `T-CDX-02`
  (`depends_on: [T-CDX-01]`). `Dag::from_tickets`. `config.client = Codex`,
  `work_dir` = tempdir.
- Insert a Running `Thread` for `T-CDX-01` at `Phase::Research`.
- Loop: for each phase research→…→implement, write the phase artifact
  (`research.md`…`review.md`, Implement keyed on `review.md`) into
  `work_dir/T-CDX-01/`, call `check_artifact_advances`, assert the thread's
  `current_phase` advanced to the expected next phase and the ticket file's
  `phase:` field matches. End state: `review`. **No signal files written.**
- Asserts: parity phase-advance rides artifact presence alone.

### 2. `test_codex_stopped_auto_completes_review_respecting_deps`
- Same 2-ticket DAG. `T-CDX-01` set to Review on disk; dep-free ⇒ eligible.
- Codex slot on pane 1 (`transition_state: Idle`, `has_session: true`),
  Running Review thread. Call `handle_stopped_signal(1)`.
- Assert: thread removed, slot released, ticket file `phase: done` + `status: done`.
- Negative: put `T-CDX-02` (dep on `T-CDX-01`, not Done) in Review on pane 2,
  call `auto_complete_review("T-CDX-02", 2)` directly; assert an
  `ActivityEvent::Error` containing "dependencies are not all done", thread still
  present, ticket file still `phase: review`.

### 3. `test_codex_heartbeat_honest_then_genuine_hang_reclaimed`
- `config.stuck_threshold_secs = 600` ⇒ hard silence 1200s. `session_timeout`
  irrelevant (drive `detect_stale_threads` directly, as `test_detect_stale_threads`
  does).
- Case honest: Running thread, `last_activity = now - 300s` (< 1200) ⇒
  `detect_stale_threads` leaves it (thread present, slot bound).
- Case hang: set `last_activity = now - 2000s` (> 1200) ⇒ `detect_stale_threads`
  removes the thread and releases the slot.
- Two `State`s or one reused; mirror `test_detect_stale_threads` (3843) +
  `test_stale_thread_not_stale_yet` (3903).

### 4. `test_codex_error_signal_fails_thread_promptly`
- tempdir `signal_dir` with `pane-1.error`. Codex config. Running thread on pane 1,
  bound slot. Call `check_error_signals`.
- Assert: file consumed, thread removed, slot `ticket_id = None` + `has_session`
  retained, one `error_alerts` entry `("T-CDX-01", 1)`. Mirrors
  `test_check_error_signals_fails_running_thread` (7717) under Codex config.

### 5. `test_codex_review_timeout_finish_up_is_agent_exec_resume`
- Part (a): `config { client: Codex, lisa_bin: Some("/abs/lisa"),
  review_timeout_secs: 10, wind_down_secs: 180 }`. Running Review thread on pane 1,
  `last_phase_change`/`last_activity` = now-200s (past both). A DAG containing the
  ticket (so `resolve_adapter_or_native` finds it). Call `check_review_timeouts`;
  assert `finish_up_sent` contains the ticket + a `FinishUpPromptSent` event.
- Part (b): call `resolve_adapter_or_native(dag.get_ticket(id), Codex,
  Some("/abs/lisa")).follow_up(&FollowUpContext{..})`; assert it is
  `FollowUp::SpawnCommand(cmd)` with `cmd.contains("agent-exec --resume")` and
  `cmd.contains(&finish_up_prompt(..))`. (Delivery via `send_line_to_pane` is
  exercised in (a); the string shape is asserted in (b).)

### 6. `test_codex_pane_never_phantom_awaiting`
- tempdir `signal_dir` containing only `pane-1.heartbeat` + `pane-1.stopped`
  (Codex's whole vocabulary sans `.error`). Codex config, `initialized = true`.
- Insert a Running thread on pane 1. Call `check_awaiting_signals` then
  `check_heartbeat_signals`.
- Assert: `awaiting_human` empty, `is_pane_awaiting(1)` false, and in
  `to_ui_state()` the thread's UI entry has `awaiting == false`.

### 7. `test_mixed_panes_error_attributed_per_pane`
- `signal_dir` with only `pane-2.error`. Two Running threads: `T-CDX-01` on pane 1,
  `T-CDX-02` on pane 2, both with bound slots. Call `check_error_signals`.
- Assert: `T-CDX-02` removed + pane-2 slot released; `T-CDX-01` still Running +
  pane-1 slot still bound; exactly one `error_alerts` entry for pane 2. Proves
  per-`pane-<id>` attribution (the guarantee under the "mixed loop" AC).

## `validate-codex-loop.sh` shape

Sections, mirroring `setup-gate-harness.sh` (T-020-05):
1. **Resolve repo root**, `set -euo pipefail`.
2. **Build**: `cargo build -p lisa-plugin --target wasm32-wasip1 --release` →
   `touch crates/lisa-cli/.../lisa.wasm` (defeat stale-embed) →
   `cargo build -p lisa-cli --release`.
3. **Scaffold** `/tmp/lisa-codex-dryrun`: `rm -rf` + `git init`; real
   `lisa init`; write `client = "codex"` into `.lisa.toml`; drop `T-CDX-01`
   (research) and `T-CDX-02` (depends_on `[T-CDX-01]`).
4. **Preflight**: warn (not fail) if `codex` is not on PATH — the run needs it, the
   scaffold does not.
5. **Print** the run command (`cd … && lisa loop`) and the PASS/FAIL checklist
   (each live AC bullet → observable file/state). Cleanup left to the next run's
   `rm -rf` (re-inspectable artifacts, T-020-05 rationale).

## `checklist.md` shape

One table: AC bullet | how to observe | PASS sign | FAIL sign, covering the live
remainder (spawn/stream/render, real `.stopped`→auto-complete, `--resume`
re-entry, no phantom awaiting on the live dashboard, mixed via two loops).

## Ordering

Docs (this set) → tests appended to `lib.rs` → `cargo test --workspace` +
`cargo build -p lisa-plugin --target wasm32-wasip1 --release` green → harness
script + checklist → `progress.md` → `review.md`. Each test is independently
compilable/assertable; no inter-test ordering.

## Explicit non-goals

- No product `src/**` change (validation, not feature).
- No PTY / live-codex automation (rejected Option A).
- No per-pane routing (S-026) — mixed-in-one-loop is a documented finding.
- No `git commit` (shared dirty tree; footprint documented in `review.md`).
</content>
