# T-021-01 Structure — spike harness blueprint

The "shape of the code" for a **spike**. There is no production code here; the
artifact this ticket produces is a set of throwaway probe scripts plus written
verdicts. This file specifies every file, its responsibility, boundaries, and the
order the probes run in.

## Boundary rule (load-bearing)

**Everything lives under `docs/active/work/T-021-01/harness/`.** Nothing is created,
modified, or deleted under `crates/`. This is enforced by convention and stated in
the acceptance criteria ("No production code merged; stubs kept clearly separate").
The harness may be deleted wholesale once verdicts are transcribed and the wrapper
(T-023-01) is built.

## Files

### Created (all under `harness/`)

| File | Responsibility | Boundary |
|---|---|---|
| `00-common.sh` | Sourced by every probe. Version guard (`require_codex`), evidence-dir helper (`probe_out`), `SANDBOX_HOME`/`OUT_DIR`, logging. Records the running codex version next to every evidence set. | Never invoked directly; no probe logic. |
| `q1-env-inheritance.sh` | Probe Q1: exports `LISA_PANE_ID=7`, forces a shell tool call that echoes it, greps the child's view out of the JSONL. | Only Q1. Writes `out/q1/`. |
| `q2-json-fidelity.sh` | Probe Q2: runs the RDSPI fixture through `codex exec --json` in a throwaway git repo; tallies event types; cross-checks terminal turn event vs. exit code. | Only Q2. Writes `out/q2/`. |
| `q3-inpane-rendering.sh` | Probe Q3: stdout→file, **stderr→separate file**; analyses stderr richness and whether stdout carries `*delta*` (partial) vs completed-only items. | Only Q3. Writes `out/q3/`. |
| `q4-directory-trust.sh` | Probe Q4: three cases (A unseeded / B seeded trust / C bypass flag), each with its own fresh `CODEX_HOME`. | Only Q4. Writes `out/q4/{A,B,C}/`. |
| `q5-resume-followup.sh` | Probe Q5: turn 1 plants a codeword + captures `thread_id`; turn 2 `resume`s and must recall it. | Only Q5. Writes `out/q5/`. |
| `fixtures/rdspi-ticket-prompt.txt` | The one representative multi-step ticket prompt Q2 feeds codex (forces file read + write + shell). | Data, not code. |
| `run-all.sh` | Driver. No-ops with a clear message if codex is absent; otherwise runs all five probes and points at `out/`. | Orchestration only; no probe logic. |
| `README.md` *(this ticket's harness/)* | One-paragraph "what/why/how to run", and the spike-only warning. | Docs. |

### Modified / Deleted

- **None.** No files outside `harness/` change. `docs/active/tickets/T-021-01-…md`
  frontmatter is **not** touched (Lisa advances phases from artifacts).

## Evidence layout (produced at run time, git-ignored in spirit)

```
harness/out/
  q1/  codex-version.txt  wrapper-env.txt  stdout.jsonl  stderr.log  exit-code  child-saw.txt
  q2/  codex-version.txt  stdout.jsonl  stderr.log  exit-code  event-histogram.txt  anchor-check.txt
  q3/  codex-version.txt  stdout.jsonl  stderr.log  exit-code  stderr-analysis.txt  granularity.txt
  q4/  A-fresh-unseeded/{…,exit-code}  B-seeded-trusted/{…,seeded-config.toml}  C-bypass/{…}
  q5/  codex-version.txt  turn1.jsonl  turn2.jsonl  thread-id.txt  recall.txt  {turn1,turn2}.exit
```

Each evidence set is **self-describing about its codex version** so a verdict can
be trusted or rejected on version grounds — the single most important property for
a version-pinned spike.

## Internal interfaces (the small contract the probes share)

- `require_codex <out_dir>` — hard-fails if codex is absent; warns (does not fail)
  if the version ≠ `rust-v0.142.5`, and writes `codex-version.txt`.
- `probe_out <name>` — `mkdir -p out/<name>` and echoes the path.
- Every probe ends by logging a `VERDICT INPUT:` line and a `PASS if …` line so the
  operator can read pass/fail straight from stderr without opening files.

## Ordering (why this sequence)

`run-all.sh` runs **Q1 → Q2 → Q3 → Q4 → Q5**:

1. **Q1 first** — if env inheritance fails, the whole attribution premise is void;
   cheapest to run and most foundational.
2. **Q2 next** — the hard go/no-go gate (event fidelity). If it fails, later probes
   are moot for the wrapper decision (escalate to app-server).
3. **Q3** — only meaningful if Q2's stream is usable; reuses the same run shape.
4. **Q4** — independent, but naturally grouped after the stream probes; may block
   *all* runs if trust isn't seeded, so its finding is a prerequisite the operator
   applies before re-running Q1–Q3 if needed.
5. **Q5** — builds on a working exec (Q2) to test the multi-turn extension.

Probes are **independently runnable** (each sources `00-common.sh` and self-sets up
its own throwaway repo / `CODEX_HOME`), so the operator can re-run just one after
fixing an environment issue.

## What the harness deliberately does NOT do

- No hooks, no TUI, no keystroke injection (all refuted upstream — doc 04).
- No `.idle`/`.awaiting`/`.cleared` probing — those signals don't occur on the
  autonomous Codex path (doc 05 §reframe).
- No wrapper implementation, no signal-file writing — that is T-023-01. The harness
  only *observes* codex; it does not translate its output into `.lisa/signals/`.
