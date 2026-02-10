# Structure: T-002-01 — Add .lisa.toml Config File

## New Files

### `crates/lisa-cli/src/config.rs`

New module for .lisa.toml parsing. Contains:

```rust
/// TOML config file representation
#[derive(Debug, Default, Deserialize)]
pub struct LisaConfig {
    #[serde(default)]
    pub dirs: DirsConfig,
    #[serde(default)]
    pub scheduling: SchedulingConfig,
}

#[derive(Debug, Deserialize)]
pub struct DirsConfig {
    pub tickets: Option<String>,
    pub stories: Option<String>,
    pub work: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SchedulingConfig {
    pub max_threads: Option<usize>,
    pub auto_advance: Option<bool>,
}

/// Load .lisa.toml from project root. Returns defaults if file absent.
pub fn load_config(root: &Path) -> Result<LisaConfig, String>

/// Merge config with CLI overrides into values for KDL layout generation.
pub fn resolve_config(config: &LisaConfig, cli_max_threads: Option<usize>) -> ResolvedConfig

/// Final merged config values ready for layout generation.
pub struct ResolvedConfig {
    pub ticket_dir: String,
    pub story_dir: String,
    pub work_dir: String,
    pub max_threads: usize,
    pub auto_advance: bool,
}
```

## Modified Files

### `crates/lisa-cli/Cargo.toml`

Add dependency:
```toml
toml = "0.8"
```

### `crates/lisa-cli/src/main.rs`

- Add `mod config;`
- Modify `Commands::Loop` to make `max_threads` optional (`Option<usize>`) so we can distinguish "user passed --max-threads" from "use config file default"
- In `Loop` handler: call `config::load_config()`, then `config::resolve_config()`, pass resolved config to `loop_cmd::run_loop()`

### `crates/lisa-cli/src/loop_cmd.rs`

- Change `run_loop()` signature: accept `ResolvedConfig` instead of just `max_threads: usize`
- Change `generate_layout()`: accept `ResolvedConfig` to emit all config values into KDL
- Change `run_dry()`: use `ResolvedConfig` for directory paths and max_threads
- Update hardcoded `"docs/active/tickets"` references to use resolved config values

### `crates/lisa-cli/src/init.rs`

- `plan_init_actions()`: add `.lisa.toml` to the file creation plan (skip if exists, create with defaults otherwise)
- `run_validate()`: if `.lisa.toml` exists, parse and validate it; report errors

### `crates/lisa-cli/src/templates.rs`

No changes. The TOML config default content is generated in `config.rs`, not templates.

## Unchanged Files

- `crates/lisa-core/` — no changes. `PluginConfig` and `from_config_map()` already handle all needed fields.
- `crates/lisa-plugin/` — no changes. Plugin receives config through Zellij KDL as before.

## Module Boundaries

```
lisa-cli
├── config.rs          NEW — TOML parsing, config resolution
├── main.rs            MOD — wire config loading, optional --max-threads
├── loop_cmd.rs        MOD — use ResolvedConfig instead of bare max_threads
├── init.rs            MOD — create .lisa.toml, validate config
├── detect.rs          unchanged
├── templates.rs       unchanged
└── build.rs           unchanged

lisa-core              unchanged
lisa-plugin            unchanged
```

## Public Interface Changes

### loop_cmd.rs

```
- pub fn run_loop(root: &Path, max_threads: usize, dry_run: bool) -> Result<(), String>
+ pub fn run_loop(root: &Path, config: &ResolvedConfig, dry_run: bool) -> Result<(), String>

- fn generate_layout(wasm_path: &Path, max_threads: usize) -> String
+ fn generate_layout(wasm_path: &Path, config: &ResolvedConfig) -> String

- fn run_dry(root: &Path, max_threads: usize) -> Result<(), String>
+ fn run_dry(root: &Path, config: &ResolvedConfig) -> Result<(), String>
```

### main.rs

```
Commands::Loop {
    path: PathBuf,
-   max_threads: usize,
+   max_threads: Option<usize>,  // None = use config file or default
    dry_run: bool,
}
```

## Default .lisa.toml Content

```toml
# Lisa project configuration
# See: https://github.com/... (TODO: add docs link)

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
# auto_advance = false
```

`auto_advance` is commented out by default since it's an advanced feature.
