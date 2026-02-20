# T-017-05 Progress: Tag and cut alpha release

## Completed
- [x] Research phase — mapped version state, release infra, dependencies
- [x] Design phase — decided on v0.2.0, documented tag strategy
- [x] Structure phase — identified 2 files to edit + Cargo.lock regen
- [x] Plan phase — 6-step implementation plan with rollback strategy

## Blocked
This ticket cannot proceed to implementation. Preconditions not met:

1. **T-017-03 (commit pending work)**: status=open, phase=ready
   - ~30 modified + ~60 untracked files in working tree
   - None of the S-012 through S-017 work has been committed
2. **T-017-04 (CI green)**: status=open, phase=ready
   - Latest CI run on main: **failure** (commit 7ea8a6e, 2026-02-20)
   - All 5 most recent CI runs show failure
3. **CI must be green** before tagging — otherwise the release build will also fail

## What Remains (once unblocked)
- [ ] Step 1: Verify CI green on main
- [ ] Step 2: Edit Cargo.toml + crates/lisa-cli/Cargo.toml (version → 0.2.0)
- [ ] Step 3: cargo check, commit "Bump version to 0.2.0", push, wait for CI
- [ ] Step 4: git tag v0.2.0 && git push origin v0.2.0
- [ ] Step 5: Monitor release workflow
- [ ] Step 6: Verify GitHub Release artifacts

## Deviations
None yet — plan is intact, just blocked on upstream dependencies.
