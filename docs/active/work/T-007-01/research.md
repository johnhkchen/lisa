# Research: T-007-01 — lisa-setup-guide-command

## Objective

Map the existing CLI structure, project detection, template generation, and content
embedding patterns to understand exactly what's available for a `setup-guide` subcommand.

---

## CLI Structure (main.rs)

The CLI uses clap `#[derive(Parser)]` with a `Commands` enum. Each subcommand:
- Is a variant of `Commands` with doc comment (used as help text)
- Accepts `--path` with `default_value = "."` (resolved to absolute via `resolve_path()`)
- Dispatches in `main()` match block, calling a module-level `run_*` function
- Error handling: print to stderr, `exit(1)`

Current subcommands: `Init`, `Validate`, `Status`, `Loop`.

Modules declared in main.rs: `config`, `detect`, `init`, `loop_cmd`, `status`, `templates`.

Pattern for new subcommand: add variant to `Commands`, add match arm, create module file,
declare `mod` in main.rs. All existing commands follow this pattern exactly.

---

## Project Detection (detect.rs)

`detect_project(root: &Path) -> DetectedProject` checks marker files in priority order:
Cargo.toml > package.json > go.mod > pyproject.toml > Unknown.

`DetectedProject` fields:
- `project_type: ProjectType` (enum: Rust, Node, Go, Python, Unknown)
- `name: String` (parsed from manifest file)
- `build_command: String`
- `test_command: String`
- `lint_command: String`
- `source_layout: String` (scanned from source dirs)

This is exactly what setup-guide needs. No changes to detect.rs required.

---

## Template Generation (templates.rs)

Two embedded constants:
- `RDSPI_WORKFLOW: &str` — full rdspi-workflow.md via `include_str!`
- `PLUGIN_WASM: &[u8]` — compiled WASM plugin via `include_bytes!`

One generator function:
- `generate_claude_md(project: &DetectedProject) -> String` — produces a CLAUDE.md with
  build/test/lint commands, source layout, and directory conventions pre-filled.

The RDSPI workflow is already available as a compile-time constant. The CLAUDE.md template
is already generated with project-specific content. Both can be reused directly.

---

## Config Defaults (config.rs)

`default_config_toml() -> &'static str` returns the default .lisa.toml content:
```toml
[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
# auto_advance = false
```

This is available for embedding in the guide output.

---

## Init Logic (init.rs)

`plan_init_actions(root, project) -> Vec<InitAction>` checks what exists:
- 6 directories: active/{tickets,stories,work}, archive/{tickets,stories,work}
- 3 files: CLAUDE.md, docs/rdspi-workflow.md, .lisa.toml

Each gets `CreateDir`/`CreateFile` or `Skip { reason: "already exists" }`.

The setup-guide command can reuse this same existence-check logic to decide which
steps to include vs skip. Alternatively, it can perform its own simpler checks since
it only needs to know "were directories already created?" not plan actual filesystem ops.

---

## RDSPI Workflow Content (docs/knowledge/rdspi-workflow.md)

Already embedded in `templates::RDSPI_WORKFLOW`. Contains:
- Five phase descriptions (Research, Design, Structure, Plan, Implement)
- Artifact paths: `docs/active/work/{ticket-id}/{phase}.md`
- Phase rules (all five run, ~200 lines each, phase transitions, review points)
- Ticket format: full YAML frontmatter spec with all fields
- Concurrency model: DAG computation, commit serialization, dependency modeling rule

This covers multiple acceptance criteria items:
- Ticket format (fields, body structure, dependency rules)
- Dependency modeling rules ("if two tickets modify the same files" rule)
- Phase transitions

Missing from the workflow doc (setup-guide must add):
- Story format (when and how to use stories)
- Archiving convention
- Exact `mkdir` commands

---

## Story Format

No formal story format documentation exists in the codebase. From examining
docs/active/stories/S-007.md, stories use this frontmatter:
```yaml
---
id: S-007
title: distribution-and-onboarding
type: story
status: open
priority: high
tickets: [T-007-01, T-007-02, T-007-03, T-007-04]
---
```

Body contains: narrative description, track breakdown, DAG for concurrency.
The setup-guide must document this since it's not in rdspi-workflow.md.

---

## Archiving Convention

No formal archiving documentation exists. The init command creates archive directories:
`docs/archive/{tickets,stories,work}`. Convention is presumably: move completed
tickets/stories/work from `active/` to `archive/`. The setup-guide should document this.

---

## What the Guide Must Output

Per acceptance criteria, the guide includes:
1. Directory structure to create (with exact mkdir commands)
2. CLAUDE.md template with detected build/test/lint pre-filled
3. RDSPI workflow content (full embed)
4. .lisa.toml default config
5. Ticket format: required frontmatter fields, body structure, dependency rules
6. Story format: when and how to use stories
7. Dependency modeling rules
8. Archiving convention
9. Clear "when done, run `lisa validate`" instruction

Items 2-5 and 7 are already available as existing code/constants.
Items 1, 6, 8, 9 need new content (simple strings).

---

## Existence Detection for Skipping Steps

The guide should skip structural steps if `lisa init` has already run. Detection signals:
- `docs/active/tickets` directory exists → init already ran for dirs
- `CLAUDE.md` exists → init already ran for CLAUDE.md
- `.lisa.toml` exists → init already ran for config
- `docs/rdspi-workflow.md` exists → init already ran for workflow

The guide can check these and adjust its output (e.g., "Step 1 already done — directories
exist" or omit the step entirely and renumber).

---

## Key Reusable Components

| Need                        | Source                               | Reuse Strategy       |
|-----------------------------|--------------------------------------|----------------------|
| Project detection           | `detect::detect_project()`           | Call directly         |
| CLAUDE.md template          | `templates::generate_claude_md()`    | Call directly         |
| RDSPI workflow text         | `templates::RDSPI_WORKFLOW`          | Embed in output       |
| .lisa.toml default          | `config::default_config_toml()`      | Embed in output       |
| Directory list              | `init::plan_init_actions()` dirs     | Reference same list   |
| Existence checks            | Simple `Path::exists()` calls        | Direct checks         |

---

## Constraints and Boundaries

- Output is stdout only (no file writes)
- Must be self-contained: an LLM reading it sets up the project without other docs
- Structured as numbered steps, not prose
- `--path` flag mirrors other subcommands
- No new dependencies needed
- Module: `setup_guide.rs` following existing naming convention
