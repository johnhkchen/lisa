# T-029-01 Review — Codex-day runbook live run (handoff)

Spike to execute `docs/knowledge/codex-day-runbook.md` end-to-end against the
installed Codex CLI. Host: **codex-cli 0.144.1**, **lisa 0.4.0-rc.5**,
2026-07-11. This is the handoff: what was proven, what changed, and the one
thing a human still needs to do.

## Outcome in one line

**The Codex integration is GO on 0.144.1.** Every S-021 empirical unknown is now
verified (was provisional), the Q2 hard gate is green, doctor + usage capture
work live, and three CLI-surface drifts were found and written back — one of
which is a real (but narrowly-scoped) bug. The only unexecuted part is the
**interactive `lisa loop`**, which is deferred to a human with the scaffold
pre-built.

## What changed (files)

**No production code touched** (spike). Edits are verdicts, knowledge, and one
new ticket:

- `docs/active/work/T-021-01/design.md` — 5× `[PROVISIONAL]` → `[VERIFIED
  0.144.1]`; banner updated; a "CLI-surface drift" section added.
- `docs/knowledge/codex-client/02,04,05.md` — dated 0.144.1 write-back sections
  (flag surface, #14345 narrowing, env-inheritance confirmation, resolved
  rendering/fidelity unknowns). Additive, never overwriting the pinned intel.
- `docs/knowledge/codex-day-runbook.md` — 2026-07-11 status-log entry.
- `docs/active/tickets/T-029-03-agent-exec-resume-argv-drift.md` — new
  `type: bug` (the one FAIL).
- `docs/active/work/T-029-01/` — this ticket's six RDSPI artifacts +
  `out-doctor.txt` (doctor evidence) + `rows-1-8-status.md` (loop disposition).
- **Not** committed: `/tmp/lisa-codex-dryrun` scaffold; `scratchpad/out-0.144.1/`
  probe evidence (session-local, referenced from the verdicts).
- `.lisa/provenance.jsonl` — **unchanged** (no fabricated Codex line).

## Verdict summary (codex 0.144.1)

| Item | Verdict | Gate/impact |
|---|---|---|
| Q1 env inheritance | PASS | pane attribution deterministic |
| **Q2 `--json` fidelity** | **PASS** | **hard gate GREEN — no app-server escalation** |
| Q3 rendering | render-from-JSON | matches shipped; no follow-up ticket |
| Q4 dir trust (exec) | no block on 0.144.1 | doctor pre-seed retained for native TUI |
| Q5 resume | PASS | finish-up analog works (with flag fix) |
| Doctor (step 1) | GREEN + trust seeded | #14345 re-verified on 0.144.1 |
| Usage capture (step 5) | PASS (live) | real `turn.completed.usage` artifact |

## Test coverage

- **Workspace suite: all green** — `cargo test --workspace` = 251 + 145 + 234 +
  6 + 2 passed, 0 failed. The 6 `test_codex_*` composition tests and the
  `agent_exec` fixtures cover every checklist mechanism.
- **Empirical probes** (the coverage CI can't reach) all ran live and are
  version-stamped `0.144.1`.
- **Gap — the live loop.** Rows 1–8 are unit-anchored but the end-to-end
  interactive run (spawn → artifacts → done, plus heartbeat/failure/finish-up
  forcing) was **not executed** — see Open concerns. This is the irreducible
  interactive remainder, not a missing test.
- **Gap — T-029-03 has no regression test yet** (the fix ticket adds it): the
  `build_codex_argv` resume branch is currently unverified against the reduced
  0.144.1 `resume` flag set.

## Open concerns / TODO (ranked)

1. **Run the deferred `lisa loop`** (`cd /tmp/lisa-codex-dryrun && lisa loop`).
   ~30–40 min, needs a Zellij session and human forcing for rows 4/5/6. Judge
   from durable artifacts per `rows-1-8-status.md`. This also produces the first
   real **Codex provenance ledger line** (step 5's deferred half). **Do not run
   it from inside a lisa-spawned agent** (scheduler nesting).
2. **Fix T-029-03** — `agent-exec --resume` argv is broken on codex ≥0.144.1
   (`-C`/`-s`/`--skip-git-repo-check` rejected) and the child should get
   `Stdio::null()` stdin. Off the live-loop path (diagnostics/headless only), so
   medium priority.
3. **Two stale checklist rows, now corrected in `rows-1-8-status.md`:** row 6's
   name (`…_is_agent_exec_resume`) is superseded by the shipped
   `…_types_into_tui` (finish-up types into the native TUI — the loop is
   unaffected by concern 2); row 8's "loop-wide client, not achievable" is stale
   — per-ticket `agent:` routing shipped (T-026-01), so the mixed loop is
   buildable and should be run as one Claude + one Codex ticket in a single loop.
4. **Follow-up ticket for the `LISA_GITIGNORE` template gap** — `templates.rs`
   ignores only `signals/`; `.lisa/claude/` and `.lisa/codex/` are runtime state
   and should be ignored too (fixed locally in `.lisa/.gitignore` 2026-07-09,
   not yet in the template). Also worth folding: `lisa init` never upserts a
   missing `[agent]` section into an existing `.lisa.toml` (this repo's config
   still lacks it — that is why baseline `lisa doctor` checks claude, not codex).
5. **Harness cleanup** — the T-021-01 harness may be deleted once its verdicts
   are transcribed (they are). Left in place until S-029 is marked done, per the
   ticket note.

## Critical issues for a human

None blocking. The Q2 hard gate passed, so there is no app-server escalation
decision to make. The one bug (T-029-03) does not affect `lisa loop`. The
highest-value next action is simply running the pre-built dry-run loop to convert
the CI-green mechanisms into an observed end-to-end pass and capture the first
real Codex provenance record.

## AC coverage map

- Doctor green + trust pre-seed re-verifies #14345 — **DONE** (`out-doctor.txt`).
- Harness run, every `[PROVISIONAL]` replaced with empirical verdict + version —
  **DONE** (`T-021-01/design.md`).
- Q2 hard gate; STOP if events dropped — **DONE, PASS** (no stop; not dropped).
- Q3 confirms render-from-JSON or files a switch ticket — **DONE, confirmed** (no
  ticket needed).
- Live loop rows 1–8 from durable artifacts, row 8 mixed — **DEFERRED**
  (interactive; scaffold + disposition delivered).
- Provenance record with real Codex usage — **PARTIAL**: usage capture proven
  live; ledger append deferred with the loop (writer path already proven).
- Write-back of drift + bugs + runbook status — **DONE** (docs 02/04/05, runbook
  log, T-029-03, template-gap note).
