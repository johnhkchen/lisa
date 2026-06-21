---
id: T-020-01
story: S-020
title: askuserquestion-spike
type: spike
status: open
priority: medium
phase: done
depends_on: [T-019-03]
---

## Context

Investigate whether lisa can reliably detect an agent invoking the
`AskUserQuestion` tool and notify the human, plus design the "awaiting human"
suppression that keeps lisa from typing over a question-blocked pane. This is the
exploratory front of S-020 — its output is a `design.md` that defines the
implementation tickets (or a recommendation to close the story).

This is a **spike**: the deliverable is findings + a design, not production code.
Prototype hooks/plugin changes are fine but live behind the spike, not merged as-is.

## Questions to answer

1. **Does `AskUserQuestion` fire `PreToolUse`?** Confirm it does, and capture the
   exact tool-name string a `PreToolUse` matcher needs. (lisa binds no `PreToolUse`
   today — see `templates.rs:74-123` / `merge_hooks` at `templates.rs:204-243`.)
2. **GATE — do skip-permissions agents even use it?** lisa spawns
   `claude --dangerously-skip-permissions` in panes it types into. Determine whether
   agents in this mode ever actually invoke `AskUserQuestion` (vs. proceeding
   autonomously / asking in plain prose). If they never do, the signal is moot —
   document that and recommend pivot/close before any further work.
3. **Payload extraction:** capture a real `PreToolUse` stdin payload for
   `AskUserQuestion` and confirm the question text is extractable in POSIX `sh`
   (no `jq`, no bashisms) to pass as the `on-notify attention` detail.
4. **Resume detection:** confirm an answered question pane emits a `PostToolUse`
   heartbeat afterward, so an "awaiting human" flag can be cleared on the existing
   `check_heartbeat_signals` path (`lib.rs:679`).
5. **Suppression design:** specify the smallest change that pauses auto-injection for
   an awaiting pane — which of `send_line_to_pane` callers to guard
   (`schedule_ready_tickets`, `handle_stopped_signal`, `handle_cleared_signal`,
   `check_transition_timeouts`, `check_review_timeouts`), how the flag is set
   (new `pane-$ID.awaiting` signal vs. event-driven), and how it's cleared. Must not
   destabilize the heartbeat liveness model (see project memory
   "Liveness heartbeat design").
6. **Timeout interaction:** decide whether awaiting-human panes are exempt from review/
   transition timeout reclamation so they aren't reclaimed mid-question.

## Acceptance Criteria

- A `design.md` artifact answering Q1–Q6 with evidence (a captured `PreToolUse`
  payload sample, and a yes/no on Q2 with how it was determined).
- A clear go / no-go recommendation on Q2 (the gate).
- If go: a proposed implementation-ticket breakdown (T-020-02+) covering the new
  `PreToolUse[AskUserQuestion]` hook binding, the plugin awaiting-human flag +
  injection/auto-advance/timeout suppression, and tests — each with file:line anchors.
- Confirmation the approach reuses the S-019 `on-notify` contract (no new user hook)
  and stays POSIX-`sh`-only on the hook side.
- No production code merged from this ticket beyond the design (and optional
  throwaway prototype kept clearly separate).

## Notes

- Reuses S-019 infrastructure: `on-notify` hook (T-019-02) and `run_command` /
  signal-reading plumbing (T-019-01). Note any hard ordering this implies for the
  implementation tickets.
- The awaiting-human suppression is the correctness fix S-019 deferred; this spike is
  where its design lands.
