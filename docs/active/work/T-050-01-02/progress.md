# Progress — T-050-01-02 never-a-dead-end surfaces

## Implement status

- Implementation is complete.
- All ticket-owned source and test changes are committed through Lisa.
- No ordinary `git add` or `git commit` command was used.
- No ticket-owned source or test path remains modified, staged, or untracked.
- Lisa-managed ticket and admitted-work paths remain outside this ticket's commits.
- Full CLI and workspace verification pass.

## Completed unit: pre-init command lead

Modified `crates/lisa-cli/src/main.rs`.

- Added the exact setup lead:

```text
This folder isn't set up yet. Run: lisa init
```

- Added a private `require_lisa_project` dispatch preflight.
- A root is recognized when `.lisa.toml` exists.
- A root is also recognized when `docs/active/tickets` is a directory.
- A generic repository carrying only source or `CLAUDE.md` remains pre-init.
- The setup sentence is written before the technical marker/root detail.
- Pre-init commands exit 1 with empty stdout.
- `doctor` is guarded before dependency checks, cache cleanup, or trust work.
- `validate` is guarded before optional tool or structure diagnostics.
- Ordinary board `status` is guarded before ticket-directory diagnostics.
- `loop` is guarded before client/config/runtime resolution.
- `init`, notes, guides, and unrelated plumbing remain unguarded.

## Completed unit: empty notes

Modified `crates/lisa-cli/src/notes.rs`.

- `run_list` now prints exactly `Nothing to read.` for an empty queue.
- Empty notes remains a successful command.
- Empty notes writes nothing to stderr.
- The low-level `note_lines` formatter remains unchanged.
- The populated `print_notes` output remains byte-identical.
- Acknowledgement behavior and provenance writes remain unchanged.

## Completed unit: empty status sections

Modified `crates/lisa-cli/src/status.rs`.

- Empty parked remedies now render:

```text
Waiting on you
Nothing waiting.
```

- Empty deferred notes now render:

```text
Notes for you
Nothing to read.
```

- Both sections retain the established Waiting, Notes, then board-summary order.
- An initialized zero-ticket board renders both sections before `No tickets found`.
- A non-empty board renders both sections before `DAG:`.
- Populated parked remedy lines remain byte-identical.
- Populated note summary, criterion, and evidence lines remain byte-identical.
- DAG construction, waves, scheduling summary, and run summary remain unchanged.

## Completed unit: empty validation guidance

Modified `crates/lisa-cli/src/init.rs`.

- A clean ticket scan with zero tickets is no longer a readiness Error.
- The validator still returns accumulated setup, tool, config, hook, and parse errors.
- A malformed board with zero successfully parsed tickets remains nonzero.
- A missing or incomplete project remains nonzero.
- A fully initialized empty board now exits 0.
- The empty output is exactly one paragraph and a newline:

```text
No tickets yet. A ticket is a Markdown file that tells Lisa what work to schedule; put one in docs/active/tickets/, then run `lisa validate` again.
```

- The implementation interpolates the resolved configured ticket directory.
- A trailing slash in configuration is trimmed before adding the displayed slash.
- The empty branch does not print a misleading all-checks-passed line.
- The empty branch does not suggest starting an empty loop.
- The empty branch does not print the ordinary config summary.
- Valid non-empty validation retains its exact prior success output.

## Completed unit: regression suite

Added `crates/lisa-cli/tests/never_dead_end.rs`.

- `pre_init_project_commands_lead_with_the_setup_sentence` table-tests:
  - `lisa loop --dry-run`;
  - `lisa status`;
  - `lisa validate`;
  - `lisa doctor`.
- The test pins status 1, empty stdout, and complete stderr for each command.
- The complete stderr assertion guarantees the setup sentence is first.
- The fixture path contains spaces to cover argument/path handling.
- `empty_notes_prints_nothing_to_read` pins exact stdout and success.
- `validate_empty_board_exits_zero_with_ticket_guidance` names and pins the chosen contract.
- That test uses a real `lisa init --no-history` scaffold, not a partial mock.
- `status_empty_board_names_absent_operator_sections` covers the direct zero-ticket board.
- `initialized_nonempty_validate_snapshot_is_unchanged` pins the old valid output.

## Updated existing snapshots

Modified `crates/lisa-cli/tests/notes_ux.rs`.

