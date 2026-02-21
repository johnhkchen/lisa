# T-017-04 Research: Push and Verify CI Green

## Current State

### Git Status
- **Branch**: `main`, 7 commits ahead of `origin/main`
- **Uncommitted changes**: 18 modified files, ~1144 insertions / ~442 deletions
- **Untracked files**: `.lisa.toml`, `.lisa/hooks/`, `docs/active/work/T-017-01/`, `docs/active/work/T-017-03/progress.md`

### Commits Ahead of Origin (7)
1. `4564ccf` S-012: Repo hygiene — ralph→lisa rename, path corrections, docs reorganization
2. `0ffd181` S-011: Plugin features — review timeout, slot cooldown, deferred Enter, concurrency cap
3. `5cb976b` S-013: Add lisa doctor command and dependency gating
4. `b987b16` S-014 + S-016: Distribution — homebrew tap, nix flake, AUR package, cargo metadata
5. `f08f880` S-015: Public documentation — README rewrite, CONTRIBUTING.md
6. `ee89b53` S-017: Alpha release prep — archive completed work, add release tickets
7. `da4a306` Fix DAG scheduling bugs, bump to 0.1.7

### Uncommitted Changes
The uncommitted changes consist of:
1. **Ticket status updates** — T-017-01 and T-017-03 marked `phase: done`, `status: done`
2. **Source code changes** — Fmt/clippy fixes + feature improvements across all 3 crates
3. **RDSPI work artifacts** — `docs/active/work/T-017-01/` and `T-017-03/progress.md`
4. **Runtime files** (should NOT be committed) — `.lisa.toml`, `.lisa/hooks/`

### Gap: T-017-03 Marked Done but Work Not Committed
T-017-03 ("Commit all pending work") is marked `done`, but significant uncommitted changes remain. The work was done but not finalized into a commit. This needs to be resolved before pushing.

## CI Workflow (.github/workflows/ci.yml)
Triggers on push to `main` and pull requests. Single job with 6 steps:
1. `cargo fmt --all -- --check`
2. `cargo clippy -p lisa-core -- -D warnings`
3. `cargo clippy -p lisa-cli -- -D warnings`
4. `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`
5. `cargo test --workspace`
6. `cargo check -p lisa-plugin --target wasm32-wasip1`

## Local CI Check Results (All Pass)
- `cargo fmt --check` — clean
- `cargo clippy -p lisa-core -- -D warnings` — clean
- `cargo clippy -p lisa-cli -- -D warnings` — clean
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` — clean
- `cargo test --workspace` — 133 tests pass
- WASM compilation — clean

## Files to Exclude from Commit
Per T-017-03 guidelines:
- `.lisa.toml` — runtime config
- `.lisa/hooks/on-clear.sh` — runtime hook
- `.lisa/hooks/on-stop.sh` — runtime hook

## Key Finding
All CI checks pass locally. The main risk is: uncommitted changes need to be committed first, then the push should succeed and CI should be green. The local environment matches CI's check suite exactly.
