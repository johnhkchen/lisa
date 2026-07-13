# Progress: orient and separate help

## Status

Implementation is complete and verified. Review is next.

## Completed

- Read `CLAUDE.md`, the ticket, and the complete RDSPI workflow.
- Read the parent story and epic to confirm the ticket boundary.
- Mapped the Clap parser, command variants, dispatch, and existing help tests.
- Wrote `research.md`.
- Wrote `design.md`.
- Wrote `structure.md`.
- Wrote `plan.md`.
- Ran the baseline focused test:
  `cargo test -p lisa-cli --test help_surface`.
- Baseline result: 3 passed, 0 failed.
- Captured and inspected the baseline top-level help.
- Confirmed the baseline has no everyday-path line.
- Confirmed the baseline renders operator and plumbing commands under one
  undifferentiated `Commands:` heading.
- Added `before_help` with
  `Everyday path: init → validate → status → loop`.
- Added a labeled `after_help` plumbing block.
- Hid the four plumbing variants from Clap's generated command list.
- Ran `cargo fmt --all`.
- Inspected the new `lisa --help` output.
- Confirmed the generated list contains the five operator commands and built-in
  help, while the footer contains all four plumbing commands.
- Confirmed `agent-exec --help`, `capture-usage --help`,
  `commit-ticket --help`, and `complete-ticket --help` all still succeed.
- Ran the old focused tests against the new production output.
- Expected red result: 2 passed and 1 failed because the old about test treats
  the new orientation as the about line.
- Confirmed the old relative-order grouping test still passes, demonstrating
  that it does not enforce a real section boundary.
- Committed `crates/lisa-cli/src/main.rs` through the repository-built Lisa
  isolated transaction.
- Production commit: `6a0fff1254e65e7a595027de9a7aae67a1d61db7`.
- Added an inline full-output `lisa --help` snapshot.
- Added `top_level_help_matches_snapshot`.
- Replaced the weak relative-offset grouping check with a structural split at
  the plumbing heading.
- The structural check now requires all five operator commands in the primary
  section, rejects all four plumbing commands from that section, and requires
  all four in the plumbing footer.
- Kept the three internal commands absent from top-level help.
- Updated the about-line lookup to find the `coding agents` masthead after the
  new orientation rather than assuming it is the first line.
- Kept all twelve direct command-resolution checks.
- Ran `cargo fmt --all` after the test edit.
- Focused result: `cargo test -p lisa-cli --test help_surface` passed 4 tests,
  0 failed.
- Committed `crates/lisa-cli/tests/help_surface.rs` through the
  repository-built Lisa isolated transaction.
- Test commit: `6698c12aa0784836a88501013fbaab0419c3f227`.
- Final review of the test source found stale `HOOK_COMMANDS`/lower-band wording
  inherited from S-036-01. Renamed the constant to `PLUMBING_COMMANDS` and
  updated the comment and assertion variable to describe the new curated
  footer accurately.
- Re-ran formatting check and all 4 focused help tests after that cleanup; both
  passed.
- Cleanup commit:
  `7ed3c24609df5a038b86f30fb104ca26e36bb271`.
- Re-ran the full `cargo test -p lisa-cli` acceptance command after the cleanup;
  all executed tests passed again and the single environment-gated real Zellij
  test remained ignored.
- Re-ran `cargo fmt --all -- --check`; passed.
- Ran `git diff --check` across the complete ticket source change; passed.
- Ran the acceptance command `cargo test -p lisa-cli`.
- Acceptance result: all executed tests passed. The crate reported 14 library
  unit tests, 269 binary unit tests, 4 help-surface tests, 5 other executed
  integration tests, and 0 doc tests passing; the environment-gated real
  Zellij integration test remained ignored as designed.
- Ran `cargo fmt --all -- --check`; result passed.
- Inspected both ticket commits and their path lists.
- Commit `6a0fff1` contains only `crates/lisa-cli/src/main.rs`.
- Commit `6698c12` contains only
  `crates/lisa-cli/tests/help_surface.rs`.
- Commit `7ed3c24` contains only
  `crates/lisa-cli/tests/help_surface.rs`.
- Confirmed both ticket-owned source files have no unstaged diff.
- Confirmed both ticket-owned source files have no ordinary-index staged diff.
- Preserved unrelated concurrent `.lisa`, plugin, epic, story, ticket, and work
  entries visible in the shared worktree.

## Current step

- Add the exact snapshot and stronger structural assertions in
  `crates/lisa-cli/tests/help_surface.rs`.

## Remaining

- Complete Review artifacts and disposition.

## Deviations

- `/opt/homebrew/bin/lisa` is version `0.4.0-rc.5` and rejects
  `commit-ticket` as an unrecognized subcommand. The repository-built
  `target/debug/lisa` is version `0.4.0-rc.8` and implements the required
  isolated transaction. The production unit was committed with that current
  Lisa binary and the exact planned include path. No ordinary Git staging or
  commit command was used.
- The plan described the source commit before the first expected-red test run;
  the red run was performed immediately before the source commit instead. This
  did not alter scope or commit contents and provided the planned evidence that
  the existing test assumptions were insufficient.
