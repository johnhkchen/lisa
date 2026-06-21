# Research: T-018-01 timeout-config-parsing

## Objective

Add `session_timeout_secs` to `.lisa.toml` scheduling config so projects can control how long a single agent session runs before Lisa considers it stalled.

## Codebase Mapping

### Config Layer (CLI side)

**`crates/lisa-cli/src/config.rs`** — Central config parsing module.

- `LisaConfig` (line 8–15): Top-level TOML structure with `version`, `dirs`, `scheduling` sections.
- `SchedulingConfig` (line 26–31): Currently has `max_threads`, `auto_advance`, `review_timeout_secs`. This is where `session_timeout_secs` must be added.
- `ResolvedConfig` (line 34–42): Flat struct with all defaults applied. Currently has `ticket_dir`, `story_dir`, `work_dir`, `max_threads`, `auto_advance`, `review_timeout_secs`. Needs the new field.
- `resolve_config()` (line 79–98): Merges TOML config with CLI overrides. Pattern: `config.scheduling.field.unwrap_or(default)`.
- `validate_config()` (line 111–157): Checks unknown keys against `known_scheduling` list (line 114). Must add `"session_timeout_secs"` to this list.
- `default_config_toml()` (line 184–201): Template for `lisa init`. Could add a commented-out line for the new field.

**`crates/lisa-cli/src/loop_cmd.rs`** — Consumes `ResolvedConfig`, generates KDL layout.

- `generate_layout()` (line 193–239): Passes config values into KDL plugin block. Currently passes `review_timeout_secs` — same pattern for `session_timeout_secs`.
- `run_dry()` (line 83–157): Prints config summary. Could display timeout.

### Core Types Layer

**`crates/lisa-core/src/types.rs`** — `PluginConfig` struct (line 434–456).

- Used by the WASM plugin (not the CLI). Has `from_config_map()` which reads from `BTreeMap<String, String>` — the Zellij plugin config mechanism.
- Currently has `stuck_threshold_secs` (600s default) and `review_timeout_secs` (240s default).
- `session_timeout_secs` needs to be added here with a default constant and parsing in `from_config_map()`.

### Plugin Layer

**`crates/lisa-plugin/src/scheduler.rs`** — Scheduler that manages threads.

- This is where T-018-02 (enforcement) will use the timeout value. For T-018-01 we just need to parse and surface it.

### CLI Commands

**`crates/lisa-cli/src/status.rs`** — `run_status()` (line 8–126).

- Currently prints DAG stats and execution waves.
- Does NOT print scheduling config (no max_threads, no timeouts in output).
- Acceptance criteria says: "shows the timeout setting in its summary header."

**`crates/lisa-cli/src/init.rs`** — `run_validate()` delegates to `validate()` then `print_diagnostics()`.

- `print_diagnostics()` (line 742–772): Only prints errors/warnings and a summary line with ticket/ready counts.
- Acceptance criteria says: "reports the configured timeout in its output."

## Patterns and Conventions

### Adding a new scheduling field (established pattern from `review_timeout_secs`)

1. Add `Option<u64>` field to `SchedulingConfig` in config.rs
2. Add non-optional field to `ResolvedConfig` with default
3. Add default constant to `PluginConfig` in types.rs
4. Add field to `ResolvedConfig::default()` using the constant
5. Add resolution logic in `resolve_config()`
6. Add field name to `known_scheduling` array in `validate_config()`
7. Pass through to KDL layout in `generate_layout()`
8. Parse in `PluginConfig::from_config_map()`
9. Add to default TOML template (commented out)

### Default Value

The ticket suggests 900s (15 minutes) as default. The story motivation mentions 170-second test suites. A 15-minute default seems reasonable — long enough for most workflows, short enough to catch truly stalled sessions.

## Constraints

- The WASM plugin receives config via `BTreeMap<String, String>` from Zellij's KDL layout, not directly from TOML. The CLI is the bridge.
- `session_timeout_secs` is conceptually different from `stuck_threshold_secs` (per-phase staleness) — it's the total session wall-clock time. Must not conflate these.
- The ticket says `Option<u64>` in the acceptance criteria. In `PluginConfig` (core), the field should be `u64` with a default. In `SchedulingConfig` (CLI), it should be `Option<u64>` since TOML may omit it.

## Files to Modify

1. `crates/lisa-core/src/types.rs` — Add field + constant to `PluginConfig`, parse in `from_config_map()`
2. `crates/lisa-cli/src/config.rs` — Add to `SchedulingConfig`, `ResolvedConfig`, `resolve_config()`, `validate_config()`, `default_config_toml()`
3. `crates/lisa-cli/src/loop_cmd.rs` — Pass through in `generate_layout()`
4. `crates/lisa-cli/src/status.rs` — Display in summary header
5. `crates/lisa-cli/src/init.rs` — Display in validate output

## Open Questions

- Should `session_timeout_secs = 0` mean "no timeout" (infinite)? That's a reasonable UX choice. The ticket doesn't specify.
- Should validate warn if session_timeout_secs < stuck_threshold_secs? That would be a contradictory config.
