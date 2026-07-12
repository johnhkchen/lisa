# Progress — T-036-01-02: plain, verb-forward command help

## Status: implementation complete, committed via `lisa commit-ticket`.

## Steps (from plan.md)

- [x] **Step 1 — Apply twelve `///` rewrites** to `crates/lisa-cli/src/main.rs`.
  All five operator commands, three hidden commands, and four hook commands
  reworded. `git diff` confirmed **comment-only** (only `///` lines changed; no
  attribute, field, or dispatch line in the diff).
- [x] **Step 2 — Build + render.** `cargo build -p lisa-cli --release` clean.
  `lisa --help` command list and each `lisa <cmd> --help` opening line show the
  new plain sentences. AgentExec long help still carries its LISA_PANE_ID /
  `codex exec` body (verified present).
- [x] **Step 3 — Jargon-ban check.** Grep of the operator command list for
  `dag|orchestrat|scheduling|leverage|solutions` → **zero hits**. Loop line now
  reads "Start a run: … in parallel where they don't collide."
- [x] **Step 4 — No-regression.** All 12 subcommands resolve (`<cmd> --help`
  exits 0). `cargo test --workspace` → **286 passed, 0 failed**.
- [x] **Step 5 — Commit** through `lisa commit-ticket` with only
  `crates/lisa-cli/src/main.rs` included.

## Deviations from plan

None. The change matched structure.md exactly. No new files, no deletions, no
attribute or dispatch edits.

## Rendered result (operator command list)

```
init             Set up a project to run with Lisa
validate         Check your tickets and project setup for problems before a run
status           Show which tickets are ready to run and which are waiting, and why
doctor           Check that the tools Lisa needs are installed
loop             Start a run: work through the ready tickets, in parallel where they don't collide
```

## Notes for reviewer / sibling ticket

- No test added — T-036-01-03 owns the help-surface regression lock, and its
  file is disjoint from main.rs. The strings are left jargon-clean so that test
  can pin them.
- Per-flag `///` help (e.g. `--path`, `--dry-run`) was intentionally left
  untouched — out of the subcommand-level AC scope.
