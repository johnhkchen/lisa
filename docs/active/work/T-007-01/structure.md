# Structure: T-007-01 — lisa-setup-guide-command

## Files Changed

### New File: `crates/lisa-cli/src/setup_guide.rs`

The sole new file. Contains all setup-guide logic.

**Public interface:**
```rust
pub fn run_setup_guide(root: &Path) -> Result<(), String>
```

**Internal types:**
```rust
struct GuideSection {
    title: String,
    body: String,
}
```

**Internal functions:**
```rust
/// Build the guide content as a String (testable without stdout)
fn build_guide(root: &Path) -> Result<String, String>

/// Render sections into a numbered markdown document
fn render_guide(header: &str, sections: Vec<GuideSection>) -> String

/// Build the directory creation section
fn section_directories(root: &Path) -> GuideSection

/// Build the .lisa.toml section
fn section_config(root: &Path) -> GuideSection

/// Build the CLAUDE.md section
fn section_claude_md(root: &Path, project: &DetectedProject) -> GuideSection

/// Build the RDSPI workflow section
fn section_rdspi_workflow() -> GuideSection

/// Build the ticket format section
fn section_ticket_format() -> GuideSection

/// Build the story format section
fn section_story_format() -> GuideSection

/// Build the dependency modeling section
fn section_dependencies() -> GuideSection

/// Build the archiving section
fn section_archiving() -> GuideSection

/// Build the final "validate" section
fn section_validate() -> GuideSection
```

**Tests (in `#[cfg(test)] mod tests`):**
- `test_guide_rust_project` — full guide for Rust project, check key content
- `test_guide_node_project` — Node project, different commands
- `test_guide_unknown_project` — Unknown project, graceful handling
- `test_guide_already_initialized` — dirs/files exist, sections adapted
- `test_guide_contains_rdspi_workflow` — RDSPI content embedded
- `test_guide_contains_ticket_format` — ticket frontmatter documented
- `test_guide_step_numbering` — steps are numbered sequentially

---

### Modified File: `crates/lisa-cli/src/main.rs`

Changes:
1. Add `mod setup_guide;` declaration (line ~5, with other mod declarations)
2. Add `SetupGuide` variant to `Commands` enum:
   ```rust
   /// Output LLM-friendly setup instructions for this project
   SetupGuide {
       /// Path to the project root (defaults to current directory)
       #[arg(long, default_value = ".")]
       path: PathBuf,
   },
   ```
3. Add match arm in `main()`:
   ```rust
   Commands::SetupGuide { path } => {
       let path = resolve_path(&path);
       if let Err(e) = setup_guide::run_setup_guide(&path) {
           eprintln!("Error: {}", e);
           std::process::exit(1);
       }
   }
   ```

---

## Files NOT Changed

- `detect.rs` — used as-is via `detect::detect_project()`
- `templates.rs` — `RDSPI_WORKFLOW` and `generate_claude_md()` used as-is
- `config.rs` — `default_config_toml()` used as-is
- `init.rs` — not coupled; setup_guide does its own existence checks
- `loop_cmd.rs` — unrelated
- `status.rs` — unrelated
- `lisa-core` — no changes
- `lisa-plugin` — no changes
- `Cargo.toml` — no new dependencies needed

---

## Module Dependency Graph

```
main.rs
  └── setup_guide.rs
        ├── detect.rs        (detect_project)
        ├── templates.rs     (RDSPI_WORKFLOW, generate_claude_md)
        └── config.rs        (default_config_toml)
```

No circular dependencies. setup_guide imports from sibling modules only.

---

## Content Organization

The guide string is built from 9 `GuideSection` values, each produced by a dedicated
function. Sections 1-3 (directories, config, CLAUDE.md) take `root: &Path` to check
existence. Sections 4-9 are static content.

`render_guide()` joins sections with `## Step N: {title}` headings and a header block.
This keeps numbering automatic — adding or reordering sections doesn't break anything.

Guide header includes: project name, detected type, and a one-line description of what
the guide is for.

---

## Test Architecture

All tests use `build_guide()` (returns String) rather than `run_setup_guide()` (prints).
This avoids capturing stdout in tests.

Tests create `tempfile::tempdir()` with appropriate marker files (Cargo.toml,
package.json, etc.) and optionally pre-create init directories to test skip behavior.

Assertions check for presence of key content strings, not exact output — the guide
text can evolve without breaking tests.
