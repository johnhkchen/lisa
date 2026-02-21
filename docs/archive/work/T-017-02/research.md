# T-017-02 Research: Archive completed stories and tickets

## Current State

### docs/active/stories/ (8 files)
- **Git-tracked:** S-010, S-011
- **Untracked:** S-012, S-013, S-014, S-015, S-016, S-017
- **To archive (S-010 through S-016):** 7 stories
- **To keep:** S-017 (current sprint)

### docs/active/tickets/ (28 files)
- **Git-tracked:** T-010-01, T-010-02, T-010-03, T-011-01, T-011-02, T-011-03
- **Untracked:** T-012-* (3), T-013-* (2), T-014-* (3), T-015-* (2), T-016-* (3), T-017-* (6), T-TEST-* (3)
- **To archive (T-010 through T-016 + T-TEST):** 22 tickets
- **To keep:** T-017-01 through T-017-06 (6 tickets)

### docs/active/work/ (24 directories + .gitkeep)
- **Git-tracked:** T-010-01, T-010-02, T-010-03 (full RDSPI artifacts), .gitkeep files in T-011-01/02/03
- **Untracked:** T-011-01 through T-016-03, T-TEST-01/02/03, T-017-02, T-017-05
- **To archive:** T-010-* (3), T-011-* (3), T-012-* (3), T-013-* (2), T-014-* (3), T-015-* (2), T-016-* (3), T-TEST-* (3) = 22 directories
- **To keep:** T-017-02 (this ticket), T-017-05

### docs/archive/ (already populated)
- stories/: S-001 through S-009
- tickets/: T-001-01 through T-009-04
- work/: T-001-03 through T-009-04
- Also: README.md, project-recap.md, specification.md at root

## Git tracking implications
- Git-tracked files need `git mv` to preserve history
- Untracked files can use regular `mv` (or `git mv` after staging — simpler to just `mv`)
- Mixed approach: `git mv` for tracked, `mv` for untracked, then `git add` everything
- Alternative: `mv` everything (auto-detects renames), then `git add`. Simpler and equivalent result.

## Risks
- .gitkeep in docs/active/work/ must remain so the directory stays in git
- T-011-01/02/03 have both .gitkeep (tracked) and work artifacts (untracked) — moving the whole directory moves both
- No cross-references found in remaining active docs that point to archived tickets
