---
id: T-023-02
story: S-023
title: codex-adapter
type: feature
status: open
priority: high
phase: done
depends_on: [T-022-02, T-023-01]
---

## Context

Implement the native Codex adapter behind the T-022-01 interface, driving the
T-023-01 wrapper. This is where "launching a different binary" meets the
lifecycle differences the interface was shaped for:

- **Launch**: build the pane command around the wrapper subcommand using the
  absolute lisa binary path passed through the layout/plugin config
  (`current_exe()` captured at `lisa loop` time — no PATH assumption).
- **Reuse**: no `/clear` handshake — a finished exec leaves the pane's shell
  at its prompt, so "reuse" is typing a fresh wrapper command
  (`has_session` semantics differ: the codex process is *not* still running).
  The `WaitingForStop`/`WaitingForClear` machinery must not engage for Codex
  panes.
- **Follow-up**: the review finish-up prod (`check_review_timeouts`) invokes
  the adapter's follow-up operation → `agent-exec --resume <thread>` with the
  finish-up prompt, instead of typing into a live TUI.
- **Signal expectations**: `.idle`/`.awaiting`/`.cleared` never arrive; phase
  advancement rides `check_artifact_advances` (artifact presence alone), and
  `.stopped` arrives once per run.

## Acceptance Criteria

- A Codex adapter implementing launch, reuse, follow-up, and expected-signal
  declarations; resolvable at spawn via the T-022-01 resolver (still not
  user-selectable — T-025-01 adds the toggle; a test-only resolution path is
  fine here).
- lisa loop passes its own absolute binary path into the plugin config for
  wrapper invocation.
- A Codex-adapter ticket runs end-to-end in a pane: launch → artifacts advance
  phases → `.stopped` at completion → auto-complete Review; `.error` fails the
  thread promptly (T-022-02 path).
- Claude behaviour untouched (existing tests green).
- Native tests cover the adapter's command construction and the
  reuse-without-handshake path (mirroring `test_build_claude_command*`).

## Notes

- Prompt content: reuse `ticket_prompt` but reference `AGENTS.md` once
  T-025-02 lands; until then the explicit file paths in the prompt suffice
  (Codex reads explicitly-named files regardless — epic Intel B.7).
- Keep the adapter free of provider-quota logic; that's T-026-02.
