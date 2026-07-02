# T-024-01 Progress — Codex loop parity validation

Living record of execution against `plan.md`. Commits are the operator's call
(shared dirty tree — see `review.md` §footprint).

## Status: Implement complete

| Step | Description | Status | Notes |
|---|---|---|---|
| 1 | Baseline green | ✅ | `cargo test -p lisa-plugin` = 196 passed; WASM release build clean. Delta baseline recorded. |
| 2 | Phase-advance parity test | ✅ | `test_codex_dag_advances_all_phases_via_artifacts` — walks research→…→Done via artifacts, asserts zero signal files written. |
| 3 | `.stopped`→Review auto-complete + dep guard | ✅ | `test_codex_stopped_auto_completes_review_respecting_deps` — positive (dep-free→Done) + negative (dep-open→guard blocks, error logged). |
| 4 | Liveness (heartbeat honest / genuine hang) | ✅ | `test_codex_heartbeat_honest_then_genuine_hang_reclaimed` — 300s recent survives, 2000s silence reclaimed. |
| 5 | Prompt-failure `.error` | ✅ | `test_codex_error_signal_fails_thread_promptly` — thread removed, slot released, alert raised, file consumed. |
| 6 | Review-timeout finish-up | ✅ | `test_codex_review_timeout_finish_up_is_agent_exec_resume` — (a) path fires; (b) `SpawnCommand` carries `agent-exec --resume` + finish-up prompt. |
| 7 | Dashboard sanity (no phantom awaiting) | ✅ | `test_codex_pane_never_phantom_awaiting` — `awaiting_human` empty, `to_ui_state` `awaiting=false`. |
| 8 | Per-pane attribution | ✅ | `test_mixed_panes_error_attributed_per_pane` — `pane-2.error` fails only pane-2 thread. |
| 9 | Full-suite + wasm + clippy gate | ✅ | `cargo test --workspace` = 203 plugin (196→203, +7) / 118 cli / 218+203 core, 0 failed. WASM release build clean. Clippy clean on `lisa-plugin`. |
| 10 | Live-codex scaffold + checklist | ✅ | `validate-codex-loop.sh` (parses, executable, smoke-run OK: builds, scaffolds, sets `client="codex"`, DAG validates 2 tickets/1 ready) + `checklist.md`. |
| 11 | Findings + progress + review | ✅ | this file + `review.md`. |

## What was built

- **7 native composition tests** appended to `crates/lisa-plugin/src/lib.rs`
  `mod tests`, in a `// --- T-024-01: Codex loop parity ---` section, plus two
  small test helpers (`codex_state_with_dag`, `codex_slot`). No non-test /
  `src/**` production lines changed.
- **`validate-codex-loop.sh`** — one-command live-codex scaffold + PASS/FAIL
  runbook (T-020-05 pattern), covering the surface CI cannot reach.
- **`checklist.md`** — the live-run PASS/FAIL table + the mixed-loop scope finding.

## Deviations from plan

- **None material.** One implementation detail sharpened during Step 2: writing
  `review.md` cascades Implement→Review→Done in a single `check_artifact_advances`
  fixpoint pass (both boundaries key on `review.md`). The test walks
  research→plan boundary-by-boundary, then asserts the terminal cascade to Done —
  a fuller RDSPI proof than originally sketched. Test 2 deliberately drives the
  `.stopped`→auto-complete path *without* artifacts present so it isolates the
  signal path (not the artifact-advance path), keeping the two mechanisms tested
  independently.
- **Harness heredoc quoting fix (Step 10):** the runbook body must use a quoted
  heredoc delimiter (`<<'RUNBOOK'`) — its literal backticks (`` `codex exec` ``
  etc.) were otherwise executed as command substitutions against the PATH `lisa`.
  The dynamic run-command line is emitted via `echo` with the expanded `$DEST` /
  `$LISA` before the quoted body. Verified clean on re-run.

## Verification evidence

- `cargo test -p lisa-plugin codex` → 6 `tests::test_codex_*` green;
  `test_mixed_panes_error_attributed_per_pane` green separately. All 7 pass.
- `cargo test --workspace` → 0 failed; plugin count 196→203.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → clean.
- `cargo clippy -p lisa-plugin` → clean.
- `./validate-codex-loop.sh` → builds plugin→CLI, scaffolds `/tmp/lisa-codex-dryrun`,
  inserts `client = "codex"` under `[agent]`, drops the T-CDX-01→T-CDX-02 DAG;
  `lisa validate` in the scaffold → "2 tickets, 1 ready, DAG valid".
- Freshly-built `./target/release/lisa agent-exec --help` confirms the wrapper
  subcommand exists (the harness premise holds).

## Residual (by design)

The live `codex exec` spawn/stream/render and `--resume` re-entry are **not**
CI-runnable (no `codex` binary in CI — the reality T-023-01/02 documented). They
are the `validate-codex-loop.sh` + `checklist.md` manual remainder, gated on codex
availability. This is the same split T-023-01 uses and the ticket Notes endorse.
</content>
