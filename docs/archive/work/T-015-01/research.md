# T-015-01 Research: Rewrite README for External Audience

## Current README State

The existing README (~138 lines) is functional but has issues for an external audience:

1. **Opening line references "ralph loop"** — internal jargon that means nothing to outsiders
2. **Install section is bloated** — four platform-specific curl commands, a crates.io section with a caveat about missing WASM, and a from-source section with prerequisites buried under it
3. **Build section is separate from install** — confusing for users who just want to install vs. contributors who want to build
4. **Quick start requires `just release`** first — this is a contributor workflow, not a user workflow
5. **"Setting Up Your Project" section** is redundant with Quick Start
6. **No `lisa doctor`** mentioned anywhere
7. **No shell installer one-liner** — despite cargo-dist now generating one (T-014-01 is done)

## Distribution Infrastructure (S-014 State)

T-014-01 (cargo-dist integration) is **done**. Key facts:

- **dist-workspace.toml** configures cargo-dist v0.30.4
- **Targets:** x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
- **Installers:** `shell` (cargo-dist generates a platform-detecting shell installer)
- **Install path:** CARGO_HOME (~/.cargo/bin)
- **Release workflow:** `.github/workflows/release.yml` is cargo-dist generated, triggers on version tags
- **Package:** only `lisa-cli` is distributed (binary name: `lisa`)
- **Repository:** `https://github.com/johnhkchen/lisa`
- **Current version:** 0.1.6

The cargo-dist shell installer URL pattern is:
```
https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh
```
(Standard cargo-dist format — needs verification on actual release)

## CLI Commands Available

From main.rs, the user-facing commands are:
- `lisa init` — Initialize a project for lisa-loop
- `lisa validate` — Validate ticket DAG and project setup
- `lisa status` — Show DAG status
- `lisa doctor` — Check runtime dependencies (zellij, claude, wasm target)
- `lisa loop` — Launch zellij with the Lisa plugin
- `lisa version` — Print version
- `lisa setup-guide` — Output LLM-friendly setup instructions

## Prerequisites (from doctor.rs)

Required:
- **Zellij** — terminal multiplexer (`zellij --version`)
- **Claude Code** — AI coding assistant (`claude --version`)

Optional (only for building from source):
- **wasm32-wasip1** Rust target

## CONTRIBUTING.md

Already exists (94 lines). Covers prerequisites, building, project structure, tests, code style, submitting changes, and the ticket system. T-015-02 handles any cleanup.

## Structural Requirements from Ticket

The ticket specifies this exact order:
1. Header + one-liner
2. What it does (2-3 paragraphs)
3. Prerequisites (Claude Code, Zellij, lisa doctor)
4. Install (shell installer → cargo install → from source)
5. Quick start (lisa init → create tickets → lisa loop)
6. How it works (RDSPI, DAG, threads)
7. Project layout (for contributors)
8. Contributing (link to CONTRIBUTING.md)
9. License (MIT)

## Tone Constraints

- Technical audience (Rust devs, CLI tool users, AI dev enthusiasts)
- Assume they know terminal multiplexers but not RDSPI
- No: "Ralph", sprint references, internal jargon
- Concise and scannable

## Key Content Decisions

### Shell installer URL
Cargo-dist generates the installer script. The exact URL depends on whether a release has been published. The standard cargo-dist pattern for a shell installer is:
```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```
If no release exists yet, use this format with a note that it becomes active after first release.

### cargo install caveat
The current README says `cargo install` doesn't include the WASM plugin. The cli Cargo.toml has `readme = "../../README.md"` and proper metadata. Need to check: does `cargo install lisa-cli` from crates.io actually work end-to-end? The build.rs embeds WASM via include_bytes!, but cargo install from crates.io won't have the WASM pre-built. This caveat may still be relevant — **keep it but phrase it clearly**.

### Quick start flow
The user flow should be:
1. Install lisa (any method)
2. `lisa doctor` to verify dependencies
3. `cd your-project && lisa init`
4. Create ticket files in `docs/active/tickets/`
5. `lisa loop`

No `just release` or build steps for end users.

### What to cut
- The entire "Build" section (belongs in CONTRIBUTING.md)
- "Setting Up Your Project" section (merged into Quick Start)
- Ralph reference
- Platform-specific curl commands (replaced by single shell installer)

## Files to Modify

- `README.md` — Complete rewrite (this is the only file in scope)

## Existing Assets to Reference

- RDSPI workflow explanation from `docs/knowledge/rdspi-workflow.md`
- Project structure from `CLAUDE.md`
- Doctor command behavior from `crates/lisa-cli/src/doctor.rs`
- Ticket format from `docs/knowledge/rdspi-workflow.md`
