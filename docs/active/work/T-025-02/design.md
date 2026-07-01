# T-025-02 Design — AGENTS.md generation + toggle documentation

Decisions, grounded in Research. Four sub-problems: (1) how AGENTS.md is
generated, (2) how the Codex prompt points at it, (3) validate acceptance, (4)
docs.

## D1 — AGENTS.md content: pointer file vs rendered twin

The AC allows "one template rendered to both, or a pointer file — so they cannot
drift".

**Options**
- **(a) Pointer file** — `AGENTS.md` is a short static file that names
  `CLAUDE.md` as the single source of truth and carries the RDSPI reference.
- **(b) Render `generate_claude_md` to both** — two files with identical body.
- **(c) Symlink `AGENTS.md → CLAUDE.md`** (doc 06 mentions it).

**Assessment against codebase reality**
- (b) duplicates the full project body. The instant a user edits `CLAUDE.md`
  (the template literally ends with a "TODO: add a one-line project description"),
  `AGENTS.md` drifts. It also doubles the count-coupled init logic and would need
  its own never-overwrite handling. Directly violates "cannot drift".
- (c) symlinks are not portable (Windows, some CI checkouts, tarball releases),
  and `init.rs`'s `InitAction` model writes file *content*, not symlinks — new
  action variant + platform branches for marginal benefit.
- (a) duplicates **zero** body content, so drift is structurally impossible. It
  mirrors the existing `CLAUDE.md` skip-if-exists action exactly (one more
  `InitAction`). Codex, pointed at `AGENTS.md` by its prompt, reads the pointer
  and then `CLAUDE.md`; both agents converge on one authored file.

**Decision: (a) pointer file.** Strongest anti-drift guarantee, smallest diff,
reuses the proven CLAUDE.md init pattern. Content is a `pub const AGENTS_MD` in
`templates.rs` (static — no per-project fields to drift) that (i) links to
`CLAUDE.md` as the source of truth and (ii) includes the RDSPI workflow reference
line verbatim, satisfying the AC "including the RDSPI workflow reference".

## D2 — Unconditional vs Codex-only emission

The story says "Codex projects get AGENTS.md"; the ticket AC says "`lisa init`
… generate an `AGENTS.md`".

**Options**: emit always, or only when `[agent].client = codex`.

Gating on the config means `init` must load `.lisa.toml` and branch, and a
project that *later* switches to Codex would lack `AGENTS.md` until re-init. A
pointer file is inert and harmless for a Claude-only project (Claude never reads
it). Emitting unconditionally keeps `init` config-independent, makes the toggle a
one-line `.lisa.toml` edit with no re-scaffold, and the "cannot drift" pointer
makes it costless.

Zero-regression check: the promise (T-025-01 AC) is about *runtime behaviour and
doctor output* with no opt-in — Claude still reads `CLAUDE.md`, the Claude prompt
is unchanged, doctor is unchanged. A new inert pointer file does not change any
behaviour. **Decision: emit unconditionally**, handled exactly like `CLAUDE.md`
(skip if present, never overwrite authored content).

## D3 — Where the context filename is chosen (the prompt parity)

`ticket_prompt` hardcodes `CLAUDE.md`. Two shapes to make it client-aware:

- **(a) Parameterize `ticket_prompt(ticket_dir, ticket_id, context_file)`** and
  have each adapter pass its filename.
- **(b) Two prompt functions** (`claude_ticket_prompt` / `codex_ticket_prompt`).

(b) duplicates the entire prompt body — the same drift problem one level up, and
the prompt is long. (a) is a one-argument change; the body stays single-sourced.

**Where does the filename come from?** Put it on the shared vocabulary type:
`AgentClient::context_file() -> &'static str` (`CLAUDE.md` / `AGENTS.md`) in
`lisa-core/client.rs`. This is consistent with the module's stated role as the
one place the client vocabulary lives, is unit-testable in isolation, and keeps
the plugin from sprinkling magic strings. Each adapter passes its client's
`context_file()`:
- `ClaudeCodeAdapter` (via `build_claude_command` and `reuse_prompt`) →
  `AgentClient::Claude.context_file()` = `CLAUDE.md` (unchanged output).
- `CodexAdapter::agent_exec_line` → `AgentClient::Codex.context_file()` =
  `AGENTS.md`.

**Decision: (a) + `AgentClient::context_file()`.** `build_claude_command`'s
output is byte-identical (still `CLAUDE.md`), so the Claude leg is a no-op proof;
only the Codex wrapper line changes, which is the intended fix.

## D4 — validate accepts both context files

Research showed `validate` already never rejects `AGENTS.md` (it only *requires*
`CLAUDE.md`). Making `AGENTS.md` required would error on every existing
Claude-only project — a regression. **Decision: no new required-file check.** Add
a test that a project containing *both* `CLAUDE.md` and `AGENTS.md` validates
clean, locking in the AC and guarding against a future over-eager check. `lisa
loop`'s preflight (`loop_cmd.rs:12`) also only requires `CLAUDE.md`; unchanged.

## D5 — Documentation

Add a **"Codex client (experimental)"** section to `README.md` plus targeted
edits, all grounded in the shipped mechanism (T-025-01) so docs match code:

- **Toggle**: `[agent] client = "codex"` in `.lisa.toml` and/or
  `lisa loop --client codex`; default is `claude`; precedence flag > config >
  default. Add an `agent.client` row to the config table.
- **Prerequisites**: `codex` binary (`npm i -g @openai/codex`); **version-pinning
  caveat** — Codex's CLI surface drifts (doc 04 / issue #14345), so pin/track a
  tested version and note `lisa doctor` reports the installed one.
- **Trust pre-seeding**: unattended `codex exec` needs directory trust; `lisa
  doctor` / `lisa loop` pre-seed `trust_level = "trusted"` in
  `$CODEX_HOME/config.toml`; `--dangerously-bypass-approvals-and-sandbox` is the
  escape hatch.
- **Wrapper behaviour in the pane**: a Codex ticket runs `lisa agent-exec` (wraps
  `codex exec --json`) instead of the Claude TUI; reuse is a fresh exec (no
  `/clear`); it reads `AGENTS.md`.
- **Claude default unchanged**; explicitly state only the two native clients are
  supported and ACP/others are future — **do not imply broader support**.
- Update the Prerequisites list to mark Codex as an alternative to Claude, and
  the CLI reference (`loop --client`, `doctor` checks the selected client).

`default_config_toml()` already documents the `[agent]` toggle (commented) — no
change. `setup_guide.rs` is LLM-onboarding for the *current* project and is not
required by the AC; leaving it out keeps scope tight (noted in review).

## Rejected / out of scope

- Symlinks (D1c). Rendered twin (D1b). Codex-only gated emission (D2).
- Making `AGENTS.md` a validate/loop hard requirement (D4) — regression.
- Changing `finish_up_prompt` — it names no context file.
- New doctor logic — T-025-01 already reports the codex version + trust.

## Risk / blast radius

`ticket_prompt` signature change is `pub(crate)`, contained to the plugin crate +
its tests and the two adapters. `AgentClient::context_file()` is additive. init
adds one action (touches two count-coupled tests). README is docs-only. No public
API or config-schema change; no behavioural change on the Claude default path.
