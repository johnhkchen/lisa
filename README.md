# Lisa

A Zellij plugin for DAG-driven concurrent task scheduling. An homage to the ralph loop, but smarter.

Lisa reads ticket files with YAML frontmatter, computes a dependency graph, and spawns concurrent Claude Code sessions that work through the RDSPI workflow (Research, Design, Structure, Plan, Implement). It carries between projects as a single `.wasm` file.

## Status

Early development. Core modules (types, ticket parsing, DAG, scheduler, UI) are implemented with tests. The plugin compiles for `wasm32-wasip1`. The `lisa` CLI supports `init`, `validate`, and `loop` commands.

## Build

```bash
# Prerequisites
rustup target add wasm32-wasip1

# Build the plugin
just build

# Build the CLI
just build-cli

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
