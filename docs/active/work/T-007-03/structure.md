# Structure: T-007-03 crates-io-publishing

## Files Modified

### 1. `Cargo.toml` (workspace root)

Add `[workspace.package]` section with shared metadata:
```toml
[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/johnhkchen/lisa"
homepage = "https://github.com/johnhkchen/lisa"
```

### 2. `crates/lisa-core/Cargo.toml`

Inherit shared fields from workspace. Add crate-specific metadata:
```toml
[package]
name = "lisa-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Core types, ticket parsing, and DAG computation for Lisa"
keywords = ["task-scheduling", "dag", "workflow", "ticket"]
categories = ["development-tools"]
readme = "../../README.md"
```

### 3. `crates/lisa-cli/Cargo.toml`

Inherit shared fields. Add version to lisa-core dependency. Add crate-specific metadata:
```toml
[package]
name = "lisa-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "CLI tool for Lisa DAG-driven concurrent task scheduling"
keywords = ["task-scheduling", "dag", "zellij", "cli", "workflow"]
categories = ["command-line-utilities", "development-tools"]
readme = "../../README.md"

[dependencies]
lisa-core = { version = "0.1.0", path = "../lisa-core" }
```

### 4. `crates/lisa-plugin/Cargo.toml`

Add `publish = false`. Remove duplicate `[profile.release]` (it's ignored for non-root packages). Inherit workspace fields for consistency:
```toml
[package]
name = "lisa-plugin"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Zellij WASM plugin for Lisa DAG-driven task scheduling"
publish = false
```

### 5. `crates/lisa-cli/data/rdspi-workflow.md` (NEW)

Copy of `docs/knowledge/rdspi-workflow.md` for standalone crate packaging. Exact same content. A comment at the top is not needed — the file is already self-explanatory and adding a comment would change the content injected into agent sessions.

### 6. `crates/lisa-cli/src/templates.rs`

Change the `include_str!` path:
```rust
// Before:
pub const RDSPI_WORKFLOW: &str = include_str!("../../../docs/knowledge/rdspi-workflow.md");

// After:
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");
```

### 7. `crates/lisa-cli/src/loop_cmd.rs`

Update the WASM-not-embedded error message to be useful for `cargo install` users:
```rust
// Before:
"WASM plugin not embedded. Build the plugin first:\n  \
 just build && cargo build -p lisa-cli --release"

// After:
"WASM plugin not embedded in this binary.\n\n  \
 If installed via `cargo install`, the WASM plugin is not included.\n  \
 Build from source instead:\n    \
 git clone https://github.com/johnhkchen/lisa && cd lisa && just release"
```

### 8. `README.md` (workspace root)

Add a `## Install` section before the Build section with `cargo install` instructions:
```markdown
## Install

```bash
# From crates.io (CLI only — `lisa init`, `lisa validate`, `lisa status`)
cargo install lisa-cli

# From source (full functionality including `lisa loop`)
git clone https://github.com/johnhkchen/lisa
cd lisa
rustup target add wasm32-wasip1
just release
```
```

Update the Status section to reflect publishability.

## Files NOT Changed

- `crates/lisa-cli/build.rs` — already handles missing WASM gracefully
- `docs/knowledge/rdspi-workflow.md` — canonical copy stays in place
- `.gitignore` — no new generated files
- `justfile` — no new build tasks needed

## No New Modules or Interfaces

All changes are metadata and configuration. No new Rust modules, traits, or public APIs. The only code changes are:
1. One `include_str!` path in `templates.rs`
2. One error message string in `loop_cmd.rs`

## Ordering

Changes can be made in any order. No circular dependencies between the modifications. For verification, `cargo publish --dry-run` must run after all Cargo.toml and file changes are complete.
