---
id: S-023
title: codex-launch-lifecycle
status: open
---

## Codex client: launch & session lifecycle parity

Codex sessions launch, reset, and reuse panes to the same standard as Claude,
via the decided driving model: a **wrapper subcommand inside `lisa-cli`**
(epic Decision 5) that runs `codex exec --json`, writes lisa's signal files
(`.heartbeat`/`.stopped`/`.error`), and renders a chunked human-readable view
of the conversation to the pane (chunked is acceptable per Decision 1 — no
app-server needed).

Key mechanics (see [05](../../knowledge/codex-client/05-bridging-the-discrepancy.md)):

- lisa types `LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec …` into a
  fresh pane instead of `claude …`; the wrapper inherits the env vars, so pane
  attribution is deterministic — no hooks, no TUI injection.
- Event → signal mapping: `item.*` → `.heartbeat`; `turn.completed` + exit 0 →
  `.stopped`; `turn.failed`/non-zero exit → `.error`. Turn events + process
  exit are authoritative; item statuses are best-effort heartbeat only.
- Session reuse = a fresh `codex exec` per ticket (no `/clear` handshake);
  follow-up prods (review finish-up) = `codex exec resume <thread_id>`.
- The pane invocation uses the absolute lisa binary path passed through the
  layout config (`current_exe()` at `lisa loop` time) — no PATH assumption.

### Tickets

- **T-023-01** — `lisa agent-exec` wrapper subcommand (JSONL → signals + pane renderer)
- **T-023-02** — Codex adapter in the plugin (launch, reuse, follow-up via resume)
