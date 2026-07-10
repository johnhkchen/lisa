# 09 · Native-TUI parity decision

**Decision date:** 2026-07-09
**Verified client:** Codex CLI 0.144.0

## Root cause of the visual disparity

Zellij was not rendering the two providers differently. Lisa selected two
different Codex products:

- Claude panes launched `claude <prompt>`, leaving Claude Code's interactive TUI
  resident in the pane.
- Codex panes launched `lisa agent-exec <prompt>`. That wrapper launched
  `codex exec --json`, parsed JSONL lifecycle events, printed a minimal text
  projection, and exited after each turn.

The unreadable pane was therefore an adapter decision, not a terminal limitation.
The same adapter choice also forced Codex reuse into fresh processes and review
follow-ups into `exec resume`, while Claude reused its live composer through
`/clear` and Zellij input injection.

## Why the wrapper was originally chosen

The research packet was pinned to Codex 0.142.5 and optimized for unattended
signal reliability over native presentation. Its main concerns were legitimate:

1. Interactive Stop and SessionStart hook delivery had open regressions.
2. Hook subprocess inheritance of Lisa's pane-id environment variable was not
   documented.
3. Codex's paste-burst handling made bulk text followed by Enter look fragile.
4. `PostToolUse` has no cadence guarantee and cannot prove liveness during a long
   tool call or reasoning-only stretch.
5. The safer `workspace-write` sandbox protects `.git`, conflicting with RDSPI's
   requirement that an agent commit its completed work.

`codex exec --json` resolved the first three by giving Lisa deterministic process
exit and machine-readable events. It also made the permission posture look more
controlled. The cost was the exact disparity this change removes: no first-class
chat UI, no resident composer, and a home-grown renderer coupled to a drifting
event schema.

## Re-verification on 0.144.0

An isolated PTY smoke test used project-local `.codex/hooks.json`, a unique
`LISA_PANE_ID`, and Lisa's two-stage input pattern (write text, delay, write CR).
It verified:

| Contract | Result |
|---|---|
| `codex [PROMPT]` starts the native TUI and submits the prompt | pass (`INITIAL_OK`) |
| Stop hook runs and inherits `LISA_PANE_ID` | pass; payload included transcript, cwd, model, and turn ids |
| `/clear` starts a fresh chat and emits `SessionStart` with `source: clear` | pass |
| Text then delayed Enter submits a follow-up after clear | pass (`FOLLOWUP_OK`) |
| Stop transcript exposes cumulative `event_msg/token_count` usage | pass; `lisa capture-usage` produced the Codex usage artifact |

The directory-trust prompt still appears before the first session in an untrusted
repository, even with hook-trust bypass. Lisa's existing trust pregrant remains
necessary. `--dangerously-bypass-hook-trust` is also retained because Lisa
generated and validated these project hook definitions.

## Current decision

The loop's default Codex adapter now launches:

```text
codex --dangerously-bypass-approvals-and-sandbox \
  --dangerously-bypass-hook-trust [--model MODEL] "PROMPT"
```

Full access is deliberate parity with Claude's existing
`--dangerously-skip-permissions` launch and is required for the RDSPI commit step;
Codex `workspace-write` alone cannot write `.git`. Lisa pregrants directory trust,
generates/merges `.codex/hooks.json`, uses Stop/PostToolUse/SessionStart to emit
normalized signals, sends `/clear` before pane reuse, and types review follow-ups
into the resident composer. The JSON wrapper remains an explicit headless and
diagnostic fallback.

## Remaining asymmetries

This is presentation and chaining parity, not a claim that the clients expose
identical lifecycle APIs:

- Codex has no Claude-style `idle_prompt` notification, so its adapter does not
  advertise `.idle`.
- Codex has no equivalent Lisa can safely bind to Claude's
  `PreToolUse[AskUserQuestion]`, so it does not advertise `.awaiting`.
- `PostToolUse` remains event-driven rather than periodic. A long-running tool or
  reasoning-only interval can still be silent; the existing timeout policy is the
  safety net, not proof of perfect heartbeat parity.
- Hook behavior is a version-sensitive integration surface. `lisa doctor` reports
  the installed Codex version, and this contract should be smoked again after a
  Codex upgrade.
