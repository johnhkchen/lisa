# Research: T-002-01 — Add .lisa.toml Config File

## Current Configuration System

### Plugin Configuration (`PluginConfig` in `crates/lisa-core/src/types.rs:347-417`)

The existing config struct lives in lisa-core:

```rust
pub struct PluginConfig {
    pub ticket_dir: PathBuf,    // default: "docs/active/tickets"
    pub story_dir: PathBuf,     // default: "docs/active/stories"
    pub work_dir: PathBuf,      // default: "docs/active/work"
    pub max_threads: usize,     // default: 2
    pub auto_advance: bool,     // default: false
}
```

It provides:
- `PluginConfig::new()` — constructs defaults
- `PluginConfig::from_config_map(&BTreeMap<String, String>)` — parses from Zellij's KDL config map
- Constants for defaults: `DEFAULT_TICKET_DIR`, `DEFAULT_STORY_DIR`, `DEFAULT_WORK_DIR`, `DEFAULT_MAX_THREADS`

### How Config Flows Today

1. **Plugin path** (`crates/lisa-plugin/src/lib.rs:247-250`): `load()` receives `BTreeMap<String, String>` from Zellij's KDL layout. Calls `PluginConfig::from_config_map()`. Then prefixes relative paths with `/host/` for WASI sandbox access.

2. **CLI path** (`crates/lisa-cli/src/loop_cmd.rs:42-43`): `generate_layout()` hardcodes directory paths into the KDL layout. `max_threads` comes from a CLI flag (`--max-threads`, default 2).

3. **CLI init/validate** (`crates/lisa-cli/src/init.rs`): Hardcodes `docs/active/tickets`, `docs/active/stories`, `docs/active/work` paths directly. No config file awareness.

### Scheduler Config Duplication (`crates/lisa-plugin/src/scheduler.rs:188-215`)

`SchedulerConfig` in scheduler.rs is a separate struct with overlapping fields:
- `tickets_dir`, `stories_dir`, `work_dir`, `repo_root`, `max_concurrent_threads`, `claude_binary`
- This struct is currently **unused** (dead code warnings) — the plugin's `State` uses `PluginConfig` directly and does its own scheduling in `lib.rs`.

## Consumers of Configuration

| Consumer | What it reads | Where from |
|----------|--------------|-----------|
| Plugin `load()` | ticket_dir, story_dir, work_dir, max_threads, auto_advance | Zellij KDL config map |
| Plugin `rebuild_dag()` | ticket_dir | `self.config.ticket_dir` |
| Plugin `schedule_ready_tickets()` | max_threads, ticket_dir | `self.config` |
| Plugin `handle_filesystem_update()` | ticket_dir, work_dir | `self.config` |
| CLI `run_loop()` | max_threads | CLI flag `--max-threads` |
| CLI `generate_layout()` | max_threads, dirs | Hardcoded strings + CLI arg |
| CLI `run_init()` | dirs | Hardcoded in `plan_init_actions()` |
| CLI `run_validate()` | dirs | Hardcoded paths |
| CLI `run_dry()` | ticket_dir | Hardcoded `docs/active/tickets` |

## File Format Considerations

### TOML characteristics relevant to Lisa
- Rust ecosystem standard (Cargo.toml, rustfmt.toml, clippy.toml)
- `toml` crate is mature, well-maintained, serde-compatible
- Human-readable, easy to edit by hand
- Supports nested tables and inline tables
- Not yet in the dependency tree — would need to add `toml` crate to lisa-core or lisa-cli

### Current dependency footprint
- lisa-core: serde, serde_yaml_ng, serde_json
- lisa-plugin: lisa-core, zellij-tile, serde, serde_json, libc (unix)
- lisa-cli: lisa-core, clap

### WASM size concern
- Current WASM plugin is ~993KB
- The `toml` crate compiles to ~200-300KB in WASM
- The plugin doesn't necessarily need to parse .lisa.toml — it receives config via Zellij's KDL map
- Parsing .lisa.toml could be CLI-only (lisa-cli) or core (lisa-core)

## Where .lisa.toml Would Be Read

Two distinct paths:

1. **CLI reads .lisa.toml** → injects values into KDL layout → plugin receives via `from_config_map()`. This keeps the plugin unchanged and adds zero WASM bloat.

2. **Plugin reads .lisa.toml directly** from `/host/.lisa.toml`. This means the plugin can reload config without restarting. But adds TOML dependency to the WASM binary.

## Existing Patterns and Constraints

- `init.rs` uses a plan-then-execute pattern (plan actions, print, execute). Config file creation should follow this.
- `init.rs` never overwrites existing files — same should apply to `.lisa.toml`.
- The plugin mounts project root at `/host/` in WASI sandbox.
- `loop_cmd.rs` already accepts `--max-threads` on the CLI. A config file would provide defaults that CLI flags override.
- `from_config_map()` accepts arbitrary string keys — easy to extend.
- Validation is already centralized in `run_validate()` — config validation belongs there.

## Acceptance Criteria Mapping

1. "Config file parsed at plugin startup" → Either CLI injects into KDL layout, or plugin reads from `/host/.lisa.toml`.
2. "Sensible defaults when no config file exists" → Already handled by `PluginConfig::new()`.
3. "Validation errors surfaced in dashboard" → Plugin already logs `ActivityEvent::Error` to the dashboard.
