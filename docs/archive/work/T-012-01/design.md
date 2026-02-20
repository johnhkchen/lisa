# T-012-01 Design: Fix broken symlink and Ralph naming remnants

## Decision 1: Symlink removal strategy

**Chosen approach: Delete symlink, update all references to canonical path**

The symlink `docs/rdspi-workflow.md` → absolute path is broken for everyone except the author. The canonical file at `docs/knowledge/rdspi-workflow.md` is the source of truth.

Options considered:
1. **Replace absolute symlink with relative symlink** — Still fragile, adds a file that's just an indirection. Rejected.
2. **Copy the file to `docs/rdspi-workflow.md`** — Creates two copies to maintain. Rejected.
3. **Delete symlink, update all references to `docs/knowledge/rdspi-workflow.md`** — Clean, single source of truth. Chosen.

This means `lisa init` and `lisa validate` will use `docs/knowledge/rdspi-workflow.md` as the expected path. The generated CLAUDE.md and the plugin's `ticket_prompt()` will reference `docs/knowledge/rdspi-workflow.md`.

## Decision 2: Scope of Ralph → Lisa renaming

**Chosen approach: Rename all source code references; leave archive and ticket/story descriptions alone**

The acceptance criteria say: `grep -ri ralph` on source files returns no hits (excluding archive).

What counts as "source files":
- `crates/**/*.rs` — all Rust source code, including doc comments
- `CLAUDE.md` — project instructions
- `.gitignore` — config
- `README.md` — public-facing (line 3 only: "homage to the ralph loop")

What we leave alone:
- `docs/archive/**` — explicitly excluded
- `docs/active/work/**` — research/design artifacts are historical records
- `docs/active/stories/**` — story descriptions reference the work being done
- `docs/active/tickets/**` — ticket descriptions describe existing state
- `CONTRIBUTING.md` — uses "Lisa" already

For `README.md` line 3: change "An homage to the ralph loop, but smarter." to "A Zellij plugin for DAG-driven concurrent task scheduling." since T-015-01 will do the full rewrite. But since we need to clear the grep, a minimal fix is sufficient.

## Decision 3: Doc comment style

**Chosen approach: Simple s/Lisa\/Ralph/Lisa/ and s/Ralph/Lisa/**

For doc comments like `//! Core data structures for the Lisa/Ralph Zellij plugin.`, just replace with `//! Core data structures for the Lisa Zellij plugin.`

For comments like `/// Threads are managed by Ralph and can be paused...`, replace with `/// Threads are managed by Lisa and can be paused...`

For `//! Replaces just dag-status, just ralph-status, and just ralph-logs`, update to remove the ralph-specific references: `//! Replaces manual status checking with a single live view.`

## Decision 4: init.rs workflow path

**Chosen approach: Update to `docs/knowledge/rdspi-workflow.md`**

`plan_init()` currently creates `docs/rdspi-workflow.md`. Update to create at `docs/knowledge/rdspi-workflow.md` and ensure the parent directory `docs/knowledge/` is created.

`validate()` currently checks `docs/rdspi-workflow.md`. Update to check `docs/knowledge/rdspi-workflow.md`.

## Rejected alternatives

- **Leaving README.md for T-015-01**: The acceptance criteria require no ralph hits on grep. We do a minimal edit now; T-015-01 will rewrite the whole file later.
- **Creating a LISA_COMMIT_LOCK constant**: Overkill for 2 occurrences. Just change the string literals.
