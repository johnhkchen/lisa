# T-012-01 Progress: Fix broken symlink and Ralph naming remnants

## Completed

### Step 1: Delete broken symlink
- Deleted `docs/rdspi-workflow.md` (was absolute-path symlink)
- Verified: file no longer exists, no broken symlinks in repo

### Step 2: Update rdspi-workflow.md path references
- Updated `CLAUDE.md:58` to `docs/knowledge/rdspi-workflow.md`
- Updated `crates/lisa-cli/src/templates.rs` — generated CLAUDE.md template + test assertion
- Updated `crates/lisa-plugin/src/lib.rs` — `ticket_prompt()` + 2 test assertions
- Updated `crates/lisa-cli/src/init.rs` — `plan_init()` path, `validate()` check, diagnostic message
- Updated `crates/lisa-cli/src/setup_guide.rs` — setup guide references
- Updated all 27 test fixtures to create `docs/knowledge/` directory before writing workflow file

### Step 3: Rename `.ralph-commit.lock` → `.lisa-commit.lock`
- Updated `crates/lisa-plugin/src/lib.rs:1822`
- Updated `crates/lisa-core/src/diagnostics.rs:138`
- Updated `.gitignore:5`

### Step 4: Fix dashboard header and initializing message
- Changed `LISA/RALPH Dashboard` → `LISA Dashboard` in `ui.rs:988`
- Changed `Lisa/Ralph initializing...` → `Lisa initializing...` in `lib.rs:1921`

### Step 5: Clean up remaining Ralph references
- Updated doc comments in 6 source files (lib.rs, ui.rs, types.rs, dag.rs, ticket.rs)
- README.md was already updated by another change (no ralph reference present)

### Step 6: Verification
- `grep -ri ralph crates/ CLAUDE.md .gitignore README.md` — 0 hits
- `cargo test --workspace` — 332 tests pass (123 + 78 + 131)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — passes
- `find . -type l -not -exec test -e {} \; -print` — no broken symlinks

## Deviations from plan
- README.md line 3 was already changed by a concurrent/prior change (no action needed)
- Discovered additional references in `setup_guide.rs` not in the original ticket — updated those too
- Tests needed `docs/knowledge/` parent directory created (27 test fixtures updated)
