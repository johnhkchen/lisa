---
id: S-013
title: Runtime dependency checking with lisa doctor
type: story
status: Active
created: 2026-02-20
---

# S-013: Runtime dependency checking with `lisa doctor`

## Problem

Lisa depends on Claude Code and Zellij at runtime. If either is missing, `lisa loop` will fail in confusing ways. New users installing from a GitHub Release or `cargo install` won't necessarily have these prerequisites. The tool should check for them proactively and tell the user exactly what's missing and how to fix it.

## Goal

Add a `lisa doctor` subcommand that checks the user's environment for all required runtime dependencies, reports their status, and provides actionable install instructions for anything missing. `lisa loop` should also run these checks before launching and bail with a clear message if prerequisites aren't met.

## Tickets

- **T-013-01:** Implement `lisa doctor` subcommand (feature)
- **T-013-02:** Add dependency checks to `lisa loop` (task)

## Dependencies

```
T-013-01 (lisa doctor)
  └── T-013-02 (gate lisa loop)
```

## Success Criteria

1. `lisa doctor` checks for `claude` and `zellij` in PATH
2. `lisa doctor` reports version info when dependencies are found
3. `lisa doctor` prints install instructions for missing dependencies
4. `lisa loop` refuses to start if dependencies are missing, pointing user to `lisa doctor`
5. Exit codes are meaningful (0 = all good, non-zero = something missing)
