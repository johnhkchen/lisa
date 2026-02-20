---
id: T-012-01
title: Fix broken symlink and Ralph naming remnants
type: chore
phase: done
status: done
priority: high
story: S-012
created: 2026-02-20
depends_on: []
---

# T-012-01: Fix broken symlink and Ralph naming remnants

## Objective

Remove the broken absolute-path symlink and rename all "Ralph" references to "Lisa" throughout the codebase.

## Tasks

### 1. Fix broken symlink

`docs/rdspi-workflow.md` is a symlink pointing to the absolute path `/Users/johnchen/swe/repos/lisa/docs/knowledge/rdspi-workflow.md`. This breaks for every other user.

- Delete the symlink `docs/rdspi-workflow.md`
- The canonical file lives at `docs/knowledge/rdspi-workflow.md` — no duplication needed
- Update any references that point to `docs/rdspi-workflow.md` to use `docs/knowledge/rdspi-workflow.md` instead
- Check `CLAUDE.md` and `templates.rs` for path references

### 2. Rename `.ralph-commit.lock` to `.lisa-commit.lock`

Locations to update:
- `crates/lisa-plugin/src/lib.rs` — hardcoded path `/host/.ralph-commit.lock`
- `crates/lisa-core/src/diagnostics.rs` — test constant with `.ralph-commit.lock`
- `.gitignore` — entry for `.ralph-commit.lock`

### 3. Remove "RALPH" from dashboard header

In `crates/lisa-plugin/src/ui.rs`, the dashboard banner reads `LISA/RALPH Dashboard`. Change to `LISA Dashboard`.

### 4. Grep for any remaining "ralph" or "Ralph" references

Search the entire codebase (excluding `docs/archive/` and `target/`) and update or remove any remaining references.

## Acceptance Criteria

- [ ] `docs/rdspi-workflow.md` symlink is gone
- [ ] No broken symlinks in the repo (`find . -type l -not -exec test -e {} \; -print` returns nothing)
- [ ] `grep -ri ralph` on source files returns no hits (excluding archive)
- [ ] `.gitignore` references `.lisa-commit.lock`
- [ ] `cargo test --workspace` passes
- [ ] `cargo check -p lisa-plugin --target wasm32-wasip1` succeeds
