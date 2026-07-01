# T-025-01 · Progress

All six plan steps executed. `cargo test --workspace` green (517 tests),
`cargo clippy --workspace` clean, WASM plugin builds, manual smoke tests pass.

## Completed

- **Step 1 — shared `AgentClient` (lisa-core).** New `crates/lisa-core/src/client.rs`
  (enum + `parse`/`as_str`/`Display` + `VALID`), re-exported via
  `pub mod client;` in `lib.rs`. 7 unit tests.
- **Step 2 — `PluginConfig.client` (lisa-core).** Field added (default Claude),
  read leniently in `from_config_map` (bad value → default, no panic). 3 tests.
- **Step 3 — resolver reads loop default (lisa-plugin).** `adapter_for_client`
  helper; `resolve_adapter`/`resolve_adapter_or_native` take `default_client`;
  four `lib.rs` call sites pass `self.config.client`. Codex arm is a documented
  T-023-02 placeholder (falls back to Claude). Existing no-op tests pass
  unmodified; 1 new codex-fallback test; 2 resolver tests updated for the new arg.
- **Step 4 — CLI config (lisa-cli/config.rs).** `AgentConfig` + `LisaConfig.agent`;
  `ResolvedConfig.client`; `resolve_config` third param `cli_client`;
  `validate_config` registers `[agent]`/`client` and rejects invalid values with
  an actionable error; `default_config_toml` ships a commented `[agent]` example.
  8 new tests. In-crate call sites (`init.rs`, `status.rs`) updated to 3-arg form.
- **Step 5 — doctor per client + codex trust (lisa-cli/doctor.rs).** `check_codex`;
  `build_checks(client)` / `check_required_deps(client)`; `codex_home` (CODEX_HOME
  → ~/.codex); `pregrant_codex_trust_in`/`pregrant_codex_trust` (idempotent,
  preserve-existing, best-effort); `run_doctor` loads the selected client and, for
  Codex, seeds trust + prints a version-volatility note. Claude path byte-identical.
  6 new tests.
- **Step 6 — flag + layout + preflight (lisa-cli).** `lisa loop --client`
  (parsed fail-fast via `AgentClient::parse`); `generate_layout` emits
  `client "<name>"`; loop preflight checks the selected client and seeds codex
  trust. 2 new layout tests.

## Deviations from plan

- **`status.rs` also called `resolve_config`** (not listed in structure.md's
  known call sites). Updated to the 3-arg form along with `init.rs`. No behaviour
  change (passes `None` for the new CLI client).
- **Commits not made.** Implemented as one continuous working-tree change (no
  per-step commits) — the RDSPI prompt for this run does not commit; artifacts are
  the handoff. Left for the human/committer. All other plan details followed.

## Verification performed

- `cargo test --workspace` → 213 + 116 + 188 = 517 passed, 0 failed.
- `cargo clippy --workspace` → no warnings.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → ok.
- `lisa loop --dry-run` → `client "claude"`; `--client codex` → `client "codex"`;
  `--client bogus` → `Error: unknown client 'bogus'; valid clients: claude, codex`.
- `lisa doctor` (codex-selected, temp `CODEX_HOME`) → seeds
  `[projects."<abs>"] trust_level = "trusted"`, preserves prior config, prints the
  #14345 note.
- `lisa doctor` (no `.lisa.toml`) → no codex/client lines (output identical to
  pre-opt-in).
</content>
