# Design: External Project Dogfood (T-009-03)

## Problem

Research identified one blocking bug (rdspi-workflow.md path mismatch in the plugin
prompt) and the need to execute the full dogfood test plan on an external project.

## Decision: Fix the Path Mismatch

### Option A: Change plugin prompt to use `docs/rdspi-workflow.md`

Change `ticket_prompt()` in `crates/lisa-plugin/src/lib.rs` to reference
`docs/rdspi-workflow.md` instead of `docs/knowledge/rdspi-workflow.md`.

**Pros**: One-line change. Aligns with what `lisa init` creates, what `lisa validate`
checks, and what the CLAUDE.md template references. Single canonical path.

**Cons**: Lisa's own repo has the file at `docs/knowledge/rdspi-workflow.md` (with a
symlink at `docs/rdspi-workflow.md`). After this change, the plugin prompt would
point to the symlink, not the canonical source — but the symlink works fine.

### Option B: Change `lisa init` to create `docs/knowledge/rdspi-workflow.md`

Modify init to place the workflow at the `knowledge/` subdirectory path.

**Pros**: Matches the current plugin prompt. Organizes docs into a `knowledge/` subdirectory.

**Cons**: Changes 3+ files (init.rs, validate, templates.rs, setup_guide.rs) vs 1 file.
Breaks existing projects that already ran `lisa init` (they have `docs/rdspi-workflow.md`).
The `knowledge/` convention is lisa-specific, not universally appropriate for external projects.

### Decision: **Option A**

Minimal change. The standard path `docs/rdspi-workflow.md` is simpler, consistent
with all other pipeline components, and is what external projects will have.
Update the test that asserts the old path.

## Dogfood Approach

This ticket is a test/dogfood task, not a feature implementation. The "work" is:

1. **Fix the bug** (one-line change + test update)
2. **Build a working CLI binary** (`just build-cli` or `just release`)
3. **Run the test plan** on an external project (manual, observational)
4. **Document findings** in this work directory

Since this is a Claude Code session without access to run `lisa loop` (which requires
zellij with a GUI terminal), the scope of what we can do in-session is:

- Fix the bug in code
- Verify all tests pass
- Run `lisa init` and `lisa validate` against a temp external project directory
- Run `lisa loop --dry-run` to verify the pipeline
- Document the results

The actual `lisa loop` (step 6 of the test plan) requires a human in a terminal.

## Rejected Alternatives

### Make the workflow path configurable

Could add a `workflow_path` field to `.lisa.toml`. Rejected as premature — there's
no evidence users need a different path. The standard path `docs/rdspi-workflow.md`
is fine for all projects. If needed later, it's a simple addition.

### Move lisa's own workflow file

Could move `docs/knowledge/rdspi-workflow.md` → `docs/rdspi-workflow.md` in the lisa
repo. Rejected — the `knowledge/` directory organization in lisa is intentional and
other docs live there. The symlink serves its purpose.

## Summary

1. Fix `ticket_prompt()` path: `docs/knowledge/rdspi-workflow.md` → `docs/rdspi-workflow.md`
2. Update the test asserting the old path
3. Simulate dogfood with `lisa init`, `lisa validate`, `lisa loop --dry-run` on a temp project
4. Document results
