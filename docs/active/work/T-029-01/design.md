# T-029-01 Design — how to execute the runbook honestly

Research established the instruments, the host state (Codex `0.144.1`, lisa
`0.4.0-rc.5`), and one hard constraint: the runbook has a **headless half**
(steps 1, 3) and an **interactive half** (step 4's `lisa loop`) that cannot run
to completion inside a scheduled agent. This document decides *how* to run it,
what the go/no-go handling is, and how to reconcile a tension the research
surfaced.

## The tension to resolve first: wrapper vs native TUI

The harness (`T-021-01`) and the runbook's step 3 are built to settle the
**`codex exec --json` wrapper** decision (S-021, docs 05/08). But doc 09
(`native-tui-parity.md`, verified on Codex **0.144.0**, 2026-07-09) **already
reversed** that: the shipped default adapter launches the **native TUI**
(`codex --dangerously-bypass-approvals-and-sandbox --dangerously-bypass-hook-trust
"PROMPT"`), reads the Stop-hook transcript for usage, and does *not* use the
`exec --json` wrapper at all.

**Decision: run the harness anyway, but re-frame its output.** The q1–q5 probes
are still worth running because they measure *properties of `codex exec --json`*
that (a) remain the fallback/headless path (`lisa agent-exec`, used for
review-timeout `--resume`), and (b) double as a **drift detector** for the
event/usage vocabulary the ledger and adapter depend on. What changes is the
*verdict framing*: a Q2 "fail" no longer blocks the whole integration (native
TUI is the live path) — it blocks the *headless exec path* only. This is
recorded so the go/no-go gate is read correctly, not mechanically.

## Options considered for executing the interactive half (step 4)

### Option A — Drive `lisa loop` from inside this agent
Rejected. This ticket is worked by a lisa-spawned agent; launching `lisa loop`
here nests a scheduler inside a scheduled pane. Rows 4/5/6 additionally need a
human to Ctrl-C a pane, force a non-zero turn, and stall a Review session. Not
achievable, and not a valid test of the outer loop even if it booted.

### Option B — Skip step 4 entirely, mark the ticket blocked
Rejected. It discards the ~70% of the runbook that *is* headless (doctor + the
five-probe go/no-go gate + the scaffold build), which is exactly the
high-leverage empirical work the spike exists to produce.

### Option C — Execute the headless half for real; scaffold + hand off the interactive half (chosen)
Run everything that is a plain child process to completion and record real
verdicts; build the dry-run scaffold so the interactive step is one command
away; document rows 1–8 as an **operator runbook with pre-filled expected
observables**, explicitly marking which rows need a human. This maximizes
empirical yield, keeps every recorded verdict honest about the version it ran
against, and leaves a clean seam for a human (or a future non-nested session) to
finish step 4.

**Chosen: C.** It is the only option that honors both "run it end-to-end" and
"never fabricate a verdict."

## Execution strategy (what Implement will do)

1. **Step 1 — doctor.** Run `lisa doctor` with the Codex client selected.
   Verify it reports Codex `0.144.1` and pre-seeds directory trust
   (`pregrant_codex_trust_in`) into a `CODEX_HOME` config. Confirm the trust
   pre-seed still unblocks a fresh home on 0.144.1 (re-verify #14345). Capture
   output to `out-doctor.txt` under the work dir.
2. **Step 3 — harness (the gate).** `cd docs/active/work/T-021-01/harness &&
   bash run-all.sh`, wrapped in a wall-clock timeout so a hung `codex exec`
   cannot stall the phase. Transcribe q1–q5 evidence into
   `T-021-01/design.md`, replacing each `[PROVISIONAL]` with an empirical
   verdict pinned to `0.144.1` and an evidence pointer under `out/`.
3. **Q2 hard gate.** If `--json` drops events under builtin tool use such that
   no reliable terminal `turn.completed`/`turn.failed` survives to agree with
   the exit code → **STOP the headless exec path**, do not attempt to design
   around it, and surface the app-server fallback (doc 05 Option 2) as a human
   decision. Given doc 09, native TUI remains the live path regardless — record
   both facts.
4. **Q3 verdict.** Confirm render-from-JSON vs tee-stderr from `stderr-analysis`
   + `granularity`. Per AC, if it disagrees with the wrapper's current mode,
   **file a follow-up ticket** — do not rework inline.
5. **Step 4 — scaffold + hand off.** Run `validate-codex-loop.sh` to build the
   plugin+CLI and stand up `/tmp/lisa-codex-dryrun`. Record that the scaffold
   built and the DAG is correct. Produce a pre-filled rows 1–8 result table in
   the work dir marking each row: headless-verifiable (from scaffold/tests),
   needs-interactive-human, or needs-native-TUI-session.
6. **Step 5 — provenance.** The existing Claude record proves the writer path.
   A real Codex record requires step 4's live loop, so it is **gated**; record
   the dependency explicitly rather than fabricating a line. If a headless
   `lisa agent-exec` path can emit a `.lisa/codex/<t>.usage.json` from
   `turn.completed.usage` without a full loop, note it as the fallback capture
   route.
7. **Write-back.** Apply the confirmed drift (Codex `0.144.1`; anything q1–q5
   surface about event names / trust / usage) to codex-client docs 02/04/05,
   append a status-log entry to the runbook, and file any FAIL as a `type: bug`
   ticket. Fold the `LISA_GITIGNORE` template gap in.

## Go/no-go and failure handling, decided up front

- **Q2 fail** → headless-exec path stops; native TUI (doc 09) unaffected;
  app-server escalation is a *human* decision, filed not designed.
- **Q4 shows fresh-home block persists on 0.144.1** → the doctor pre-seed is
  *still required*; confirm doctor writes the `[projects."<path>"]
  trust_level="trusted"` block. Watch the **auth confound**: a fresh
  `CODEX_HOME` has no ChatGPT credentials, so an A-case failure may be *auth*,
  not *trust* — the verdict must be read off the trust/approval stderr strings,
  not a bare non-zero exit.
- **Any probe hangs** → the timeout kills it; the affected verdict is recorded
  as `INCONCLUSIVE (timed out)` with the partial evidence, never guessed PASS.
- **A checklist row needs a human** → recorded as `DEFERRED (interactive)` with
  the exact forcing technique, not silently PASSed.

## What is deliberately *not* done

- No production code changes. This is a spike; the only file edits are
  verdict transcription (`T-021-01/design.md`), knowledge write-backs
  (`codex-client/`), the runbook status log, and any new `type: bug` tickets.
- No `lisa loop` launched from this agent (recursion + interactivity).
- The T-021-01 harness is **not deleted** — the ticket note permits deletion
  only once T-029-01 is done, and Review is the gate for that.

## Success criteria for this spike's own execution

Design counts as correctly executed when: doctor ran and its trust behaviour on
0.144.1 is recorded; q1–q5 ran (or timed out, recorded as such) with verdicts
transcribed and version-pinned; the Q2 gate is adjudicated with the native-TUI
caveat; the scaffold built; rows 1–8 are each tagged with an honest status; the
provenance dependency is stated; and every confirmed drift is written back with
tickets filed for FAILs. Nothing above requires an interactive session to be
*honest* — only step 4's live rows are deferred, and they are deferred visibly.
