# T-026-02 · Structure — file-level blueprint

The shape of the change. Ordering: core config type → core parser (plugin
config-map) → CLI config + layout emission + validation → plugin cap gate →
plugin slot affinity → stress/measurement tests. Each layer compiles and tests
green before the next. No code here — interfaces and boundaries.

## Modified files

### `crates/lisa-core/src/types.rs` — `PluginConfig` gains the cap map

- New field on `PluginConfig` (after `client` / `lisa_bin`):
  ```rust
  /// Optional per-provider concurrency sub-caps, keyed by agent client. A
  /// provider with an entry may run at most that many concurrent threads,
  /// *within* the global `max_threads` ceiling. Absent → only the global cap
  /// applies (today's behaviour). Never raises the global ceiling.
  #[serde(default)]
  pub provider_caps: HashMap<AgentClient, usize>,
  ```
- Initialise to `HashMap::new()` in `PluginConfig::new()`.
- New method:
  ```rust
  pub fn provider_cap_for(&self, client: AgentClient) -> Option<usize>
  ```
  returns `self.provider_caps.get(&client).copied()`.
- `from_config_map`: add a `provider_cap_` prefix loop mirroring the existing
  `phase_timeout_` loop (`types.rs:690-699`):
  strip prefix → `AgentClient::parse(name).ok()` + `val.parse::<usize>()` →
  insert. A `0` value or bad name is skipped leniently (plugin never panics on
  its config map; the CLI `validate` is the gate — same contract as `client`).
- Tests: `provider_cap_for` present/absent; `from_config_map` parses
  `provider_cap_codex=8`; bad name / `0` / non-numeric ignored; default map empty;
  a legacy config map without the keys yields an empty map (back-compat).

### `crates/lisa-cli/src/config.rs` — TOML surface + resolve + validate

- `SchedulingConfig` gains
  `provider_caps: Option<HashMap<String, usize>>` (raw string keys; validated
  later, mirroring how `client` is kept a raw `String`).
- `ResolvedConfig` gains `provider_caps: HashMap<String, usize>`; `Default`
  initialises empty.
- `resolve_config`: `provider_caps: config.scheduling.provider_caps.clone()
  .unwrap_or_default()`. (No CLI override flag — caps are config-file only; keeps
  the CLI surface small. Documented in review.)
- `validate_config`:
  - add `"provider_caps"` to `known_scheduling`.
  - new block mirroring `phase_timeouts` validation: for each key in
    `[scheduling.provider_caps]`, warn if the name is not in
    `AgentClient::VALID`; **error** if any value is `0`
    ("provider cap for '<name>' must be at least 1", parity with `max_threads`).
- `default_config_toml`: add a commented example block:
  ```toml
  # [scheduling.provider_caps]
  # claude = 8
  # codex = 8
  ```
- Tests: parse `[scheduling.provider_caps]`; resolve into `ResolvedConfig`;
  unknown provider name warns; `0` value errors; absent → empty; commented
  example stays inert.

### `crates/lisa-cli/src/loop_cmd.rs` — emit cap keys into the layout

- In `generate_layout`, after the existing scalar keys (around
  `loop_cmd.rs:251-257`), emit one `provider_cap_<name>` line per entry in
  `config.provider_caps`, sorted for determinism (like the phase-timeout
  emission if present; else a small helper building the lines). Absent map ⇒ no
  lines ⇒ layout byte-for-byte unchanged (regression proof for single-provider).
- `run_loop` / `run_dry` operator echo: if any caps set, print them alongside the
  `max_threads` line so a `--dry-run` shows the effective per-provider limits.
- Tests: layout with a cap contains `provider_cap_codex 8`; layout without caps
  contains no `provider_cap_` substring (byte-for-byte guard).

### `crates/lisa-plugin/src/lib.rs` — cap gate + slot affinity

**Cap gate — `schedule_ready_tickets` (`lib.rs:544-716`)**
- Resolve the route **before** the cap gate (Design Decision 2). Replace the
  single `resolve_adapter_or_native` call at `:611` with:
  1. `let route = resolve_route(self.dag.get_ticket(&ticket_id)…, self.config.client);`
     early (pure; no `&mut self`).
  2. Global gate stays (`running_total >= max_threads`).
  3. New per-provider gate: compute `running_provider` (Running threads whose
     `client == route.agent`); if `provider_cap_for(route.agent)` is `Some(cap)`
     and `running_provider >= cap`, `unscheduled += 1; continue;`.
  4. After a slot is chosen, build the adapter from the already-resolved route
     via `adapter_for_route(&route, lisa_bin)` (already in `adapter.rs:338`) — so
     no double resolution; the launch/reuse block and Thread construction are
     otherwise untouched, `thread.route = Some(route)` as today.
