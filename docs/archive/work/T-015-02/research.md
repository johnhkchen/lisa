# T-015-02 Research: CONTRIBUTING.md and Docs Cleanup

## Current Repo Root Files

The repo root contains:
- `README.md` — 137 lines, covers install (prebuilt/crates.io/source), build, quick start, how it works, project layout
- `CLAUDE.md` — 58 lines, project description + build/test + source layout + directory conventions
- `LICENSE` — MIT, John Chen, 2026
- `justfile` — 90 lines, comprehensive task runner with build/test/lint/fmt recipes
- `.gitignore` — 7 lines, minimal (target, .DS_Store, .lisa-layout.kdl, .lisa-state-dump.txt, .ralph-commit.lock, .obsidian/)
- No `CONTRIBUTING.md` exists yet

## Docs Directory Structure

```
docs/
  rdspi-workflow.md        → symlink to docs/knowledge/rdspi-workflow.md
  specification.md         14,211 bytes — "Ralph: Design Document", uses old project name throughout
  project-recap.md         5,423 bytes — build metrics, sprint history, personal project recap
  ROADMAP.md               6,688 bytes — sprint log + candidate sprints, references "moron" project
  knowledge/
    rdspi-workflow.md      RDSPI workflow definition (the canonical copy)
    lisa-loop-setup-guide.md
  active/
    tickets/               19 tickets (S-010 through S-016)
    stories/               7 stories (S-010 through S-016)
    work/                  7 work dirs (T-010-01 through T-016-02)
  archive/
    tickets/               30 archived tickets (T-001-01 through T-009-04)
    stories/               9 archived stories (S-001 through S-009)
    work/                  28 work dirs, each with 5 RDSPI artifacts (~140 files total)
```

## What External Visitors See

A new visitor browsing the repo would encounter:
1. `README.md` — good, covers usage and install
2. `CLAUDE.md` — fine, it's for Lisa's own agent context
3. `docs/` — immediately confusing:
   - `specification.md` titled "Ralph: Design Document" — who is Ralph? Old project name.
   - `project-recap.md` — internal development history with specific metrics
   - `ROADMAP.md` — sprint log with "moron Rust motion graphics engine" reference
   - `rdspi-workflow.md` symlink — ok but symlink to knowledge/ is slightly odd
   - `archive/` — 28 work dirs with 140+ files, no explanation of what they are
   - `active/` — current development tickets, expected for a lisa-managed project

## Overlap with T-012-02

T-012-02 ("Clean up tracked local files and internal docs") has overlapping scope:
- T-012-02 handles: `.claude/settings.local.json` gitignore, ROADMAP.md "moron" reference, specification.md and project-recap.md evaluation
- T-015-02 handles: CONTRIBUTING.md creation, docs/archive/ README, overall docs cleanup
- The specification.md and project-recap.md decisions overlap — both tickets say "evaluate" or "handle"
- T-012-02 status is `open`, phase `ready` — not yet started

Since both are independent (`depends_on: []`), this ticket should handle what it can without conflicting. The CONTRIBUTING.md and archive README are clearly in scope. For specification.md and project-recap.md, the simplest approach is to move them to archive since T-012-02 lists that as an option and it aligns with this ticket's goal of reducing docs noise.

## Existing Build/Test Information (for CONTRIBUTING.md)

From CLAUDE.md and justfile:
- **Prerequisites**: Rust toolchain (rustup), wasm32-wasip1 target, just, Zellij
- **Build**: `just build` (WASM), `just build-cli` (CLI with embedded WASM), `just release` (full)
- **Test**: `cargo test --workspace` or `just test`; `just check` does WASM check + tests
- **Lint**: `just lint` (clippy on all 3 crates, WASM target for plugin)
- **Format**: `just fmt` / `just fmt-check`
- **Install**: `just install` builds WASM then `cargo install` the CLI

Tests run on native target (not WASM) because they avoid zellij APIs. Currently 88+ tests.

## Existing Code Style

- Rust 2021 edition, standard cargo workspace
- `cargo fmt --all` for formatting
- `cargo clippy` with `-D warnings` for linting
- No explicit style guide documented beyond fmt/clippy
- Conventional commit style observed in git log

## Archive Content Character

The archive is actually interesting: it's Lisa building itself via its own RDSPI workflow. Each of the 28 work directories has research.md, design.md, structure.md, plan.md, and progress.md — showing the tool's output for real development tasks. This is valuable as a demonstration of the RDSPI approach and as dogfooding evidence.

## Summary of Findings

1. **CONTRIBUTING.md**: Doesn't exist. All needed information is available across CLAUDE.md, justfile, and README.md — just needs consolidation
2. **docs/archive/**: No README, 140+ files with no context. Needs explanation for visitors
3. **specification.md**: Old "Ralph" branding, internal design doc. Move to archive
4. **project-recap.md**: Internal build metrics. Move to archive
5. **ROADMAP.md**: "moron" reference (T-012-02 scope), but otherwise reasonable to keep
6. **rdspi-workflow.md symlink**: Minor oddity, functional, leave it
7. **T-012-02 overlap**: Coordinate by handling archive-related moves here, leaving .gitignore and ROADMAP fixes to T-012-02
