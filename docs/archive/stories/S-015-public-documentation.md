---
id: S-015
title: Public documentation
type: story
status: Active
created: 2026-02-20
---

# S-015: Public documentation

## Problem

Lisa's README was written during development and reads like developer notes. There's no CONTRIBUTING.md. The `docs/` directory contains archived sprint artifacts that are noise for external users. For people evaluating Lisa after hearing about it at a talk or finding the repo, the first impression needs to be clear and inviting.

## Goal

Rewrite public-facing documentation for an external audience. Make the README a strong landing page, add contributor guidance, and reduce noise from internal development history.

## Tickets

- **T-015-01:** Rewrite README for external audience (task)
- **T-015-02:** Add CONTRIBUTING.md and clean up docs directory (task)

## Dependencies

```
T-015-01 (README rewrite)
T-015-02 (CONTRIBUTING + docs cleanup)
```

Both tickets are independent. S-015 should be worked after S-012 (hygiene) lands so the README reflects the cleaned-up state.

## Success Criteria

1. README has: clear one-liner description, what Lisa does, prerequisites, install instructions (multiple paths), quickstart, how it works overview, link to RDSPI workflow, license
2. README install section covers: curl installer, cargo install, build from source
3. CONTRIBUTING.md exists with: how to build, how to run tests, how to submit changes
4. `docs/archive/` situation is resolved (moved, gitignored, or clearly labeled)
5. No internal jargon or sprint references in public-facing docs
