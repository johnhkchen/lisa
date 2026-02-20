# T-017-01 Design: Fix formatting and clippy warnings

## Approach

**Only viable approach:** Run the standard toolchain fixes.

1. `cargo fmt` — automated, no alternatives
2. `cargo clippy --fix` for auto-fixable warnings, manual fixes for the rest

## Decisions

- Run `cargo fmt` first so clippy operates on clean formatting
- Use `cargo clippy --fix --allow-dirty` for auto-fixable warnings
- Manually fix remaining warnings that clippy can't auto-fix
- Verify with `-D warnings` flag to treat warnings as errors

No alternatives considered — this is a mechanical chore with one correct approach.
