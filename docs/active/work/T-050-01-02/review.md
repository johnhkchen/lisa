# Review — T-050-01-02 never-a-dead-end surfaces

## Disposition

- Ready to complete.
- Both acceptance criteria are satisfied.
- No blocking issue remains.
- All ticket-owned source and test changes are committed.
- CLI-wide and workspace-wide tests pass.

## Change summary

The CLI now distinguishes an untouched folder from a partially configured Lisa project before entering the four named project-aware commands.

The first stderr line for ordinary `loop`, `status`, `validate`, and `doctor` in an untouched folder is exactly:

```text
This folder isn't set up yet. Run: lisa init
```

Technical marker and root detail follows in a separate paragraph.

The setup lead is emitted before doctor dependency checks, validate diagnostics, status scanning, or loop configuration/runtime work.

Standalone empty notes now says:

```text
Nothing to read.
```

Status now keeps the operator-facing sections visible even when both are empty:

```text
Waiting on you
Nothing waiting.

Notes for you
Nothing to read.
```

Those sections also appear on a fully initialized zero-ticket board before the existing no-ticket sentence.

Clean empty validation now exits 0 with one guidance paragraph that defines a ticket and names the configured tickets directory.

Malformed tickets, missing setup, invalid configuration, missing hooks, and tool failures remain nonzero.

## Files modified

### `crates/lisa-cli/src/main.rs`

- Added the exact setup-line constant.
- Added the private project-marker preflight.
- Guarded `doctor`, `validate`, ordinary board `status`, and `loop`.
- Recognizes `.lisa.toml` or the default ticket directory as an existing project marker.
- Preserved the project-independent `status --ticket --ledger` plumbing path.

### `crates/lisa-cli/src/notes.rs`

- Added a caller-level empty branch to `run_list`.
- Preserved the populated note formatter byte for byte.

### `crates/lisa-cli/src/status.rs`

- Added explicit empty Waiting rendering.
- Added a status-specific empty Notes renderer.
- Moved optional section rendering before the zero-ticket early return.
- Preserved populated section and DAG rendering.

### `crates/lisa-cli/src/init.rs`

- Removed clean zero-ticket state from validation errors.
- Suppressed generic valid-board success text for the clean zero-count result.
- Added the configured-path guidance paragraph and early success return.
- Preserved every accumulated non-empty diagnostic.

### `crates/lisa-cli/tests/never_dead_end.rs`

- Added five compiled-binary regression snapshots.
- Covers all four pre-init command failures in one table.
- Covers empty notes.
- Covers zero-ticket status sections.
- Covers zero-ticket validate success and exact paragraph.
- Covers byte-exact valid non-empty validation output.

### `crates/lisa-cli/tests/notes_ux.rs`

- Updated the empty queue and post-ack snapshots.
- Retained restart durability and scheduling neutrality checks.

### `crates/lisa-cli/tests/parked_ux.rs`

- Updated three populated Waiting snapshots to include the explicit empty Notes companion section.
- Retained exact asks, reviewer reasons, ordering, and legacy behavior.

## Files not changed

- No Clap command, option, flag, or help text changed.
- No `lisa-core` type or ticket/DAG behavior changed.
- No plugin/dashboard code changed.
- No config schema or default changed.
- No Chromebook grader code changed.
- No shared ticket frontmatter was manually edited.

## Acceptance criterion 1

Criterion:

> String-pinned tests: each pre-init command failure leads with the setup line; empty notes prints "Nothing to read."; empty validate output includes the tickets path and a one-paragraph explanation with a schedulable exit code contract documented (0 with guidance vs current nonzero — pick and pin one, recording the choice in the test name).

Evidence:

- `pre_init_project_commands_lead_with_the_setup_sentence` runs:
  - `loop --dry-run`;
  - `status`;
  - `validate`;
  - `doctor`.
