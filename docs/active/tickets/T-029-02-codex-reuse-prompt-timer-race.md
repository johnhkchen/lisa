---
id: T-029-02
story: S-029
title: codex-reuse-prompt-timer-race
type: bug
status: open
priority: critical
phase: ready
depends_on: []
---

## Context

During the 0.4.0 release-candidate live run, a Codex pane reused for a
consecutive ticket clears successfully but does not reliably submit the next
ticket prompt. Claude panes using the same high-level clear-and-reuse workflow
succeed.

Lisa types a line, queues its Enter keypress for two seconds later, and also
runs unrelated scheduler timers. The Timer event handler currently flushes all
queued Enter keypresses on every timer event, so a scheduler tick can submit a
Codex prompt before the TUI has committed the pasted text.

## Acceptance Criteria

- An Enter queued by `send_line_to_pane` cannot be flushed before its own
  `ENTER_DELAY_SECS` deadline by an unrelated timer event.
- Multiple pending Enter keypresses retain independent deadlines and delivery
  order.
- The reused-session `/clear` → next-ticket prompt path continues to use the
  native Codex TUI and reaches feature parity with Claude session reuse.
- Focused plugin tests, the workspace test suite, the WASM release build, and
  plugin Clippy all pass.
