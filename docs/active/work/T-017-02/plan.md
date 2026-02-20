# T-017-02 Plan: Archive completed stories and tickets

## Step 1: Move stories
```bash
mv docs/active/stories/S-01{0,1,2,3,4,5,6}-*.md docs/archive/stories/
```
Verify: `ls docs/active/stories/` shows only S-017-alpha-release.md

## Step 2: Move tickets
```bash
mv docs/active/tickets/T-010-*.md docs/archive/tickets/
mv docs/active/tickets/T-011-*.md docs/archive/tickets/
mv docs/active/tickets/T-012-*.md docs/archive/tickets/
mv docs/active/tickets/T-013-*.md docs/archive/tickets/
mv docs/active/tickets/T-014-*.md docs/archive/tickets/
mv docs/active/tickets/T-015-*.md docs/archive/tickets/
mv docs/active/tickets/T-016-*.md docs/archive/tickets/
mv docs/active/tickets/T-TEST-*.md docs/archive/tickets/
```
Verify: `ls docs/active/tickets/` shows only T-017-* files

## Step 3: Move work directories
```bash
mv docs/active/work/T-010-* docs/archive/work/
mv docs/active/work/T-011-* docs/archive/work/
mv docs/active/work/T-012-* docs/archive/work/
mv docs/active/work/T-013-* docs/archive/work/
mv docs/active/work/T-014-* docs/archive/work/
mv docs/active/work/T-015-* docs/archive/work/
mv docs/active/work/T-016-* docs/archive/work/
mv docs/active/work/T-TEST-* docs/archive/work/
```
Verify: `ls docs/active/work/` shows only T-017-* and .gitkeep

## Step 4: Verify acceptance criteria
- docs/active/stories/ contains only S-017
- docs/active/tickets/ contains only T-017-*
- docs/active/work/ contains only T-017-*
- All moved files present in docs/archive/
- Count archived items matches expected (7 stories, 22 tickets, 22 work dirs)

## Step 5: Stage and update ticket
- `git add docs/active/ docs/archive/`
- Update ticket phase to implement → done

## Testing
- Verify file counts before and after
- Verify no files were deleted (only moved)
- No code changes, so no unit tests needed
