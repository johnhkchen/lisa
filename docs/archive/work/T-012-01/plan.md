# T-012-01 Plan: Fix broken symlink and Ralph naming remnants

## Step 1: Delete the broken symlink

- `rm docs/rdspi-workflow.md`
- Verify: `ls -la docs/rdspi-workflow.md` should fail

## Step 2: Update all `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md` references

Files and lines to edit:
1. `CLAUDE.md:58` — path in prose
2. `crates/lisa-cli/src/templates.rs:279` — generated CLAUDE.md template string
3. `crates/lisa-cli/src/templates.rs:322` — test assertion
4. `crates/lisa-plugin/src/lib.rs:34` — `ticket_prompt()` function
5. `crates/lisa-plugin/src/lib.rs:2376` — test assertion
6. `crates/lisa-plugin/src/lib.rs:2463` — test assertion
7. `crates/lisa-cli/src/init.rs:71-72` — `plan_init()` workflow path
8. `crates/lisa-cli/src/init.rs:374-377` — `validate()` check

Verify: `grep -r "docs/rdspi-workflow.md" crates/ CLAUDE.md` returns no hits (ROADMAP.md and archive are okay)

## Step 3: Rename `.ralph-commit.lock` → `.lisa-commit.lock`

1. `crates/lisa-plugin/src/lib.rs:1822` — change string literal
2. `crates/lisa-core/src/diagnostics.rs:138` — change string literal in test helper
3. `.gitignore:5` — change entry

Verify: `grep -r "ralph-commit" crates/ .gitignore` returns no hits

## Step 4: Fix dashboard header and initializing message

1. `crates/lisa-plugin/src/ui.rs:988` — `LISA/RALPH Dashboard` → `LISA Dashboard`
2. `crates/lisa-plugin/src/lib.rs:1921` — `Lisa/Ralph initializing...` → `Lisa initializing...`

## Step 5: Clean up all remaining Ralph references in source code

Doc comments (simple find-replace per file):
1. `crates/lisa-plugin/src/lib.rs:1` — `Lisa/Ralph` → `Lisa`
2. `crates/lisa-plugin/src/ui.rs:1` — `Lisa/Ralph` → `Lisa`
3. `crates/lisa-plugin/src/ui.rs:10` — remove ralph-specific just commands
4. `crates/lisa-core/src/types.rs:1` — `Lisa/Ralph` → `Lisa`
5. `crates/lisa-core/src/types.rs:311` — `Ralph` → `Lisa`
6. `crates/lisa-core/src/types.rs:434` — `Lisa/Ralph` → `Lisa`
7. `crates/lisa-core/src/dag.rs:1` — `Lisa/Ralph` → `Lisa`
8. `crates/lisa-core/src/dag.rs:396` — `Ralph` → `Lisa`
9. `crates/lisa-core/src/ticket.rs:1` — `Lisa/Ralph` → `Lisa`
10. `README.md:3` — remove ralph mention

## Step 6: Verify

1. `grep -ri ralph crates/ CLAUDE.md .gitignore README.md` — no hits
2. `cargo test --workspace` — all tests pass
3. `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM check passes
4. `find . -type l -not -exec test -e {} \; -print` — no broken symlinks

## Testing strategy

No new tests needed. Existing tests cover:
- `test_build_claude_command_includes_rdspi_reference` — updated assertion string
- `test_ticket_prompt_content` — updated assertion string
- `test_generate_claude_md_rust` — updated assertion string
- Diagnostics tests — updated lock path

All changes are string replacements. If tests pass with the new strings, the changes are correct.
