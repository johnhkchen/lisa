---
id: T-012-02
title: Clean up tracked local files and internal docs
type: chore
phase: done
status: done
priority: medium
story: S-012
created: 2026-02-20
depends_on: []
---

# T-012-02: Clean up tracked local files and internal docs

## Objective

Stop tracking files that are machine-local or internal-only, and clean up documents that would confuse external readers.

## Tasks

### 1. Gitignore `.claude/settings.local.json`

This file contains development-time Claude Code permission settings. It's machine-local and shouldn't be tracked.

- Add `.claude/settings.local.json` to `.gitignore`
- Remove it from git tracking: `git rm --cached .claude/settings.local.json`

### 2. Clean up ROADMAP.md

- Replace "moron Rust motion graphics engine" with a generic description (e.g., "an external Rust project") or remove the reference entirely
- Review the rest of the file for any other internal-sounding references

### 3. Evaluate internal docs

These files are tracked and read like internal development notes:

- `docs/specification.md` — titled "Ralph: Design Document", uses the old project name throughout
- `docs/project-recap.md` — personal build metrics and sprint history

Options (decide during implementation):
- Move to `docs/archive/` alongside other historical artifacts
- Remove from tracking entirely
- Keep but rename/edit for external audience

## Acceptance Criteria

- [ ] `.claude/settings.local.json` is in `.gitignore` and untracked
- [ ] ROADMAP.md has no awkward external project references
- [ ] `docs/specification.md` and `docs/project-recap.md` are handled (moved, removed, or cleaned up)
- [ ] `git status` shows no unintended tracked files
