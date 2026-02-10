# Design: T-002-01 — Add .lisa.toml Config File

## Decision: CLI-only TOML parsing, inject into KDL layout

### Approach A: CLI reads .lisa.toml, injects into KDL layout (CHOSEN)

The CLI (`lisa loop`) reads `.lisa.toml` from the project root, merges with CLI flag overrides, and writes the values into the generated KDL layout. The plugin receives config through Zellij's existing `BTreeMap<String, String>` mechanism via `PluginConfig::from_config_map()`.

**Pros:**
- Zero WASM size increase — no new dependencies in lisa-plugin
- Plugin code is unchanged — `from_config_map()` already handles all fields
- Config parsing happens once at CLI startup, not repeatedly in the plugin
- CLI flags override config file values (standard precedence: defaults < config file < CLI flags)
- `toml` crate only added to lisa-cli (not in WASM critical path)

**Cons:**
- Plugin can't reload config without restarting zellij session
- Config file is opaque to the plugin — it just sees KDL key-value pairs

### Approach B: Plugin reads .lisa.toml directly from /host/ (REJECTED)

The plugin reads `/host/.lisa.toml` in its `load()` method and on filesystem events.

**Why rejected:**
- Adds ~200-300KB to WASM binary (toml crate) for marginal benefit
- Plugin already has a working config mechanism via Zellij KDL
- Hot-reloading config is a nice-to-have but not in acceptance criteria
- WASI filesystem access is slower than native — parsing TOML on every FS event is wasteful

### Approach C: Config in lisa-core, used by both CLI and plugin (REJECTED)

A `LisaConfig` struct and `load_config()` function in lisa-core.

**Why rejected:**
- lisa-core currently has no filesystem awareness beyond ticket parsing
- Would add toml dependency to lisa-core, which flows into the WASM plugin
- Over-engineering: the CLI is the only entry point that needs to read .lisa.toml

## Config File Format

```toml
# .lisa.toml — Lisa project configuration

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
auto_advance = false
```

### Field mapping to PluginConfig

| TOML field | PluginConfig field | KDL key | Default |
|------------|-------------------|---------|---------|
| `dirs.tickets` | `ticket_dir` | `ticket_dir` | `docs/active/tickets` |
| `dirs.stories` | `story_dir` | `story_dir` | `docs/active/stories` |
| `dirs.work` | `work_dir` | `work_dir` | `docs/active/work` |
| `scheduling.max_threads` | `max_threads` | `max_threads` | `2` |
| `scheduling.auto_advance` | `auto_advance` | `auto_advance` | `false` |

### Precedence (lowest to highest)

1. Built-in defaults (`PluginConfig::new()`)
2. `.lisa.toml` file
3. CLI flags (`--max-threads`)

## Config Parsing Location

New module `config.rs` in **lisa-cli** (not lisa-core). Rationale:
- Only the CLI needs to read .lisa.toml
- The plugin already receives config through Zellij KDL
- Keeps the `toml` dependency out of the WASM build

## Validation

`run_validate()` in init.rs should:
- Check if `.lisa.toml` exists (optional — not an error if missing)
- If present, parse and validate it (report parse errors)
- Validate that configured directories exist

The plugin surfaces errors via `ActivityEvent::Error` in the dashboard log — this already works for any config issues that reach the plugin level.

## Init Integration

`lisa init` should:
- Offer to create a `.lisa.toml` with defaults (following the existing plan-then-execute pattern)
- Never overwrite an existing `.lisa.toml`

## CLI Integration

`lisa loop` should:
1. Read `.lisa.toml` if present (ignore if absent)
2. Use values from config file as defaults
3. Allow CLI flags to override config file values
4. Pass merged config into `generate_layout()`

`generate_layout()` needs to accept the full config instead of just `max_threads`.
