# Lisa

A Zellij plugin for DAG-driven concurrent task scheduling. An homage to the ralph loop, but smarter.

Lisa reads ticket files with YAML frontmatter, computes a dependency graph, and spawns concurrent Claude Code sessions that work through the RDSPI workflow (Research, Design, Structure, Plan, Implement). It carries between projects as a single `.wasm` file.

## Install

### Prebuilt binaries (recommended)

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-aarch64-macos.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-x86_64-macos.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-x86_64-linux.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# Linux (ARM64)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-aarch64-linux.tar.gz | tar xz
sudo mv lisa /usr/local/bin/
```

Or download from the [releases page](https://github.com/johnhkchen/lisa/releases).

### From crates.io

```bash
cargo install lisa-cli
```

> **Note:** `cargo install` provides the CLI without the embedded Zellij WASM plugin.
> The `lisa loop` command requires building from source or using a prebuilt binary.

### From source

```bash
git clone https://github.com/johnhkchen/lisa
cd lisa
rustup target add wasm32-wasip1

# Build + install the `lisa` CLI to ~/.cargo/bin
just install
```

This builds the WASM plugin, embeds it in the CLI binary, and installs it via `cargo install`. Make sure `~/.cargo/bin` is on your `PATH`.

#### Prerequisites

- Rust toolchain (`rustup`)
- `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- [just](https://github.com/casey/just) command runner
- [Zellij](https://zellij.dev/) terminal multiplexer (for `lisa loop`)

## Build

```bash
# Prerequisites
rustup target add wasm32-wasip1

# Build the WASM plugin
just build

# Build the CLI (with embedded WASM plugin)
just build-cli

# Build + install to ~/.cargo/bin
just install

# Run tests
just test

# Build + test
just check
```

## Quick Start

```bash
# Build (WASM plugin + CLI with embedded plugin)
just release

# Initialize any project for lisa-loop
cd your-project
lisa init

# Write tickets in docs/active/tickets/, then:
lisa loop
```

## How It Works

1. You write tickets as markdown files with YAML frontmatter defining dependencies
2. Lisa computes a DAG from `depends_on` relationships
3. Tickets with satisfied dependencies get scheduled as Claude Code sessions
4. Each session works through 5 phases: Research, Design, Structure, Plan, Implement
5. Phase artifacts (~200 lines each) provide review checkpoints and crash recovery
6. Concurrent sessions share a branch with commit serialization via file locking

## Project Layout

```
crates/
  lisa-core/          Shared types, ticket parsing, DAG computation
  lisa-plugin/        Zellij WASM plugin (scheduler, UI, plugin entry)
  lisa-cli/           CLI binary (lisa init, lisa validate, lisa loop)

docs/
  knowledge/
    rdspi-workflow.md   RDSPI workflow definition
    lisa-loop-setup-guide.md  Setup guide for other projects
  active/tickets/     Example ticket files
  active/stories/     Example story files
  active/work/        Phase artifacts (auto-populated)
  ROADMAP.md          Sprint log and candidates
```

## Setting Up Your Project

Run `lisa init` in your project root. It will:
- Detect your project type (Rust, Node, Go, Python)
- Create `docs/active/{tickets,stories,work}` and `docs/archive/` directories
- Generate a project-specific `CLAUDE.md`
- Copy the RDSPI workflow document

Then run `lisa validate` to check your setup.

## License

MIT
