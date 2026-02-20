---
id: T-014-02
title: Verify cargo install and package metadata
type: task
phase: done
status: done
priority: medium
story: S-014
created: 2026-02-20
depends_on: []
---

# T-014-02: Verify `cargo install` and package metadata

## Objective

Ensure `cargo install lisa-cli` works correctly and that all crate metadata is complete and professional for the crates.io listing.

## Requirements

### Package metadata

Review and complete `Cargo.toml` fields for all publishable crates (`lisa-cli`, `lisa-core`):

- `authors` — add `["John Chen <john.hk.chen@gmail.com>"]`
- `description` — verify it's clear and concise
- `keywords` — verify keywords are relevant (max 5)
- `categories` — verify categories match crates.io taxonomy
- `repository` — `https://github.com/johnhkchen/lisa`
- `homepage` — same or a dedicated page if one exists
- `license` — `MIT`
- `readme` — point to `README.md`
- `documentation` — optional, can point to docs.rs or omit

Verify `lisa-plugin` has `publish = false`.

### Cargo install verification

1. Run `cargo publish --dry-run` for `lisa-core` and `lisa-cli` to check for packaging issues
2. Verify that `cargo install --path crates/lisa-cli` works from a fresh clone
3. Verify the installed binary name is `lisa` (check `[[bin]]` section)
4. Ensure the WASM build step in `build.rs` works during `cargo install`

### Binary naming

The installed binary should be called `lisa`, not `lisa-cli`. Verify the `[[bin]]` section in `lisa-cli/Cargo.toml` sets `name = "lisa"`.

## Acceptance Criteria

- [ ] All `Cargo.toml` metadata fields are complete
- [ ] `cargo publish --dry-run` succeeds for publishable crates
- [ ] `cargo install --path crates/lisa-cli` produces a `lisa` binary
- [ ] `lisa-plugin` has `publish = false`
