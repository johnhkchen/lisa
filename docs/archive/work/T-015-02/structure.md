# T-015-02 Structure: CONTRIBUTING.md and Docs Cleanup

## Files Created

### CONTRIBUTING.md (repo root, ~90 lines)

```
# Contributing to Lisa

## Prerequisites
  - Rust toolchain, wasm32-wasip1 target, just, Zellij

## Building from Source
  - just build, just build-cli, just install

## Running Tests
  - cargo test --workspace / just test / just check
  - Explanation of WASM check vs native tests

## Project Structure
  - Brief overview of 3 crates (core, plugin, cli) and what each does

## Code Style
  - cargo fmt, cargo clippy with -D warnings
  - just lint, just fmt, just fmt-check

## Submitting Changes
  - Fork, branch, make changes, run checks, open PR

## Lisa's Ticket System
  - Brief explanation that Lisa uses its own RDSPI ticket system
  - Points to docs/active/ for current work and docs/archive/ for history
  - Links to RDSPI workflow doc
```

### docs/archive/README.md (~30 lines)

```
# Development Archive

  - What: Lisa's development history, managed by Lisa itself
  - Structure: tickets/, stories/, work/ (RDSPI artifacts)
  - Context: Each work/ directory contains 5 phase artifacts
  - Link to RDSPI workflow for understanding the artifact structure
```

## Files Moved

### docs/specification.md → docs/archive/specification.md
- `git mv docs/specification.md docs/archive/specification.md`
- Original "Ralph: Design Document" — historical, uses old project name

### docs/project-recap.md → docs/archive/project-recap.md
- `git mv docs/project-recap.md docs/archive/project-recap.md`
- Development metrics and sprint history — historical artifact

## Files Not Modified

- `README.md` — no changes needed, already comprehensive
- `CLAUDE.md` — no changes needed
- `docs/ROADMAP.md` — T-012-02 scope
- `docs/rdspi-workflow.md` — symlink, leave as-is
- `docs/knowledge/*` — no changes
- `.gitignore` — no changes (T-012-02 scope)

## Module Boundaries

No code changes. All modifications are documentation files. No Rust source files touched. No build or test changes.
