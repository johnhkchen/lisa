# T-025-02 Progress

Status: **complete**. All plan steps executed; `cargo test --workspace` green
(218 cli + 118 core + 196 plugin), WASM plugin builds, clippy clean on all three
crates.

## Completed steps

- **Step 1 — `AgentClient::context_file()`** (`lisa-core/src/client.rs`).
  Added the accessor (`Claude → CLAUDE.md`, `Codex → AGENTS.md`) + doc + test
  `context_file_per_client`.
- **Steps 2 & 3 — prompt parity** (`lisa-plugin/src/{lib.rs,adapter.rs}`).
  `ticket_prompt` now takes `context_file: &str` and interpolates it in place of
  the hardcoded `CLAUDE.md`. `build_claude_command` and `ClaudeCodeAdapter` pass
  `AgentClient::Claude.context_file()` (output unchanged); `CodexAdapter` passes
  `AgentClient::Codex.context_file()`. Landed as one compile unit (adapter.rs
  calls the changed fn). Added `test_ticket_prompt_uses_given_context_file` and
  `codex_prompt_references_agents_not_claude`; updated the three existing tests
  that call `ticket_prompt`.
- **Step 4 — AGENTS.md template** (`lisa-cli/src/templates.rs`). Added the
  `AGENTS_MD` pointer const + `test_agents_md_points_to_claude`.
- **Step 5 — init scaffolding + validate acceptance** (`lisa-cli/src/init.rs`).
  Added the AGENTS.md `InitAction` (skip-if-exists, unconditional). Updated
  `test_plan_init_actions_empty_dir` (18→19 + comment), extended
  `test_run_init_creates_files`, added `test_run_init_never_overwrites_agents_md`
  and `test_validate_accepts_both_context_files`. No change to validate's
  required-file set (AGENTS.md is accepted, not required — zero regression).
- **Step 6 — README** (`README.md`). Prerequisites note, `agent.client` config
  row, new "Codex client (experimental)" section (toggle + precedence,
  prerequisites + version-pinning caveat, trust pre-seeding, in-pane wrapper
  behaviour, Claude-default-unchanged, two-natives-only guard), and CLI-reference
  updates for `loop --client` / `doctor` / `init`.
- **Step 7 — verification**. `cargo test --workspace`, WASM release build, and
  `cargo clippy` on the three crates all pass with no warnings.

## Deviations from plan

None material. Steps 2 and 3 were committed as one unit as the plan anticipated
(the crate does not compile between them). No new dependencies, no config-schema
or layout-map changes (client plumb-through already shipped in T-025-01).

## Deliberately out of scope

- `setup_guide.rs` (LLM onboarding text) still mentions only `CLAUDE.md`. The AC
  is satisfied by README; touching setup-guide was called optional in Design and
  left to keep the diff tight. Flagged in review.
- No symlink / rendered-twin AGENTS.md (Design D1 rejected both in favour of the
  pointer).
