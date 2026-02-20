# T-TEST-02 Design: Build System Summary

## Problem

The task is to produce a focused summary of Lisa's build system. The research phase identified four build tools, a custom WASM embedding pipeline, and a two-stage build constraint. The summary needs to be useful — not just a list, but organized so a reader understands what to run and why.

## Options

### Option A: Organize by Tool

Group by build tool (Cargo, just, Nix, cargo-dist). Each section explains what the tool does and its key commands.

**Pros**: Matches how a developer would look up "how do I use X?"
**Cons**: The WASM embedding pipeline spans multiple tools, so it would be split across sections.

### Option B: Organize by Stage

Group by build stage (compile WASM, compile CLI, test, lint, release). Each section explains the stage and which tools participate.

**Pros**: Shows the build flow clearly. The two-stage constraint is front and center.
**Cons**: Some tools (like `just`) appear in every stage, which could feel repetitive.

### Option C: Organize by Audience

Group by use case: developer (daily workflow), CI (automated checks), release (distribution). Each section lists relevant commands.

**Pros**: Goal-oriented — readers find what they need based on what they're doing.
**Cons**: Overlaps between audiences. A developer also runs tests, which is also CI.

## Decision: Option A (Organize by Tool) with a Pipeline Overview

Option A is the most natural organization for a build system summary. A reader will typically want to know "what does each tool do?" rather than tracing a build stage across tools.

To address the WASM embedding concern, the summary will open with a short pipeline overview section that explains the two-stage build flow and the embedding mechanism. This gives context before diving into per-tool details.

## Rejected

- **Option B**: The stage-based organization would be better for a build system design doc, but this is a summary. Tool grouping is more scannable.
- **Option C**: The audience split creates too much overlap for a concise summary. Better suited for a contributor guide (which already exists in CONTRIBUTING.md).

## Format

The summary will be a single `progress.md` file (the Implement artifact) since this task's output IS the summary. The file structure:

1. Pipeline overview (2-stage build, WASM embedding)
2. Workspace structure (3 crates, dependency graph)
3. Per-tool sections (Cargo, just, Nix, cargo-dist)
4. Key commands quick reference table
