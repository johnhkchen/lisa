# T-026-02 · Plan — implementation steps

Ordered, independently-committable steps. Each step: change + tests + `cargo test
--workspace` green + `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
(WASM must still compile) before committing. Native tests only — no live agents.

## Step 1 — Core: `PluginConfig.provider_caps` + parsing

**Files:** `crates/lisa-core/src/types.rs`

1. Add `provider_caps: HashMap<AgentClient, usize>` to `PluginConfig`
   (`#[serde(default)]`), init empty in `new()`.
2. Add `provider_cap_for(&self, AgentClient) -> Option<usize>`.
3. In `from_config_map`, add the `provider_cap_<name>` prefix loop (mirror
   `phase_timeout_`): parse name via `AgentClient::parse`, value via
   `parse::<usize>()`, skip `0` / bad name / non-numeric leniently.

**Tests:** `provider_cap_for` present/absent; `from_config_map` parses
`provider_cap_codex=8`; ignores bad name, `0`, non-numeric; default empty.

**Verify:** `cargo test -p lisa-core`.

## Step 2 — CLI: TOML surface, resolve, validate

**Files:** `crates/lisa-cli/src/config.rs`

1. `SchedulingConfig.provider_caps: Option<HashMap<String, usize>>`.
2. `ResolvedConfig.provider_caps: HashMap<String, usize>` + `Default` empty +
   `resolve_config` wiring (`unwrap_or_default`).
3. `validate_config`: add to `known_scheduling`; warn unknown provider name
   (against `AgentClient::VALID`); **error** on any `0` value.
4. `default_config_toml`: commented `[scheduling.provider_caps]` example.

**Tests:** parse table; resolve; unknown-name warns; `0` errors; absent → empty;
commented example inert (`resolve` → empty).

**Verify:** `cargo test -p lisa-cli`.

## Step 3 — CLI: emit cap keys into the layout

**Files:** `crates/lisa-cli/src/loop_cmd.rs`

1. In `generate_layout`, emit `provider_cap_<name> <n>` lines (sorted) for each
   entry in `config.provider_caps`, inside the plugin config block. Requires
   `ResolvedConfig.provider_caps` to be threaded into `generate_layout`'s config
   arg (it already receives the resolved config).
2. `run_loop` / `run_dry` operator echo: print caps when non-empty.

**Tests:** layout with `codex=8` contains `provider_cap_codex 8`; layout with no
caps contains no `provider_cap_` substring (byte-for-byte regression guard);
`max_threads*2` pane count unchanged.

**Verify:** `cargo test -p lisa-cli`.

## Step 4 — Plugin: per-provider cap gate

**Files:** `crates/lisa-plugin/src/lib.rs`

1. `use lisa_core::route::resolve_route;`.
2. In `schedule_ready_tickets`, resolve `route` **before** the cap gate (pure).
3. Keep the global gate. Add the per-provider gate: count Running threads with
   `client == route.agent`; if `provider_cap_for(route.agent)` is `Some(cap)`
   and count `>= cap`, `unscheduled += 1; continue;`.
4. Build the adapter later from the same `route` via `adapter_for_route(&route,
   lisa_bin)` — replaces the old `resolve_adapter_or_native` call so the route is
   resolved exactly once. `thread.route = Some(route)` unchanged.

**Tests:** `test_provider_cap_blocks_one_provider_not_other`;
`test_global_cap_still_hard_ceiling`; `test_no_provider_caps_is_unchanged`.

**Verify:** `cargo test -p lisa-plugin`; WASM build.

## Step 5 — Plugin: slot provider-affinity

**Files:** `crates/lisa-plugin/src/lib.rs`

1. `AgentSlot.last_client: Option<AgentClient>` (default `None` in
   `discover_slots`).
2. `find_idle_slot(&self, want: AgentClient) -> Option<usize>` — predicate adds
   `last_client.is_none() || last_client == Some(want)`. Update all call sites
   (spawn passes `route.agent`; test helpers pass a provider).
3. At spawn, set `last_client = Some(route.agent)`. `release_slot_for_ticket`
   leaves it intact.

**Tests:** `test_find_idle_slot_provider_affinity` (fresh/matching/mismatch);
update existing `test_find_idle_slot_*` to the new signature.

**Verify:** `cargo test -p lisa-plugin`; WASM build.

## Step 6 — Stress + signal-cost tests + knowledge note

**Files:** `crates/lisa-plugin/src/lib.rs`, `docs/knowledge/provider-concurrency.md`

1. `test_mixed_provider_stress_16`: `max_threads=16`, `claude=8`+`codex=8`, 32
   slots, >16 ready tickets split across providers. Assert invariants: ≤16 total
   running; ≤8 per provider; each running thread on a unique slot; no slot serves
   a provider != its `last_client`; surplus tickets stay ready (not dropped).
2. `test_poll_tick_signal_scan_cost_at_32_panes`: pre-populate `signal_dir` with
   ~32 panes' signal files; run one `poll_tick`; assert it completes and consumes
   the expected files. Documents the O(5×files) scan (measurement, not change).
3. `docs/knowledge/provider-concurrency.md`: caps + affinity interaction, the
   realistic ceiling, the live 16-agent run recipe, and what to watch (git
   `index.lock` retries, provider 429s, pane starvation). Feeds T-027-01 + epic
   open question 8.

**Verify:** full `cargo test --workspace`; WASM build.

## Testing strategy summary

- **Unit** (Steps 1–3): config parse/resolve/validate/emit — pure functions.
- **Scheduler behaviour** (Steps 4–6): construct `State` with synthetic slots +
  threads + a stub DAG (as the existing `test_concurrency_cap_respects_max_threads`
  and `test_find_idle_slot_*` do) and assert gating/selection. No Zellij host, no
  live agents — satisfies acceptance criterion 4.
- **Regression invariants asserted by test**: empty caps ⇒ unchanged layout and
  unchanged scheduling; global cap remains the hard ceiling.
- **Stress** (Step 6): the 16-agent simulation is the acceptance-criterion-2
  artifact; the live recipe is documented, not automated.

## Risks & mitigations

- **Pane starvation under affinity** if the provider mix shifts mid-loop: 2×
  overprovisioning + caps ≤ global keeps headroom; a starved ticket waits
  visibly (ready + idle_slots>0), never crashes. Noted as a limitation.
- **Route resolved twice** if Step 4 is done carelessly: resolve once, pass the
  `&route` to `adapter_for_route`. Asserted implicitly by no behavioural test
  regression.
- **WASM compile drift** (HashMap<AgentClient,…> serde): `AgentClient` already
  derives the needed traits; add a serde round-trip test if the derive is
  insufficient.

## Verification criteria (done = all true)

- Per-provider cap enforced at spawn; global cap unchanged; single-provider loops
  byte-for-byte identical (layout + scheduling).
- Mixed-provider reuse never mis-injects (affinity test green).
- 16-agent simulation holds all invariants.
- Signal-dir cost measured and documented.
- `cargo test --workspace` green; WASM release build succeeds.
- Findings written to review.md feeding T-027-01 + open question 8.
