---
id: S-022
title: adapter-interface
status: open
---

## Adapter interface & per-pane-resolvable selection (no behaviour change)

Extract the Claude-Code-specific behaviour behind a single **adapter interface**
resolved **per ticket at spawn time**, as a provable no-op refactor. This is the
seam that makes the three-leg portfolio (native Claude, native Codex, ACP) and
per-pane `(provider, model)` routing possible — see
[08 · Design thesis §5](../../knowledge/codex-client/08-design-thesis.md) and
the epic's S-022 needs.

The interface must own everything that differs per integration method:

- launch command construction (today `build_claude_command`, `lib.rs:53`)
- session reuse/reset (today the `/clear` + `.cleared` handshake; for Codex,
  reuse = a fresh `codex exec`)
- follow-up injection (today `finish_up_prompt` typed into a live TUI; for
  Codex, `codex exec resume` — a spawned command, not keystrokes)
- expected signal set (`.idle`/`.awaiting` are Claude-only; the normalized
  contract is `.heartbeat`/`.stopped`/`.error` + usage/cost)
- usage/cost extraction

The scheduler consumes only normalized signals and stays client-agnostic. The
scheduler-side gap this story also closes: there is **no `.error` signal
consumer today** — the normalized contract requires one.

### Constraints

- Native Claude is the only adapter implemented here; with no opt-in, every
  existing behaviour is byte-for-byte unchanged (existing tests prove it).
- The interface must accommodate native Codex and a future ACP adapter without
  redesign (per epic Decision 4: Codex first, adapter-shaped for extension).
- No whole-loop-only assumption: selection resolves `(method, provider, model)`
  per ticket at spawn even while the MVP sets it loop-wide.

### Tickets

- **T-022-01** — Adapter interface extraction (no-op refactor, Claude adapter)
- **T-022-02** — `.error` signal consumer + normalized signal contract
