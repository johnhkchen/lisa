---
id: T-025-01
story: S-025
title: client-selection-config
type: feature
status: open
priority: medium
phase: done
depends_on: [T-022-01]
---

## Context

A discoverable, safe opt-in for the Codex client: a `.lisa.toml` field (e.g.
an `[agent]`/client setting) and/or a `lisa loop` flag, defaulting to Claude,
plumbed through `ResolvedConfig` → layout config map → `PluginConfig` so the
T-022-01 resolver reads it as the **loop-level default**. Per epic Decision 4
and the S-022 constraint, the config shape is a default the later per-ticket
routing (S-026) overrides — not a whole-loop-only switch.

`lisa doctor` (and the pre-loop dependency check in `run_loop`,
`loop_cmd.rs:27`) must check the **selected** client's dependencies instead of
unconditionally requiring `claude` (`check_claude`, `doctor.rs:86`): `codex
--version` when Codex is selected, plus the headless trust pre-seeding the
T-021-01 verdict prescribes.

## Acceptance Criteria

- Client selection in `.lisa.toml` (documented default: claude) parsed into
  `ResolvedConfig` and passed to the plugin via the generated layout's config
  block; `PluginConfig::from_config_map` reads it; the spawn-time resolver
  uses it as the loop default.
- `lisa doctor` and `lisa loop`'s preflight check the selected client only;
  Codex selection checks the codex binary and reports/pre-seeds directory
  trust for unattended `codex exec`.
- No opt-in → identical behaviour and identical doctor output to today.
- Config parse errors produce actionable messages (`lisa validate` covers the
  new field).
- Tests: config parsing, layout plumb-through, doctor branch per client.

## Notes

- Keep the value shape extensible toward `(method, provider, model)` (S-026
  frontmatter uses the same vocabulary) — a bare client name today, but parsed
  through one place both readers share (lisa-core), not two ad-hoc parsers.
