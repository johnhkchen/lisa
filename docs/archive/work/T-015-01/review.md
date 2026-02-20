# T-015-01 Review: Rewrite README for External Audience

## Changes Made

Complete rewrite of `README.md` from a developer-notes style to an external-audience landing page.

## Files Modified

- **README.md** — Full rewrite (~105 lines, down from 138)

## What Changed

- Replaced opening "ralph loop" reference with clean one-liner
- Rewrote "What It Does" as three focused paragraphs (problem, solution, workflow)
- Added Prerequisites section with Claude Code and Zellij links + `lisa doctor`
- Simplified Install to three paths: shell installer (cargo-dist curl one-liner), cargo install, from source
- Rewrote Quick Start as a user flow (not contributor flow) with copy-pasteable example ticket
- Structured "How It Works" into Workflow / Scheduling / Concurrency subsections
- Trimmed Project Layout to essentials
- Added Contributing link to CONTRIBUTING.md
- Kept License as MIT

## What Was Removed

- "Ralph loop" reference
- Four platform-specific curl download commands (replaced by single cargo-dist installer)
- Separate "Build" section (belongs in CONTRIBUTING.md)
- Redundant "Setting Up Your Project" section (merged into Quick Start)
- `just release` as a prerequisite for users

## Open Concerns

1. **Shell installer URL** — Uses standard cargo-dist pattern (`lisa-cli-installer.sh`). Will only resolve after first tagged release is published. The URL format is correct.
2. **cargo install caveat** — Noted that `cargo install lisa-cli` requires `wasm32-wasip1` target because build.rs compiles and embeds the WASM plugin. This is accurate but could be confusing; a future improvement might make the build.rs fallback gracefully.

## Acceptance Criteria Status

- [x] README follows the 9-section structure from the ticket
- [x] Install section covers shell installer, cargo install, from source
- [x] No "Ralph", no sprint references, no internal jargon
- [x] Quick start is copy-pasteable (includes full example ticket)
- [x] Reads well to someone with zero project context
