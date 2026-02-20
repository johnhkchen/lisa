# T-015-02 Progress: CONTRIBUTING.md and Docs Cleanup

## Completed

### Step 1: Move historical docs to archive
- `git mv docs/specification.md docs/archive/specification.md`
- `git mv docs/project-recap.md docs/archive/project-recap.md`
- Both files now live in archive alongside the other historical artifacts

### Step 2: Create docs/archive/README.md
- Created 25-line README explaining the archive contents
- Covers: what the archive is, directory structure, link to RDSPI workflow
- Mentions the moved specification.md and project-recap.md

### Step 3: Create CONTRIBUTING.md
- Created 88-line contributor guide at repo root
- Six sections per ticket requirements:
  1. Prerequisites (Rust, wasm32-wasip1, just, Zellij)
  2. Building from source (just build, build-cli, install)
  3. Project structure (3-crate overview table)
  4. Running tests (cargo test, just test, just check, WASM check explanation)
  5. Code style (just fmt, just lint, just fmt-check)
  6. Submitting changes (fork, branch, PR workflow)
  7. Lisa's ticket system (RDSPI explanation, active/archive context)

### Step 4: Verify acceptance criteria
- [x] `CONTRIBUTING.md` exists at repo root
- [x] Covers build, test, and contribution workflow (all 6 sections)
- [x] `docs/archive/` has context for external visitors (README.md)
- [x] No orphaned or confusing docs visible browsing the repo
  - `docs/` top-level now shows: active/, archive/, knowledge/, rdspi-workflow.md, ROADMAP.md
  - specification.md and project-recap.md moved to archive

## Deviations from Plan
None.

## Left for T-012-02
- ROADMAP.md "moron" reference cleanup
- `.claude/settings.local.json` gitignore
