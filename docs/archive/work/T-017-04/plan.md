# T-017-04 Plan: Push and Verify CI Green

## Step 1: Final local CI verification
Run all 6 CI checks locally one more time to confirm green before committing:
- `cargo fmt --check`
- `cargo clippy -p lisa-core -- -D warnings`
- `cargo clippy -p lisa-cli -- -D warnings`
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`
- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`

**Verify**: All 6 exit 0.

## Step 2: Stage and commit
Stage all modified source files, ticket files, and work artifacts explicitly by name.
Do NOT stage `.lisa.toml` or `.lisa/hooks/`.

Commit message: "S-017: Fmt/clippy fixes, ticket updates, work artifacts"

**Verify**: `git status` shows only untracked runtime files remaining.

## Step 3: Push to origin/main
Run `git push origin main`.

**Verify**: Push succeeds without errors.

## Step 4: Monitor CI
Check CI status via `gh run list --limit 1` or `gh run watch`.

**Verify**: All checks pass (green).

## Step 5: Handle CI failure (if needed)
If any check fails:
1. Read failure output via `gh run view <id> --log-failed`
2. Fix the issue locally
3. Re-run local CI checks
4. Commit the fix
5. Push again
6. Repeat until green

## Step 6: Update ticket
Mark T-017-04 as `phase: done`, `status: done`.
Record CI run URL in progress.md.

## Testing Strategy
- All testing is via the CI checks themselves (steps 1 and 4)
- No additional tests needed — this ticket is about verifying existing tests pass in CI
