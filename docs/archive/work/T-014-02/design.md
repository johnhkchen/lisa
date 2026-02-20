# T-014-02 Design: Verify cargo install and package metadata

## Problem

Only one metadata field is missing: `authors`. Everything else is already configured. The ticket also requires verifying `cargo publish --dry-run` and `cargo install --path` work correctly.

## Approach: Add authors at workspace level

Add `authors` to `[workspace.package]` in the root `Cargo.toml`, then add `authors.workspace = true` to `lisa-cli` and `lisa-core`. The plugin doesn't need it since it's `publish = false`.

This is the simplest, DRY approach — matches how `version`, `edition`, `license`, `repository`, and `homepage` are already shared.

## Verification Steps

1. Add the `authors` field
2. Run `cargo publish --dry-run -p lisa-core` to validate metadata
3. Run `cargo publish --dry-run -p lisa-cli` to validate metadata (need WASM built first)
4. Run `cargo install --path crates/lisa-cli` to verify binary installation
5. Verify the installed binary is named `lisa`
6. Confirm `lisa-plugin` has `publish = false`

## Rejected Alternatives

### Set authors per-crate instead of workspace
Would work but creates duplication. Every other shared field already uses workspace inheritance. No reason to deviate.

### Add documentation field
Ticket says optional. docs.rs auto-generates documentation for published crates. Adding it explicitly would just duplicate what happens automatically.

## Risk Assessment

Very low risk. Adding one metadata field and verifying existing configuration. No behavioral changes.
