# Research: Init Hardening for External Projects

## Overview

This ticket hardens `lisa init` and `lisa validate` so they work correctly on
external (non-lisa) projects. The codebase already has solid foundations; this
research maps what exists, identifies gaps, and surfaces edge cases.

## Relevant Files

| File | Role |
|------|------|
| `crates/lisa-cli/src/init.rs` | `plan_init_actions()`, `run_init()`, `run_validate()` — 884 lines including 21 tests |
| `crates/lisa-cli/src/templates.rs` | `generate_claude_md()`, constants for hooks/gitignore/settings — 185 lines, 7 tests |
| `crates/lisa-cli/src/detect.rs` | `detect_project()`, per-language detection — 305 lines, 7 tests |
| `crates/lisa-cli/src/config.rs` | `.lisa.toml` loading, validation, resolution — 374 lines, 17 tests |
| `crates/lisa-core/src/ticket.rs` | `parse_ticket()`, `scan_tickets_with_diagnostics()` — 854 lines, 16 tests |
| `crates/lisa-core/src/dag.rs` | `Dag::from_tickets()`, cycle detection, topo sort — 1059 lines, 22 tests |

## Current State of `lisa init`

### What it does
1. Detects project type (Rust/Node/Go/Python/Unknown) from marker files
2. Plans actions: 8 dirs + 6 files = 14 create actions for a clean directory
3. Never overwrites existing files (Skip action)
4. Creates directories: `docs/active/{tickets,stories,work}`, `docs/archive/{tickets,stories,work}`, `.lisa/{hooks,signals}`
5. Creates files: `CLAUDE.md`, `docs/rdspi-workflow.md`, `.lisa.toml`, `.lisa/hooks/on-idle.sh`, `.lisa/.gitignore`, `.claude/settings.local.json`
6. Sets executable permission on `on-idle.sh` (unix only)

### Files generated and their content
- **CLAUDE.md**: Project name, detected build/test/lint commands, source layout, directory conventions, RDSPI workflow reference. Template references `docs/rdspi-workflow.md` correctly.
- **docs/rdspi-workflow.md**: Embedded at compile time from `crates/lisa-cli/data/rdspi-workflow.md`
- **.lisa.toml**: Default config with `docs/active/tickets`, `max_threads = 2`
- **.lisa/hooks/on-idle.sh**: Shell script that writes idle signal files, references `LISA_TICKET_ID` env var
- **.lisa/.gitignore**: Contains `signals/`
- **.claude/settings.local.json**: Hooks config for `idle_prompt` notification → `on-idle.sh`

## Current State of `lisa validate`

### What it checks (in order)
1. **Tool checks** (optional, `--check-tools`): zellij and claude on PATH
2. **CLAUDE.md exists** — error
3. **docs/rdspi-workflow.md exists** — error
4. **.lisa.toml** — loads and validates if present; unknown keys → warnings, invalid TOML → error
5. **Hook infrastructure** — `.claude/settings.local.json` and `.lisa/hooks/on-idle.sh` — warnings only
6. **Directory structure** — `docs/active/stories` and `docs/active/work` — warnings only
7. **Ticket directory** — must exist — error (stops early if missing)
8. **Ticket scanning** — uses `scan_tickets_with_diagnostics()`, surfaces per-file parse errors
9. **At least one ticket** — error if empty
10. **Acceptance Criteria section** — warning per ticket
11. **DAG build** — catches `MissingDependency` and `CycleDetected` errors
12. **At least one ready ticket** — error

### What's missing per requirements

| Requirement | Current Status |
|-------------|---------------|
| Check `idle_prompt` hook **content** in settings.local.json | Only checks file existence (warning) |
| Check on-idle.sh is **executable** | Only checks file existence (warning) |
| Tickets with `type: ticket` | Already caught by `parse_ticket_type()` — returns `InvalidField` error |
| Tickets with invalid `phase` values | Already caught by `parse_phase()` — returns `InvalidField` error |
| Tickets with missing `depends_on` refs | Already caught by `Dag::from_tickets()` — returns `MissingDependency` |
| Empty ticket directory | Already caught — returns error "No tickets found" |
| Hook checks should be errors, not warnings | Currently warnings; ticket says "catch and report clearly" |

## CLAUDE.md Template Quality

### Current template (from `generate_claude_md`)
- Includes project name and TODO placeholder for description
- Includes build/test/lint commands when detected
- Includes source layout from scanning `src/` directory
- Includes directory conventions for tickets/stories/work
- References `docs/rdspi-workflow.md` path
- Does NOT contain any lisa-internal references (no "lisa-plugin", "crates/", etc.)

### Gaps
- Does not include detected project type label (e.g., "Rust project", "Node.js project")
- Requirement says: "Include project name and detected type"
- The `name` field from `DetectedProject` is used, but `ProjectType` is only printed to stdout during init, not included in the generated CLAUDE.md

## Edge Cases for External Projects

1. **No marker files at all** — `detect_project()` returns `Unknown` with empty commands/layout. `generate_claude_md()` produces a minimal CLAUDE.md with just name and directory conventions. Build section is empty string (no `### Build and Test` header).

2. **Workspace Rust projects** — `parse_cargo_name()` does naive line scanning; for a workspace `Cargo.toml` without `[package]` section, it may pick up a `name` from `[workspace.package]` or miss it entirely. Falls back to dir name.

3. **Settings.local.json already exists with different content** — init skips it, validate only checks existence. A user who has existing Claude settings may not have the idle_prompt hook configured.

4. **Permissions on non-Unix** — `#[cfg(unix)]` guard is correct; on Windows, executable bit is irrelevant.

5. **Ticket dir configured differently in .lisa.toml** — validate correctly reads `ticket_dir_rel` from config and uses it for scanning. This path works.

## Existing Test Coverage

- 21 tests in init.rs cover: empty dir, existing files, dry run, never-overwrites, validation of missing deps, parse errors, empty ticket dir, no ready tickets, missing CLAUDE.md, missing rdspi-workflow
- 7 tests in templates.rs cover: embedded content, generation for Rust/Unknown, hook/settings content
- 7 tests in detect.rs cover: each project type, priority order, source layout
- 17 tests in config.rs cover: parsing, loading, validation, resolution

## Key Observations

1. **Validation is already quite thorough.** The ticket parsing catches invalid types and phases at the `parse_ticket_type()`/`parse_phase()` level with clear error messages. The DAG builder catches missing dependencies. The scan returns per-file diagnostics.

2. **Hook validation could be stronger.** Currently just existence checks as warnings. Should be errors and should verify content (idle_prompt in settings.json) and permissions (executable on unix).

3. **CLAUDE.md template is clean.** No lisa-internal references leak through. The only enhancement needed is adding project type label.

4. **The biggest gap is test coverage for the validate-on-init-output scenario.** No test currently does `init` → `validate` round-trip to verify that init output passes validation.

5. **`settings.local.json` content validation** — currently only checks existence, not whether the `idle_prompt` hook is actually configured inside it.
