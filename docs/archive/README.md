# Development Archive

This directory contains Lisa's development history — managed by Lisa itself.

Lisa uses its own RDSPI ticket system to plan and execute development work. The artifacts here are the output of that process: 30 completed tickets across 9 stories, each with full RDSPI phase artifacts.

## Structure

```
tickets/    Completed ticket definitions (markdown with YAML frontmatter)
stories/    Completed story definitions (groups of related tickets)
work/       Phase artifacts — one subdirectory per ticket, each containing:
              research.md    Codebase mapping
              design.md      Options evaluated, decision with rationale
              structure.md   File-level change blueprint
              plan.md        Sequenced implementation steps
              progress.md    Implementation log
```

## Context

The [RDSPI workflow](../knowledge/rdspi-workflow.md) defines the five phases every ticket passes through. Each phase produces a ~200-line artifact that serves as both a review checkpoint and crash recovery insurance.

Also archived here:
- `specification.md` — the original design document (written when the project was named "Ralph")
- `project-recap.md` — development metrics and sprint history from the initial build
