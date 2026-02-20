# T-014-02 Research: Verify cargo install and package metadata

## Current Cargo.toml State

### Workspace root (`Cargo.toml`)
Already has `[workspace.package]` with shared fields:
- `version = "0.1.6"` — shared by all crates
- `edition = "2021"`
- `license = "MIT"`
- `repository = "https://github.com/johnhkchen/lisa"`
- `homepage = "https://github.com/johnhkchen/lisa"`
- **Missing: `authors`** — not set at workspace level

### `lisa-cli/Cargo.toml`
Inherits: version, edition, license, repository, homepage from workspace.
Has:
- `description = "CLI for Lisa DAG-driven concurrent task scheduling"`
- `keywords = ["task-scheduling", "dag", "zellij", "cli", "workflow"]` — 5 keywords (max)
- `categories = ["command-line-utilities", "development-tools"]`
- `readme = "../../README.md"` — points to workspace root README
- `[[bin]] name = "lisa"` — correct binary name
- **Missing: `authors`** — not inherited, not set locally

### `lisa-core/Cargo.toml`
Inherits: version, edition, license, repository, homepage from workspace.
Has:
- `description = "Core types, ticket parsing, and DAG computation for Lisa"`
- `keywords = ["task-scheduling", "dag", "workflow", "ticket"]` — 4 keywords
- `categories = ["development-tools"]`
- `readme = "../../README.md"`
- **Missing: `authors`** — not inherited, not set locally

### `lisa-plugin/Cargo.toml`
Has:
- `publish = false` — correct, not publishable
- `description` present
- Inherits version, edition, license only (no repository/homepage — fine since not published)

## What's Missing

### `authors` field
The ticket requires `["John Chen <john.hk.chen@gmail.com>"]`. This is not set anywhere.
Best approach: add to `[workspace.package]` and inherit via `authors.workspace = true`.

### `documentation` field
Ticket says optional. Can omit — docs.rs will auto-generate for published crates.

## Build Pipeline for `cargo install`

### How WASM embedding works
1. `build.rs` in lisa-cli looks for `target/wasm32-wasip1/release/lisa.wasm`
2. If found, copies to `OUT_DIR/lisa.wasm`
3. If not found, writes an empty placeholder
4. `loop_cmd.rs` uses `include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"))` to embed

### `cargo install --path crates/lisa-cli` flow
- Runs `build.rs` which needs the WASM file to already exist at `target/wasm32-wasip1/release/lisa.wasm`
- The `just install` recipe builds WASM first, then runs cargo install — this works
- But a bare `cargo install --path crates/lisa-cli` without prior WASM build embeds an empty placeholder

### `cargo install lisa-cli` from crates.io
- build.rs runs, but `target/wasm32-wasip1/release/lisa.wasm` won't exist in the crates.io download
- Placeholder (empty bytes) gets embedded — plugin won't work
- README already warns: "requires wasm32-wasip1 Rust target"
- **This is a known limitation** — the build.rs doesn't compile the WASM itself

### `cargo publish --dry-run`
- For `lisa-core`: should work, no build.rs, no special deps
- For `lisa-cli`: depends on `lisa-core` by version (`version = "0.1.6"`, `path = "../lisa-core"`). Both version and path specified — crates.io uses version, local uses path. Should work.

## Existing Infrastructure

### License
MIT license file exists at `LICENSE` with correct copyright holder.

### dist-workspace.toml
cargo-dist already configured:
- Packages: `["lisa-cli"]`
- Targets: macOS (x86/arm), Linux (x86/arm)
- Installers: shell
- Uses custom build-setup for WASM compilation

### Binary naming
`[[bin]] name = "lisa"` is already correct in `lisa-cli/Cargo.toml`.

## Categories Validation

crates.io taxonomy requires exact category slugs. Current values:
- `lisa-cli`: `["command-line-utilities", "development-tools"]` — both are valid crates.io categories
- `lisa-core`: `["development-tools"]` — valid

## Summary of Gaps

| Field | Workspace | lisa-cli | lisa-core | lisa-plugin |
|-------|-----------|----------|-----------|-------------|
| authors | **MISSING** | inheritable | inheritable | N/A (unpublished) |
| description | N/A | OK | OK | OK |
| keywords | N/A | OK (5) | OK (4) | N/A |
| categories | N/A | OK | OK | N/A |
| repository | OK | inherited | inherited | N/A |
| homepage | OK | inherited | inherited | N/A |
| license | OK | inherited | inherited | inherited |
| readme | N/A | OK | OK | N/A |
| publish=false | N/A | N/A | N/A | OK |

Only real gap: **`authors` field needs to be added at workspace level** and inherited by crates.
Everything else is already in good shape.
