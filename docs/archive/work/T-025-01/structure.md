# T-025-01 · Structure — file-level changes

The blueprint: which files change, the shape of each change, public interfaces,
and the order that keeps the tree compiling. Not code.

## New files

### `crates/lisa-core/src/client.rs` (new)

The single shared client vocabulary + parser.

- `pub enum AgentClient { Claude, Codex }` — derives `Debug, Clone, Copy,
  PartialEq, Eq, Hash, Serialize, Deserialize, Default`; `#[serde(rename_all =
  "lowercase")]`; `#[default] Claude`.
- `impl AgentClient`:
  - `pub fn parse(s: &str) -> Result<Self, String>` — trims + lowercases input;
    maps `"claude"`/`"codex"`; else `Err("unknown client '<s>'; valid clients:
    claude, codex")`.
  - `pub fn as_str(&self) -> &'static str` — `"claude"` | `"codex"`.
  - `pub const VALID: &[&str]` (or inline in the error) — the valid-name list.
- `impl fmt::Display` → `as_str`.
- `#[cfg(test)] mod tests` — parse ok/err/case/whitespace, `as_str`, default,
  serde lowercase round-trip.

### `crates/lisa-core/src/lib.rs` (modify)

- `pub mod client;`
- `pub use client::AgentClient;` (mirrors how existing modules re-export; both
  crates import via `lisa_core::AgentClient` or `lisa_core::client::AgentClient`).

## Modified files — lisa-core

### `crates/lisa-core/src/types.rs`

- `use crate::client::AgentClient;` (or `crate::AgentClient`).
- `PluginConfig`: add `pub client: AgentClient` (after `wind_down_secs`).
- `PluginConfig::new()`: init `client: AgentClient::default()` (Claude).
- `from_config_map`: after the existing key reads, add
  ```
  if let Some(v) = config.get("client") {
      if let Ok(c) = AgentClient::parse(v) { result.client = c; }
  }
  ```
  (lenient — unknown value keeps the default; never errors/panics).
- Tests: `test_config_client_default` (Claude), `test_config_client_from_map`
  (codex), `test_config_client_bad_value_defaults_claude`.

## Modified files — lisa-cli

### `crates/lisa-cli/src/config.rs`

- `use lisa_core::AgentClient;`
- New `#[derive(Debug, Default, Deserialize)] struct AgentConfig { pub client:
  Option<String> }`.
- `LisaConfig`: add `#[serde(default)] pub agent: AgentConfig`.
- `ResolvedConfig`: add `pub client: AgentClient`.
- `impl Default for ResolvedConfig`: `client: AgentClient::default()`.
- `resolve_config(config, cli_max_threads, cli_client: Option<AgentClient>)`:
  new third param. `client` resolution:
  `cli_client.or_else(|| config.agent.client.as_deref().and_then(|s|
  AgentClient::parse(s).ok())).unwrap_or_default()`. (The `.ok()` is safe:
  `validate_config` has already rejected an invalid value before `resolve_config`
  runs on a loaded config; the fallback is defensive.)
- `validate_config`:
  - `known_top` += `"agent"`.
  - Add `known_agent = &["client"]`; warn on unknown `[agent]` keys (mirror the
    `[dirs]`/`[scheduling]` blocks).
  - After deserialize, semantic check: if `config.agent.client` is `Some(s)`,
    `AgentClient::parse(&s).map_err(...)?` → actionable `Err`.
- `default_config_toml()`: append
  ```
  [agent]
  # client = "claude"  # or "codex"
  ```
- Tests: parse `[agent] client`; resolve default/from-config/CLI-override;
  invalid → Err; unknown `[agent]` key → warning; known key → no warning;
  `default_config_toml` still parses.

### `crates/lisa-cli/src/loop_cmd.rs`

- `generate_layout`: add `client "{client}"` line to the plugin config block
  (alongside `wind_down_secs`); `client = config.client.as_str()`.
- `run_loop` preflight: `check_required_deps(config.client)`; when
  `config.client == AgentClient::Codex`, call `doctor::pregrant_codex_trust(root)`
  (best-effort) before exec.
- Tests: `test_generate_layout` asserts `client "claude"`; add
  `test_generate_layout_codex_client` (config.client = Codex →
  `client "codex"`). Existing `default_config()` helper unaffected (client
  defaults to Claude).

### `crates/lisa-cli/src/doctor.rs`

- `use lisa_core::AgentClient;`
- `check_codex()` — new; `codex --version`; NotFound hint =
  codex install docs URL.