- Renamed the empty-queue regression for the new sentence.
- Updated post-ack empty output to exactly `Nothing to read.`.
- Retained durable restart, ordering, acknowledgement, and scheduling assertions.

Modified `crates/lisa-cli/tests/parked_ux.rs`.

- Kept exact populated Waiting lines unchanged.
- Added the explicit empty Notes section between populated Waiting and `DAG:`.
- Retained plain-ask-first, reviewer-reason, legacy, and world-owned snapshots.

## Commit record

### Runtime surfaces

```text
a3c394cfdd7c9fedc3656093cfaafd98eb713864
Lead empty CLI surfaces with guidance
```

Exact include paths:

- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/notes.rs`
- `crates/lisa-cli/src/status.rs`

### Existing output snapshots

```text
288fb78bcaf6622fbe6cb0e6d29aa868dde31cff
Pin explicit status and notes emptiness
```

Exact include paths:

- `crates/lisa-cli/tests/notes_ux.rs`
- `crates/lisa-cli/tests/parked_ux.rs`

### Empty validation and cross-command regression

```text
1bf178c957040b96bfea3c22a0726a7f6e1d6394
Guide validation on an empty board
```

Exact include paths:

- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/tests/never_dead_end.rs`

### Ledger-status compatibility correction

```text
3c858bf81a57556218d82f10ba0760b25066e369
Keep ledger status project-independent
```

Exact include path:

- `crates/lisa-cli/src/main.rs`

## Shared-file concurrency event

- Sibling ticket `T-050-01-01` began editing `init.rs` during this implementation.
- Its changes concerned init history defaults and were unrelated to validation.
- Both attempts detected that exact-path commits could absorb the other ticket's hunks.
- This ticket first committed its non-overlapping main/notes/status unit.
- It then temporarily removed only its own validation hunks from `init.rs`.
- The sibling verified the remaining `init.rs` diff was exclusively its own.
- The sibling committed `init.rs` and `tests/init_history.rs` as `0dd3b68`.
- This ticket then reapplied its validation hunk onto that clean committed base.
- No sibling-owned hunk was staged, reverted, or included by this ticket.
- Focused init and history tests passed after the serialized handoff.

## Plan deviations

- Runtime work was split into four commits rather than the planned two.
- The split was required to preserve exact path ownership during the `init.rs` collision.
- Status rendering was expanded to cover a zero-ticket board directly.
- Structure had initially preserved the early zero-ticket return before sections.
- The final ordering better matches the ticket's “nothing parked” first-run wording.
- Full CLI verification exposed that `status --ticket --ledger` is project-independent plumbing.
- The first preflight placement guarded that explicit-ledger path too broadly.
- The final compatibility commit moved the guard into the ordinary status branch only.
- The required public `lisa status` pre-init contract remains fully pinned.

## Focused verification

Passed:

```text
cargo test -p lisa-cli --test never_dead_end
```

- 5 passed; 0 failed.

Passed:

```text
cargo test -p lisa-cli init::tests::test_validate
```

- 24 selected binary validation tests passed.

Passed:

```text
cargo test -p lisa-cli --test init_history
```

- 7 passed; 0 failed.

Passed:

```text
cargo test -p lisa-cli --test notes_ux
```

- 3 passed; 0 failed.

Passed:

```text
cargo test -p lisa-cli --test parked_ux
```

- 13 passed; 0 failed.

Passed:

```text
cargo test -p lisa-cli --test seal_visibility
cargo test -p lisa-cli --test preownership_status
```

- Doctor/status initialized fixtures passed.
- Explicit-ledger status compatibility passed.

## Full verification

Passed:

```text
cargo test -p lisa-cli
```

- All CLI unit and integration targets passed.
- The real-Zellij integration remains intentionally ignored by its existing environment gate.

Passed:

```text
cargo test --workspace
```

- `lisa-core`, `lisa-cli`, and `lisa-plugin` tests passed.
- All doc tests passed.

Passed:

```text
cargo fmt --check
git diff --check
```

- Formatting is clean.
- No whitespace error exists.

## Remaining work

- Review the four ticket commits against both acceptance criteria.
- Confirm ticket-owned source/test cleanliness from final status.
- Write `review.md`.
- Write and validate `review-disposition.json`.
- Remain on this ticket and let Lisa perform publication/completion.
