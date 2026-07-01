# T-024-01 Plan — Codex loop parity validation

Ordered, independently-verifiable steps. Testing strategy inline. No production
`src/**` changes; commits are the operator's call (shared dirty tree — see Review).

## Testing strategy

- **Automated (this ticket's deliverable):** 7 native composition tests in
  `lib.rs` `mod tests`, each mapping to one AC check, run by `cargo test
  --workspace`. They drive the real scheduler consumers under `client = Codex`
  with Codex-shaped signal files / artifacts and assert on scheduler state and the
  on-disk ticket files — the same vehicle every existing consumer test uses.
- **Verification gates per step:** `cargo test -p lisa-plugin` green, then
  `cargo test --workspace` green, then the WASM release build clean (the plugin is
  shipped as wasm; a test-only change must not break the wasm build), then clippy
  clean on `lisa-plugin`.
- **Manual (intrinsic remainder):** the `validate-codex-loop.sh` scaffold + the
  PASS/FAIL `checklist.md`, exercised by an operator with a real `codex` binary.
  Not asserted in CI by design (the live spawn/stream is the untestable surface).

## Steps

### Step 1 — Baseline green
Run `cargo test -p lisa-plugin` and `cargo build -p lisa-plugin --target
wasm32-wasip1 --release` on the current (shared, dirty) tree to confirm the
starting point compiles and passes before adding tests. Record counts.
- *Verify:* both succeed; note the plugin test count as the delta baseline.

### Step 2 — Phase-advance parity test (AC: phases advance on artifacts)
Add `test_codex_dag_advances_all_phases_via_artifacts`. 2-ticket DAG on disk,
Codex config, Running thread at Research; write each artifact and drive
`check_artifact_advances`, asserting the phase walks to Review with no signals.
- *Verify:* `cargo test -p lisa-plugin test_codex_dag_advances` passes.

### Step 3 — `.stopped`→Review auto-complete + dep guard (AC: stopped→review, deps)
Add `test_codex_stopped_auto_completes_review_respecting_deps`: positive
(dep-free Review ticket, Idle Codex slot ⇒ Done on disk) and negative (dependent
ticket, dep not Done ⇒ guard blocks, error logged, stays Review).
- *Verify:* test passes; both branches asserted.

### Step 4 — Liveness test (AC: heartbeat honest; genuine hang reclaimed)
Add `test_codex_heartbeat_honest_then_genuine_hang_reclaimed`: recent activity ⇒
survives `detect_stale_threads`; silence past 2×`stuck_threshold_secs` ⇒ reclaimed.
- *Verify:* test passes.

### Step 5 — Prompt-failure test (AC: `.error` fails thread, releases slot)
Add `test_codex_error_signal_fails_thread_promptly`: Codex `pane-1.error` ⇒ thread
removed, slot released (session kept), `error_alerts` entry.
- *Verify:* test passes.

### Step 6 — Finish-up test (AC: review-timeout finish-up via `agent-exec --resume`)
Add `test_codex_review_timeout_finish_up_is_agent_exec_resume`: part (a) drive
`check_review_timeouts` for a quiet timed-out Codex Review thread ⇒ `finish_up_sent`
+ event; part (b) assert the resolved Codex `follow_up` is a `SpawnCommand` carrying
`agent-exec --resume` + the finish-up prompt.
- *Verify:* test passes; both parts asserted.

### Step 7 — Dashboard-sanity test (AC: no phantom awaiting)
Add `test_codex_pane_never_phantom_awaiting`: only Codex signals present ⇒
`awaiting_human` empty, `is_pane_awaiting` false, `to_ui_state` `awaiting=false`.
- *Verify:* test passes.

### Step 8 — Attribution test (AC: mixed loop, signals per pane)
Add `test_mixed_panes_error_attributed_per_pane`: `pane-2.error` fails only the
pane-2 thread, pane-1 untouched.
- *Verify:* test passes.

### Step 9 — Full-suite + wasm + clippy gate
`cargo test --workspace`, `cargo build -p lisa-plugin --target wasm32-wasip1
--release`, `cargo clippy -p lisa-plugin`. All green/clean.
- *Verify:* workspace suite passes (baseline + 7); wasm builds; no new clippy warnings.

### Step 10 — Live-codex scaffold + checklist
Write `validate-codex-loop.sh` (build→scaffold→print runbook) and `checklist.md`.
`bash -n validate-codex-loop.sh` parses; `chmod +x`.
- *Verify:* script parses and is executable; checklist covers every live AC bullet.

### Step 11 — Findings + progress + review
`progress.md` (step-by-step actuals, deviations) and `review.md` (footprint, test
coverage, the residual manual verdict, and any contract-violation bugs to file
against S-025, plus the mixed-in-one-loop → S-026 scope finding).
- *Verify:* review.md maps every AC to automated-pass or manual-runbook, and
  states the honest live-codex residual.

## What would make this ticket file a bug (blocking S-025)

If any Step 2–8 test reveals a consumer *misreading* Codex semantics — e.g.
`handle_stopped_signal` mis-firing on a pre-Review `.stopped`, `check_awaiting_signals`
inventing an awaiting state, or `check_review_timeouts` typing a Claude-shaped
prompt for a Codex pane — that is a contract violation, filed as a bug ticket and
noted in `review.md` as blocking S-025's "documented toggle is safe" claim. The
expectation from research is that all pass (the mechanisms are already correct);
the value is turning "should be correct" into "is, and here is the executing proof."

## Rollback / risk

Tests are append-only and isolated; if any fails to compile against the shared
tree, it is fixed in place (no product code depends on them). No rollback of
product behaviour is possible or needed — nothing in `src/**` changes.
</content>
