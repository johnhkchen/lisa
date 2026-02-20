---
id: T-015-02
title: Add CONTRIBUTING.md and clean up docs directory
type: task
phase: done
status: done
priority: medium
story: S-015
created: 2026-02-20
depends_on: []
---

# T-015-02: Add CONTRIBUTING.md and clean up docs directory

## Objective

Add contributor guidance and reduce noise in the docs directory for external users.

## Requirements

### CONTRIBUTING.md

Create a `CONTRIBUTING.md` at the repo root covering:

1. **Building from source** — prerequisites, build commands, running tests
2. **Project structure** — brief overview of the three crates and what they do
3. **Running tests** — `cargo test --workspace`, what the WASM check does
4. **Submitting changes** — fork, branch, PR workflow
5. **Code style** — `cargo fmt`, `cargo clippy`, any conventions
6. **Ticket/story system** — brief explanation of how Lisa uses its own ticket system for development

### Docs directory cleanup

The `docs/archive/` directory contains 30+ tickets, 9 stories, and ~150 work artifact files from Lisa's own development history. Options:

- **Recommended:** Keep `docs/archive/` but add a `docs/archive/README.md` explaining it's the project's development history (Lisa manages its own tickets with Lisa — this is actually a neat story for the audience)
- **Alternative:** Move to a separate `development-history` branch
- **Alternative:** Gitignore and remove from tracking

Also evaluate:
- `docs/specification.md` — if it wasn't moved in T-012-02, handle it here
- `docs/project-recap.md` — same

## Acceptance Criteria

- [ ] `CONTRIBUTING.md` exists at repo root
- [ ] Covers build, test, and contribution workflow
- [ ] `docs/archive/` has context for external visitors (README or equivalent)
- [ ] No orphaned or confusing docs visible to someone browsing the repo
