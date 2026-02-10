# CLAUDE.md

## Project

Lisa is a Zellij WASM plugin (Rust) that implements DAG-driven concurrent task scheduling for the RDSPI workflow. It manages Claude Code sessions -- spawning, tracking, and scheduling them based on ticket dependencies. It carries between projects as a single `.wasm` file with zero project-specific dependencies.

### Build and Test

```bash
# Build the WASM plugin
cargo build -p lisa-plugin --target wasm32-wasip1 --release

# Build the CLI
cargo build -p lisa-cli --release

# Run tests (native target, not wasm)
cargo test --workspace

# Quick check (WASM check + tests)
just check
```

### Source Layout

```
crates/
  lisa-core/          Shared types, ticket parsing, DAG computation
    src/
      lib.rs          Re-exports modules
      types.rs        Core types (Ticket, Phase, Thread, Config)
      ticket.rs       Ticket parsing from markdown frontmatter
      dag.rs          DAG computation from ticket dependencies
  lisa-plugin/        Zellij WASM plugin
    src/
      lib.rs          Plugin entry point, ZellijPlugin trait impl
      scheduler.rs    Thread scheduling and commit serialization
      ui.rs           Dashboard rendering
  lisa-cli/           CLI binary (lisa init, lisa validate, lisa loop)
    src/
      main.rs         Clap CLI entry point
      detect.rs       Project type detection
      init.rs         Init and validate commands
      loop_cmd.rs     Loop command: embeds WASM, generates layout, execs zellij
      templates.rs    CLAUDE.md generation, embedded RDSPI workflow + WASM
    build.rs          Copies WASM plugin to OUT_DIR for embedding
```

### Directory Conventions

```
docs/active/tickets/    # Ticket files (markdown with YAML frontmatter)
docs/active/stories/    # Story files (same frontmatter pattern)
docs/active/work/       # Work artifacts, one subdirectory per ticket ID
```

---

The RDSPI workflow definition is in docs/rdspi-workflow.md and is injected into agent context by lisa automatically.
