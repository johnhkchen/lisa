# T-025-02 Research — AGENTS.md generation + toggle documentation

Descriptive map of the code and docs this ticket touches. No solutions here.

## Ticket in one line

Codex auto-loads `AGENTS.md`; Claude Code auto-loads `CLAUDE.md`. We must (a)
emit an `AGENTS.md` from `lisa init` that cannot drift from `CLAUDE.md`, (b) make
the Codex ticket prompt point at `AGENTS.md` where the Claude prompt points at
`CLAUDE.md`, (c) document the client toggle / Codex prerequisites, and (d) keep
`lisa validate` happy with both context files present. Claude stays the default;
a project that never opts in behaves exactly as today.

## Dependencies (both `done`/present)

- **T-025-01** (`phase: done`) landed the client-selection seam: `[agent].client`
  in `.lisa.toml`, the `lisa loop --client` flag, `ResolvedConfig.client`, and
  the per-client `lisa doctor` branch. So the toggle *mechanism* already exists;
  this ticket documents it and adds the context-file parity.
- **T-023-02** (Codex adapter) is present in `crates/lisa-plugin/src/adapter.rs`
  (`CodexAdapter`) and wired through `resolve_adapter`.

## The shared vocabulary: `lisa_core::client::AgentClient`

`crates/lisa-core/src/client.rs` is the single place both crates parse the client
name. Enum `AgentClient { Claude (default), Codex }` with `parse`, `as_str`,
`VALID`, `Display`. Module doc explicitly frames it as the one shared vocabulary
seam ("one place both readers share, not two ad-hoc parsers"). This is the
natural home for a `context_file()` accessor (`CLAUDE.md` vs `AGENTS.md`).

## The ticket prompt (the parity change)

`crates/lisa-plugin/src/lib.rs:39` `ticket_prompt(ticket_dir, ticket_id)` builds
the RDSPI prompt and **hardcodes `CLAUDE.md`**:

> "Read the ticket at {path}, CLAUDE.md, and docs/knowledge/rdspi-workflow.md. …"

Callers:
- `build_claude_command` (lib.rs:58) — wraps it in the `claude …` launch line.
- `ClaudeCodeAdapter::reuse_prompt` (adapter.rs:162).
- `CodexAdapter::agent_exec_line` (adapter.rs:223) — wraps the *same* prompt into
  `lisa agent-exec "<prompt>"`. **This is where the Codex prompt currently still
  says `CLAUDE.md`** — the bug this ticket fixes.

`finish_up_prompt` (lib.rs:68) references only `review.md`, no context file — no
change needed.

Each adapter is already client-specific (`ClaudeCodeAdapter` vs `CodexAdapter`),
so each can pass the correct context filename into `ticket_prompt`. The adapters
do not currently hold an `AgentClient` value, but `adapter_for_client` knows it.

Tests that pin the prompt text: `test_ticket_prompt_content` (lib.rs:3426,
asserts `CLAUDE.md`), `test_build_claude_command` (lib.rs:3306, asserts
`CLAUDE.md`), and adapter.rs `native_reuse_prompt_matches_free_fn` (341),
`codex_launch_command_shape` (413, asserts `contains(&ticket_prompt(...))`).

## `lisa init` scaffolding — `crates/lisa-cli/src/{init.rs,templates.rs}`

`templates.rs`:
- `generate_claude_md(project)` renders the project-specific `CLAUDE.md`
  (name, type, build/test, source layout, directory conventions, RDSPI pointer).
- Hook script consts, `settings_local_json`, `merge_hooks`, `RDSPI_WORKFLOW`,
  `HOOKS_GUIDE`, `PLUGIN_WASM`.
- No `AGENTS.md` template exists yet.

`init.rs` `plan_init_actions` (200) builds an ordered `Vec<InitAction>`
(CreateDir / CreateFile / UpdateFile / Skip). CLAUDE.md handling (226-237):
**skip if it exists** (never overwrite — see `test_run_init_never_overwrites_claude_md`),
otherwise create from `generate_claude_md`. This is the exact pattern an
`AGENTS.md` action should mirror.

Count-coupled tests: `test_plan_init_actions_empty_dir` asserts **18** non-skip
actions (8 dirs + 10 files); adding a file makes it 19. The inline comment lists
the 10 files. `test_run_init_creates_files` asserts each created path exists.

## `lisa validate` — `crates/lisa-cli/src/init.rs:567` `validate()`

Structure checks require `CLAUDE.md` to exist (592-600) as an **error**. It does
**not** look for `AGENTS.md` at all, so an extra `AGENTS.md` is already accepted
(never rejected). The AC "validate accepts a project with both context files"
therefore needs a *confirming test*, not new required-file logic — making
`AGENTS.md` required would regress every Claude-only project (opposite of the
zero-regression promise). Hook checks iterate a fixed script list; unrelated.

## `lisa doctor` — `crates/lisa-cli/src/doctor.rs`

Already per-client (T-025-01): `build_checks(client)` checks `zellij` + exactly
one agent binary (`claude`/`codex`) via `--version`, so it *reports the installed
codex version* (the ticket's version-pinning note). Codex selection also
pre-seeds directory trust (`pregrant_codex_trust`) and prints the version-volatility
note (`#14345`). No doctor change required — this ticket only documents it.

## Config surface — `crates/lisa-cli/src/config.rs`

`[agent].client` parsed to `AgentConfig.client: Option<String>`, validated via
`AgentClient::parse`, resolved into `ResolvedConfig.client` with precedence
`--client > [agent].client > default(claude)`. `default_config_toml()` already
ships a commented `[agent]` example documenting the toggle. Unknown `[agent]`
keys warn. `main.rs` wires `--client` (parsed up front) and the `Loop` path.

## Docs surface

- `README.md`: Prerequisites list Claude + Zellij only; "What It Does" is
  Claude-centric; config table has `dirs.*` + `scheduling.max_threads` but **no
  `agent.client` row**; CLI reference documents `loop`/`doctor` without `--client`
  or the Codex path. No mention of Codex anywhere user-facing.
- `docs/knowledge/codex-client/06-off-the-shelf-tooling.md:64` "Operational
  aside: `AGENTS.md`" is the authoritative source: Codex reads AGENTS.md
  natively; Claude reads CLAUDE.md; cheapest fix is to also emit AGENTS.md (or
  symlink). Confirms parity intent.
- `docs/PROMPT_CODEX.md`, `docs/ROADMAP.md`, `crates/lisa-cli/src/setup_guide.rs`
  exist; setup_guide emits LLM setup steps (mentions CLAUDE.md only).

## Constraints / assumptions

- **Anti-drift is the hard requirement.** A pointer file (AGENTS.md → CLAUDE.md)
  cannot drift because it duplicates no content; two rendered templates can.
- **Zero regression for Claude.** CLAUDE.md content, the Claude prompt, doctor
  output with no opt-in, and validate's required set must be unchanged.
- **Don't imply >2 clients.** Docs mention only Claude + Codex; ACP is explicitly
  future.
- WASM plugin can't do host I/O; adapters return command strings only (unchanged).
- `ticket_prompt` is a `pub(crate)` fn in the plugin; changing its signature is
  contained to the plugin crate + its tests.
