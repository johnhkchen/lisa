---
id: T-017-02
title: Archive completed stories and tickets
type: chore
phase: done
status: done
priority: medium
story: S-017
created: 2026-02-20
depends_on: []
---

# T-017-02: Archive completed stories and tickets

## Objective

Move all completed stories, tickets, and work artifacts from `docs/active/` to `docs/archive/`. The active directory should only contain stories and tickets that are genuinely in-progress or upcoming.

## What to archive

### Stories (move to `docs/archive/stories/`)
- `S-010-event-driven-transitions.md` — completed
- `S-011-cross-device-verification.md` — completed
- `S-012-repo-hygiene.md` — completed
- `S-013-lisa-doctor.md` — completed
- `S-014-distribution-infrastructure.md` — completed (infra in place, release pending)
- `S-015-public-documentation.md` — completed
- `S-016-package-manager-distribution.md` — completed (config in place, verification pending post-release)

### Tickets (move to `docs/archive/tickets/`)
All tickets from S-010 through S-016 with `status: done`:
- T-010-01, T-010-02, T-010-03
- T-011-01, T-011-02, T-011-03
- T-012-01, T-012-02, T-012-03
- T-013-01, T-013-02
- T-014-01, T-014-02, T-014-03
- T-015-01, T-015-02
- T-016-01, T-016-02, T-016-03

### Test tickets (move to `docs/archive/tickets/`)
- T-TEST-01, T-TEST-02, T-TEST-03

### Work artifacts (move to `docs/archive/work/`)
All work directories for the above tickets:
- `docs/active/work/T-010-*` through `docs/active/work/T-016-*`
- `docs/active/work/T-TEST-*`
- `docs/active/work/T-011-*`

## Steps

1. `git mv` each story/ticket/work directory from active to archive
2. Verify `docs/active/stories/` only contains S-017
3. Verify `docs/active/tickets/` only contains T-017-* tickets
4. Verify no broken cross-references in remaining active docs

## Acceptance Criteria

- [ ] `docs/active/stories/` contains only S-017
- [ ] `docs/active/tickets/` contains only T-017-* tickets
- [ ] `docs/active/work/` contains only T-017-* work directories
- [ ] All archived content is under `docs/archive/`
- [ ] No files deleted — everything preserved in archive
