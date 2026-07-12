# T-029-01 Research — Codex-day runbook live run

Descriptive map of the instruments this spike drives, the host state at run
time, and the constraints that bound what "live" can mean inside a scheduled
agent. No solutions here — those are in `design.md`.

## What this ticket is

A **spike**, not a code change. The deliverable is *evidence*: run
`docs/knowledge/codex-day-runbook.md` end-to-end against the installed Codex
CLI, replace every `[PROVISIONAL]` verdict with an empirical PASS/FAIL, capture
a real provenance record, and write drift back into the knowledge base. The
"implementation" phase is therefore *executing pre-written instruments and
transcribing results*, not authoring new production code.

## The instruments (all already exist)

| Instrument | Path | Role |
|---|---|---|
| Runbook | `docs/knowledge/codex-day-runbook.md` | The 5-step script; source of truth for order + exit criteria |
| Spike harness | `docs/active/work/T-021-01/harness/` | `run-all.sh` → q1..q5 probes, evidence under `out/` |
| Provisional verdicts | `docs/active/work/T-021-01/design.md` | Five `[PROVISIONAL]` tags to promote to empirical |
| Loop checklist | `docs/active/work/T-024-01/checklist.md` | Rows 1–8, live-loop PASS/FAIL |
| Loop scaffolder | `docs/active/work/T-024-01/validate-codex-loop.sh` | Builds a throwaway Codex-client project at `/tmp/lisa-codex-dryrun` |
| Provenance schema | `docs/knowledge/provenance-ledger.md` | Shape of `.lisa/provenance.jsonl` records |
| Codex client docs | `docs/knowledge/codex-client/02,04,05` | Write-back targets for any drift |

## The five probes (q1–q5), by what they actually run

Each sources `00-common.sh` (isolated `SANDBOX_HOME`, `require_codex` records
the running version beside every verdict) and writes falsifiable evidence:

- **q1 env-inheritance** — exports `LISA_PANE_ID=7`, runs `codex exec --json`
  forcing a `env | grep LISA_PANE_ID` shell tool call; PASS if `child-saw.txt`
  contains the var. Uses the ambient `CODEX_HOME` (logged-in `~/.codex`).
- **q2 json-fidelity** — the **hard go/no-go gate**. Feeds
  `fixtures/rdspi-ticket-prompt.txt` (forces read+write+shell tool calls) through
  `codex exec --json`, tallies event types, cross-checks a terminal
  `turn.completed`/`turn.failed` against the process exit. Tests #15451 (events
  dropped under tools) and #14691 (item-status misreport).
- **q3 in-pane rendering** — captures stderr separately to decide tee-stderr vs
  render-from-JSON; greps stdout for `*delta*` item events (partial vs
  completed-only text).
- **q4 directory-trust** — three cases on **fresh** `CODEX_HOME`s: A unseeded,
  B seeded `[projects."<path>"].trust_level="trusted"`, C
  `--dangerously-bypass-approvals-and-sandbox`. Re-verifies bug #14345.
- **q5 resume-followup** — turn 1 plants codeword MARMALADE, captures
  `thread_id` from `thread.started`, `codex exec resume` recalls it; proves the
  `finish_up_prompt` analog carries context.

## Host state observed at research time (2026-07-11)

```
codex   → /Users/johnchen/.local/bin/codex   codex-cli 0.144.1
lisa    → /opt/homebrew/bin/lisa             lisa 0.4.0-rc.5
zellij  → /opt/homebrew/bin/zellij           present
CODEX_HOME → unset (defaults to ~/.codex, ChatGPT-logged-in per status log)
.lisa/provenance.jsonl → 1 record (T-029-02, claude, tokens null)
```

**Drift, already visible before running anything:**

1. **Codex `0.144.1`** installed — drifts from both the pinned research target
   `rust-v0.142.5` *and* the `0.144.0` recorded in the runbook status log on
   2026-07-09. `00-common.sh` will stamp `0.144.1` into every `codex-version.txt`
   and log a "provisional" warning; per the runbook's step-0 note, the probes are
   now the drift detector, not a confirmation of the pin.
2. **`lisa` resolves to `/opt/homebrew/bin`, not `~/.cargo/bin`** — the runbook
   preflight assumes brew is 0.3.0 (no Codex code) and `~/.cargo/bin` shadows it.
   Here brew already reports **0.4.0-rc.5**, which *does* carry the Codex client.
   The preflight's *intent* (a lisa that has the Codex client) is satisfied; the
   *mechanism* it describes (unlink brew, prefer cargo) is stale. This is a
   runbook-preflight drift to note, not a blocker.
3. **Provenance already seeded** — one Claude record for T-029-02 exists
   (tokens `null`, a 5-second run). The ledger file is present and append-only,
   so the live run must *add* a Codex record, not create the file.

## The hard structural constraint: interactivity

The runbook splits cleanly into two halves by *what kind of process each step
needs*:

- **Steps 1 + 3 (doctor, harness) are headless.** `lisa doctor` and every
  `codex exec --json` invocation are ordinary child processes that run to
  completion and write files. These are fully executable from a non-interactive
  agent turn.
- **Step 4 (`lisa loop`) is an interactive Zellij session.** It boots a
  multiplexer, spawns panes running interactive Claude/Codex TUIs, and renders a
  live dashboard. Three of the eight checklist rows require a **human at the
  terminal**: row 4 (Ctrl-C a pane to force a genuine hang), row 5 (force a
  non-zero turn), row 6 (let a Review session go quiet past the timeout). This
  half cannot be driven to completion inside a scheduled agent process.
- **A recursion hazard compounds it:** this ticket is itself being worked by a
  lisa-spawned agent. Launching `lisa loop` from inside that agent would nest a
  scheduler inside a scheduled pane — not a valid live test of the outer loop.

`validate-codex-loop.sh` is the seam: everything up to and including
`lisa init` + DAG scaffolding is headless (it just builds and writes files);
only the final `cd /tmp/lisa-codex-dryrun && lisa loop` it prints is
interactive.

## Provenance dependency

A real *Codex* provenance record (AC bullet 5) is written by the plugin only at
the teardown of a real loop run (`crates/lisa-core/src/provenance.rs`,
schema v1, write-after-frontmatter). It therefore *depends on step 4*: no live
loop → no new Codex ledger line. The existing Claude record proves the writer
path works; the Codex record with real `turn.completed.usage` tokens is
gated on an interactive loop.

## Constraints & assumptions carried into Design

- Running the harness makes **real, billed** `codex exec` calls against the
  user's ChatGPT login and executes real (sandboxed, `workspace-write`,
  `-a never`) tool commands in throwaway temp dirs. The ticket explicitly
  authorizes this; it is the whole point of a "live run."
- **q4's fresh `CODEX_HOME`s carry no credentials.** A fresh home may fail at
  *auth* before it ever reaches the *trust* check — a known confound to watch
  for when reading A/B/C exit codes; the trust verdict must not be read off an
  auth failure.
- Evidence lives under `docs/active/work/T-021-01/out/`; the harness is
  spike-scaffolding, deletable only after this ticket is done (runbook + ticket
  note). Keep it until Review.
- Dry-run project stays **outside this repo** at `/tmp/lisa-codex-dryrun`; commit
  only recorded results, never the scaffold.
- Known template gap to fold into write-back: `LISA_GITIGNORE` (`templates.rs`)
  ignores only `signals/`; `.lisa/claude/` and `.lisa/codex/` are runtime state
  too (a stray `last.usage.json` was committed 2026-07-09; fixed locally in
  `.lisa/.gitignore`).