- `build_checks(client: AgentClient)` — signature change: zellij; then
  `match client { Claude => check_claude, Codex => check_codex }`; wasm target.
- `check_required_deps(client)` → `check_required_deps_inner(build_checks(client))`.
- `codex_home() -> Option<PathBuf>` — `CODEX_HOME` env else `~/.codex`.
- `pregrant_codex_trust_in(codex_home: &Path, work_tree: &Path) -> bool` — the
  seed writer (idempotent, preserve-existing), modeled on
  `pregrant_plugin_permissions_in`.
- `pregrant_codex_trust(work_tree: &Path)` — resolves `codex_home()` and calls
  the `_in` variant (best-effort wrapper, like `pregrant_plugin_permissions`).
- `run_doctor(root)`: load config → client; `build_checks(client)`; when Codex,
  append a "Checking Codex trust…" section (seeded path + version-volatility
  note) via `pregrant_codex_trust`. Claude path unchanged.
- Tests: `build_checks(Codex)` includes codex/excludes claude; `build_checks
  (Claude)` unchanged; `check_required_deps(Codex)` behavior via `_inner`;
  `pregrant_codex_trust_in` writes/idempotent/preserves/`CODEX_HOME`; existing
  mock-based tests unchanged (they call `run_checks`/`_inner` directly).

### `crates/lisa-cli/src/main.rs`

- `Commands::Loop`: add `#[arg(long)] client: Option<String>`.
- Loop handler: parse `client` via `AgentClient::parse` (actionable error →
  `exit(1)`) into `Option<AgentClient>`; pass to `resolve_config(&config,
  max_threads, cli_client)`.
- `use lisa_core::AgentClient;` (or fully-qualify).

## Modified files — lisa-plugin

### `crates/lisa-plugin/src/adapter.rs`

- `use lisa_core::AgentClient;` (already imports `lisa_core::types::Ticket`).
- `resolve_adapter(ticket: &Ticket, default_client: AgentClient) -> Box<dyn
  AgentAdapter>` — signature change.
- `resolve_adapter_or_native(ticket: Option<&Ticket>, default_client:
  AgentClient)` — signature change; both delegate to
  `adapter_for_client(default_client)` (ticket ignored in MVP, as today).
- `fn adapter_for_client(client: AgentClient) -> Box<dyn AgentAdapter>` — new,
  private; `Claude`/`Codex` both → `ClaudeCodeAdapter` (Codex arm doc-commented
  as the T-023-02 placeholder / Decision-3 fallback).
- Tests: update `resolver_returns_claude_for_any_ticket` and
  `resolver_or_native_handles_missing_ticket` to pass a client; add
  `resolver_codex_falls_back_to_claude_until_t_023_02`.

### `crates/lisa-plugin/src/lib.rs`

- Four call sites (`575`, `1295`, `1387`, `1447`): add `, self.config.client`
  argument to `resolve_adapter_or_native(...)`.
- No struct/field changes (`self.config` already carries `PluginConfig`, which
  now has `client`). `State: Default` unaffected (PluginConfig::default gains a
  Claude client).

## Ordering (keeps the tree compiling at each step)

1. lisa-core `client.rs` + lib re-export (leaf, no deps).
2. lisa-core `types.rs` `PluginConfig.client` (depends on 1).
3. lisa-plugin `adapter.rs` + `lib.rs` call sites (depends on 2).
4. lisa-cli `config.rs` (depends on 1).
5. lisa-cli `doctor.rs` (depends on 1).
6. lisa-cli `loop_cmd.rs` (depends on 4, 5).
7. lisa-cli `main.rs` (depends on 4).

Each step + its tests compiles independently; the workspace builds green after
each. Commit boundaries follow this order (see plan.md).

## Public-interface deltas (summary)

| Symbol | Before | After |
|---|---|---|
| `lisa_core::AgentClient` | — | new enum + `parse`/`as_str` |
| `PluginConfig.client` | — | `AgentClient` (default Claude) |
| `ResolvedConfig.client` | — | `AgentClient` |
| `resolve_config` | `(cfg, Option<usize>)` | `(cfg, Option<usize>, Option<AgentClient>)` |
| `resolve_adapter[_or_native]` | `(ticket)` | `(ticket, AgentClient)` |
| `build_checks` / `check_required_deps` | `()` | `(AgentClient)` |
| `lisa loop` | `--max-threads` | `+ --client` |
</content>
