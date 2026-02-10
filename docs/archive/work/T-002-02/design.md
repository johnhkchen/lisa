# T-002-02 Design: Config Validation

## Problem

Three validation gaps in `.lisa.toml` handling:
1. `max_threads = 0` silently accepted, producing a useless loop with zero agent panes
2. Unknown keys (typos like `max_thread`) silently ignored, user gets unexpected defaults
3. Invalid numeric values produce raw serde error messages, not user-friendly ones

## Approach: Validate function with warnings

**Decision**: Add a `validate_config` function that takes raw TOML content (as `toml::Value`) and returns a `ConfigValidation` result containing both errors and warnings. This runs between `load_config` (parsing) and `resolve_config` (merging).

### Why not serde `deny_unknown_fields`?

The ticket says unknown keys should be "warned about", not rejected. `deny_unknown_fields` makes them hard errors. We need warnings — the config should still load, but with feedback.

### Why not `serde_ignored` crate?

Adds a dependency for a small feature. The manual approach (parse as `toml::Value`, check keys) is ~20 lines and zero dependencies. The toml crate already supports `toml::Value`.

### Why validate on `toml::Value` rather than post-deserialization?

Unknown keys are lost after serde deserialization. We need the raw TOML table to detect them. Parse as `toml::Value` first, check keys, then deserialize into typed struct.

## Design

### New types in config.rs

```rust
/// Validation result from config checking.
pub struct ConfigValidation {
    pub config: LisaConfig,
    pub warnings: Vec<String>,
}
```

### New function: `validate_config`

```rust
pub fn validate_config(content: &str) -> Result<ConfigValidation, String>
```

1. Parse `content` as `toml::Value`. On parse failure → `Err` with clear message.
2. Walk top-level keys, warn on anything not in `{"dirs", "scheduling"}`.
3. Walk `[dirs]` keys, warn on anything not in `{"tickets", "stories", "work"}`.
4. Walk `[scheduling]` keys, warn on anything not in `{"max_threads", "auto_advance"}`.
5. Deserialize into `LisaConfig`. On failure → `Err` with context about which field.
6. Validate semantics:
   - `max_threads == Some(0)` → error: "max_threads must be at least 1"
   - (Negative values already fail TOML→usize parsing, but we catch that in step 5 with a better message)
7. Return `ConfigValidation { config, warnings }`.

### Updated `load_config`

Change signature to return `ConfigValidation` instead of plain `LisaConfig`:

```rust
pub fn load_config(root: &Path) -> Result<ConfigValidation, String>
```

When file doesn't exist → return `ConfigValidation { config: LisaConfig::default(), warnings: vec![] }`.

### Callers

- `main.rs` Loop handler: unwrap config from validation, print warnings to stderr, proceed.
- `init.rs` `run_validate`: unwrap config, include warnings in validation output.

### What about the plugin path?

`PluginConfig::from_config_map` in the plugin receives values from the KDL layout, which the CLI generates from already-validated `ResolvedConfig`. Invalid values never reach the plugin. No changes needed there.

## Rejected alternatives

### A. Validate inside `resolve_config`
Unknown keys are already lost by this point. Can validate max_threads=0 here but not typos.

### B. Separate validate-only command
Over-engineering. Validation at load time is sufficient. `lisa validate` already calls `load_config`.

### C. Return errors for unknown keys instead of warnings
Ticket explicitly says "warned about". Warnings are friendlier — a config from a newer Lisa version might have keys the current version doesn't know about.

## Summary

Single new function `validate_config` that parses TOML as Value, checks keys, deserializes into typed struct, validates semantics. `load_config` updated to use it. ~60 lines of new code + tests.
