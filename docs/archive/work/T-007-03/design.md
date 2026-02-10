# Design: T-007-03 crates-io-publishing

## Decision 1: Workspace Metadata Sharing

### Options

**A. Workspace inheritance (`[workspace.package]`)**
Share `version`, `edition`, `license`, `repository`, `homepage` in the workspace root Cargo.toml. Each crate inherits with `field.workspace = true`. Single source of truth for shared fields.

**B. Duplicate metadata in each crate**
Each Cargo.toml gets its own copy of all fields. Simpler but error-prone — version bumps need updating in multiple places.

### Decision: Option A — workspace inheritance

Rationale: Cargo workspace inheritance is stable since Rust 1.64. It prevents version drift between crates, which is critical when lisa-cli depends on lisa-core with a version constraint. The workspace Cargo.toml already exists and is minimal.

Fields to share: `version`, `edition`, `license`, `repository`, `homepage`, `rust-version` (if added later).
Fields kept per-crate: `name`, `description`, `keywords`, `categories` (these are crate-specific).

## Decision 2: Fixing include_str! for RDSPI Workflow

### Options

**A. Copy file into crate directory**
Place `rdspi-workflow.md` at `crates/lisa-cli/data/rdspi-workflow.md`. Update `include_str!` to use the local path. Maintain the canonical copy in `docs/knowledge/` and add a comment that this is a distribution copy.

**B. Use build.rs to copy at build time**
Have `build.rs` copy from `../../../docs/knowledge/rdspi-workflow.md` into `OUT_DIR` and use `include_str!(concat!(env!("OUT_DIR"), "/rdspi-workflow.md"))`. Problem: when building from crates.io tarball, the workspace-relative path doesn't exist, so `build.rs` would need a fallback — and there's no file to fall back to.

**C. Inline the workflow content as a string constant**
Embed the ~100 lines directly in templates.rs. Loses the single-source-of-truth.

**D. Include from workspace root via Cargo.toml `include` directive**
Use the `include` key in Cargo.toml to pull extra files into the published tarball. Then `include_str!` can reference them. However, `include` paths must be relative to the crate root — no `../` allowed for crates.io packaging.

### Decision: Option A — copy into crate directory

Rationale: Simplest, most robust. The file is ~100 lines and rarely changes. The canonical version stays in `docs/knowledge/`. The crate-local copy ensures `cargo publish` and `cargo install` both work. A comment in the copy notes its origin. If the canonical file changes, the copy needs manual sync — acceptable for a rarely-changing document.

## Decision 3: README Strategy

### Options

**A. Per-crate READMEs**
Create `crates/lisa-core/README.md` and `crates/lisa-cli/README.md` with crate-specific content. More work, but each crate page on crates.io gets tailored documentation.

**B. Point both crates to workspace root README**
Use `readme = "../../README.md"` in each crate's Cargo.toml. Cargo resolves this relative path and includes the README in the published tarball. Both crate pages on crates.io show the same content.

**C. Workspace root README only, no per-crate readme field**
Rely on crates.io not showing a README. Users navigate to the repository link instead.

### Decision: Option B — shared workspace README

Rationale: The workspace README already has install instructions, quick start, and project layout. It's a good landing page for both crates on crates.io. Adding `cargo install lisa-cli` instructions to the existing README satisfies the acceptance criteria without creating duplicate documentation. Cargo supports `readme = "../../README.md"` for this purpose.

## Decision 4: WASM Placeholder Behavior

### Current behavior (keep as-is)

`build.rs` writes an empty placeholder if the WASM file doesn't exist. `loop_cmd.rs` checks `PLUGIN_WASM.is_empty()` and returns a clear error. This is exactly what the acceptance criteria specify.

No changes needed. The error message should mention `cargo install` as a known limitation and point users to building from source for full functionality.

### Enhancement: Improve the error message

Current message: "WASM plugin not embedded. Build the plugin first: just build && cargo build -p lisa-cli --release"

Better message for crates.io users: explain that `cargo install` doesn't include the WASM plugin and link to the repository for build-from-source instructions.

## Decision 5: Plugin publish = false

Straightforward. Add `publish = false` to `lisa-plugin/Cargo.toml`. Also remove the duplicate `[profile.release]` section since it's ignored for non-root packages and produces a warning.

## Decision 6: lisa-core Dependency Version

Add `version = "0.1.0"` to the lisa-core dependency in lisa-cli. Use `version = "0.1.0", path = "../lisa-core"` so local development still uses the path and crates.io uses the version. Standard pattern for workspace crates.

Same for lisa-plugin's dependency on lisa-core — not strictly needed since lisa-plugin is `publish = false`, but good practice for consistency.

## Summary of Approach

1. Add `[workspace.package]` to workspace Cargo.toml for shared metadata
2. Update all three crate Cargo.toml files with inherited and crate-specific metadata
3. Copy `rdspi-workflow.md` into `crates/lisa-cli/data/` and update `include_str!` path
4. Update README.md with `cargo install` instructions
5. Add version to path dependencies
6. Verify with `cargo publish --dry-run` for both published crates
7. Verify `cargo install --path crates/lisa-cli` works end-to-end
