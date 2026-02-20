# T-011-02 Review: Run lisa loop end-to-end on a real project

## Summary

This spike exercised `lisa init`, `lisa validate`, and `lisa loop --dry-run` against the lisa repo itself. The full interactive `lisa loop` run was not completed (requires taking over the terminal with zellij), but all pre-flight steps passed and a critical bug was discovered.

## Changes Made

### Infrastructure fixes (persistent, beneficial)
- **`.lisa/hooks/on-idle.sh`** — Fixed legacy `LISA_TICKET_ID` env var to `LISA_PANE_ID`; updated signal filename from `$TICKET_ID.idle` to `pane-$PANE_ID.idle`
- **`.lisa/hooks/on-stop.sh`** — Created by `lisa init` (was missing)
- **`.lisa/hooks/on-clear.sh`** — Created by `lisa init` (was missing)
- **`.claude/settings.local.json`** — Updated by `lisa init`: added `Stop` and `SessionStart` hook entries, upgraded bare-path idle command to guarded form, preserved permissions block
- **`.lisa.toml`** — Created by `lisa init` with defaults (max_threads=2)
- **`docs/active/tickets/T-011-02-run-lisa-loop.md`** — Fixed `depends_on` from multi-line YAML to inline syntax

### Test artifacts (to clean up)
- **`docs/active/tickets/T-TEST-{01,02,03}.md`** — Test tickets with dependency chain
- **`docs/active/work/T-011-02/`** — research.md, design.md, structure.md, plan.md, progress.md, this review.md

## Key Findings

### Critical: Multi-line YAML `depends_on` silently ignored

The ticket parser (`lisa-core/src/ticket.rs:parse_string_vec`) only handles inline array syntax. Multi-line YAML lists are silently dropped, causing **7 existing tickets** to have broken dependency edges. This is silent data loss that breaks DAG correctness.

**Affected tickets**: T-011-02, T-011-03, T-013-02, T-014-03, T-016-01, T-016-02, T-016-03

**Recommended fix**: Add multi-line YAML list support to the parser, or switch to serde_yaml. This should be a new high-priority ticket.

### Medium: Old hook scripts not upgraded by `lisa init`

`lisa init` never overwrites existing files. Users upgrading from pre-Sprint-10 versions with old on-idle.sh (using `LISA_TICKET_ID`) must manually update the script. Consider adding a `lisa doctor` or `lisa upgrade` command.

### Verified working
- `lisa init` correctly scaffolds a project, handles existing files, merges hooks
- `lisa validate --check-tools` catches missing infrastructure
- `lisa loop --dry-run` correctly builds DAG, shows execution order, generates layout
- `lisa status` shows clear DAG visualization with waves and dependency info
- Dependency chain (T-TEST-01 → 02 → 03) correctly parsed with inline syntax
- All three binaries (lisa, zellij, claude) available and compatible

## Open Concerns

1. **Live run not completed** — The interactive `lisa loop` step requires zellij to take over the terminal. Pre-flight is green; the user should run `lisa loop --max-threads 1` manually to complete validation.
2. **Test ticket cleanup** — T-TEST-{01,02,03} files and work dirs should be removed after the live run.
3. **Multi-line YAML bug** — Needs a follow-up ticket. High priority since it silently breaks the DAG.

## Acceptance Criteria

- [x] `lisa init` + `lisa validate` succeed on the target project
- [~] `lisa loop` launches and schedules at least one ticket — dry-run passed, live run pending
- [ ] At least one ticket completes a full phase cycle — requires live run
- [~] Hook signals observed — hooks created and verified, signal generation requires live run
- [x] All observations documented in progress.md
