# Plan: T-007-03 crates-io-publishing

## Step 1: Add workspace-level shared metadata

Edit `Cargo.toml` at workspace root to add `[workspace.package]` with shared fields (version, edition, license, repository, homepage).

Verify: `cargo check --workspace` still passes.

## Step 2: Update lisa-core Cargo.toml

Replace hardcoded fields with workspace inheritance. Add `keywords`, `categories`, `readme`.

Verify: `cargo check -p lisa-core` passes.

## Step 3: Update lisa-plugin Cargo.toml

Inherit workspace fields. Add `publish = false`. Remove duplicate `[profile.release]` section.

Verify: `cargo check -p lisa-plugin --target wasm32-wasip1` passes. Profile warning disappears.

## Step 4: Update lisa-cli Cargo.toml

Inherit workspace fields. Add `version = "0.1.0"` to lisa-core dependency. Add `keywords`, `categories`, `readme`.

Verify: `cargo check -p lisa-cli` passes.

## Step 5: Copy RDSPI workflow into lisa-cli crate

Copy `docs/knowledge/rdspi-workflow.md` to `crates/lisa-cli/data/rdspi-workflow.md`.
Update `include_str!` in `templates.rs` to use the crate-local path.

Verify: `cargo test -p lisa-cli test_rdspi_workflow_embedded` passes.

## Step 6: Update WASM error message in loop_cmd.rs

Change the error string for the empty-WASM case to guide `cargo install` users.

Verify: Existing tests still pass (they don't test the exact error string).

## Step 7: Update README.md

Add Install section with `cargo install` and build-from-source instructions.

Verify: Manual review of content.

## Step 8: Run full test suite

`cargo test --workspace` — all tests pass.

## Step 9: Verify publish dry-run for lisa-core

`cargo publish --dry-run --allow-dirty -p lisa-core` — succeeds with no warnings about missing metadata.

## Step 10: Verify publish dry-run for lisa-cli

`cargo publish --dry-run --allow-dirty -p lisa-cli` — succeeds.

## Step 11: Verify cargo install from path

`cargo install --path crates/lisa-cli` — succeeds and produces a working `lisa` binary.

## Testing Strategy

- **Existing tests**: All 88+ workspace tests must continue to pass (Steps 1-8)
- **Dry-run publish**: Cargo's own verification builds from the tarball (Steps 9-10)
- **End-to-end install**: `cargo install --path` proves the binary builds correctly (Step 11)
- **No new unit tests needed**: Changes are metadata/config only, verified by Cargo tooling
