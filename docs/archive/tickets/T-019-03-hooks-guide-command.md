---
id: T-019-03
story: S-019
title: hooks-guide-command
type: feature
status: open
priority: medium
phase: done
depends_on: [T-019-01]
---

## Context

Add a `lisa hooks-guide` subcommand that prints an agent-facing guide for setting
up a project's Claude Code hooks — what each hook does, the `.lisa/signals/`
contract, and how to customize `on-notify` (e.g. wire it to ntfy.sh). The goal is
that an agent dropped into an arbitrary project can run `lisa hooks-guide`, read it,
and set up (or repair) hooks correctly — complementing `lisa init`, which scaffolds
them automatically.

Runs last in the S-019 chain (after T-019-01, which follows T-019-02) so the guide
documents the final, working hook set — the `on-notify` hook and attention
`Notification` binding (T-019-02) plus the plugin fire paths (T-019-01) — and so the
`templates.rs` edits don't collide with the earlier tickets.

Touches `crates/lisa-cli/src/main.rs`, a new handler module, a new embedded doc, and
one const in `templates.rs`.

Key anchors (verify before editing):
- clap `Commands` enum — `SetupGuide` variant at `main.rs:52-57` (template to copy;
  clap derives `HooksGuide` → `hooks-guide` automatically).
- Dispatch arm — `main.rs:117-123`; `resolve_path` — `main.rs:149-157`; `mod` list —
  `main.rs:1-8`.
- Embed convention — `include_str!("../data/<name>.md")` from `templates.rs:4`
  (`RDSPI_WORKFLOW`). Embed **source** lives at `crates/lisa-cli/data/`, NOT
  `docs/knowledge/` (the latter is an init output target, not reachable at runtime).
- Existing guide handler shape — `setup_guide::run_setup_guide(root: &Path)`,
  `setup_guide.rs:267-271`.

## Acceptance Criteria

- New embedded doc at `crates/lisa-cli/data/hooks-guide.md` covering:
  - The four lifecycle hooks (idle/stop/clear/heartbeat), what Claude Code event each
    binds to, and the signal file each writes — and that the plugin reads + deletes them.
  - The `on-notify` user hook: the `on-notify <event> [detail]` contract, the full env
    var list (per S-019), the `complete` vs `attention` events, and the `test -x` opt-in
    model. Include a copy-paste ntfy.sh example and the
    `cp on-notify.sample on-notify && chmod +x on-notify` step.
  - How `lisa init` scaffolds all of this, and how to set hooks up by hand in a project
    that wasn't `lisa init`'d (the manual `.claude/settings.local.json` + `.lisa/hooks/`
    layout).
  - That lisa never depends on ntfy or any transport — the hook is project-owned.
- `pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");` added next to
  `templates.rs:4`.
- New `hooks_guide` module with `pub fn run_hooks_guide() -> Result<(), String>` that
  prints `templates::HOOKS_GUIDE` to stdout. (No project path needed — pure dump. If you
  prefer consistency with `setup-guide`, an optional `--path` that contextualizes the
  guide is acceptable but not required.)
- `HooksGuide` variant added to `Commands`, `mod hooks_guide;` declared, and a dispatch
  arm added mirroring `SetupGuide` (`main.rs:117-123`).
- `lisa hooks-guide` runs and prints the guide; exit code 0.
- A test asserting the command's output is non-empty and contains the `on-notify`
  contract marker (e.g. the string `on-notify` and `LISA_EVENT`).
- `just check` passes.

## Implementation notes

- Keep `crates/lisa-cli/data/hooks-guide.md` and any human-facing copy in
  `docs/knowledge/` in sync if you choose to also write a `docs/knowledge/` copy
  (optional — only `data/` is compiled in). Do not assume `docs/knowledge/` is readable
  at runtime.
- The guide is read by agents, so write it as instructions an agent can act on directly
  (concrete file paths, exact commands), in the style of `docs/knowledge/lisa-loop-setup-guide.md`.
