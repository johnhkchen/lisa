# T-017-02 Structure: Archive completed stories and tickets

## Directory changes

### Create directories (if not existing)
- `docs/archive/stories/` — already exists
- `docs/archive/tickets/` — already exists
- `docs/archive/work/` — already exists

### Move stories: docs/active/stories/ → docs/archive/stories/
| Source | Destination |
|--------|-------------|
| S-010-event-driven-transitions.md | docs/archive/stories/ |
| S-011-cross-device-verification.md | docs/archive/stories/ |
| S-012-repo-hygiene.md | docs/archive/stories/ |
| S-013-lisa-doctor.md | docs/archive/stories/ |
| S-014-distribution-infrastructure.md | docs/archive/stories/ |
| S-015-public-documentation.md | docs/archive/stories/ |
| S-016-package-manager-distribution.md | docs/archive/stories/ |

**Remains:** S-017-alpha-release.md only

### Move tickets: docs/active/tickets/ → docs/archive/tickets/
All T-010-* through T-016-* and T-TEST-* tickets (22 files).

**Remains:** T-017-01 through T-017-06 only (6 files)

### Move work: docs/active/work/ → docs/archive/work/
All T-010-* through T-016-* and T-TEST-* directories (22 directories).

**Remains:** T-017-02, T-017-05 only, plus .gitkeep

## Files modified
- `docs/active/tickets/T-017-02-archive-completed.md` — phase transitions only

## Files NOT modified
- No source code changes
- No config changes
- No cross-references to update (checked: remaining T-017-* tickets don't reference archived IDs)
