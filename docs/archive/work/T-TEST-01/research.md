# T-TEST-01 Research: Top-Level Repository Files

## Overview

The Lisa repository root contains 12 visible entries plus several dotfiles/directories. This document maps each one with its purpose and role in the project.

## Top-Level Files

### Cargo.toml
Workspace root manifest. Declares three workspace members (`crates/*`), shared package metadata (version 0.1.6, MIT license, edition 2021), and release profile settings (opt-level "s", LTO enabled). Also defines a `dist` profile inheriting from release for cargo-dist.

### Cargo.lock
Lockfile for deterministic dependency resolution across builds. Checked into version control (standard practice for binary/application crates).

### CLAUDE.md
Project instructions for Claude Code sessions. Documents build commands, source layout, and directory conventions. Lisa injects this into agent context automatically, so it serves as the canonical reference for any AI session working in this repo.

### CONTRIBUTING.md
Contributor guide covering prerequisites (Rust toolchain, wasm32-wasip1 target, just, Zellij), build instructions, test commands, code style (cargo fmt + clippy), and PR workflow.

### README.md
Public-facing project documentation. Explains what Lisa does, installation methods (shell installer, crates.io, source), quick start guide, workflow overview (RDSPI), scheduling/concurrency model, and project layout.

### LICENSE
MIT license, copyright 2026 John Chen.

### justfile
Task runner recipes for development. Key targets: `check` (WASM check + tests, the default), `build` (WASM plugin), `build-cli` (WASM then CLI), `release` (full distribution build), `install` (WASM build + cargo install to ~/.cargo/bin), `test`, `lint`, `fmt`, `watch`. Also includes convenience recipes for `init-dry-run`, `init`, and `validate`.

### dist-workspace.toml
Configuration for cargo-dist (v0.30.4). Targets four platforms (x86_64/aarch64 for macOS and Linux), produces shell installer, packages only `lisa-cli`, and references a custom GitHub build setup file. Install path is CARGO_HOME.

### flake.nix
Nix flake for reproducible builds and development environments. Uses crane for Rust builds and rust-overlay for toolchain management. Builds the WASM plugin as a pre-build step then the CLI. Dev shell provides Rust (with wasm32-wasip1 target), just, zellij, and cargo-watch. Supports four platforms (x86_64/aarch64 for Linux and macOS).

### .gitignore
Ignores build artifacts (`/target`), macOS metadata (`.DS_Store`), Lisa runtime files (`.lisa-layout.kdl`, `.lisa-state-dump.txt`, `.lisa-commit.lock`), Obsidian config, Claude local settings, and a `result` symlink (from Nix builds).

### .lisa.toml
Project-level Lisa configuration. Sets custom directory paths for tickets, stories, and work artifacts (all under `docs/active/`). Configures `max_threads = 2`. Has commented-out options for `auto_advance` and `review_timeout_secs`.

## Top-Level Directories

### crates/
Cargo workspace members. Three crates:
- **lisa-core** — Shared types (`Ticket`, `Phase`, `Thread`, `PluginConfig`), ticket parsing from YAML frontmatter, DAG computation. No Zellij dependencies; fully testable on native targets.
- **lisa-plugin** — Zellij WASM plugin. Implements scheduler, dashboard UI, and `ZellijPlugin` trait. Compiles to `wasm32-wasip1` (cdylib).
- **lisa-cli** — CLI binary providing `lisa init`, `lisa validate`, `lisa loop`, `lisa doctor`. Embeds the WASM plugin at compile time via `include_bytes!`.

### docs/
Documentation and ticket system:
- **docs/active/tickets/** — 22 ticket files (markdown with YAML frontmatter)
- **docs/active/stories/** — 7 story files grouping related tickets
- **docs/active/work/** — Phase artifacts organized by ticket ID (14 tickets have work directories)
- **docs/archive/** — Completed sprint artifacts
- **docs/knowledge/** — Reference docs including `rdspi-workflow.md`
- **docs/ROADMAP.md** — Sprint log and candidate sprints

### target/
Build output directory (gitignored). Contains native and WASM compilation artifacts.

### .github/
GitHub CI configuration:
- **workflows/** — CI/CD workflow definitions
- **build-setup.yml** — Custom build setup for cargo-dist (referenced by dist-workspace.toml)

### .lisa/
Lisa runtime directory:
- **hooks/** — Shell scripts for lifecycle events (on-idle, on-clear, on-stop)
- **signals/** — Runtime signal files for inter-process communication

## Relationships

The build chain flows: `crates/lisa-plugin` (WASM) → embedded into `crates/lisa-cli` (native binary) via `build.rs` + `include_bytes!`. The `justfile` orchestrates this two-stage build. Distribution is handled by `dist-workspace.toml` (cargo-dist) and `flake.nix` (Nix).

At runtime, `lisa loop` writes the embedded WASM to `/tmp/lisa-plugin.wasm`, generates a KDL layout file, and execs Zellij. The plugin reads tickets from the paths configured in `.lisa.toml`, computes a DAG, and spawns Claude Code sessions for ready tickets.

## Constraints and Observations

- The repo has 12 top-level visible items + 4 dotfiles/directories = 16 total entries (excluding `target/`)
- All Rust source lives under `crates/`; the root only has configuration and documentation
- The project uses three build systems: Cargo (primary), just (task runner), Nix (reproducible), and cargo-dist (release)
- `.lisa.toml` is the only project-specific config; everything else is standard Rust/Nix/GitHub tooling
