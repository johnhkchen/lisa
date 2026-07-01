# T-021-01 Plan — spike execution sequence

Ordered steps to build the harness (done in this pass) and to run it to authoritative
verdicts (blocked on a host with codex `rust-v0.142.5`). A spike's "implementation"
is the harness + the write-up; there is no product code to commit.

## Testing strategy for a spike

The probes **are** the tests. There are no unit tests to add (no product code).
Verification = "does each probe capture falsifiable evidence and log a readable
PASS/FAIL line?" Two layers:

- **Static (runnable now, no codex):** `bash -n` every script; run `run-all.sh` and
  confirm it no-ops cleanly when codex is absent. ✅ done in this pass.
- **Empirical (needs codex `rust-v0.142.5`):** `bash run-all.sh`, then read the
  evidence and transcribe verdicts into `design.md`. ⛔ blocked on install.

Each step below is independently checkable.

## Steps

### Step 1 — Shared harness scaffold ✅ (this pass)
Build `00-common.sh`: version guard `require_codex` (writes `codex-version.txt`,
warns on version mismatch), `probe_out`, `OUT_DIR`/`SANDBOX_HOME`, logging.
*Verify:* `bash -n 00-common.sh`; sourcing it exposes the helpers.

### Step 2 — Q1 env-inheritance probe ✅ (this pass)
`q1-env-inheritance.sh`: export `LISA_PANE_ID=7`, force an `env | grep LISA_PANE_ID`
tool call, extract the child's view.
*Verify (empirical):* `out/q1/child-saw.txt` contains `LISA_PANE_ID=7`.

### Step 3 — RDSPI fixture + Q2 fidelity probe ✅ (this pass)
`fixtures/rdspi-ticket-prompt.txt` (forces read+write+shell) and
`q2-json-fidelity.sh` (histogram + anchor cross-check vs. exit code).
*Verify (empirical):* a terminal `turn.completed`/`turn.failed` exists **and**
agrees with `exit-code`; `command_execution` items present (events not dropped).
**This is the hard go/no-go gate.**

### Step 4 — Q3 rendering probe ✅ (this pass)
`q3-inpane-rendering.sh`: stdout→file, stderr→separate file; analyse stderr
richness and stdout delta-vs-completed granularity.
*Verify (empirical):* `stderr-analysis.txt` classifies stderr rich/spinner;
`granularity.txt` shows whether any `*delta*` events exist → picks tee vs render.

### Step 5 — Q4 directory-trust probe ✅ (this pass)
`q4-directory-trust.sh`: cases A (unseeded) / B (seeded trust) / C (bypass flag),
each with a fresh `CODEX_HOME`.
*Verify (empirical):* compare `A/B/C exit-code`; derive the minimal doctor seed.

### Step 6 — Q5 resume probe ✅ (this pass)
`q5-resume-followup.sh`: plant codeword in turn 1, capture `thread_id`, recall in
turn 2 via `resume`.
*Verify (empirical):* `recall.txt` == `MARMALADE` and turn-2 exit 0.

### Step 7 — Driver + README ✅ (this pass)
`run-all.sh` (graceful no-op without codex) and `harness/README.md`.
*Verify:* running without codex prints the notice and exits 1. ✅ confirmed.

### Step 8 — Empirical run on pinned codex ⛔ (blocked — needs `rust-v0.142.5`)
On a host with the pinned binary: `bash run-all.sh`; inspect `out/`.
*Verify:* every `out/*/codex-version.txt` shows `rust-v0.142.5`.

### Step 9 — Transcribe verdicts ⛔ (follows Step 8)
Update `design.md`: flip each `[PROVISIONAL]` to a confirmed verdict with the
captured evidence path, and apply any event-name corrections to doc 05's mapping
table. Confirm/deny the go/no-go.

## Commit strategy

Spike artifacts (harness + the six phase docs) are documentation/tooling, committed
together, **not** touching `crates/`. No incremental product commits — there is no
product code in this ticket.

## Exit criteria (acceptance criteria restated)

- [x] `design.md` with a verdict + evidence-plan per question, pinned to the codex
      version (provisional now; the pin is enforced by `codex-version.txt`).
- [x] Go/no-go recorded, with the Q2 hard gate and the app-server fallback trigger.
- [x] Tee-stderr vs. render-from-JSON recommendation stated (render-from-JSON,
      provisional, with the flip condition).
- [x] Event-mapping correction fed back to doc 05 (`.error` has no consumer).
- [x] No production code; stubs isolated under `harness/`.
- [ ] **Empirical confirmation on `rust-v0.142.5`** — the one open item (Steps 8–9),
      blocked on codex not being installed on this host.
