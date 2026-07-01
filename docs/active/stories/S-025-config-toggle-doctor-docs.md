---
id: S-025
title: config-toggle-doctor-docs
status: open
---

## Config toggle, environment doctoring, and docs

Users need a discoverable, safe way to opt into Codex and to know their
environment is ready. Claude Code stays the default; a project that never opts
in behaves exactly as today.

### Needs (from the epic)

- A documented client selection: a `.lisa.toml` field (and/or `lisa loop`
  flag), defaulting to Claude, plumbed through to the plugin config so the
  adapter resolution seam (S-022) can read it. The config shape must not bake
  in whole-loop-only selection — it is the loop-level *default* that per-ticket
  routing (S-026) later overrides.
- `lisa doctor` checks dependencies for the **selected** client (`codex
  --version`, trust pre-seeding for headless runs) instead of unconditionally
  requiring `claude` (`doctor.rs:86` today).
- Codex projects get `AGENTS.md` with content equivalent to the generated
  `CLAUDE.md` (Codex reads `AGENTS.md` natively; Claude still reads `CLAUDE.md`
  — emit both, see [06 §AGENTS.md](../../knowledge/codex-client/06-off-the-shelf-tooling.md)).
- README / setup guide document the toggle, Codex prerequisites, and wrapper
  behaviour, without implying support for clients beyond these two natives.

### Tickets

- **T-025-01** — Client selection config + doctor checks per selected client
- **T-025-02** — AGENTS.md generation + toggle documentation
