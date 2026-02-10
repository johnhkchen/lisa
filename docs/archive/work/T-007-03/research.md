# Research: T-007-03 crates-io-publishing

## Current State

### Workspace Structure
Cargo workspace with 3 crates: `lisa-core`, `lisa-plugin`, `lisa-cli`.
- Workspace root: `Cargo.toml` with `members = ["crates/*"]`
- All crates at version `0.1.0`, edition `2021`

### Existing Metadata

**lisa-core/Cargo.toml:**
- Has: `name`, `version`, `edition`, `description`, `license`
- Missing: `repository`, `homepage`, `keywords`, `categories`, `documentation`
- No `readme` field — crates.io auto-discovers `README.md` in the crate dir (none exists)

**lisa-cli/Cargo.toml:**
- Has: `name`, `version`, `edition`, `description`, `license`
- Missing: `repository`, `homepage`, `keywords`, `categories`, `documentation`, `readme`
- Dependency on `lisa-core` uses path-only: `lisa-core = { path = "../lisa-core" }`
- **This blocks `cargo publish`** — crates.io requires a version for all dependencies

**lisa-plugin/Cargo.toml:**
- Has basic metadata (same as above)
- Missing: `publish = false` (needed since it's not published separately)
- Has its own `[profile.release]` that produces a warning about non-root profiles

### License
MIT license file exists at workspace root (`LICENSE`). All Cargo.toml files specify `license = "MIT"`. The `license-file` field is not needed since `license = "MIT"` is a well-known SPDX identifier and Cargo auto-discovers the workspace root LICENSE.

### README
`README.md` exists at workspace root with install instructions, quick start, project layout, and setup guide. It covers all the acceptance criteria content. No per-crate READMEs exist.

### Git Remote
Repository: `https://github.com/johnhkchen/lisa.git`

## Key Issues

### 1. Path-Only Dependency (Blocker)
`lisa-cli` depends on `lisa-core` with `path = "../lisa-core"` but no version. crates.io strips the `path` and uses only the version to resolve dependencies. Fix: add `version = "0.1.0"` alongside the path.

Same issue exists in `lisa-plugin` (also depends on `lisa-core` via path-only), but since lisa-plugin is `publish = false` this doesn't matter for publishing.

### 2. include_str! Outside Crate Directory
`templates.rs` uses `include_str!("../../../docs/knowledge/rdspi-workflow.md")` to embed the RDSPI workflow. When cargo packages a crate, it only includes files within the crate directory. The `../../../` reference escapes the crate root and would fail when building from the crates.io tarball.

**Fix option:** Copy `rdspi-workflow.md` into the `lisa-cli` crate directory (e.g., `crates/lisa-cli/data/rdspi-workflow.md`) and update the include_str! path. Or use `build.rs` to copy it at build time from the workspace. The simplest approach is to keep a copy in the crate dir, since it's small (~100 lines) and must be self-contained for crates.io.

### 3. WASM Plugin Embedding (build.rs)
`build.rs` looks for `target/wasm32-wasip1/release/lisa.wasm` relative to the workspace root. When installed from crates.io:
- The workspace root won't exist — the crate is built in isolation
- The WASM file won't exist — it's not published

The current fallback already handles this: `build.rs` writes an empty placeholder if the WASM isn't found, and `loop_cmd.rs:29` checks `PLUGIN_WASM.is_empty()` with a clear error message. **This already works for the acceptance criteria.**

However, `cargo install lisa-cli` from crates.io will produce a binary without the WASM plugin embedded, making `lisa loop` non-functional. The user would need to:
1. Install the WASM target: `rustup target add wasm32-wasip1`
2. Build the plugin separately
3. Rebuild the CLI with the plugin present

This is a known limitation documented in the ticket: "build.rs works when WASM isn't pre-built (empty placeholder, with clear error in `lisa loop`)". The dry-run approach is intentional.

### 4. Missing Metadata Fields
Both published crates need:
- `repository = "https://github.com/johnhkchen/lisa"`
- `homepage = "https://github.com/johnhkchen/lisa"`
- `keywords` — up to 5 keywords, e.g., `["task-scheduling", "dag", "zellij", "workflow", "cli"]`
- `categories` — from crates.io's taxonomy, e.g., `["command-line-utilities", "development-tools"]`
- `readme` — point to workspace root README for both crates

### 5. Workspace-Level Metadata
Cargo supports `[workspace.package]` for shared metadata fields. Could centralize `version`, `edition`, `license`, `repository`, `homepage` in the workspace Cargo.toml and inherit with `field.workspace = true` in each crate. This reduces duplication and ensures consistency.

### 6. Dry-Run Verification
`cargo publish --dry-run -p lisa-core` currently succeeds (with dirty flag).
`cargo publish --dry-run -p lisa-cli` fails due to the version issue (#1).

After fixes, both should succeed. `cargo publish --dry-run` packages, verifies the package builds from the tarball, and simulates upload.

### 7. Publish Order
crates.io requires dependencies to be published before dependents. Order:
1. `lisa-core` (no internal deps)
2. `lisa-cli` (depends on `lisa-core`)

`lisa-plugin` gets `publish = false`.

## Files That Need Changes

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `[workspace.package]` shared metadata |
| `crates/lisa-core/Cargo.toml` | Add repository, keywords, categories, readme; inherit from workspace |
| `crates/lisa-cli/Cargo.toml` | Add repository, keywords, categories, readme; version on lisa-core dep |
| `crates/lisa-plugin/Cargo.toml` | Add `publish = false`; clean up duplicate profile |
| `crates/lisa-cli/src/templates.rs` | Fix include_str! path for standalone packaging |
| `crates/lisa-cli/data/rdspi-workflow.md` | Copy of workflow doc for crate-local include |
| `README.md` | Add `cargo install` instructions |

## Boundaries

- This ticket does NOT actually publish to crates.io — just prepares everything so dry-run succeeds.
- The WASM-less binary is an accepted limitation for `cargo install` from crates.io.
- No CI/CD pipeline changes are in scope.
- The `lisa-plugin` crate is intentionally not published.
