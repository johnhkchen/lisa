# T-025-02 Plan — ordered, verifiable steps

Each step is independently compilable/testable and committable. Verification is
`cargo test --workspace` plus the WASM check (`cargo build -p lisa-plugin
--target wasm32-wasip1 --release`) at the end.

## Step 1 — `AgentClient::context_file()` (lisa-core)

- Add the accessor + doc comment on `AgentClient`.
- Add test `context_file_per_client`.
- Verify: `cargo test -p lisa-core`.
- Commit: "core: add AgentClient::context_file() (CLAUDE.md / AGENTS.md)".

## Step 2 — parameterize `ticket_prompt` (lisa-plugin)

- Import `AgentClient` in lib.rs; add `context_file: &str` param; swap the literal
  for `{context}`.
- `build_claude_command` passes `AgentClient::Claude.context_file()`.
- Fix `test_ticket_prompt_content`; add `test_ticket_prompt_uses_given_context_file`.
- Verify: `cargo test -p lisa-plugin` (native). Expect the two adapter call sites
  to fail to compile until Step 3 — so Steps 2–3 land in one commit if needed;
  compile-gate by doing 2+3 together.
- **Sequencing note:** because `adapter.rs` calls `ticket_prompt`, the crate will
  not compile between Steps 2 and 3. Treat Steps 2 and 3 as a single commit unit.

## Step 3 — per-adapter context file (lisa-plugin/adapter.rs)

- `ClaudeCodeAdapter::reuse_prompt` → Claude context file.
- `CodexAdapter::agent_exec_line` → Codex context file.
- Update `native_reuse_prompt_matches_free_fn`, `codex_launch_command_shape`.
- Add `codex_prompt_references_agents_not_claude`.
- Verify: `cargo test -p lisa-plugin`.
- Commit (with Step 2): "plugin: Codex ticket prompt reads AGENTS.md, Claude reads
  CLAUDE.md".

## Step 4 — AGENTS.md template (lisa-cli/templates.rs)

- Add `AGENTS_MD` const + doc.
- Add `test_agents_md_points_to_claude`.
- Verify: `cargo test -p lisa-cli templates`.
- Commit: "cli: add AGENTS.md pointer template".

## Step 5 — scaffold AGENTS.md in init + validate acceptance (lisa-cli/init.rs)

- Add the AGENTS.md `InitAction` block (skip-if-exists) after CLAUDE.md.
- Update `test_plan_init_actions_empty_dir` (18→19, comment), extend
  `test_run_init_creates_files`, add `test_run_init_never_overwrites_agents_md`.
- Add `test_validate_accepts_both_context_files` (no validate logic change).
- Verify: `cargo test -p lisa-cli`.
- Commit: "cli: lisa init scaffolds AGENTS.md; validate accepts both context files".

## Step 6 — README Codex documentation

- Prerequisites note, config-table `agent.client` row, "Codex client
  (experimental)" section, CLI-reference `--client` + doctor note.
- Verify: prose only; re-read for the four documented items (toggle, prereqs +
  version caveat, trust seeding, wrapper behaviour) and the "two natives only"
  guard.
- Commit: "docs: document the Codex client toggle, prerequisites, and wrapper".

## Step 7 — full verification

- `cargo test --workspace` (all native tests green).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` (WASM still
  builds — the plugin change is signature-only).
- `cargo clippy --workspace` if available (no new warnings).
- Manual doc read-through against the ACs.

## Testing strategy

- **Unit** covers every behavioural change: the accessor, the prompt
  parameterization (both filenames), the AGENTS.md content, the init action count
  and never-overwrite, and validate-with-both.
- **Parity proof**: `codex_prompt_references_agents_not_claude` asserts the Codex
  line says `AGENTS.md` and the Claude line says `CLAUDE.md` — the core AC.
- **No-regression proof**: `test_build_claude_command` and
  `native_reuse_prompt_matches_free_fn` still assert `CLAUDE.md`, so the Claude
  default path is byte-identical.
- **Docs** have no automated test; verified by read-through (acceptable — the AC
  is documentation existence/accuracy, and the mechanism it documents is already
  test-covered by T-025-01).

## Acceptance-criteria trace

| AC | Step(s) | Verification |
|----|---------|--------------|
| AGENTS.md generated, cannot drift, has RDSPI ref | 4, 5 | templates + init tests |
| Codex prompt → AGENTS.md, Claude → CLAUDE.md | 1–3 | adapter/plugin tests |
| README documents toggle/prereqs/version/trust/wrapper/default | 6 | read-through |
| Docs don't imply >2 clients | 6 | read-through |
| validate accepts both context files | 5 | init test |

## Rollback

Each commit is self-contained. Reverting Step 6 leaves code intact; reverting 4–5
removes AGENTS.md scaffolding without touching prompt parity; reverting 1–3 restores
the hardcoded `CLAUDE.md` prompt. No migrations, no persisted-format changes.
