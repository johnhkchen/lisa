# T-025-01 · Review — Client selection config + doctor per client

Handoff for a human reviewer. What changed, test coverage, and open concerns.

## What changed

A `.lisa.toml [agent] client` field + `lisa loop --client` flag (default Claude)
now plumbs through the full config chain to the T-022-01 adapter resolver, and
`lisa doctor` / the loop preflight check the **selected** client's dependencies
(codex binary + directory-trust pre-seed) instead of unconditionally requiring
`claude`.

### Files created
- `crates/lisa-core/src/client.rs` — `AgentClient` enum + the **single shared
  parser** (`parse`/`as_str`), the one place both readers (CLI + plugin) parse a
  client name. The extension seam toward S-026 `(method, provider, model)`.

### Files modified
- `crates/lisa-core/src/lib.rs` — `pub mod client;`.
- `crates/lisa-core/src/types.rs` — `PluginConfig.client: AgentClient` (default
  Claude), read leniently in `from_config_map`.
- `crates/lisa-plugin/src/adapter.rs` — `adapter_for_client`; `resolve_adapter` /
  `resolve_adapter_or_native` take `default_client`; documented Codex placeholder.
- `crates/lisa-plugin/src/lib.rs` — four resolver call sites pass
  `self.config.client`.
- `crates/lisa-cli/src/config.rs` — `AgentConfig`, `LisaConfig.agent`,
  `ResolvedConfig.client`, `resolve_config` third param, `[agent]` validation,
  commented `[agent]` in `default_config_toml`.
- `crates/lisa-cli/src/doctor.rs` — `check_codex`, client-parametric
  `build_checks`/`check_required_deps`, `codex_home`, `pregrant_codex_trust[_in]`,
  client-aware `run_doctor`.
- `crates/lisa-cli/src/loop_cmd.rs` — preflight on selected client + codex trust
  seed; `generate_layout` emits `client`.
- `crates/lisa-cli/src/main.rs` — `--client` flag, fail-fast parse, threaded into
  `resolve_config`.
- `crates/lisa-cli/src/{init,status}.rs` — `resolve_config` 3-arg call sites.

## Acceptance-criteria mapping

1. **Client selection parsed → ResolvedConfig → layout → PluginConfig → resolver
   as loop default.** ✔ Verified end-to-end: `lisa loop --dry-run` emits
   `client "claude"`, `--client codex` emits `client "codex"`; `from_config_map`
   reads it; `resolve_adapter` branches on `self.config.client`.
2. **doctor + loop preflight check the selected client; Codex checks the codex
   binary + reports/pre-seeds trust.** ✔ `build_checks(Codex)` checks
   `codex --version`, not `claude`; `run_doctor`/preflight seed
   `[projects."<abs>"] trust_level = "trusted"` into `$CODEX_HOME/config.toml`.
3. **No opt-in → identical behaviour + identical doctor output.** ✔ Default is
   Claude everywhere; the claude checks/sections are byte-unchanged; the codex
   trust section only prints when Codex is selected (verified: no codex/client
   lines on a config-less project). The T-022-01 no-op tests
   (`test_build_claude_command*`, transitions) pass **unmodified**.
4. **Config parse errors actionable; `lisa validate` covers the field.** ✔
   `validate_config` rejects an unknown client with
   `unknown client 'gpt'; valid clients: claude, codex`, surfaced by both
   `lisa validate` and `lisa loop` (shared `load_config`).
5. **Tests: config parsing, layout plumb-through, doctor branch per client.** ✔
   see below.

## Test coverage

- **New tests: 26** (core client 7, PluginConfig 3, adapter 1 + 2 updated, config
  8, loop_cmd 2, doctor 6). **Total workspace: 517 passing, 0 failing.**
- `cargo clippy --workspace` clean; WASM plugin builds.
- **Gaps (intentional):**
  - `run_doctor` has no success-path integration test — it shells out to real
    `zellij`/`claude`/`codex` binaries, so it stays covered by the pure
    `build_checks`/`format_report`/seed-writer unit tests plus manual smoke runs.
    This matches the pre-existing test strategy (no `run_doctor` test existed).
  - The clap `--client` wiring is exercised by manual `lisa loop --dry-run` runs,
    not an automated test (consistent with the crate's existing CLI-arg coverage).
  - No test asserts codex *accepts* the trust seed — unverifiable without the
    binary and version-specific (#14345); tests assert the file we write.

## Open concerns / notes for the reviewer

1. **Codex selection does not yet route to Codex behaviour.** By design: the
   Codex `AgentAdapter` is **T-023-02**. Until it lands, `client = "codex"`
   validates, seeds trust, and checks the codex binary, but the loop still
   launches native Claude (the `adapter_for_client` Codex arm falls back to
   `ClaudeCodeAdapter`, per epic Decision 3). A reviewer expecting codex to
   *run* here should note the scope boundary — this ticket is config + doctor.
   T-023-02 fills the one documented arm; no caller changes.
2. **Trust-seed intel is [PROVISIONAL] (T-021-01, never run against live codex).**
   The seed writes exactly what the T-021-01 verdict prescribes
   (`$CODEX_HOME/config.toml`, `[projects."<abs>"] trust_level = "trusted"`), is
   best-effort (never a hard doctor/loop failure), and surfaces the
   version-volatility caveat (#14345). If the pinned codex version rejects this
   shape, the operator falls back to `--dangerously-bypass-approvals-and-sandbox`
   (already wired in `agent_exec.rs`).
3. **Precedence asymmetry avoided.** `--client` is threaded *through*
   `resolve_config` exactly like `--max-threads`, so there is one precedence site
   (default < `[agent].client` < `--client`), not a post-hoc patch in `main`.
4. **`[agent]` section, not `[client]` key.** Chosen so S-026 can add `model` /
   route vocabulary under the same section without another top-level key. The
   value stays a bare name today; only the parser is the extension seam.
5. **AGENTS.md generation + README/setup docs are T-025-02** (S-025's second
   ticket) — deliberately out of scope here; only the inline `.lisa.toml` comment
   and doctor strings needed for this ticket's AC were added.

## Suggested follow-ups

- T-023-02: implement `CodexAdapter`, fill the `adapter_for_client` Codex arm.
- T-025-02: `AGENTS.md` generation + README/setup-guide toggle docs.
- When a pinned codex version is available, run the T-021-01 Q4 harness to
  confirm the trust-seed shape and drop the [PROVISIONAL] tag.
</content>
