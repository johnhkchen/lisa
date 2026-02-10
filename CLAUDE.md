# CLAUDE.md

## Project

Lisa is a Zellij WASM plugin (Rust) that implements DAG-driven concurrent task scheduling for the RDSPI workflow. It manages Claude Code sessions -- spawning, tracking, and scheduling them based on ticket dependencies. It carries between projects as a single `.wasm` file with zero project-specific dependencies.

### Build and Test

```bash
# Build the WASM plugin
cargo build --target wasm32-wasi --release

# Run tests (native target, not wasm)
cargo test
```

### Source Layout

```
src/
  lib.rs        # Plugin entry point, ZellijPlugin trait impl
  types.rs      # Core data types (Ticket, Phase, Status, etc.)
  ticket.rs     # Ticket parsing from markdown frontmatter
  dag.rs        # DAG computation from ticket dependencies
  scheduler.rs  # Thread scheduling based on DAG state
  ui.rs         # Dashboard rendering
```

### Directory Conventions

```
docs/active/tickets/    # Ticket files (markdown with YAML frontmatter)
docs/active/stories/    # Story files (same frontmatter pattern)
docs/active/work/       # Work artifacts, one subdirectory per ticket ID
```

---

The RDSPI workflow definition is in docs/rdspi-workflow.md and is injected into agent context by lisa automatically.
