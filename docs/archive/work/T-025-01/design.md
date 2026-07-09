# T-025-01 · Design — Client selection config + doctor per client

Goal: a discoverable, safe opt-in to the Codex client — a `.lisa.toml` field +
`lisa loop --client` flag, defaulting to Claude, plumbed through the four-hop
config chain to the T-022-01 resolver as the **loop-level default**; and
`lisa doctor` / the loop preflight checking the **selected** client's deps
(codex binary + directory-trust pre-seed) instead of unconditionally requiring
`claude`. No-opt-in behaviour (incl. doctor output) stays byte-for-byte
identical.

## Decision summary

Introduce one shared value type, `AgentClient`, in **lisa-core**, with a single
`parse(&str) -> Result<AgentClient,String>` both readers use. Thread it through
the existing config chain exactly as `max_threads` is threaded, add a
`client: AgentClient` field to `PluginConfig`, and have `resolve_adapter` take
the loop-default client. Make `doctor::build_checks` client-parametric and add a
codex-trust pre-seed that mirrors the existing `pregrant_plugin_permissions`
pattern.

## Key decisions & rationale

### D1 — `AgentClient` enum in lisa-core, one parser (chosen)

```rust
// crates/lisa-core/src/client.rs
#[serde(rename_all = "lowercase")]
pub enum AgentClient { #[default] Claude, Codex }
impl AgentClient {
    pub fn parse(s: &str) -> Result<Self, String>; // trims, lowercases; Err lists valid values
    pub fn as_str(&self) -> &'static str;           // "claude" | "codex"
}
```
Derives `Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize` so
it can live inside `PluginConfig` (which derives the same set) without friction.

- **Why lisa-core, not lisa-cli:** the plugin (`PluginConfig::from_config_map`)
  and the CLI (`resolve_config`) are in different crates; lisa-core is the only
  shared dependency. Ticket Notes require "one place both readers share, not two
  ad-hoc parsers." Precedent: `Phase::from_name` already lives here.
- **Why an enum, not a struct now:** the value is a bare client name today. The
  *parser* is the extension seam: S-026 replaces the enum's innards with
  `(method, provider, model)` while `parse`/`as_str` keep their signatures, so
  call sites don't churn. Encoding `(method, provider, model)` today would
  over-build ahead of S-026 (epic graduation guardrail).
- **Rejected — `String` client field:** pushes validation to every reader and
  invites the two-parser split the Notes forbid.
- **Rejected — reuse the adapter `trait`/a plugin-only enum:** the value must be
  parsed CLI-side (for `lisa validate`) *before* any adapter exists; it has to
  be a plain data type in the shared crate.

### D2 — Config field: `[agent] client = "…"` + `--client` flag

`.lisa.toml`:
```toml
[agent]
client = "claude"   # or "codex"; default claude
```
- `LisaConfig` gains `agent: AgentConfig { client: Option<String> }`
  (`#[serde(default)]`, raw `String` so an invalid value is a *validation* error,
  not a deserialize panic).
- `ResolvedConfig` gains `client: AgentClient`.
- **Precedence via `resolve_config`**, symmetric with `max_threads`:
  `resolve_config(config, cli_max_threads, cli_client: Option<AgentClient>)`.
  Order: default `Claude` < `[agent].client` < `--client`. Threading the CLI
  override *through* `resolve_config` (rather than patching `resolved` in main)
  matches how `cli_max_threads` already works — one precedence site, not two.
- **Section name `[agent]`** (not `[client]`): the ticket says "an
  `[agent]`/client setting", and S-026 will add `[agent]`-adjacent routing
  vocabulary; a section leaves room for `model`/route later without another
  top-level key.
- **Validation:** register `agent`/`client` as known keys in `validate_config`;
  parse the value through `AgentClient::parse`, returning an actionable `Err`
  ("unknown client 'foo'; valid: claude, codex"). This gives `lisa validate` and
  `lisa loop`'s load step coverage for free (both go through `load_config`).

### D3 — Layout plumb-through: one more KDL key

`generate_layout` emits `client "<as_str>"` in the plugin block.
`PluginConfig::from_config_map` reads `client` via `AgentClient::parse(...)`,
falling back to `Claude` on absence *or* an unrecognized value (the plugin is
lenient — CLI-side validation is the gate; the plugin must never panic on a
config map). Default-Claude keeps the emitted block identical to today when no
opt-in (the `client` line is always present but reads `"claude"`, and the plugin
already tolerates unknown/again-defaulted keys — existing layout tests still
pass, one new assertion added).

### D4 — Resolver reads the loop default (placeholder codex arm)

