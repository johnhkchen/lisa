# T-012-01 Research: Fix broken symlink and Ralph naming remnants

## 1. Broken symlink: `docs/rdspi-workflow.md`

**Current state:** `docs/rdspi-workflow.md` is a symlink pointing to the absolute path `/Users/johnchen/swe/repos/lisa/docs/knowledge/rdspi-workflow.md`. This works only on the author's machine.

**Canonical file:** `docs/knowledge/rdspi-workflow.md` (102 lines, the real content).

**Embedded copy:** `crates/lisa-cli/data/rdspi-workflow.md` is used via `include_str!` in `templates.rs:4` and written out during `lisa init`.

### References to `docs/rdspi-workflow.md` (the symlink path)

| File | Line | Context |
|------|------|---------|
| `CLAUDE.md` | 58 | "The RDSPI workflow definition is in docs/rdspi-workflow.md" |
| `crates/lisa-cli/src/templates.rs` | 279 | Same string in generated CLAUDE.md template |
| `crates/lisa-cli/src/templates.rs` | 322 | Test assertion: `result.contains("docs/rdspi-workflow.md")` |
| `crates/lisa-plugin/src/lib.rs` | 34 | `ticket_prompt()` — the prompt sent to Claude Code agents |
| `crates/lisa-plugin/src/lib.rs` | 2376 | Test: `cmd.contains("docs/rdspi-workflow.md")` |
| `crates/lisa-plugin/src/lib.rs` | 2463 | Test: `prompt.contains("docs/rdspi-workflow.md")` |
| `crates/lisa-cli/src/init.rs` | 71-83 | `plan_init()` creates file at `docs/rdspi-workflow.md` |
| `crates/lisa-cli/src/init.rs` | 374-382 | `validate()` checks for `docs/rdspi-workflow.md` |
| `docs/ROADMAP.md` | 38, 40 | Historical sprint notes |
| `README.md` | 117 | Source layout listing |

### Decision point: where should `lisa init` put the workflow file?

Currently `init.rs` creates `docs/rdspi-workflow.md`. The canonical copy is at `docs/knowledge/rdspi-workflow.md`. The ticket says the canonical file lives at `docs/knowledge/` and no duplication is needed. This means:
- `init.rs` should create at `docs/knowledge/rdspi-workflow.md`
- `validate()` should check `docs/knowledge/rdspi-workflow.md`
- All prompt/template references should use `docs/knowledge/rdspi-workflow.md`

## 2. `.ralph-commit.lock` references

| File | Line | Current value | New value |
|------|------|---------------|-----------|
| `crates/lisa-plugin/src/lib.rs` | 1822 | `/host/.ralph-commit.lock` | `/host/.lisa-commit.lock` |
| `crates/lisa-core/src/diagnostics.rs` | 138 | `/host/.ralph-commit.lock` | `/host/.lisa-commit.lock` |
| `.gitignore` | 5 | `.ralph-commit.lock` | `.lisa-commit.lock` |

## 3. "LISA/RALPH Dashboard" header

| File | Line | Current | New |
|------|------|---------|-----|
| `crates/lisa-plugin/src/ui.rs` | 988 | `LISA/RALPH Dashboard` | `LISA Dashboard` |

## 4. All "Ralph" or "ralph" references in source (excluding `docs/archive/`, `target/`)

### Doc comments and module-level comments (Lisa/Ralph → Lisa)
| File | Line | Text |
|------|------|------|
| `crates/lisa-plugin/src/lib.rs` | 1 | `//! Lisa/Ralph - A Zellij plugin...` |
| `crates/lisa-core/src/types.rs` | 1 | `//! Core data structures for the Lisa/Ralph Zellij plugin.` |
| `crates/lisa-core/src/types.rs` | 311 | `/// Threads are managed by Ralph and can be paused...` |
| `crates/lisa-core/src/types.rs` | 434 | `/// Configuration for the Lisa/Ralph plugin.` |
| `crates/lisa-core/src/dag.rs` | 1 | `//! DAG computation module for Lisa/Ralph Zellij plugin.` |
| `crates/lisa-core/src/dag.rs` | 396 | `/// These are the tickets that Ralph can spawn threads for...` |
| `crates/lisa-core/src/ticket.rs` | 1 | `//! Ticket parsing and management module for the Lisa/Ralph...` |
| `crates/lisa-plugin/src/ui.rs` | 1 | `//! UI/Dashboard module for the Lisa/Ralph Zellij plugin.` |
| `crates/lisa-plugin/src/ui.rs` | 10 | `//! Replaces ...just ralph-status...just ralph-logs...` |

### Render / user-facing strings
| File | Line | Text |
|------|------|------|
| `crates/lisa-plugin/src/lib.rs` | 1921 | `println!("Lisa/Ralph initializing...");` |
| `crates/lisa-plugin/src/ui.rs` | 988 | `LISA/RALPH Dashboard` (covered above) |

### README
| File | Line | Text |
|------|------|------|
| `README.md` | 3 | `An homage to the ralph loop, but smarter.` |

Note: `README.md` is covered by T-015-01 (readme rewrite). We should still fix the ralph reference here since T-012-01 specifically says "grep -ri ralph on source files returns no hits (excluding archive)".

## 5. Files NOT to modify

- `docs/archive/` — explicitly excluded from scope
- `docs/active/work/T-011-03/` and other work artifacts — these are historical research documents
- `docs/active/stories/S-012-repo-hygiene.md` — story description references the work being done, not code
- `docs/active/tickets/T-012-01-symlink-and-ralph-rename.md` — the ticket itself
- `docs/active/tickets/T-012-02-clean-local-files.md` — references ralph as description of existing state
- `docs/active/tickets/T-015-01-readme-rewrite.md` — ticket descriptions

## 6. Test impact

Tests that assert on `docs/rdspi-workflow.md` path:
- `crates/lisa-plugin/src/lib.rs:2376` — `test_build_claude_command_includes_rdspi_reference`
- `crates/lisa-plugin/src/lib.rs:2463` — `test_ticket_prompt_content`
- `crates/lisa-cli/src/templates.rs:322` — `test_generate_claude_md_rust`

These must be updated to assert `docs/knowledge/rdspi-workflow.md`.

Tests in `init.rs` that reference the workflow path will also need updating — need to check for test assertions there.

## Summary

Total scope: ~25 individual string edits across 10 files, plus deleting 1 symlink. No structural changes. No new files. All tests that assert on changed paths must be updated in lockstep.
