# Progress: T-007-03 crates-io-publishing

## Completed

### Step 1: Workspace shared metadata
Added `[workspace.package]` to root `Cargo.toml` with version, edition, license, repository, homepage.

### Step 2: lisa-core Cargo.toml
Switched to workspace inheritance. Added keywords, categories, readme.

### Step 3: lisa-plugin Cargo.toml
Switched to workspace inheritance. Added `publish = false`. Removed duplicate `[profile.release]`.

### Step 4: lisa-cli Cargo.toml
Switched to workspace inheritance. Added `version = "0.1.0"` to lisa-core dependency. Added keywords, categories, readme.

### Step 5: RDSPI workflow crate-local copy
Copied `docs/knowledge/rdspi-workflow.md` to `crates/lisa-cli/data/rdspi-workflow.md`.
Updated `include_str!` in `templates.rs` from `"../../../docs/knowledge/rdspi-workflow.md"` to `"../data/rdspi-workflow.md"`.

### Step 6: WASM error message
Updated error string in `loop_cmd.rs` to guide `cargo install` users to build from source.

### Step 7: README.md
Added Install section with `cargo install` and build-from-source instructions, plus note about WASM limitation.

### Step 8: Full test suite
All 238 tests pass (65 cli + 77 core + 96 plugin).

### Step 9: lisa-core publish dry-run
`cargo publish --dry-run --allow-dirty -p lisa-core` succeeds with no warnings.

### Step 10: lisa-cli publish dry-run
`cargo publish --dry-run -p lisa-cli` fails because lisa-core is not yet on crates.io. This is expected — cargo's dry-run resolves dependencies from the registry. The dry-run will succeed after `cargo publish -p lisa-core` is run first. Package file listing confirms correct contents including `data/rdspi-workflow.md` and `README.md`.

### Step 11: cargo install from path
`cargo install --path crates/lisa-cli` succeeds. `lisa --help` produces expected output.

## Acceptance Criteria Status

- [x] `lisa-cli/Cargo.toml` has complete metadata
- [x] `lisa-core/Cargo.toml` has complete metadata
- [x] `lisa-plugin/Cargo.toml` has `publish = false`
- [x] build.rs works when WASM isn't pre-built (empty placeholder, clear error)
- [x] `cargo publish --dry-run -p lisa-core` succeeds
- [~] `cargo publish --dry-run -p lisa-cli` — will succeed after lisa-core is published
- [x] README.md with install instructions, quick start, and link to docs
- [x] License file present (MIT)
- [x] `cargo install --path crates/lisa-cli` works end-to-end
