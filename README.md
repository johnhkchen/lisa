# Lisa

A Zellij plugin for DAG-driven concurrent task scheduling. An homage to the ralph loop, but smarter.

Lisa reads ticket files with YAML frontmatter, computes a dependency graph, and spawns concurrent Claude Code sessions that work through the RDSPI workflow (Research, Design, Structure, Plan, Implement). It carries between projects as a single `.wasm` file.

## Status

Early development. Core modules (types, ticket parsing, DAG, scheduler, UI) are implemented with tests. The plugin compiles for `wasm32-wasip1`.

## Build

```bash
# Prerequisites
rustup target add wasm32-wasip1

# Build the plugin
just build

# Run tests
just test

# Build + test
just check
```

## How It Works

1. You write tickets as markdown files with YAML frontmatter defining dependencies
2. Lisa computes a DAG from `depends_on`/`blocks` relationships
3. Tickets with satisfied dependencies get scheduled as Claude Code sessions
4. Each session works through 5 phases: Research, Design, Structure, Plan, Implement
5. Phase artifacts (~200 lines each) provide review checkpoints and crash recovery
6. Concurrent sessions share a branch with commit serialization via file locking

## Project Layout

```
src/
  lib.rs          Plugin entry point, ZellijPlugin trait impl
  types.rs        Core types (Ticket, Phase, Thread, Config)
  ticket.rs       Ticket parsing from markdown frontmatter
  dag.rs          DAG computation from ticket dependencies
  scheduler.rs    Thread scheduling and commit serialization
  ui.rs           Dashboard rendering

docs/
  specification.md        Design document
  lisa-loop-setup-guide.md  Setup guide for other projects
  active/tickets/         Example ticket files
  active/stories/         Example story files
  active/work/            Phase artifacts (auto-populated)
```

## Setting Up Your Project

See [docs/lisa-loop-setup-guide.md](docs/lisa-loop-setup-guide.md) for a guide on setting up any project for lisa-loop completion.

## License

MIT
