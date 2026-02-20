# T-TEST-01: Top-Level Repository File Listing

## Files

- **`.gitignore`** — Git ignore rules for build artifacts, macOS metadata, Lisa runtime files, and Nix output
- **`.lisa.toml`** — Lisa project configuration: ticket/story/work directory paths and scheduling settings
- **`Cargo.lock`** — Dependency lockfile for deterministic Rust builds
- **`Cargo.toml`** — Cargo workspace root defining three crates, shared metadata (v0.1.6, MIT), and release profiles
- **`CLAUDE.md`** — Project instructions for Claude Code sessions: build commands, source layout, directory conventions
- **`CONTRIBUTING.md`** — Contributor guide covering prerequisites, build instructions, test commands, and PR workflow
- **`dist-workspace.toml`** — cargo-dist configuration for cross-platform release builds and shell installer
- **`flake.nix`** — Nix flake for reproducible builds and dev shell with Rust, just, Zellij, and cargo-watch
- **`justfile`** — Task runner with recipes for building, testing, linting, formatting, and installing Lisa
- **`LICENSE`** — MIT license, copyright 2026 John Chen
- **`README.md`** — Public documentation: what Lisa does, installation, quick start, workflow, and project layout

## Directories

- **`.github/`** — GitHub CI workflows and cargo-dist build setup configuration
- **`.lisa/`** — Lisa runtime directory with lifecycle hooks (on-idle, on-clear, on-stop) and signal files
- **`crates/`** — Cargo workspace members: lisa-core (types/parsing/DAG), lisa-plugin (Zellij WASM plugin), lisa-cli (CLI binary)
- **`docs/`** — Documentation, ticket system (active tickets/stories/work artifacts), knowledge base, roadmap, and archive
- **`target/`** — Build output directory (gitignored)

## Completion

All acceptance criteria met:
- [x] `docs/active/work/T-TEST-01/research.md` exists with codebase map
- [x] `docs/active/work/T-TEST-01/design.md` exists with format decision
- [x] `docs/active/work/T-TEST-01/structure.md` exists with file-level plan
- [x] `docs/active/work/T-TEST-01/plan.md` exists with implementation steps
- [x] `docs/active/work/T-TEST-01/progress.md` exists documenting completion
