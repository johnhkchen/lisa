---
id: T-020-05
story: S-020
title: interactive-gate-harness
type: task
status: review
priority: medium
phase: done
depends_on: [T-020-04]
---

## Context

Close the remaining S-020 gate residual — the *interactive* block + resume cycle
that can't be exercised headlessly — by making it **observable** in a real
`lisa loop` dry run instead of relying on a manual eyeball. The automated portion of
the gate is already closed (see `T-020-02/progress.md`: hook fires + writes
`.awaiting` + fires `on-notify` validated three ways; 11 plugin tests cover
consume/suppress/exempt/surface). What's left is watching claude's TUI actually
halt on `AskUserQuestion` and resume after an answer, and confirming lisa does **not**
clobber the blocked pane.

The plugin is already instrumented for this: it logs
`"Suppressed injection into pane N (awaiting human)"` in `send_line_to_pane`
(`lib.rs:283`) and marks awaiting threads in the dashboard
(`to_ui_state`, `lib.rs:2736`). The harness adds a **persistent hook-firing
timeline** on top so the question→block→resume sequence is reviewable after the run.

This ticket is **harness + runbook only** — no production code changes. It builds a
throwaway, instrumented project from `lisa init` output. Marked `review` (not `ready`)
so a `lisa loop` in this repo won't schedule it as agent work; it is run by the human.

## Deliverable

`docs/active/work/T-020-05/setup-gate-harness.sh` — one command that:
- builds the CLI, scaffolds a throwaway project, runs `lisa init`,
- drops a trigger ticket that forces the agent to call `AskUserQuestion` first,
- instruments the scaffolded hooks (`.lisa/hooks/on-*.sh`) to append a timestamped
  line to `.lisa/trace.log`, and installs a logging `on-notify`,
- prints the exact `lisa loop` command and the PASS/FAIL checklist.

## Acceptance Criteria (observed during the human run)

- **Trigger:** agent invokes `AskUserQuestion`; `on-notify.log` gets an
  `EVENT=attention ... LISA_REASON=question` line.
- **(b) Block, no clobber:** the pane shows the question and lisa does **not** type
  `/clear` or a next prompt over it. Dashboard shows the pane's `[AWAITING]` marker;
  if any timeout path tries to inject, dashboard shows the
  `"Suppressed injection ... (awaiting human)"` line.
- **(c) Resume + clear:** after the human answers in the TUI, the agent resumes; its
  next tool call writes a `heartbeat` line to `.lisa/trace.log` and the `[AWAITING]`
  marker clears.
- **FAIL signs (documented):** pane gets `/clear`'d or a prompt typed over the
  question; no `[AWAITING]` marker; marker never clears after answering.

## Notes

- If FAIL, the regression is in the live block/resume assumption, not the unit-tested
  lisa machinery — reassess before relying on S-020 in production.
- Harness lives under `work/T-020-05/`; the dry run executes in a separate temp project
  so it never touches this repo's tickets.
