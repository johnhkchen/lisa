# T-025-01 · Research — Client selection config + doctor per client

Descriptive map of the code the ticket touches. What exists, where, how it
connects. No solutions here.

## The config plumbing chain (the spine this ticket extends)

A `.lisa.toml` value reaches the plugin resolver through four hops. Each hop is
a separate struct/function, and the ticket must extend all four:

1. **`.lisa.toml` → `LisaConfig`** (`crates/lisa-cli/src/config.rs:8-34`).
   `LisaConfig` has `version`, `dirs: DirsConfig`, `scheduling: SchedulingConfig`,
   each `#[serde(default)]`, all fields `Option<_>`. Loaded by `load_config`
   (`config.rs:70`) which delegates to `validate_config` (`config.rs:129`).
2. **`LisaConfig` → `ResolvedConfig`** (`config.rs:37-116`). `resolve_config`
   applies precedence *defaults < .lisa.toml < CLI flags*. Only `max_threads`
   currently has a CLI override (`cli_max_threads: Option<usize>`). All other
   fields collapse `config.<section>.<field>.unwrap_or(default)`.
3. **`ResolvedConfig` → KDL layout config block** (`loop_cmd.rs:199-248`,
   `generate_layout`). Emits `key "value"` lines inside
   `plugin location="file://…" { … }`. Eight keys today (`ticket_dir` …
   `wind_down_secs`). The plugin reads this block as its config map.
4. **KDL block → `PluginConfig`** (`crates/lisa-core/src/types.rs:466-632`).
   `PluginConfig::from_config_map(&BTreeMap<String,String>)` (`types.rs:559`)
   reads each key, parsing/`unwrap_or`-ing to a default. `lib.rs:2571` calls it
   in `load()`; then relative dirs get the host prefix (`lib.rs:2577+`).

**Key observation (ticket Notes):** hops 1–2 live in `lisa-cli`, hop 4 lives in
`lisa-core`. Today the "client" concept exists nowhere; there is no shared
parser. The ticket wants the client parsed in **one place both readers share
(lisa-core)**, not a CLI parser + a plugin parser.

## The resolver seam (T-022-01, already landed)

`crates/lisa-plugin/src/adapter.rs` is the S-022 adapter interface (phase
`done`). Relevant shape:

- `trait AgentAdapter` — `launch_command`, `reset_strategy`, `reuse_prompt`,
  `follow_up`, `signals`. One impl: `ClaudeCodeAdapter` (`adapter.rs:149`),
  delegating to the free fns `build_claude_command` / `ticket_prompt` /
  `finish_up_prompt` in `lib.rs`.
- `resolve_adapter(ticket: &Ticket) -> Box<dyn AgentAdapter>` (`adapter.rs:184`)
  and `resolve_adapter_or_native(Option<&Ticket>)` (`adapter.rs:193`). The MVP
  **ignores the ticket and always returns `ClaudeCodeAdapter`**. Doc comment
  (`adapter.rs:178-183`) says S-026 will read `(provider, model)` here.
- Called at four scheduler sites in `lib.rs`: `575`, `1295`, `1387`, `1447`
  (spawn, reuse, cleared handler, review follow-up), each as
  `resolve_adapter_or_native(self.dag.get_ticket(&id))`.
- **There is no Codex adapter.** `ResetStrategy::FreshExec` and
  `FollowUp::SpawnCommand` exist as documented, `#[allow(dead_code)]` seams
  "consumed by T-023-02" — the Codex adapter body is explicitly *not* this
  ticket, nor T-025-01's. So the resolver has exactly one adapter to return.

The T-022-01 design (`work/T-022-01/design.md`) established the pattern this
ticket must follow: extend the seam, leave the not-yet-built arm as a documented
placeholder, keep the no-opt-in path byte-for-byte identical.

## `PluginConfig` (types.rs) — the plugin's config struct

Fields are concrete (no `Option`): `ticket_dir`, `story_dir`, `work_dir`,
`max_threads`, `auto_advance`, `stuck_threshold_secs`, `review_timeout_secs`,
`session_timeout_secs`, `phase_timeouts: HashMap<Phase,u64>`, `wind_down_secs`.
Derives `Serialize, Deserialize, PartialEq, Eq, Clone`. `new()` sets defaults;
`from_config_map` overrides present keys. **Any new field must derive the same
traits** and get a default in `new()`. There is already a `Phase::from_name`
string parser in this file — precedent for a name→enum parser living in
lisa-core.

## `doctor.rs` — the dependency checker

- `DependencyCheck { name, required, check: Box<dyn Fn()->CheckResult> }`.
  `build_checks()` (`doctor.rs:125`) hardcodes three: `zellij` (required),
  `claude` (required, `check_claude` = `claude --version`, `doctor.rs:86`),
  `wasm target` (optional).
- `check_required_deps()` (`doctor.rs:186`) → `check_required_deps_inner(build_checks())`;
  called by `run_loop` preflight (`loop_cmd.rs:27`). Takes **no client**.
