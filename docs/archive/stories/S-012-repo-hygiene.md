---
id: S-012
title: Repo hygiene and security sweep
type: story
status: Active
created: 2026-02-20
---

# S-012: Repo hygiene and security sweep

## Problem

Lisa is about to be shared publicly for the first time. The repo contains broken symlinks, hardcoded personal paths, legacy "Ralph" naming remnants, tracked local config files, and internal documents that shouldn't be in a public-facing project. These need to be cleaned up before anyone clones the repo.

## Goal

Make the repository safe and clean for public consumption. No new features — just removing or fixing anything that would confuse external users, leak personal details, or break on other people's machines.

## Tickets

- **T-012-01:** Fix broken symlink and Ralph naming remnants (chore)
- **T-012-02:** Clean up tracked local files and internal docs (chore)
- **T-012-03:** Fill placeholder URLs and fix dead code warnings (chore)

## Dependencies

```
T-012-01 (symlink + ralph rename)
T-012-02 (local files + internal docs)
T-012-03 (URLs + warnings)
```

All tickets are independent and can be worked in parallel.

## Success Criteria

1. `docs/rdspi-workflow.md` is not a broken symlink
2. No `.ralph-` references remain in source, config, or `.gitignore`
3. Dashboard header says "LISA" not "LISA/RALPH"
4. `.claude/settings.local.json` is gitignored, not tracked
5. No placeholder `<lisa-repo-url>` strings in any docs
6. `cargo check -p lisa-plugin --target wasm32-wasip1` produces no warnings
7. ROADMAP.md has no awkward references to external projects
