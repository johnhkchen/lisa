# T-012-02 Plan: Clean up tracked local files and internal docs

## Steps

### Step 1: Update `.gitignore`
- Add `.claude/settings.local.json` to `.gitignore`
- Verify: `git check-ignore .claude/settings.local.json` matches project gitignore

### Step 2: Edit ROADMAP.md
- Replace "moron project" and "moron Rust motion graphics engine" with generic references
- Verify: grep for "moron" returns no results

### Step 3: Verify pre-existing archive moves
- Confirm `docs/archive/specification.md` and `docs/archive/project-recap.md` exist and are staged
- Confirm original paths are gone

### Step 4: Commit all changes
- Stage `.gitignore`, `docs/ROADMAP.md`, and ticket/work artifacts
- Include the already-staged archive renames
- Single commit

### Step 5: Verify acceptance criteria
- `.claude/settings.local.json` is in `.gitignore` and untracked
- `grep -r "moron" docs/ROADMAP.md` returns nothing
- `docs/specification.md` and `docs/project-recap.md` no longer at root of docs/
- `git status` is clean

## Testing

No code changes — verification is via git status and grep checks.
