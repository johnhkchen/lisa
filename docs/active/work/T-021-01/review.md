# T-021-01 Review — codex-wrapper-spike

Handoff for a human reviewer. What this spike produced, how far it got, and the one
thing a reviewer must decide before T-023 starts.

## What this ticket was

A **spike** gating the whole Codex-client build (S-021 / E-001): confirm, on codex
`rust-v0.142.5`, the five empirical unknowns the `codex exec --json` wrapper
decision rests on. Deliverable = a written verdict + evidence per unknown, using
throwaway stubs. No production code.

## What changed (files)

All under `docs/active/work/T-021-01/` — **nothing under `crates/`**:

- **Phase docs:** `research.md`, `design.md` (the findings artifact — verdicts +
  go/no-go), `structure.md`, `plan.md`, `progress.md`, this `review.md`.
- **Harness (`harness/`):** `00-common.sh` (shared version-guard + evidence
  helpers), five probes `q1..q5`, `run-all.sh` driver, `README.md`, and
  `fixtures/rdspi-ticket-prompt.txt`. All executable, all `bash -n`-clean.

The ticket frontmatter was intentionally **not** modified (Lisa advances phases
from artifacts).

## Verdict summary (from `design.md`)

| Q | Unknown | Verdict | Confidence |
|---|---|---|---|
| Q1 | env inheritance (`LISA_PANE_ID` → codex child) | PASS expected | [PROVISIONAL] |
| Q2 | `--json` fidelity under tools + anchor rule | PASS with anchor rule; **hard go/no-go gate** | [PROVISIONAL] |
| Q3 | in-pane rendering | **render-from-JSON** recommended (flips if stderr is rich) | [PROVISIONAL] |
| Q4 | directory trust headless | doctor must pre-seed `trust_level="trusted"` | [PROVISIONAL] |
| Q5 | `exec resume` follow-up | PASS expected (proves context carry) | [PROVISIONAL] |

**Go/No-Go:** provisional **GO** on the `codex exec --json` wrapper, contingent on
Q2 surviving one real run; app-server (doc 05 Option 2) is the documented fallback
if `--json` drops events under tools (#15451) — a human decision, not to be
designed around.

## "Test" coverage (a spike's tests are its probes)

- **Static, run on this host:** `bash -n` on all 7 scripts → clean; `run-all.sh`
  with codex absent → graceful no-op (notice + exit 1, no side effects). ✅
- **Empirical, NOT run:** the probes have never executed against a codex binary —
  see the critical gap below. Each probe captures falsifiable evidence and logs a
  readable `PASS if …` line, so an empirical run is turnkey.

## Critical issue for human attention

**The empirical half of this spike did not run: codex is not installed on the host**
(`which codex` → not found; `~/.codex` absent). A spike whose purpose is empirical
verification cannot be fully closed here. The verdicts are reasoned from the pinned
intel packet and cited `openai/codex` issues, and are honestly tagged
`[PROVISIONAL]` — **not** presented as confirmed. To close the loop, run
`bash docs/active/work/T-021-01/harness/run-all.sh` on a machine with
`rust-v0.142.5`, then flip the verdicts in `design.md` using the captured evidence
(each evidence set carries its own `codex-version.txt`).

**Reviewer decision required:** accept the spike as "harness + provisional
verdicts, empirical run deferred to whoever has codex installed," **or** hold
T-023 until the harness is run. The design's *shape* (wrapper over `exec --json`,
env attribution, render-from-JSON, doctor trust-seed, resume for follow-up) does
not depend on the run; only the go/no-go *confidence* does. The single hard gate is
Q2.

## Open concerns / TODOs (carried forward)

1. **`.error` has no consumer in the current scheduler** (`crates/lisa-plugin/src/
   lib.rs` — no `.error` reader). Doc 05 maps `turn.failed`/non-zero-exit → `.error`,
   but until T-023 adds a consumer, codex failures must surface as `.stopped`.
   Flagged into `design.md` §Corrections — T-023 structure must resolve this.
2. **Q4 trust behaviour is version-volatile** (#14345). The doctor seed must be
   re-verified per codex version, not assumed stable — carry into T-025-01.
3. **Event-name casing/`usage` placement** (Q2) must be transcribed from a real
   `stdout.jsonl` before T-023-02's adapter parser hardcodes strings; T-027-02 cost
   capture depends on where `usage` rides.
4. **Q3 recommendation may flip** to tee-stderr if the real stderr is rich and
   stable — cheap, so worth checking during the empirical run before T-023-01
   commits to a renderer.

## Bottom line

The wrapper approach is sound and the harness makes confirming it a one-command
job. The honest state is: **design-ready, empirically-unconfirmed.** The gating
question for the epic is whether that is enough to start T-023, or whether the
harness must be run against `rust-v0.142.5` first — with Q2 as the one result that
could still say no-go.
