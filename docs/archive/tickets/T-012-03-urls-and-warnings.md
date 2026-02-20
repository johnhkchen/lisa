---
id: T-012-03
title: Fill placeholder URLs and fix dead code warnings
type: chore
phase: done
status: done
priority: low
story: S-012
created: 2026-02-20
depends_on: []
---

# T-012-03: Fill placeholder URLs and fix dead code warnings

## Objective

Fix remaining rough edges: placeholder URLs in docs and compiler warnings in the plugin crate.

## Tasks

### 1. Fill placeholder URLs

`docs/knowledge/lisa-loop-setup-guide.md` contains `git clone <lisa-repo-url>` — replace with the actual URL: `https://github.com/johnhkchen/lisa`

Search for any other `<lisa-repo-url>` or similar placeholders across all docs.

### 2. Fix dead code warnings in ui.rs

The `pane_id` field is flagged as never read in three structs:
- `ActiveThread` (line ~142)
- `ParkedThread` (line ~153)
- `SlotInfo` (line ~180)

Options:
- If `pane_id` is needed for future use, prefix with `_` to suppress the warning
- If it's genuinely dead, remove the field

Check whether `pane_id` is used anywhere else in the codebase before deciding.

## Acceptance Criteria

- [ ] No `<lisa-repo-url>` or similar placeholder strings in any file
- [ ] `cargo check -p lisa-plugin --target wasm32-wasip1` produces zero warnings
- [ ] `cargo test --workspace` passes
