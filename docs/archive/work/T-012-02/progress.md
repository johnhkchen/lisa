# T-012-02 Progress: Clean up tracked local files and internal docs

## Completed

### Step 1: Update `.gitignore`
- Added `.claude/settings.local.json` to project `.gitignore`
- Verified: `git check-ignore -v .claude/settings.local.json` matches `.gitignore:7`
- Note: `git rm --cached` was unnecessary — file was never tracked

### Step 2: Edit ROADMAP.md
- Replaced "moron project" → "external project" in Sprint 4 heading
- Replaced "the moron Rust motion graphics engine" → "an external Rust project" in Sprint 4 description
- Verified: `grep moron docs/ROADMAP.md` returns no results

### Step 3: Verify archive moves
- `docs/archive/specification.md` and `docs/archive/project-recap.md` exist
- Original paths gone (confirmed by `ls`)
- Renames were already staged before this ticket started
- `docs/archive/README.md` already references both files

## Deviations from Plan

- `git rm --cached .claude/settings.local.json` skipped — file was already untracked (ignored by user's global gitignore). Adding to project `.gitignore` is sufficient.

## Acceptance Criteria Status

- [x] `.claude/settings.local.json` is in `.gitignore` and untracked
- [x] ROADMAP.md has no awkward external project references
- [x] `docs/specification.md` and `docs/project-recap.md` are handled (moved to `docs/archive/`)
- [ ] `git status` shows no unintended tracked files — pending commit