- `run_doctor(root)` (`doctor.rs:372`): runs checks, appends a project-version
  section (`check_project_version`), then a Zellij-cache-clean section. Prints,
  returns `Err` on any required failure.
- Precedent for "doctor does an environment side-effect, best-effort,
  tempdir-testable": `pregrant_plugin_permissions_in(cache_dir, wasm_path)`
  (`doctor.rs:335`) — reads existing file, idempotency-checks by exact key line,
  appends a block, `create_dir_all` + `write`, returns bool. The codex trust
  seed is structurally the same pattern against a different file.
- `run_doctor` does **not** currently read `.lisa.toml` for a client (only the
  version). To check the *selected* client it must load config first.

## Codex trust pre-seeding (from T-021-01 verdict + intel packet)

The verdict lisa doctor must implement (`work/T-021-01/design.md:102-119`,
`review.md:33`), all tagged **[PROVISIONAL]** (harness authored, never run):

- **Binary check:** `codex --version` (pinned target `rust-v0.142.5`).
- **Trust seed file:** user-level `$CODEX_HOME/config.toml` (default
  `~/.codex/config.toml`; `CODEX_HOME` env overrides the dir). A repo-local
  `.codex/config.toml` **cannot** carry trust — must be the user-level file.
- **Seed block** (exact):
  ```toml
  [projects."<abs-working-tree>"]
  trust_level = "trusted"
  ```
  Without it, `codex exec -a never` blocks on an untrusted repo (headless =
  unattended, so the interactive trust prompt would hang the pane).
- **Escape hatch, not default:** `--dangerously-bypass-approvals-and-sandbox`
  also works but disables the sandbox, so trust-seeding is preferred. (The
  existing `agent_exec.rs` wrapper already exposes `--bypass-sandbox` for this.)
- **Version-volatile (#14345):** trust behaviour shifts across codex versions —
  the seed must be re-verified per version, so doctor should surface the version
  alongside the seed, never assume it stable.
- **Auth prereq:** headless codex reuses the saved CLI login, or `CODEX_API_KEY`
  (honored only in `codex exec`). Not a hard doctor gate, but worth reporting.

## The Codex wrapper already in the tree (context, not this ticket)

`agent_exec.rs` (`lisa agent-exec`, T-023-01) already runs `codex exec --json`,
translating events → `.lisa/signals/pane-<id>.*`. `build_codex_argv`
(`agent_exec.rs:453`) shows the invocation shape (`-a never -s workspace-write`
by default, `--dangerously-bypass-approvals-and-sandbox` under `--bypass-sandbox`).
This confirms `codex` is the binary name and the sandbox flags, but the wrapper
is launched by the (not-yet-built) Codex adapter, not by this ticket.

## `main.rs` CLI wiring

`Commands::Loop { path, max_threads, dry_run }` (`main.rs:106`). Handler
(`main.rs:193`) loads config, prints warnings, `resolve_config(&config, max_threads)`,
`run_loop`. Adding a `--client` flag means a new arg here plus a parse step.
`Commands::AgentExec` already exists (the wrapper). `Commands::Doctor { path }`
calls `run_doctor` — no client arg today.

## `init.rs` / `config.rs` validation surface

- `validate_config` (`config.rs:129`) parses TOML generically to warn on unknown
  keys (`known_top`, `known_dirs`, `known_scheduling`, `known_phases`), then
  deserializes to `LisaConfig`, then semantic-checks (`max_threads == 0` → Err).
  A new `[agent]` section + `client` key must be registered here or it warns as
  unknown; an invalid client value must become an actionable `Err`.
- `default_config_toml()` (`config.rs:232`) is the `lisa init` template.
  `upsert_missing_config_keys` (`init.rs:52`) back-fills new keys into existing
  `.lisa.toml` as commented lines (has_key/has_section/find_section_end helpers).
- `run_validate` (`init.rs:880`) prints a config summary via `resolve_config`;
  `validate()` (`init.rs:567`) surfaces `load_config` warnings/errors as
  diagnostics — so registering the field in `validate_config` gives
  `lisa validate` coverage for free.

## Constraints / assumptions carried into Design

- **No-opt-in must stay byte-identical**, including `lisa doctor` output. Default
  client is Claude; the claude checks and doctor sections must be unchanged when
  no client is selected.
- **One shared parser in lisa-core** (ticket Notes), extensible toward
  `(method, provider, model)` (S-026 vocabulary) — a bare client name today.
- **No Codex adapter exists**; the resolver's codex arm is a documented
  placeholder (mirrors T-022-01's FreshExec/SpawnCommand seams), not live codex
  routing (that's T-023-02).
- Codex trust facts are **[PROVISIONAL]** — implement the seed the verdict
  prescribes, but keep it best-effort and version-surfacing, never a hard gate
  that could wedge a loop.
</content>
</invoke>
