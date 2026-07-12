# T-029-01 Structure — files touched, evidence layout, ordering

A spike touches *evidence and knowledge*, not `crates/`. This is the blueprint
for which files are read, written, and created, and in what order the mutations
must land so a crash leaves a coherent trail.

## Invariant: no production code

Nothing under `crates/` is created, modified, or deleted. `cargo` is invoked
only as a *build* (via `validate-codex-loop.sh`) to make the embedded plugin
current — no source edits. If the run surfaces a code bug, it becomes a
`type: bug` ticket, not an inline patch (AC + design decision).

## Files by disposition

### Created — this ticket's own artifacts (`docs/active/work/T-029-01/`)
- `research.md`, `design.md`, `structure.md`, `plan.md` — planning artifacts.
- `progress.md` — Implement-phase running log (what ran, verdict, evidence ptr).
- `review.md` — handoff.
- `out-doctor.txt` — captured `lisa doctor` output (step 1 evidence).
- `rows-1-8-status.md` — the pre-filled checklist result table with per-row
  disposition (headless / interactive / native-TUI), the durable step-4 artifact
  given the live loop is deferred.

### Created — evidence under the harness (`docs/active/work/T-021-01/harness/out/`)
Written by `run-all.sh`; **not** committed as-is (spike scaffolding), read to
fill verdicts:
- `out/q1/{child-saw.txt,stdout.jsonl,stderr.log,exit-code,codex-version.txt,wrapper-env.txt}`
- `out/q2/{anchor-check.txt,event-histogram.txt,stdout.jsonl,stderr.log,exit-code,codex-version.txt}`
- `out/q3/{stderr-analysis.txt,granularity.txt,stdout.jsonl,exit-code,codex-version.txt}`
- `out/q4/{A-fresh-unseeded,B-seeded-trusted,C-bypass}/{exit-code,stdout.jsonl,stderr.log,codex-version.txt}`
- `out/q5/{recall.txt,thread-id.txt,turn1.jsonl,turn2.jsonl,turn*.exit,codex-version.txt}`

### Modified — verdict transcription
- `docs/active/work/T-021-01/design.md` — replace each of the five
  `[PROVISIONAL]` tags (Q1–Q5) with an empirical verdict, pinned to the version
  in the matching `codex-version.txt`, plus an `out/qN/...` evidence pointer.
  Update the top "Version status" banner from "not installed" to
  "run against 0.144.1 on 2026-07-11." Preserve the reasoning prose; only the
  verdict lines and banner change.

### Modified — knowledge write-back (`docs/knowledge/codex-client/`)
Only where the run produces a *confirmed* delta versus the pinned
`rust-v0.142.5` intel. Candidate edits, gated on evidence:
- `02-codex-capabilities.md` — bump the observed-version anchor to `0.144.1`;
  correct any `--json` event `type` names / `turn.completed.usage` field names
  that q2/q3 show drifted; note whether `item.updated` appears.
- `04-risks-and-open-questions.md` — update the #14345 (trust-under-bypass) and
  Stop-hook verdicts *only if* q4 changes their status on 0.144.1; otherwise add
  a "re-verified 0.144.1, unchanged" dated line.
- `05-bridging-the-discrepancy.md` — resolve the load-bearing open question
  ("what renders on stderr under `--json`, deltas or completed-only?") from q3
  evidence; annotate the Option-1 event map with any confirmed casing.
Each edit is **additive and dated** ("2026-07-11, Codex 0.144.1: …"), never a
silent overwrite of the pinned intel.

### Modified — runbook status log
- `docs/knowledge/codex-day-runbook.md` — append a `## Status log` entry dated
  2026-07-11 recording: version drift `0.142.5 → 0.144.1`, which steps ran
  headless, the Q2 gate outcome, and that step 4's live rows are deferred to an
  interactive session with the scaffold pre-built.

### Modified — provenance (conditional)
- `.lisa/provenance.jsonl` — appended **only** if a real Codex record is
  captured (needs step 4 or a headless `agent-exec` usage capture). Not
  fabricated. If uncaptured, the dependency is documented in `progress.md` and
  `review.md`, and the file is left as-is (its one Claude record).

### Created — bug tickets (conditional, one per FAIL)
- `docs/active/tickets/T-0XX-YY-<slug>.md` — `type: bug`, one per checklist FAIL
  or probe FAIL, referencing the checklist row / probe and its evidence pointer.
  Frontmatter per the ticket format in `rdspi-workflow.md`. IDs allocated under
  a bug story; none created if everything PASSes.

### Untouched (read-only inputs)
- The runbook, the harness scripts, `validate-codex-loop.sh`, the checklist,
  the provenance schema doc, docs 01/03/06/07/08/09. `09-native-tui-parity.md`
  is the authority on the *live* path and is only *cited*, not edited, by this
  headless run.

### External to the repo (never committed)
- `/tmp/lisa-codex-dryrun/` — the scaffold `validate-codex-loop.sh` builds.
- `docs/active/work/T-021-01/harness/.codex-sandbox/` and q4's `mktemp` homes —
  isolated `CODEX_HOME`s; transient.

## Component boundaries during the run

```
lisa doctor ─────────────► out-doctor.txt            (step 1)
run-all.sh ──► q1..q5 ──► out/qN/*  ──► T-021-01/design.md verdicts   (step 3)
                              │
                              └─► confirmed drift ──► codex-client/02,04,05
validate-codex-loop.sh ──► /tmp/lisa-codex-dryrun + build   (step 4 scaffold)
                              └─► rows-1-8-status.md          (deferred rows)
(live loop, human) ──► .lisa/provenance.jsonl Codex record   (step 5, gated)
runbook status log  ◄── outcome summary                       (write-back)
```

## Ordering constraints (crash-coherent)

1. **doctor before harness** — doctor's trust pre-seed is the precondition the
   probes assume; run and capture it first.
2. **harness before transcription** — verdicts are read *from* `out/`; never
   edit `T-021-01/design.md` ahead of evidence.
3. **transcription before write-back** — codex-client edits cite the same
   evidence; the verdict must exist first.
4. **scaffold independent of harness** — `validate-codex-loop.sh` only builds +
   writes files; it can run any time after doctor, before or after the probes.
5. **provenance last, conditional** — only after a real capture exists.
6. **status-log + tickets last** — they summarize outcomes, so they close the
   run.

Each step writes its evidence file immediately on completion, so an interrupted
run leaves `progress.md` + the partial `out/` as a resumable seed (artifacts-as-
insurance, per the workflow's phase rules).