`resolve_adapter(ticket, default_client)` and `resolve_adapter_or_native(ticket,
default_client)`. A private `adapter_for_client(AgentClient) -> Box<dyn
AgentAdapter>` centralizes the mapping:
- `Claude => ClaudeCodeAdapter` (unchanged).
- `Codex  => ClaudeCodeAdapter` **for now**, with a prominent doc comment: the
  Codex adapter is T-023-02; until it lands, a codex selection resolves to native
  Claude (the epic Decision-3 "fall back to loop default when the requested route
  is unavailable" rule, applied to a not-yet-built adapter).

- **Why not build the Codex adapter here:** T-022-01's design explicitly reserves
  `FreshExec`/`SpawnCommand` bodies and the Codex adapter for T-023-02. This
  ticket's scope is *config + doctor*; wiring live codex routing would poach that
  ticket and break the no-op discipline. The resolver **reading** the default
  (AC bullet 1) is satisfied by threading + branching; the branch's Codex body is
  T-023-02's.
- **Why still thread it now:** AC requires the resolver to *use* the loop default,
  and doing it now means T-023-02 only fills the arm — no caller change. The
  no-op proof holds: default `Claude` → `ClaudeCodeAdapter`, byte-identical.
- The four `lib.rs` call sites pass `self.config.client`.

### D5 — Doctor / preflight check the selected client

- `build_checks(client: AgentClient)`: `zellij` always; then **either**
  `check_claude` **or** `check_codex` (`codex --version`; NotFound hint points at
  the codex install docs); `wasm target` always. Exactly one agent binary is
  checked — the selected one.
- `check_required_deps(client)` (used by `run_loop` preflight): same list, so the
  loop preflight gates on the selected client. Call site `loop_cmd.rs:27` passes
  `config.client`.
- `run_doctor(root)` loads `.lisa.toml` (`load_config` → `resolve_config(.., None,
  None)`) to learn the client, then `build_checks(client)`. **When Claude
  (incl. no `.lisa.toml`): output is identical to today** — same three checks,
  same version/cache sections, no extra lines. When Codex: after the checks, a
  "Checking Codex trust…" section reports the seeded path + a version-volatility
  note.
- **Codex trust pre-seed** — `pregrant_codex_trust_in(codex_home, work_tree)`,
  structurally identical to `pregrant_plugin_permissions_in`: resolve
  `$CODEX_HOME` (env, else `~/.codex`), read `config.toml`, if a
  `[projects."<abs>"]` header line is already present treat as done (idempotent),
  else append
  ```toml
  [projects."<abs-working-tree>"]
  trust_level = "trusted"
  ```
  `create_dir_all` + `write`, return bool. Best-effort: any IO failure just
  reports "could not seed" and the loop still proceeds (the operator can fall back
  to `--bypass-sandbox`). `run_doctor` and the loop preflight both invoke it when
  Codex is selected.
- **Why mirror `pregrant_plugin_permissions`:** same shape (idempotent,
  preserve-existing, tempdir-testable, best-effort), so it inherits a proven,
  reviewed pattern and test style.
- **[PROVISIONAL] safety:** the seed is written *and* the codex version is
  surfaced (per #14345 the behaviour is version-volatile). Trust is never a *hard*
  doctor failure — a missing/failed seed is a warning-level report, because the
  bypass flag is a valid fallback and we must not wedge a loop on unverified
  intel.

## What stays out of scope (guardrails)

- No Codex `AgentAdapter` implementation (T-023-02).
- No `AGENTS.md` generation, no README rewrite (T-025-02) — only the inline
  `.lisa.toml` comment + doctor strings needed for *this* ticket's AC.
- No per-ticket routing / frontmatter `agent:` (S-026); the field is loop-level
  default only.
- No `(method, provider, model)` tuple yet; enum + parser seam only.

## Test strategy (per AC)

- **Config parsing:** `[agent] client` parse; default→claude; `--client`
  override precedence; invalid value → actionable `validate_config` Err; unknown
  `[agent]` key → warning; `[agent] client` known-key → no warning.
- **Shared parser:** `AgentClient::parse` valid/invalid/whitespace/case; `as_str`
  round-trip; serde lowercase.
- **Layout plumb-through:** `generate_layout` contains `client "claude"` /
  `client "codex"`; `from_config_map` reads it; missing/garbage → Claude.
- **Doctor branch per client:** `build_checks(Codex)` contains `codex`, not
  `claude`; `build_checks(Claude)` unchanged; `pregrant_codex_trust_in` writes
  the block, is idempotent, preserves existing content, honors `CODEX_HOME`;
  `run_doctor` with no `.lisa.toml` = Claude path (no new output).
- **Resolver:** `resolve_adapter(_, Codex)` returns an adapter (ClaudeCodeAdapter
  reset strategy) — placeholder proof; existing adapter tests updated to pass a
  client.
</content>
