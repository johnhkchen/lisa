# T-011-01 Progress: Build and Install Validation

## Environment

- **Machine:** macOS Darwin 25.3.0, Apple Silicon (aarch64-apple-darwin)
- **Rust:** rustc 1.95.0-nightly (6efa357bf 2026-02-08)
- **Cargo:** 1.95.0-nightly (fe2f314ae 2026-01-30)
- **just:** 1.46.0
- **CARGO_HOME:** /Volumes/ext1/cargo (non-standard location)

## Validation Results

### Did `just install` succeed on the first try?

**Yes.** `just install` completed without errors. The full sequence ran:
1. WASM plugin built (release mode, wasm32-wasip1)
2. Touch to refresh mtime for build.rs
3. `cargo install --path crates/lisa-cli --force` compiled and installed

Build time: ~7 seconds (incremental). Clean build would be longer.

3 dead-code warnings emitted from `ui.rs` (`pane_id` fields) — not errors.

### Were there missing dependencies or unclear error messages?

**No.** All prerequisites were already installed. The build ran cleanly.

Note for a truly fresh device: the `wasm32-wasip1` target and `just` command runner must be installed manually. The README documents both but doesn't provide the `just` install command.

### Did all tests pass?

**Yes.** 332 tests, 0 failures:
- `lisa-cli`: 123 tests passed
- `lisa-core`: 78 tests passed
- `lisa-plugin`: 131 tests passed

### How long did the build take?

~7 seconds (incremental build). First-time clean build on this machine would be 30-60 seconds estimated.

### Any friction with the README instructions?

**Minor:**
- README says "just command runner" is a prerequisite but doesn't show how to install it (unlike the WASM target which has the exact `rustup target add` command).
- The crates.io install note about `lisa loop` not working is clear and well-placed.

## Acceptance Criteria

- [x] `just install` completes without errors
- [x] `lisa` binary is available on PATH (`/Volumes/ext1/cargo/bin/lisa`)
- [x] `cargo test --workspace` passes all tests (332/332)
- [x] Any issues encountered are documented (see below)

## Issues Documented

1. **3 dead-code warnings** in `crates/lisa-plugin/src/ui.rs` for `pane_id` fields on `ActiveThread`, `ParkedThread`, `SlotInfo`. Cosmetic only.
2. **README missing `just` install command.** All other prerequisites have explicit install instructions.
3. **Memory file test count stale.** Says 88 tests (Sprint 7) but actual is 332. Not a build issue.

## Completed

All RDSPI phases done. All acceptance criteria met.
