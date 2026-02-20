# Contributing to Lisa

Thanks for your interest in contributing to Lisa! This guide covers everything you need to get started.

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)
- WASM target: `rustup target add wasm32-wasip1`
- [just](https://github.com/casey/just) command runner
- [Zellij](https://zellij.dev/) terminal multiplexer (only needed for running `lisa loop`)

## Building from Source

```bash
# Clone the repo
git clone https://github.com/johnhkchen/lisa
cd lisa

# Build the WASM plugin
just build

# Build the CLI (builds WASM plugin first, embeds it in the binary)
just build-cli

# Build + install to ~/.cargo/bin
just install
```

The CLI binary embeds the WASM plugin, so `just build-cli` (or `just install`) is all you need for a complete build.

## Project Structure

Lisa is a Cargo workspace with three crates:

| Crate | What it does |
|-------|-------------|
| `lisa-core` | Shared types, ticket parsing from YAML frontmatter, DAG computation. No Zellij dependencies — fully testable on native targets. |
| `lisa-plugin` | Zellij WASM plugin. Implements the scheduler, dashboard UI, and `ZellijPlugin` trait. Compiles to `wasm32-wasip1`. |
| `lisa-cli` | CLI binary (`lisa init`, `lisa validate`, `lisa loop`). Embeds the WASM plugin via `include_bytes!`. |

See `CLAUDE.md` for a detailed source layout.

## Running Tests

```bash
# Run all tests (native target, not WASM)
cargo test --workspace

# Or equivalently
just test

# WASM type-check + all tests
just check
```

Tests run on the native target because they avoid Zellij plugin APIs. The WASM check (`cargo check -p lisa-plugin --target wasm32-wasip1`) verifies the plugin compiles for WASM without running tests in that environment.

## Code Style

```bash
# Format
just fmt

# Lint (clippy with -D warnings on all 3 crates)
just lint

# Check formatting without modifying (CI uses this)
just fmt-check
```

We use standard `cargo fmt` formatting and `cargo clippy` linting. No additional style conventions beyond what these tools enforce.

## Submitting Changes

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Run `just check` to verify WASM compilation and tests pass
5. Run `just lint` and `just fmt` to ensure clean code
6. Open a pull request against `main`

Keep PRs focused — one logical change per PR. Include a clear description of what changed and why.

## Lisa's Ticket System

Lisa uses its own RDSPI ticket system to manage development. You'll notice two directories in `docs/`:

- **`docs/active/`** — Current tickets, stories, and work-in-progress phase artifacts
- **`docs/archive/`** — Completed tickets and their RDSPI artifacts from past sprints

Every ticket goes through five phases: Research, Design, Structure, Plan, Implement. Each phase produces a ~200-line artifact. See the [RDSPI workflow](docs/knowledge/rdspi-workflow.md) for details.

This means Lisa's own development history is visible in the repo — the archive contains 30 completed tickets with full phase artifacts, all produced by Lisa orchestrating Claude Code sessions.
