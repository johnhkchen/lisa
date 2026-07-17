# Progress — T-049-06-02

## Completed phases

Research, Design, Structure, and Plan artifacts are complete in the private attempt directory.

## Implementation completed so far

- Added the new shared core queue implementation in `crates/lisa-core/src/notes.rs`.
- Added exact generation identity, confirmed-journal reduction, provenance acknowledgment facts, deterministic sorting, and lifecycle unit tests.
- Added the CLI Notes module with summary-first rendering, list, acknowledgment, and formatter tests.
- Added real-binary Notes lifecycle fixtures covering durable clearing, duplicate rejection, and zero ticket/DAG mutation.
- Prepared the dashboard NoteItem/rendering change, then removed it from the shared UI
  path after confirming T-049-07-01 also owns a proposal-aware UI change there.

## Concurrent-work boundary

T-049-07-01 is concurrently modifying shared files including core exports/provenance,
CLI main/status/help, and plugin State projection. Those changes predated this ticket's
edits to the overlapping paths and are not owned here. This ticket has not edited or
committed those overlapping files. Integration work will resume after that ticket's
isolated commits land, so exact `lisa commit-ticket` includes cannot absorb its work.

## Remaining

- Export the core notes module and register the acknowledgment row in the mixed provenance schema.
- Wire CLI main, status, and help after the overlapping ticket commits.
- Apply the dashboard renderer and plugin State projection on top of the committed
  proposal-aware UI, then add restart/zero-effect lifecycle assertions.
- Run targeted and full checks.
- Commit each source unit through Lisa with exact includes.
- Complete Review artifacts.

## Integration completed

- Exported `lisa_core::notes` and registered note acknowledgments as a fourth
  provenance transition family; provenance schema advanced from 7 to 8.
- Wired `lisa notes` and `lisa notes ack <ticket-id>` into the operator CLI.
- Added Notes for you to status after urgent Waiting on you and before the DAG.
- Updated the full operator help contract from 15 to 16 commands.
- Applied the dashboard renderer on top of the committed first-responder proposal UI.
- Added durable plugin projection plus a kill/reload fixture that proves the note
  survives restart, clears after ack, and leaves DAG readiness, ticket bytes,
  completion aggregate, threads, seats, and parking unchanged.

## Deviations from plan

The source split stayed as planned, but overlapping integration waited for
T-049-07-01's isolated commit because it owned the same provenance, CLI status/main,
plugin State, and UI paths. The draft CLI integration test was temporarily kept in
the private attempt directory so Cargo auto-discovery could not block that ticket's
workspace suite, then restored after its commit.

The first plugin compile selected a stale pre-export lisa-core artifact after the
concurrent commit sequence. `cargo clean -p lisa-core` removed only build cache and
the clean rebuild exposed one real exhaustive-match addition for the new provenance
variant. That match now explicitly ignores acknowledgment rows when summing execution
captures.

## Ticket commits

- `7bb43ee50ab691da9a38557543064f035df7167e` — durable queue reducer and provenance acknowledgment.
- `479a2f8b6f2a45e1aaffa1f2e64bcdb6842fa48d` — Notes CLI, status, help, and lifecycle fixtures.
- `406fd9407eb237c085619212ab0ed3d68219cfa6` — dashboard projection, renderer, restart, and zero-effect fixture.

All three commits used `lisa commit-ticket` with exact repository-relative includes.

## Focused verification completed

- `cargo test -p lisa-core` — passed, 242 unit tests plus integration regressions.
- `cargo test -p lisa-cli --test notes_ux` — passed, 3 black-box lifecycle tests.
- `cargo test -p lisa-cli --test help_surface` — passed, 6 help-contract tests.
- `cargo test -p lisa-cli status::` — passed, 15 focused status/preownership tests.
- `cargo test -p lisa-plugin --lib` — passed, 437 tests.

## Final verification

- `cargo test --workspace --no-fail-fast` — passed across core, CLI, plugin,
  integration, and documentation targets; the environment-gated real-Zellij test
  remained ignored as designed.
- `cargo fmt --all -- --check` — passed.
- `just check` — passed, including `wasm32-wasip1` checking and the complete test
  suite.
- `git diff --check` — passed.
- The three ticket commits contain only their declared exact include paths.
- No ticket-owned source file is staged, modified, or untracked after the commits.
  The remaining worktree entries are Lisa-managed journal, ticket, and published-work
  state (including concurrent T-049-07-02 state) and were not edited or committed by
  this implementation transaction.

## Review

Review found no blocking correctness, durability, presentation, or ownership issue.
The required `review.md` and passing disposition are present in this private attempt.
