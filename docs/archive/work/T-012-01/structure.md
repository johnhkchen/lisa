# T-012-01 Structure: Fix broken symlink and Ralph naming remnants

## Files deleted

| File | Action |
|------|--------|
| `docs/rdspi-workflow.md` | Delete symlink |

## Files modified

### 1. `CLAUDE.md` (line 58)
- Change: `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`

### 2. `crates/lisa-cli/src/templates.rs`
- Line 279: `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md` (in generated CLAUDE.md template)
- Line 322: Test assertion `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`

### 3. `crates/lisa-cli/src/init.rs`
- Lines 71-72: Change `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md` in `plan_init()`
- Line 375: Change `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md` in `validate()`
- Line 377: Change `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md` in diagnostic message
- Ensure `docs/knowledge/` directory is created by init (check if `plan_init` handles parent dirs)

### 4. `crates/lisa-plugin/src/lib.rs`
- Line 1: `Lisa/Ralph` → `Lisa`
- Line 34: `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`
- Line 1822: `/host/.ralph-commit.lock` → `/host/.lisa-commit.lock`
- Line 1921: `Lisa/Ralph initializing...` → `Lisa initializing...`
- Line 2376: Test assertion `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`
- Line 2463: Test assertion `docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`

### 5. `crates/lisa-plugin/src/ui.rs`
- Line 1: `Lisa/Ralph` → `Lisa`
- Line 10: `Replaces \`just dag-status\`, \`just ralph-status\`, and \`just ralph-logs\`` → `Replaces manual status checking`
- Line 988: `LISA/RALPH Dashboard` → `LISA Dashboard`

### 6. `crates/lisa-core/src/types.rs`
- Line 1: `Lisa/Ralph` → `Lisa`
- Line 311: `Ralph` → `Lisa`
- Line 434: `Lisa/Ralph` → `Lisa`

### 7. `crates/lisa-core/src/dag.rs`
- Line 1: `Lisa/Ralph` → `Lisa`
- Line 396: `Ralph` → `Lisa`

### 8. `crates/lisa-core/src/ticket.rs`
- Line 1: `Lisa/Ralph` → `Lisa`

### 9. `crates/lisa-core/src/diagnostics.rs`
- Line 138: `/host/.ralph-commit.lock` → `/host/.lisa-commit.lock`

### 10. `.gitignore`
- Line 5: `.ralph-commit.lock` → `.lisa-commit.lock`

### 11. `README.md`
- Line 3: `An homage to the ralph loop, but smarter.` → `A Zellij plugin for DAG-driven concurrent task scheduling.`

## Files NOT modified

- `docs/archive/**` — excluded from scope
- `docs/active/work/**` — historical research/design artifacts
- `docs/active/stories/**` — story descriptions
- `docs/active/tickets/**` — ticket descriptions (except frontmatter phase field)
- `crates/lisa-cli/data/rdspi-workflow.md` — embedded data file, unchanged

## Ordering

No ordering constraints. All changes are independent string replacements. Can be done in any order.

## Test expectations

After all changes, these commands must pass:
- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- `grep -ri ralph crates/ CLAUDE.md .gitignore README.md` returns nothing