- `use lisa_core::route::resolve_route;` (and keep `adapter_for_route` import).

**Slot affinity — `AgentSlot` + `find_idle_slot` (`lib.rs:118-137`, `:504-514`)**
- `AgentSlot` gains `last_client: Option<AgentClient>` (default `None` in
  `discover_slots`).
- `find_idle_slot(&self, want: AgentClient) -> Option<usize>` gains the provider
  arg; its predicate additionally requires
  `s.last_client.is_none() || s.last_client == Some(want)`.
- At spawn (`:661-663`), set `agent_slots[slot_idx].last_client =
  Some(route.agent)`.
- `release_slot_for_ticket` leaves `last_client` intact (the pane keeps its
  provider identity across reuse) — this is what makes a warm Claude slot reuse
  another Claude ticket via `/clear`, and a codex-vacated slot reuse codex via
  `FreshExec`.
- Update the two internal `find_idle_slot()` call sites (spawn path; and any
  test helpers) to pass the provider. The spawn caller passes `route.agent`.

**Tests (native, in-file `#[cfg(test)]`)**
- `test_provider_cap_blocks_one_provider_not_other`: 2 running Codex at
  `codex=2`, a ready Codex ticket stays unscheduled while a ready Claude ticket
  spawns.
- `test_global_cap_still_hard_ceiling`: caps set high, global `max_threads` still
  the binding limit.
- `test_no_provider_caps_is_unchanged`: empty map ⇒ only global gate (extends
  `test_concurrency_cap_respects_max_threads`).
- `test_find_idle_slot_provider_affinity`: a slot with `last_client=Some(Claude)`
  is not returned for a Codex want; a fresh slot is; a matching slot is.
- `test_mixed_provider_stress_16`: `max_threads=16`, `claude=8`+`codex=8`, 32
  slots, 20+ ready tickets split across providers → assert invariants
  (≤16 total, ≤8/provider, unique slot per ticket, no cross-provider slot,
  capped surplus stays ready). The stress-validation artifact.

### `crates/lisa-plugin/src/lib.rs` — signal-dir cost probe (measurement)

- A native test `test_poll_tick_signal_scan_cost_at_32_panes`: populate
  `signal_dir` with ~32 × several signal files, run one `poll_tick` (or the five
  scan functions), assert it completes and consumes/leaves files as expected.
  This is the measurement the ticket note asks for; it documents the O(5×files)
  behaviour rather than changing it (consolidation deferred per Design Decision
  6). Findings recorded in review.md.

## New files

### `docs/knowledge/provider-concurrency.md` (optional knowledge note)

Short companion to the existing `provenance-ledger.md`: how per-provider caps +
slot affinity interact, the realistic ceiling, and the live 16-agent run recipe.
Feeds T-027-01's concurrency-at-run interpretation and epic open question 8.

## Ordering (each independently committable)

1. Core `PluginConfig.provider_caps` + `provider_cap_for` + `from_config_map`
   parsing + tests.
2. CLI `config.rs` TOML + resolve + validate + tests.
3. `loop_cmd.rs` layout emission + tests.
4. Plugin cap gate (route-before-gate + per-provider gate) + tests.
5. Plugin slot affinity (`last_client` + `find_idle_slot` arg) + tests.
6. Stress + signal-cost tests; knowledge note; findings in review.md.

## Interfaces / boundaries preserved

- Global `max_threads` remains the hard ceiling and the pane-count driver.
- Empty `provider_caps` ⇒ byte-for-byte today's scheduling and layout.
- Caps keyed by `AgentClient` inside the plugin; raw strings only at the TOML
  edge, validated once (`validate_config`).
- Route resolved exactly once per spawn; adapter still built from it.
- No new subsystems, no live-agent test dependency.

## Out of scope (noted so reviewers don't expect them)

- Consolidating the five signal-dir readdirs into one (measured, deferred).
- A CLI `--provider-cap` flag (config-file only for now).
- Per-provider *rate* limiting / backoff on 429s (external provider concern; we
  cap concurrency, not request rate).
- Writing per-provider concurrency into the provenance record (T-027-01 derives
  it from `actual.agent` + `concurrency_at_spawn`).
