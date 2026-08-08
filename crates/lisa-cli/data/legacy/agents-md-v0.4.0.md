# AGENTS.md

Lisa runs coding agents like Claude Code and Codex through your ticket board, so you don't have to approve every step by hand.

Under Lisa, you take one ticket through every RDSPI phase, leave a reviewable record, and wait for Lisa to confirm completion.

This project's agent context lives in [CLAUDE.md](CLAUDE.md) — the single source of truth for every agent client (Claude Code reads `CLAUDE.md`; Codex reads this `AGENTS.md`). Read `CLAUDE.md` first.

The RDSPI workflow definition is in docs/knowledge/rdspi-workflow.md and is injected into agent context by lisa automatically.
