# T-015-01 Design: Rewrite README for External Audience

## Approach

Complete rewrite of README.md following the exact structure from the ticket. The current README has useful content that can be distilled but must be reorganized and reframed for someone with zero project context.

## Section-by-Section Design

### 1. Header + One-liner

```
# Lisa

DAG-driven concurrent task scheduling for AI-assisted development.
```

One sentence. No mention of Zellij (implementation detail at this level), no "RDSPI" (jargon), no "Ralph".

### 2. What It Does (2-3 paragraphs)

Paragraph 1: The problem — working on multiple interdependent tasks with AI coding assistants means manually sequencing them and context-switching between sessions.

Paragraph 2: The solution — Lisa reads markdown tickets with dependency metadata, computes a DAG, and runs multiple Claude Code sessions concurrently. It uses Zellij as the runtime.

Paragraph 3: The workflow — each ticket goes through five phases (Research → Design → Structure → Plan → Implement), producing reviewable artifacts at each step. Mention crash recovery as a benefit.

### 3. Prerequisites

Table or list format:
| Tool | Purpose |
|------|---------|
| [Claude Code](link) | AI coding assistant that does the actual work |
| [Zellij](link) | Terminal multiplexer that hosts Lisa as a plugin |

Then: "Run `lisa doctor` after install to verify."

### 4. Install

Three subsections in priority order:

**Shell installer (recommended):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```
Single command. cargo-dist handles platform detection.

**cargo install:**
```bash
cargo install lisa-cli
```
No caveat needed if it works. If the WASM embedding issue persists, one sentence note.

**From source:**
```bash
git clone https://github.com/johnhkchen/lisa
cd lisa
rustup target add wasm32-wasip1
just install
```
Four lines. Prerequisites (Rust, just) mentioned inline.

### 5. Quick Start

```bash
cd your-project
lisa init
# Create tickets in docs/active/tickets/
lisa loop
```

Include a minimal ticket example (the YAML frontmatter) so users can actually copy-paste something. This is critical for "copy-pasteability" acceptance criterion.

### 6. How It Works

Three subsections, each 2-4 sentences:

**RDSPI Workflow:** Five phases explained in one line each. Emphasize that artifacts are reviewable and provide crash recovery.

**DAG Scheduling:** Tickets declare dependencies. Lisa topologically sorts them and schedules tickets whose dependencies are satisfied.

**Concurrency:** Multiple Claude Code sessions work in parallel on the same branch. Commit serialization via file locking.

### 7. Project Layout

Simplified tree — just the three crates with one-line descriptions, plus the docs directory. Taken from CLAUDE.md but trimmed.

### 8. Contributing

One sentence + link: "See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions, test commands, and submission guidelines."

### 9. License

"MIT" with link to LICENSE file.

## Rejected Alternatives

### A: Minimal README (install + quickstart only)
Rejected: The ticket explicitly requires all nine sections. A minimal README also fails to explain RDSPI to newcomers, which is the core differentiator.

### B: Keep current structure, just clean up language
Rejected: The current structure has build instructions mixed with install, redundant sections, and wrong ordering. A rewrite is cleaner than incremental fixes.

### C: Add badges, screenshots, GIF demo
Rejected: Not in the ticket requirements. Can be added later. Keep it focused on text content.

## Estimated Size

~120-150 lines of markdown. Shorter than the current 138 lines despite covering more ground, because the install section is dramatically simplified (one curl command vs. four).

## Open Questions

1. **Shell installer URL:** Using the standard cargo-dist pattern. If no release has been published yet, the URL won't work — but the format is correct for when it does. No placeholder syntax needed; it's a real URL that activates on first release.

2. **cargo install WASM caveat:** The build.rs uses `include_bytes!` to embed the WASM plugin built for wasm32-wasip1. When `cargo install lisa-cli` runs from crates.io, it triggers build.rs which tries to copy the WASM from `target/wasm32-wasip1/release/`. This requires the user to have the wasm32-wasip1 target installed and to have built lisa-plugin first — which cargo install doesn't do. The caveat is still real. Keep a brief note.
