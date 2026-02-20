---
id: T-017-01
title: Fix formatting and clippy warnings
type: chore
phase: done
status: done
priority: high
story: S-017
created: 2026-02-20
depends_on: []
---

# T-017-01: Fix formatting and clippy warnings

## Objective

Get the entire workspace to pass `cargo fmt --check` and `cargo clippy --workspace` with zero warnings. CI currently fails on formatting (218 diffs across the workspace) and clippy reports 17 warnings (2 in lisa-cli, 15 in lisa-plugin).

## Tasks

### 1. Run `cargo fmt`

Run `cargo fmt` across the workspace. This is a mechanical fix — no manual intervention needed.

Verify: `cargo fmt --check` exits 0.

### 2. Fix clippy warnings

Run `cargo clippy --workspace` and fix all warnings. Known issues:

**lisa-cli (2 warnings):**
- `map_or` can be simplified (2 instances)

**lisa-plugin (15 warnings):**
- `format!` in `format!` args (6 instances) — use direct format string interpolation
- `format!` in `writeln!` args (1 instance)
- `literal with an empty format string` (3 instances) — remove unnecessary format wrapper
- `clamp-like pattern without using clamp function` (1 instance) — use `.clamp(min, max)`
- `map_or` can be simplified (2 instances)
- `iter().cloned().collect()` on a slice (1 instance) — use `.to_vec()`
- `redundant closure` (1 instance)

Many of these are auto-fixable: `cargo clippy --fix --workspace --allow-dirty`

### 3. Verify full CI check suite locally

Run the full check suite that CI runs:
```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

All four must pass.

## Acceptance Criteria

- [ ] `cargo fmt --check` exits 0
- [ ] `cargo clippy --workspace -- -D warnings` exits 0
- [ ] `cargo test --workspace` passes all tests
- [ ] `cargo check -p lisa-plugin --target wasm32-wasip1` is warning-free
