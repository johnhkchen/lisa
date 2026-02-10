# Plan: External Project Dogfood (T-009-03)

## Step 1: Fix rdspi-workflow.md path in plugin prompt

Edit `crates/lisa-plugin/src/lib.rs`:
- Line 30: Change `docs/knowledge/rdspi-workflow.md` → `docs/rdspi-workflow.md`
- Line 1805: Update test assertion
- Line 1892: Update test assertion

**Verify**: `cargo test -p lisa-plugin` passes.

## Step 2: Full test suite

Run `cargo test --workspace` to confirm no regressions.

**Verify**: All 94 tests pass.

## Step 3: WASM check

Run `cargo check -p lisa-plugin --target wasm32-wasip1` to confirm WASM compilation.

**Verify**: Clean compilation.

## Step 4: Dogfood dry-run

Simulate the external project pipeline in a temp directory:

1. Build the CLI: `cargo build -p lisa-cli --release`
2. Run `lisa init --path /tmp/test-dogfood-project/` on a temp project
3. Run `lisa validate --path /tmp/test-dogfood-project/` — expect errors (no tickets)
4. Create sample tickets in the temp project
5. Run `lisa validate --path /tmp/test-dogfood-project/` — expect pass
6. Run `lisa loop --dry-run --path /tmp/test-dogfood-project/` — verify output
7. Run `lisa status --path /tmp/test-dogfood-project/` — verify DAG display

**Verify**: All commands succeed with expected output.

## Step 5: Document results

Write `progress.md` documenting:
- Bug fix applied
- Test results
- Dogfood dry-run observations
- Any additional issues discovered
- Recommendations for the actual `lisa loop` test (requires human in terminal)

## Testing Strategy

- **Unit tests**: Existing tests updated for new path (step 1)
- **Integration**: Full workspace test suite (step 2)
- **WASM**: Cross-compilation check (step 3)
- **End-to-end**: CLI commands on temp project (step 4)
