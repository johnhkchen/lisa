# T-026-02 · Progress

Tracking the Implement phase against plan.md.

## Step 1 — Core `PluginConfig.provider_caps` — ✅ done

- `types.rs`: added `provider_caps: HashMap<AgentClient, usize>` field
  (`#[serde(default)]`), init empty in `new()`, `provider_cap_for()` accessor,
  and a lenient `provider_cap_<name>` parse loop in `from_config_map` (skips
  unknown provider, non-numeric, and `0`).
- Tests: default empty, present, from-map, ignores-bad-entries, absent-is-empty.
- `cargo test -p lisa-core provider_cap` → 5 passed.

## Step 2 — CLI config TOML/resolve/validate — pending
## Step 3 — CLI layout emission — pending
## Step 4 — Plugin per-provider cap gate — pending
## Step 5 — Plugin slot provider-affinity — pending
## Step 6 — Stress + signal-cost tests + knowledge note — pending
