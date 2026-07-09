# T-025-01 · Plan — sequenced steps

Ordered, independently-verifiable steps. Each ends green (`cargo test
--workspace` + `cargo build -p lisa-plugin --target wasm32-wasip1`) and is an
atomic commit. Order follows structure.md's dependency chain.

## Step 1 — Shared `AgentClient` in lisa-core

- Add `crates/lisa-core/src/client.rs`: enum + `parse` + `as_str` + `Display` +
  tests.
- `lib.rs`: `pub mod client; pub use client::AgentClient;`
- **Tests:** parse claude/codex (case + whitespace tolerant), unknown → Err with
  both valid names, `as_str` round-trips, `AgentClient::default() == Claude`,
  serde `"codex"` ⇄ `Codex`.
- **Verify:** `cargo test -p lisa-core`.
- **Commit:** `feat(core): add AgentClient shared client vocabulary`.

## Step 2 — `PluginConfig.client`

- `types.rs`: add field, default in `new()`, read in `from_config_map`
  (lenient parse).
- **Tests:** default Claude; `client="codex"` in map → Codex; `client="bogus"` →
  Claude (no panic).
- **Verify:** `cargo test -p lisa-core`.
- **Commit:** `feat(core): carry selected client in PluginConfig`.

## Step 3 — Resolver takes the loop default

- `adapter.rs`: `adapter_for_client`, new `resolve_adapter` /
  `resolve_adapter_or_native` signatures, doc-commented Codex placeholder.
- `lib.rs`: pass `self.config.client` at the four call sites.
- **Tests (adapter.rs):** update the two resolver tests to pass a client; add
  codex-falls-back-to-claude test. Existing `test_build_claude_command*` and
  transition tests must pass **unmodified** (no-op proof: default Claude path
  unchanged).
- **Verify:** `cargo test -p lisa-plugin` (native) **and** `cargo build -p
  lisa-plugin --target wasm32-wasip1 --release` (WASM still compiles).
- **Commit:** `feat(plugin): resolve adapter from loop-default client`.

## Step 4 — CLI config: field, validation, precedence

- `config.rs`: `AgentConfig`, `LisaConfig.agent`, `ResolvedConfig.client`,
  `resolve_config` third param, `validate_config` known-keys + semantic parse,
  `default_config_toml` `[agent]` block.
- Update in-crate `resolve_config` call sites to the 3-arg form (config.rs tests,
  `init.rs:886`).
- **Tests:** parse `[agent] client`; resolve default/from-file/CLI-override
  precedence; invalid client → `validate_config` Err (message names the bad value
  + valid list); unknown `[agent]` key → warning; `[agent] client` → no warning;
  `default_config_toml` parses and its `[agent]` example is inert (commented).
- **Verify:** `cargo test -p lisa-cli`.
- **Commit:** `feat(cli): parse [agent] client selection with validation`.

## Step 5 — Doctor per selected client + codex trust seed

- `doctor.rs`: `check_codex`, `build_checks(client)`,
  `check_required_deps(client)`, `codex_home`, `pregrant_codex_trust_in` /
  `pregrant_codex_trust`, `run_doctor` client-aware + trust section.
- **Tests:** `build_checks(Codex)` has codex not claude; `build_checks(Claude)`
  unchanged (still has claude); `check_required_deps(Codex)` path via `_inner`
  mock; `pregrant_codex_trust_in` writes `[projects."…"]` + `trust_level =
  "trusted"`, idempotent, preserves prior content, honors `CODEX_HOME`;
  `run_doctor` on a dir with no `.lisa.toml` succeeds and is the Claude path.
  Existing doctor tests (mock-based, call `run_checks`/`_inner` directly) compile
  unchanged.
- **Verify:** `cargo test -p lisa-cli`.
- **Commit:** `feat(cli): doctor checks the selected client + seeds codex trust`.

## Step 6 — Wire preflight + `--client` flag + layout

- `loop_cmd.rs`: `check_required_deps(config.client)`; codex trust pre-seed in
  preflight; `generate_layout` emits `client`.
- `main.rs`: `--client` arg, parse, pass to `resolve_config`.
- **Tests:** `generate_layout` default → `client "claude"`; codex config →
  `client "codex"`. (main.rs clap wiring is exercised by the existing
  build + manual `lisa loop --dry-run`.)
- **Verify:** `cargo test --workspace`; `cargo build -p lisa-cli --release`;
  smoke: `lisa loop --dry-run` shows `client "claude"`; `lisa loop --client codex
  --dry-run` shows `client "codex"`; `lisa validate` on a `[agent] client =
  "bogus"` config errors actionably.
- **Commit:** `feat(cli): loop --client flag, layout plumb-through, codex
  preflight`.

## Testing strategy

- **Unit** (the bulk): pure parse/resolve/format/seed-writer logic, all
  tempdir- or string-tested. No live `codex`/`claude`/`zellij` needed — every
  new check is either a pure list-builder assertion or a filesystem seed test.
- **No-op regression:** the existing `test_build_claude_command*`, transition,
  and doctor mock tests must pass **unmodified**. That is the byte-for-byte
  guarantee for the no-opt-in path. If any require editing beyond adding the new
  `client` argument, the change is not a no-op and must be reconsidered.
- **Integration/smoke** (manual, in Review): `lisa loop --dry-run` (± `--client
  codex`) for the layout block; `lisa doctor` in a scratch dir with a codex
  `.lisa.toml` to see the trust section written under a temp `CODEX_HOME`;
  `lisa validate` for the actionable error.
- **[PROVISIONAL] boundary:** we do **not** attempt to run codex. Trust-seed
  tests assert the *file we write*, not codex's acceptance of it (unverifiable
  without the binary; #14345 makes it version-specific).

## Verification criteria (maps to AC)

1. `[agent] client` parsed → `ResolvedConfig` → layout → `PluginConfig`; resolver
   reads it. ✔ Steps 2–4, 6.
2. `lisa doctor` + `lisa loop` preflight check only the selected client; codex →
   `codex --version` + trust report/seed. ✔ Steps 5–6.
3. No opt-in → identical behaviour + identical doctor output. ✔ default Claude
   everywhere; claude-path output untouched; no-op tests unmodified.
4. Config parse errors actionable; `lisa validate` covers the field. ✔ Step 4.
5. Tests: config parsing, layout plumb-through, doctor branch per client. ✔ all
   steps.

## Risks / mitigations

- **Signature churn breaks callers** → the compiler enumerates every site; the
  ordering keeps each crate green before dependents change.
- **Doctor output drift for Claude** → guarded by keeping the claude branch and
  sections byte-identical; a no-op test could pin the string, but the existing
  `test_format_report_*` already cover the shape.
- **Trust seed on unverified intel (#14345)** → best-effort, version-surfacing,
  never a hard gate; `--bypass-sandbox` remains the documented escape hatch.
</content>
