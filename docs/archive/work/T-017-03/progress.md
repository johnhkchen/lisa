# T-017-03 Progress: Commit all pending work

## Status: Complete

All 6 commits created successfully on main.

## Commits

| # | Hash | Message | Files |
|---|------|---------|-------|
| 1 | `4564ccf` | S-012: Repo hygiene — ralph→lisa rename, path corrections, docs reorganization | 11 |
| 2 | `0ffd181` | S-011: Plugin features — review timeout, slot cooldown, deferred Enter, concurrency cap | 4 |
| 3 | `5cb976b` | S-013: Add lisa doctor command and dependency gating | 6 |
| 4 | `b987b16` | S-014 + S-016: Distribution — homebrew tap, nix flake, AUR package, cargo metadata | 8 |
| 5 | `f08f880` | S-015: Public documentation — README rewrite, CONTRIBUTING.md | 3 |
| 6 | `ee89b53` | S-017: Alpha release prep — archive completed work, add release tickets | 169 |

## Deviation from Plan

Added a dedicated S-011 commit (Commit 2) for the substantial lib.rs plugin features. The original ticket plan grouped lib.rs into "Commit 5: everything remaining" but 592+ lines of feature work deserved proper attribution.

## Verification

- [x] All 336 tests pass (127 CLI + 78 core + 131 plugin)
- [x] Each commit has a descriptive message referencing its story ID
- [x] `git status` shows only 3 untracked runtime files: `.lisa.toml`, `.lisa/hooks/on-clear.sh`, `.lisa/hooks/on-stop.sh`
- [x] No sensitive files committed
