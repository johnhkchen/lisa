# T-015-01 Structure: Rewrite README for External Audience

## File Changes

**Modified:** `README.md` — Complete rewrite (only file in scope)

No files created or deleted.

## README.md Structure

The file is organized as nine sequential sections with no nesting beyond H3.

### Section 1: Header (lines ~1-3)
```markdown
# Lisa

DAG-driven concurrent task scheduling for AI-assisted development.
```

### Section 2: What It Does (lines ~5-15)
Three short paragraphs:
- Problem statement (manual sequencing of AI coding tasks)
- Solution (Lisa reads tickets, computes DAG, runs concurrent Claude Code sessions in Zellij)
- Workflow overview (five phases, reviewable artifacts, crash recovery)

No code blocks. No bullet lists. Prose only.

### Section 3: Prerequisites (lines ~17-25)
```markdown
## Prerequisites
```
Bulleted list with links:
- Claude Code (link to docs.anthropic.com)
- Zellij (link to zellij.dev)

One-liner: "After installing Lisa, run `lisa doctor` to verify."

### Section 4: Install (lines ~27-55)
```markdown
## Install
### Shell installer (recommended)
### From crates.io
### From source
```

Shell installer: single `curl | sh` command.

From crates.io: `cargo install lisa-cli` with note about needing wasm32-wasip1 target and pre-built WASM plugin.

From source: clone + rustup target add + just install. Note prerequisites (Rust toolchain, just).

### Section 5: Quick Start (lines ~57-85)
```markdown
## Quick Start
```

Three steps with code blocks:
1. `lisa init` in project directory
2. Example ticket file (minimal YAML frontmatter + brief body)
3. `lisa loop`

The example ticket is critical — it's the only way a user knows what to put in the ticket files. Keep it minimal: id, title, type, status, phase, plus 2-line body.

### Section 6: How It Works (lines ~87-115)
```markdown
## How It Works
### Workflow
### Scheduling
### Concurrency
```

Workflow: One sentence per phase (Research, Design, Structure, Plan, Implement). Mention artifact size (~200 lines) and review checkpoints.

Scheduling: DAG from `depends_on` fields. Tickets scheduled when dependencies satisfied.

Concurrency: Multiple sessions on same branch. Commit serialization via file locking.

### Section 7: Project Layout (lines ~117-135)
```markdown
## Project Layout
```

Code block with tree structure. Three crates with one-line descriptions. `docs/` directory with ticket/story/work subdirectories.

### Section 8: Contributing (lines ~137-139)
```markdown
## Contributing
```

One sentence linking to CONTRIBUTING.md.

### Section 9: License (lines ~141-143)
```markdown
## License
```

"MIT" — single word or link to LICENSE file.

## Content Boundaries

**In scope:** Everything above.

**Out of scope (handled by other tickets or not requested):**
- Badges, CI status shields
- Screenshots or GIFs
- Detailed ticket format documentation (lives in rdspi-workflow.md)
- CONTRIBUTING.md changes (T-015-02)
- Changelog
