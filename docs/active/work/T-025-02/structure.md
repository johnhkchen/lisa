# T-025-02 Structure — file-level blueprint

Shape of the change. Five files modified, no files created/deleted (besides the
work artifacts). Ordering matters: `lisa-core` first (both crates depend on it),
then plugin, then CLI, then docs.

## 1. `crates/lisa-core/src/client.rs` — add `context_file()`

Add an accessor on `AgentClient`, next to `as_str`:

```rust
/// The project-context filename this client auto-loads: Claude Code reads
/// `CLAUDE.md`; Codex reads `AGENTS.md` (a Linux-Foundation standard). The RDSPI
/// ticket prompt points each agent at the file its own client loads.
pub fn context_file(&self) -> &'static str {
    match self {
        AgentClient::Claude => "CLAUDE.md",
        AgentClient::Codex => "AGENTS.md",
    }
}
```

Tests (append to existing `mod tests`):
- `context_file_per_client`: Claude → `CLAUDE.md`, Codex → `AGENTS.md`.

Public interface: additive, no breakage.

## 2. `crates/lisa-plugin/src/lib.rs` — parameterize `ticket_prompt`

- Add import: `use lisa_core::client::AgentClient;` (currently only `types` is
  imported from lisa_core).
- Change signature:
  `pub(crate) fn ticket_prompt(ticket_dir: &Path, ticket_id: &str, context_file: &str) -> String`
  and replace the literal `CLAUDE.md` in the format string with `{context}` bound
  to `context_file`.
- `build_claude_command` (only in-crate caller besides adapters) passes
  `AgentClient::Claude.context_file()` → output unchanged (`CLAUDE.md`).
- Tests:
  - `test_ticket_prompt_content` (3426): pass a context file (e.g.
    `AgentClient::Claude.context_file()`); keep the `CLAUDE.md` assertion.
  - Add `test_ticket_prompt_uses_given_context_file`: calling with `"AGENTS.md"`
    yields a prompt containing `AGENTS.md` and **not** `CLAUDE.md`.
  - `test_build_claude_command` (3306) stays green (still `CLAUDE.md`).

## 3. `crates/lisa-plugin/src/adapter.rs` — per-adapter context file

`use lisa_core::client::AgentClient;` already present.

- `ClaudeCodeAdapter::reuse_prompt`:
  `ticket_prompt(ctx.ticket_dir, ctx.ticket_id, AgentClient::Claude.context_file())`.
- `CodexAdapter::agent_exec_line`:
  `ticket_prompt(ctx.ticket_dir, ctx.ticket_id, AgentClient::Codex.context_file())`.
- Update tests that call `ticket_prompt` with two args:
  - `native_reuse_prompt_matches_free_fn` (341) → add `AgentClient::Claude.context_file()`.
  - `codex_launch_command_shape` (413) → compare against
    `ticket_prompt(dir, "T-042-01", AgentClient::Codex.context_file())`; add an
    assertion that the command contains `AGENTS.md`.
- Add `codex_prompt_references_agents_not_claude`: the Codex launch command
  contains `AGENTS.md` and does not contain `CLAUDE.md`; the Claude launch
  command contains `CLAUDE.md` and not `AGENTS.md` (parity proof).

## 4. `crates/lisa-cli/src/templates.rs` — AGENTS.md pointer + generator

- Add const:

```rust
/// The `AGENTS.md` pointer file scaffolded by `lisa init`. Codex auto-loads
/// `AGENTS.md`; Claude Code auto-loads `CLAUDE.md`. To make the two impossible to
/// drift, AGENTS.md carries no project body — it points at `CLAUDE.md` as the
/// single source of truth and repeats only the RDSPI workflow reference.
pub const AGENTS_MD: &str = "# AGENTS.md\n\nThis project's agent context lives in \
[CLAUDE.md](CLAUDE.md) — the single source of truth for every agent client \
(Claude Code reads `CLAUDE.md`; Codex reads this `AGENTS.md`). Read `CLAUDE.md` \
first.\n\nThe RDSPI workflow definition is in docs/knowledge/rdspi-workflow.md \
and is injected into agent context by lisa automatically.\n";
```

  (Written as a plain `&str` const so the body is a fixed pointer — nothing
  project-specific to drift.)
- Tests: `test_agents_md_points_to_claude` — contains `CLAUDE.md`,
  `rdspi-workflow.md`, and the `AGENTS.md` heading; does not restate build/source
  sections.

## 5. `crates/lisa-cli/src/init.rs` — scaffold AGENTS.md

- In `plan_init_actions`, immediately after the CLAUDE.md block (~237), add the
  mirror block for `AGENTS.md`: skip if exists (never overwrite), else
  `CreateFile { path, content: templates::AGENTS_MD.to_string() }`.
- Count-coupled test updates:
  - `test_plan_init_actions_empty_dir`: `18` → `19`; update the "10 files"
    comment to "11 files" and add `AGENTS.md` to the list.
  - `test_run_init_creates_files`: add `assert!(dir.path().join("AGENTS.md").exists())`
    and a content assertion (`contains("CLAUDE.md")`).
  - Add `test_run_init_never_overwrites_agents_md` mirroring the CLAUDE.md one.
- `validate()` (structure section): **no change to required files**. Add test
  `test_validate_accepts_both_context_files`: a valid setup that also writes
  `AGENTS.md` still returns `Ok`.

## 6. `README.md` — Codex documentation

- Prerequisites: note Codex as an alternative agent client to Claude Code.
- Configuration table: add an `agent.client` row (default `claude`).
- New section **"Codex client (experimental)"** after Configuration: toggle,
  prerequisites + version-pinning caveat, trust pre-seeding, in-pane wrapper
  behaviour (`lisa agent-exec`, fresh-exec reuse, reads `AGENTS.md`), Claude-stays-
  default, and an explicit "only Claude + Codex; ACP is future" line.
- CLI reference: `lisa loop --client claude|codex`; note `lisa doctor` checks the
  selected client and reports its version.

## Ordering & interfaces

Compile order lisa-core → plugin → cli is satisfied by the dependency graph;
changing `context_file()` first means the plugin/cli edits reference a symbol that
already exists. The only cross-crate contract touched is the additive
`AgentClient::context_file()`. The `ticket_prompt` signature is `pub(crate)`;
blast radius is the plugin crate. No serialized config or layout-map field
changes (client plumb-through already shipped in T-025-01).

## Test inventory (new)

- core: `context_file_per_client`
- plugin: `test_ticket_prompt_uses_given_context_file`,
  `codex_prompt_references_agents_not_claude` (+ edits to 3 existing)
- templates: `test_agents_md_points_to_claude`
- init: `test_run_init_never_overwrites_agents_md`,
  `test_validate_accepts_both_context_files` (+ edits to 2 existing)
