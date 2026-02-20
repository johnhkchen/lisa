# T-TEST-02 Plan: Build System Summary

## Steps

### Step 1: Write progress.md with Build Pipeline Overview

Write the opening section of progress.md covering:
- The two-stage build constraint (WASM first, then CLI)
- The WASM embedding mechanism (build.rs → include_bytes! → runtime extraction)
- Why `cargo build --workspace` alone doesn't work

**Verify**: Section clearly explains the build flow to someone unfamiliar with the project.

### Step 2: Add Workspace Structure Section

Add to progress.md:
- Three-crate table (lisa-core, lisa-plugin, lisa-cli) with type, target, and purpose
- Internal dependency graph showing lisa-cli and lisa-plugin both depend on lisa-core
- Note that lisa-cli embeds lisa-plugin via bytes, not via Cargo dependency

**Verify**: Dependency relationships are accurate per Cargo.toml files.

### Step 3: Add Build Tools Sections

Add per-tool sections:
- **Cargo**: core commands, release profile (opt-level "s", LTO)
- **just**: key recipes (check, build, build-cli, release, install, lint, fmt, watch)
- **Nix flake**: crane-based build, dev shell, platform support
- **cargo-dist**: GitHub Actions CI, shell installer, 4-platform targets

**Verify**: Commands match the actual justfile and config files.

### Step 4: Add Quick Reference Table

Add a command lookup table mapping common tasks to their commands:
- Check types → `just check`
- Build WASM → `just build`
- Build CLI → `just build-cli`
- Run tests → `just test` or `cargo test --workspace`
- Install locally → `just install`
- Lint → `just lint`
- Nix build → `nix build`
- Nix dev shell → `nix develop`

**Verify**: Every command listed is correct and runnable.

### Step 5: Add Completion Section and Update Ticket

Add acceptance criteria checklist to progress.md. Update ticket phase to `implement`, then to `done` after all artifacts confirmed.

**Verify**: All 5 acceptance criteria files exist.

## Testing Strategy

This is a documentation-only ticket. Verification is:
1. All 5 artifact files exist in `docs/active/work/T-TEST-02/`
2. The summary is factually accurate against the source files
3. Ticket frontmatter shows `phase: done`, `status: done`

## Notes

Steps 1-4 will be written as a single file (progress.md). They're separated here for clarity, but since there's no code to compile or test incrementally, a single write is appropriate.
