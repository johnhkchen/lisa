# T-012-02 Research: Clean up tracked local files and internal docs

## Current State

### 1. `.claude/settings.local.json`

- **Exists on disk**: `.claude/settings.local.json` (756 bytes)
- **Tracked by git**: No — `git ls-files` returns empty
- **Already ignored**: Yes, by the user's *global* gitignore (`~/.config/git/ignore` rule `**/.claude/settings.local.json`)
- **In project `.gitignore`**: No — the project's `.gitignore` does not mention it
- **Action needed**: Add to project `.gitignore` so other contributors are also protected. `git rm --cached` is unnecessary since the file is not tracked.

### 2. ROADMAP.md

- **Location**: `docs/ROADMAP.md` (139 lines)
- **Problematic reference**: Line 30 — Sprint 4 heading says "First-Implementer Feedback (moron project)" and line 31 references "the moron Rust motion graphics engine"
- **Other internal references**: None found. The rest of the roadmap describes lisa's own development history cleanly.

### 3. `docs/specification.md` and `docs/project-recap.md`

- **Already moved**: Both files show as renamed in `git status`:
  - `R  docs/specification.md -> docs/archive/specification.md`
  - `R  docs/project-recap.md -> docs/archive/project-recap.md`
- **Archive README**: `docs/archive/README.md` already references these files with context explaining they're historical artifacts
- **Action needed**: These renames are staged but uncommitted. No further action beyond committing.

### 4. `.gitignore` current state

Current contents (committed):
```
/target
.DS_Store
.lisa-layout.kdl
.lisa-state-dump.txt
.ralph-commit.lock
.obsidian/
```

Unstaged change in working tree adds `result` to the end.

## Files to Touch

| File | Action |
|------|--------|
| `.gitignore` | Add `.claude/settings.local.json` |
| `docs/ROADMAP.md` | Replace "moron" project references on lines 30-31 |
| `docs/specification.md` | Already moved to archive — just commit |
| `docs/project-recap.md` | Already moved to archive — just commit |

## Constraints

- No `git rm --cached` needed (settings.local.json is already untracked)
- Archive move is already staged — just needs to be included in the commit
- The `.gitignore` already has an unstaged `result` addition that should be included
