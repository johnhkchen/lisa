---
id: T-015-01
title: Rewrite README for external audience
type: task
phase: done
status: done
priority: high
story: S-015
created: 2026-02-20
depends_on: []
---

# T-015-01: Rewrite README for external audience

## Objective

Rewrite the README so it serves as a strong landing page for someone discovering Lisa for the first time — whether from a talk, a GitHub link, or a search result.

## Requirements

### Structure

The README should follow this order:

1. **Header + one-liner** — What Lisa is in one sentence
2. **What it does** — 2-3 paragraph overview of the problem Lisa solves and how
3. **Prerequisites** — Claude Code, Zellij, and how to check (`lisa doctor`)
4. **Install** — Three paths in priority order:
   - Shell installer (curl one-liner)
   - `cargo install lisa-cli`
   - Build from source
5. **Quick start** — `lisa init` → create tickets → `lisa loop`
6. **How it works** — Brief explanation of the RDSPI workflow, DAG scheduling, thread model
7. **Project layout** — Source structure for contributors
8. **Contributing** — Link to CONTRIBUTING.md
9. **License** — MIT

### Tone

- Write for a technical audience (Rust developers, CLI tool users, AI-assisted development enthusiasts)
- Assume they know what a terminal multiplexer is but don't know what RDSPI means
- No internal jargon, no sprint references, no "Ralph"
- Concise — the README should be scannable, not a wall of text

### Install section

- Use the actual URLs from the cargo-dist release (if S-014 is done)
- If S-014 isn't done yet, use placeholder format that's easy to fill in later
- Include `lisa doctor` as the first thing to run after install

## Acceptance Criteria

- [ ] README follows the structure above
- [ ] Install section covers all three paths
- [ ] No internal jargon, no "Ralph" references, no sprint history
- [ ] Quick start section is copy-pasteable
- [ ] Reads well to someone with zero context about the project
