# Plan: T-007-01 — lisa-setup-guide-command

## Implementation Steps

### Step 1: Create setup_guide.rs with core types and rendering

Create `crates/lisa-cli/src/setup_guide.rs` with:
- `GuideSection` struct (title, body)
- `render_guide(header, sections) -> String` function
- `run_setup_guide(root) -> Result<(), String>` public entry point (stub)
- `build_guide(root) -> Result<String, String>` internal function (stub)

**Verify:** `cargo check -p lisa-cli` passes.

### Step 2: Implement conditional content sections (1-3)

Implement the three sections that check filesystem state:
- `section_directories(root)` — lists mkdir commands, notes if dirs already exist
- `section_config(root)` — .lisa.toml default content via `config::default_config_toml()`
- `section_claude_md(root, project)` — CLAUDE.md via `templates::generate_claude_md()`

Each checks `Path::exists()` and adjusts content accordingly.

**Verify:** `cargo check -p lisa-cli` passes.

### Step 3: Implement static content sections (4-9)

Implement sections that don't depend on filesystem state:
- `section_rdspi_workflow()` — embeds `templates::RDSPI_WORKFLOW`
- `section_ticket_format()` — frontmatter fields, body structure, examples
- `section_story_format()` — story frontmatter, when to use stories
- `section_dependencies()` — DAG rules, "same files = missing edge"
- `section_archiving()` — move from active/ to archive/
- `section_validate()` — "run `lisa validate`"

**Verify:** `cargo check -p lisa-cli` passes.

### Step 4: Wire up build_guide and run_setup_guide

Complete the `build_guide()` function:
- Call `detect_project(root)`
- Build header with project name and type
- Collect all sections in order
- Call `render_guide()`

Complete `run_setup_guide()`:
- Call `build_guide()`
- Print to stdout

**Verify:** `cargo check -p lisa-cli` passes.

### Step 5: Add SetupGuide to CLI in main.rs

- Add `mod setup_guide;` to module declarations
- Add `SetupGuide` variant to `Commands` enum
- Add match arm in `main()`

**Verify:** `cargo check -p lisa-cli` passes. Can manually test with
`cargo run -p lisa-cli -- setup-guide`.

### Step 6: Write tests

Add `#[cfg(test)] mod tests` in setup_guide.rs:
- `test_guide_rust_project` — tempdir with Cargo.toml, check output has cargo commands
- `test_guide_node_project` — tempdir with package.json, check npm commands
- `test_guide_unknown_project` — empty tempdir, check graceful output
- `test_guide_already_initialized` — tempdir with init dirs/files, check "already exists"
- `test_guide_contains_rdspi_workflow` — check RDSPI content present
- `test_guide_contains_ticket_format` — check frontmatter fields documented
- `test_guide_step_numbering` — check "## Step 1", "## Step 2", etc. sequential

**Verify:** `cargo test -p lisa-cli` passes, all new tests green.

### Step 7: Full workspace check

Run `cargo test --workspace` to verify no regressions.
Run `cargo check -p lisa-plugin --target wasm32-wasip1` to verify WASM still compiles.

---

## Testing Strategy

All tests operate on `build_guide()` (returns String), not `run_setup_guide()` (prints).
This avoids stdout capture complexity.

Each test creates a `tempfile::tempdir()` with appropriate marker files:
- Rust: write `Cargo.toml` with `name = "test-app"`
- Node: write `package.json` with `"name": "test-app"`
- Unknown: empty dir
- Initialized: create `docs/active/tickets/`, `CLAUDE.md`, `.lisa.toml`

Assertions use `contains()` on the output string for key phrases, not exact matching.
This keeps tests stable as guide text evolves.

No integration tests needed — the command is pure output (no side effects, no network).

---

## Commit Points

1. After Step 5 (CLI wired up, compiles) — feature-complete skeleton
2. After Step 7 (all tests passing) — fully tested