- It asserts exact complete stderr, exit 1, and empty stdout for each.
- Exact equality proves the required setup sentence is the leading line.
- `empty_notes_prints_nothing_to_read` pins exactly `Nothing to read.\n` and exit 0.
- `empty_queue_renders_nothing_to_read` independently pins the durable notes fixture.
- `validate_empty_board_exits_zero_with_ticket_guidance` records the chosen contract in its name.
- That test asserts exit 0 and empty stderr.
- It pins the entire one-line paragraph plus newline.
- The paragraph explains that a ticket is a Markdown work description.
- It includes `docs/active/tickets/`.
- It tells the operator to rerun validation after adding one.
- `status_empty_board_names_absent_operator_sections` pins both empty status sections.

Assessment: satisfied.

## Acceptance criterion 2

Criterion:

> Existing initialized-project behavior byte-unchanged for all touched commands (snapshot tests); no new flags introduced.

Evidence:

- `initialized_nonempty_validate_snapshot_is_unchanged` pins the complete valid stdout and empty stderr.
- Existing `notes_ux` populated lifecycle output remains unchanged and passes.
- Existing `parked_ux` exact populated asks and reasons remain unchanged and pass.
- Existing `notes_ux` confirms populated Notes ordering before DAG output.
- Existing `seal_visibility` fixtures confirm initialized doctor and status completion reporting.
- Existing loop module tests cover initialized dry-run and structure behavior.
- `help_surface` passes all command and help snapshots.
- No Clap declaration changed in the committed diff.
- The only intentionally changed initialized outputs are the ticket-required empty states and explicit empty companion sections.
- Project-independent explicit-ledger status was caught and restored before Review.

Assessment: satisfied.

## Test coverage

### Focused tests

Passed:

```text
cargo test -p lisa-cli --test never_dead_end
```

- 5 passed.

Passed:

```text
cargo test -p lisa-cli init::tests::test_validate
```

- 24 selected binary validation tests passed.

Passed:

```text
cargo test -p lisa-cli --test init_history
cargo test -p lisa-cli --test notes_ux
cargo test -p lisa-cli --test parked_ux
cargo test -p lisa-cli --test seal_visibility
cargo test -p lisa-cli --test preownership_status
```

- 7 history tests passed.
- 3 notes tests passed.
- 13 parked tests passed.
- 5 seal visibility tests passed.
- 1 explicit-ledger status test passed.

### Full tests

Passed:

```text
cargo test -p lisa-cli
```

- All CLI unit and integration targets passed.
- The preexisting real-Zellij fixture remained intentionally ignored by its environment gate.

Passed:

```text
cargo test --workspace
```

- Core, CLI, plugin, and documentation tests passed.

Passed:

```text
cargo fmt --check
git diff --check
```

## Commit audit

Ticket commits are:

- `a3c394c` — runtime setup, notes, and status surfaces;
- `288fb78` — notes and parked snapshot updates;
- `1bf178c` — empty validation plus cross-command regressions;
- `3c858bf` — explicit-ledger status compatibility correction.

Each was made through `lisa commit-ticket` with exact repository-relative includes.

The concurrent sibling `T-050-01-01` commit `0dd3b68` is not owned by this ticket.

The shared `init.rs` collision was serialized by removing only this ticket's validation hunks, allowing the sibling to commit its history hunks, then reapplying validation on the clean base.

No cross-ticket hunk was absorbed by either ticket's isolated commit.

## Working tree audit

- Ticket-owned source and test paths are clean.
- The ordinary Git index has no staged paths.
- Remaining modified ticket files are Lisa-managed metadata.
- Remaining untracked shared work directories are Lisa-admitted artifacts.
- Those paths must remain for Lisa's completion transaction.

## Open concerns and limitations

- The pre-init predicate deliberately recognizes modern config or the default ticket directory.
- A severely damaged custom-layout project that loses `.lisa.toml` and has no default ticket directory will receive first-time setup guidance instead of a custom-path repair diagnostic.
- That condition has lost the only source of its custom path, so the fallback is actionable and non-destructive.
- Custom ticket-path interpolation is implemented from resolved config; the acceptance snapshot pins the default release path.
- The Chromebook grader seed removal remains assigned to the next runbook amendment, as the ticket requires.
- No concern blocks completion.

## Final assessment

The change closes every named blank or bare first-contact surface, pins the operator-facing strings and process statuses, preserves populated behavior, and keeps partial-project diagnostics available.

The ticket is ready for Lisa's completion transaction.
