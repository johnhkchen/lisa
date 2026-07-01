---
id: T-022-01
story: S-022
title: adapter-interface-extraction
type: task
status: open
priority: high
phase: done
depends_on: []
---

## Context

Extract the Claude-Code-specific behaviour in the plugin behind an adapter
interface, as a **provable no-op refactor**. Native Claude Code is the only
implementation; behaviour with no opt-in is byte-for-byte unchanged. This is
the seam every later story (Codex adapter, config toggle, per-pane routing)
plugs into — see the epic's S-022 needs and Decision 4.

The interface must own what differs per integration method:

- **Launch**: command construction for a fresh pane (today
  `build_claude_command`, `lib.rs:53`).
- **Reuse/reset**: today the `/clear` → `.cleared` handshake
  (`schedule_ready_tickets` `lib.rs:568-579`, `TransitionState` `lib.rs:128`);
  for Codex, reuse is a fresh exec — so the transition state machine's
  applicability is adapter-owned, not scheduler-hardcoded.
- **Follow-up injection**: today `finish_up_prompt` typed into the live TUI
  (`check_review_timeouts`); the interface needs a "send follow-up" operation
  that a future adapter can implement as a spawned command instead of
  keystrokes.
- **Expected signal set**: which of `.idle`/`.awaiting`/`.cleared` this
  adapter emits (Claude: all; others: subset), so the scheduler/UI can treat
  absence correctly.
- **Selection seam**: the adapter for a ticket is resolved **per ticket at
  spawn time** — a resolver function, not a loop-wide constant — even though
  the MVP resolves every ticket to native Claude.

## Acceptance Criteria

- An adapter trait/interface with a native Claude implementation covering
  launch, reuse/reset, follow-up, and expected-signal-set.
- Spawn-time per-ticket resolution (resolver takes the ticket, returns the
  adapter) with the MVP resolver returning native Claude unconditionally.
- Zero behaviour change: existing tests pass unmodified; command strings,
  signal handling, and transition behaviour are identical (assert via the
  existing `test_build_claude_command*` tests plus transition tests).
- The interface shape accommodates a native-Codex adapter (exec wrapper) and a
  future ACP adapter without redesign — documented in the trait's doc comment.

## Notes

- Do not implement any Codex behaviour here; T-023-02 does that.
- WASM constraint: adapters run inside the plugin and can only write to panes
  / read signal files — no subprocess piping (doc 06 §"The constraint that
  filters everything").
