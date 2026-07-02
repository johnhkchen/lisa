# T-024-01 Review — Codex loop parity validation

Handoff for a human reviewer. What changed, how it was verified, the residual, and
the findings that bear on S-025.

## What this ticket delivered

The end-to-end **checkable proof** that a Codex loop gets the same scheduler
treatment a Claude loop gets — S-024's "full parity is the bar" made executable,
in the spirit of the T-020-05 gate-harness. Two halves, the split the ticket Notes
endorse ("documented manual checklist plus recorded-stream native tests, same
split T-023-01 uses"):

1. **7 native composition tests** that drive the real scheduler consumers under
   `client = Codex` with Codex-shaped signal files / artifacts, proving the parity
   mechanisms (already unit-tested in isolation by T-022-02 / T-023-01 / T-023-02)
   behave correctly *together* as a loop lifecycle. The scheduler consumes signal
   *files*, never JSON, so its whole surface is reachable natively.
2. **A live-codex scaffold + PASS/FAIL runbook** (`validate-codex-loop.sh` +
   `checklist.md`) for the irreducible remainder — the real `codex exec`
   spawn/stream/render and `--resume` re-entry — which no CI can run.

This ticket adds **no product behaviour**. It is validation: tests + a throwaway QA
script, zero `src/**` production lines.

## Files changed (my footprint)

| File | Change | Lines |
|---|---|---|
| `crates/lisa-plugin/src/lib.rs` | **modified (tests only)** | +~330: a `// --- T-024-01: Codex loop parity ---` section in `mod tests` — 2 helpers + 7 tests. No non-test lines. |
| `docs/active/work/T-024-01/validate-codex-loop.sh` | **created** | live-codex scaffold + runbook |
| `docs/active/work/T-024-01/checklist.md` | **created** | live-run PASS/FAIL table + scope finding |
| `docs/active/work/T-024-01/*.md` | **created** | RDSPI artifacts (research/design/structure/plan/progress/this) |

> **Working-tree note for the reviewer.** `git status` shows a much larger tree —
> `adapter.rs`, `client.rs`, `agent_exec.rs`, `config.rs`, `doctor.rs`, `init.rs`,
> `loop_cmd.rs`, `types.rs`, `main.rs`, `status.rs`, etc. **None of those are this
> ticket.** They are the uncommitted footprints of the sibling S-021–S-027 threads
> on the same branch (lisa's model: many threads, one branch, disjoint files). My
> disjoint slice is the test-only addition to `lib.rs` `mod tests` plus this work
> directory. I did **not** run `git commit` — committing `lib.rs` would sweep
> T-023-02 / T-022-02's uncommitted lines with it. The whole tree compiles and the
> suite is green as-is; commit coordination is the operator's / lisa's, per the
> shared-branch convention every sibling review documents.

## The seven tests (AC → proof)

| AC bullet | Test | Asserts |
|---|---|---|
| phases advance on artifacts, all RDSPI | `test_codex_dag_advances_all_phases_via_artifacts` | phase walks research→…→Done on artifact presence alone; **zero signal files written** (the parity load-bearer: Codex emits no `.idle`, advance rides `check_artifact_advances`). |
| `.stopped`→Review auto-complete, deps respected | `test_codex_stopped_auto_completes_review_respecting_deps` | dep-free Review ticket on an Idle Codex pane → Done on disk; dependent ticket with an open dep → `all_dependencies_done` guard blocks + logs the error, stays Review. |
| heartbeat honest; genuine hang reclaimed | `test_codex_heartbeat_honest_then_genuine_hang_reclaimed` | 300s-recent activity survives `detect_stale_threads`; 2000s silence (> 2×`stuck_threshold`) reclaimed + slot released. |
| forced failure `.error` fails promptly | `test_codex_error_signal_fails_thread_promptly` | Codex `.error` → thread removed, slot released (session kept), `error_alerts` raised, file consumed — no waiting on the silence clock. |
| review-timeout finish-up via `agent-exec --resume` | `test_codex_review_timeout_finish_up_is_agent_exec_resume` | (a) `check_review_timeouts` takes the path for a quiet timed-out Codex Review thread; (b) the resolved Codex `follow_up` is a `SpawnCommand` carrying `agent-exec --resume` + the finish-up prompt. |
| dashboard sane — no phantom awaiting | `test_codex_pane_never_phantom_awaiting` | only Codex signals present → `awaiting_human` empty, `is_pane_awaiting` false, `to_ui_state` `awaiting=false`. |
| mixed loop — per-pane attribution | `test_mixed_panes_error_attributed_per_pane` | `pane-2.error` fails only the pane-2 thread; pane-1 untouched. |

## Test coverage

`cargo test --workspace` → **0 failed**; the plugin suite went 196 → **203** (+7).
WASM release build clean; clippy clean on `lisa-plugin`. The `validate-codex-loop.sh`
smoke-run builds plugin→CLI, scaffolds `/tmp/lisa-codex-dryrun`, sets
`client = "codex"` under `[agent]`, and `lisa validate` there reports "2 tickets,
1 ready, DAG valid" (T-CDX-02 correctly gated on T-CDX-01). The freshly-built
`lisa agent-exec --help` confirms the wrapper subcommand the harness relies on.

### Coverage gaps (intentional)

- **No live `codex` in CI** — the entire live spawn/stream/render/`--resume`
  surface is the manual runbook, gated on codex availability. This is the reality
  T-023-01/02 documented, not a shortfall of this ticket. The scheduler-side
  lifecycle *is* covered natively.
- **The harness has no committed automated test of its own logic** — it is a
  throwaway QA tool (same nature as T-020-05's `setup-gate-harness.sh`), verified
  by the smoke run above rather than a CI test.
- **`send_line_to_pane` delivery is asserted indirectly** — tests assert the
  scheduler *takes* the Codex path (`finish_up_sent`, `error_alerts`, phase/state)
  and the adapter *produces* the right line (`SpawnCommand` string), not the pane
  I/O itself, which has no native host. This mirrors every existing consumer test.

## Findings

### F1 — Mixed Claude+Codex in a *single* loop is not achievable today (scope, not a bug)
The AC asks for "one Claude pane + one Codex pane in the same loop." `client` is a
**loop-wide** setting (`.lisa.toml [agent].client` / `--client`;
`resolve_adapter` currently ignores the ticket), so a loop is all-Claude or
all-Codex. Per-pane `(provider, model)` routing is **story S-026**. What holds and
is validated now is **per-`pane-<id>` signal attribution**
(`test_mixed_panes_error_attributed_per_pane`), observable live by running a Codex
loop and a Claude loop side by side. **This is a scope boundary, not a contract
violation** — the mechanism is correct; the single-loop-mixing feature is simply
not built. It does **not** block S-025; it is S-026's remit. Flagged so the S-025
"documented toggle" copy does not overclaim in-loop mixing.

### F2 — Codex JSON shape remains `[PROVISIONAL]` (inherited, unresolved here)
The scheduler-side validation is fully insulated (it consumes signal *files*), but
the live runbook (checklist row 1 FAIL mode) is the first real chance to reconcile
codex's actual event names / `usage` placement against the wrapper's pluck keys
(`agent_exec.rs`: `render_event`, `extract_usage`, the fixtures). **Before T-027-02
bakes in field names, run the harness against a real `rust-v0.142.5` and reconcile.**
This is T-023-01 Open-concern #1, unchanged — this ticket makes it *checkable*, it
does not close it.

**No contract-violation bugs to file.** Every parity mechanism the automated suite
exercises behaves as S-024 requires; the value delivered is turning "should be
correct" into an executing proof plus a runbook for the one surface CI can't reach.

## For human attention

1. **Run `validate-codex-loop.sh` against a real codex** before S-025 ships its
   "documented toggle safe" claim — it is the only remaining evidence gap, and the
   JSON-shape reconcile (F2) rides on it.
2. **S-025 copy** should scope the toggle to *loop-wide* client selection and defer
   in-loop mixing to S-026 (F1), so the documentation matches the mechanism.

## Bottom line

Codex loop parity is now checkable: 7 green composition tests pin every
scheduler-side behaviour S-024 names, and a one-command scaffold + PASS/FAIL runbook
makes the live remainder observable and triageable. The honest residual is the same
one every Codex ticket carries — no live `codex` in CI, so the in-pane loop is
reasoned, unit-anchored, and now *runbook-ready*, not yet observed against a real
binary. Two findings (single-loop mixing → S-026; `[PROVISIONAL]` shape →
reconcile before T-027-02) are documented; neither is a blocking bug.
</content>
