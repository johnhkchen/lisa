# T-017-01 Plan: Fix formatting and clippy warnings

## Steps

### Step 1: Run cargo fmt
- `cargo fmt`
- Verify: `cargo fmt --check` exits 0

### Step 2: Run cargo clippy --fix
- `cargo clippy --fix --workspace --allow-dirty`
- Check remaining warnings

### Step 3: Fix remaining clippy warnings manually
- Fix any warnings that `--fix` couldn't handle automatically
- Focus on lib.rs format string interpolation and literal inlining

### Step 4: Verify full CI suite
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- All four must exit 0

## Testing Strategy
- No new tests needed — this is a cosmetic-only change
- Existing test suite (`cargo test --workspace`) validates no behavioral regressions
