# Progress: External Project Dogfood (T-009-03)

## Completed

### Bug Fix: rdspi-workflow.md path mismatch

Changed `ticket_prompt()` in `crates/lisa-plugin/src/lib.rs` line 30:
- Before: `docs/knowledge/rdspi-workflow.md` (lisa-internal path)
- After: `docs/rdspi-workflow.md` (standard path created by `lisa init`)

Updated 2 test assertions (lines 1805, 1892) to match.

Verification:
- `cargo test --workspace`: 94 tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1`: compiles clean

### Dogfood Dry-Run on Node.js Project

Created `/tmp/test-dogfood-project/` with `package.json` and `src/index.js`.

**`lisa init`**: Generated all 14 items correctly:
- 8 directories (docs/active/*, docs/archive/*, .lisa/hooks, .lisa/signals)
- 6 files (CLAUDE.md, rdspi-workflow.md, .lisa.toml, on-idle.sh, .gitignore, settings.local.json)
- Detected as "test-dogfood-project (Node.js)"
- CLAUDE.md contains correct npm commands and source layout
- Hook script is executable (0755)
- No lisa-internal references in generated files

**`lisa validate`** (no tickets): Correctly reported 1 error:
```
docs/active/tickets/: readiness: no tickets found. Create at least one ticket file.
```
Exit code: 1

**`lisa validate`** (with 2 tickets, 1 dependency edge): Passed:
```
All checks passed. 2 tickets, 1 ready, DAG valid. Run `lisa loop` to start.
```
Exit code: 0

**`lisa loop --dry-run`**: Correct output:
- 2 tickets, 1 ready, 1 blocked
- Execution order: T-001 (ready), T-002 (blocked by T-001)
- Generated KDL layout with correct WASM path and config

**`lisa status`**: Correct output:
- DAG: 2 tickets, 1 edge, no cycles
- Wave 0: T-001 (blocks T-002)
- Wave 1: T-002 (deps: T-001)
- Ready to schedule: T-001

### Generated CLAUDE.md Quality

For the Node.js project:
- Correct project name: "test-dogfood-project"
- Correct type label: "(Node.js)"
- Correct commands: `npm run build`, `npm test`, `npm run lint`
- Source layout detected: `src: index.js`
- Directory conventions section present
- Workflow reference: `docs/rdspi-workflow.md` (correct)
- No lisa-specific references
- TODO placeholder for project description

## Not Tested (Requires Terminal)

The following items from the test plan require a human in a GUI terminal with zellij:

- **`lisa loop`** (live run) — agent sessions start, RDSPI phases fire, dashboard renders
- Phase transitions via artifact detection
- Implement → Review via idle signal
- Session reuse (slot recycling with `/exit` + re-launch)
- Dashboard rendering in real time

## Issues Found

### Fixed
1. **rdspi-workflow.md path mismatch** — Plugin prompt referenced `docs/knowledge/rdspi-workflow.md` but `lisa init` creates `docs/rdspi-workflow.md`. Fixed by changing the prompt to use the standard path.

### No Additional Issues
All CLI commands (`init`, `validate`, `status`, `loop --dry-run`) work correctly on the external project. No crashes, no confusing error messages, no missing files.

## Remaining for Acceptance

The ticket's acceptance criteria require:
> At least one ticket completes Research through Review without manual phase intervention on the external project

This requires running `lisa loop` for real in a terminal with zellij installed.
The bug fix and CLI validation are complete. The actual loop test is a manual step.
