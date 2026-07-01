# T-026-02 · Progress

Tracking the Implement phase against plan.md.

## Step 1 — Core `PluginConfig.provider_caps` — ✅ done

- `types.rs`: added `provider_caps: HashMap<AgentClient, usize>` field
  (`#[serde(default)]`), init empty in `new()`, `provider_cap_for()` accessor,
  and a lenient `provider_cap_<name>` parse loop in `from_config_map` (skips
  unknown provider, non-numeric, and `0`).
- Tests: default empty, present, from-map, ignores-bad-entries, absent-is-empty.
- `cargo test -p lisa-core provider_cap` → 5 passed.

## Step 2 — CLI config TOML/resolve/validate — ✅ done

- `config.rs`: `SchedulingConfig.provider_caps` (raw string keys),
  `ResolvedConfig.provider_caps`, `resolve_config` wiring, `validate_config`
  (known key + unknown-provider warning + reject-0 error), commented example in
  `default_config_toml`. 7 tests. `cargo test -p lisa-cli` green.

## Step 3 — CLI layout emission — ✅ done

- `loop_cmd.rs`: emit sorted `provider_cap_<name>` keys into the layout (omitted
  when empty → byte-for-byte unchanged); `format_provider_caps` operator echo in
  loop + `--dry-run`. 3 tests incl. a byte-for-byte no-caps guard.

## Step 4 — Plugin per-provider cap gate — ✅ done

- `lib.rs schedule_ready_tickets`: resolve route **before** the cap gates; keep
  the global gate; add per-provider gate via new pure `provider_under_cap()`
  helper (unit-testable, no host calls). 3 tests.

## Step 5 — Plugin slot provider-affinity — ✅ done

- `AgentSlot.last_client: Option<AgentClient>` (default None in `discover_slots`,
  set at spawn, preserved across reuse); `find_idle_slot(want)` gains the
  provider arg and only returns fresh or matching slots. Updated all call sites
  and ~48 test literals. 2 affinity tests.

## Step 6 — Stress + signal-cost tests + knowledge note — ✅ done

- `test_mixed_provider_stress_16`: drives the real gate helpers for 16 mixed
  agents (8/8 caps, 32 slots), asserting all invariants (global cap, per-provider
  caps, unique pane, no cross-provider slot, surplus stays unscheduled).
- `test_signal_scan_cost_at_32_panes`: signal-dir cost probe.
- `docs/knowledge/provider-concurrency.md`: caps + affinity, realistic ceiling,
  live-run recipe.

## Verification

- `cargo test --workspace` → all green (235 + 145 + 225 native tests).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → succeeds.
- `cargo clippy` → no new warnings (one pre-existing in adapter.rs from T-026-01,
  left untouched as out of scope).

## Deviations from plan

- The per-provider gate was extracted into a `provider_under_cap()` method (Plan
  Step 4 implied an inline check) so the decision is unit-testable without Zellij
  host functions — the codebase convention (`schedule_ready_tickets` can't run in
  native tests). No behavioural difference.
