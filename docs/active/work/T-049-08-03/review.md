# Review: T-049-08-03 notes acknowledgment

## Result

The deferred notes queue no longer has a same-ticket multi-generation dead end.

Bare `lisa notes ack <ticket-id>` now always has a natural forward move.

With multiple active notes, it acknowledges the oldest numeric generation.

With one active note, it retains the prior ticket acknowledgment output.

With no active note, it succeeds with a ticket-specific nothing-to-read line.

Operators can target a listed note using `--generation <n>`.

Every successful acknowledgment still appends one exact provenance row.

## Changed files

### `crates/lisa-core/src/notes.rs`

Added `NoteAcknowledgment`, a structured outcome with:

- `NothingToRead` for a successful no-write bare operation;
- exact acknowledged `QueuedNote`;
- same-ticket remaining count;
- an oldest-of-multiple marker for CLI presentation.

Extended `acknowledge_note` with `Option<u64>` generation selection.

Bare selection sorts same-ticket entries by numeric generation and then exact
`NoteKey`, rather than relying on the map's attempt-first ordering.

Explicit selection finds the requested active generation.

Unknown explicit generations return plain text naming sorted listed values.

The existing append schema, exact ticket/attempt/generation identity, timestamp,
and append-error context are unchanged.

Added core tests for oldest selection, reverse attempt-name ordering, two-step
draining, empty repetition, exact selection, unknown selection, and provenance.

### `crates/lisa-cli/src/notes.rs`

The formatter counts active notes per ticket.

Only tickets holding more than one active note show `Generation N` labels.

This preserves the existing list/status line for every single-note ticket even
when the global queue includes other tickets.

`run_ack` now maps structured outcomes to operator text.

The required multi-note sentence is exact:

`Marked the oldest note read — 1 more remains.`

Additional remaining counts use the plural form.

Bare empty output is exact:

`Nothing to read for T-046-06-03.` in the built-binary fixture.

Explicit success names the generation that was acknowledged.

Added formatter unit coverage for per-ticket conditional labels.

### `crates/lisa-cli/src/main.rs`

Added optional `--generation <GENERATION>` to `lisa notes ack`.

Changed the ack summary to cover oldest and selected behavior.

Added a generation-targeting example to notes help.

Dispatch forwards the optional value without changing common error handling.

### `crates/lisa-cli/tests/notes_ux.rs`

Generalized journal fixtures to emit multiple attempts and generations.

The tests continue to execute the built `lisa` binary in separate processes.

The original single-note lifecycle still pins list order, acknowledgment text,
provenance identity, ticket bytes, DAG readiness, and global empty output.

Its repeated bare acknowledgment now pins successful ticket-specific emptiness
and proves that no duplicate provenance row is appended.

The two-note lifecycle pins both generation labels.

It drains generations 1 then 2 through two fresh acknowledgment processes.

It pins the oldest/remaining sentence after the first command.

It proves the remaining single note loses its unnecessary generation label.

It deserializes both provenance rows and verifies generation and attempt order.

The explicit lifecycle rejects generation 9 with listed generations 1 and 2.

It proves the rejection writes no provenance file.

It then acknowledges generation 2 exactly and verifies generation 1 remains
visible after reconstruction in a fresh process.

### `crates/lisa-cli/tests/help_surface.rs`

Updated the string-pinned notes help snapshot for the revised summary and new
generation example.

### `crates/lisa-plugin/src/lib.rs`

Updated one dashboard projection test caller to pass `None` for bare selection.

The production plugin does not perform note acknowledgments.

The focused test continues to prove acknowledgment has no scheduler, ticket,
thread, slot, or completion-aggregate effect.

### `docs/knowledge/flag-audit.md`

Added the CI-required inventory row for `--generation`.

The row classifies omission as a working default because bare acknowledgment
selects oldest and drains the queue.

It cites the built-binary two-active-note fixture.

## Acceptance evidence

### Two active notes and bare draining

`two_active_notes_are_labeled_and_bare_ack_drains_oldest_first` creates two
confirmed note-bearing generations for one ticket.

It runs list, first ack, list, second ack, and final list as fresh processes.

It asserts the exact one-remaining line and final global empty line.

It deserializes two appended provenance rows in generation order `[1, 2]`.

The core reverse-attempt-name fixture independently proves oldest means minimum
generation rather than incidental `NoteKey` iteration order.

### Exact and unknown generation

`generation_flag_targets_exactly_and_unknown_names_listed_generations` invokes
the built binary with `--generation 9` and pins the error naming `1, 2`.

It then invokes `--generation 2`, verifies the exact provenance row, and confirms
generation 1 survives durable reconstruction.

### Single and empty behavior

`list_ack_and_restart_processes_follow_durable_lifecycle` preserves the prior
single-note list and bare acknowledgment output.

The same test pins successful `Nothing to read for <ticket>.` repetition and an
unchanged one-row ledger.

`empty_queue_renders_nothing_to_read` preserves global empty list behavior.

### Generation labels and obsolete error

The built-binary two-note fixture string-pins both `Generation 1` and
`Generation 2` lines.

The formatter unit test proves labels are per-ticket, not global-count based.

A final search found no `multiple active notes` or
`acknowledgment requires an exact generation` occurrence under `crates/`.

## Verification

- Formatting check passed.
- Core notes suite passed: 6 tests.
- Built-binary notes UX suite passed: 5 tests.
- Help surface suite passed: 6 tests.
- Plugin scheduler-noninterference fixture passed: 1 test.
- Executable flag audit passed: 1 test.
- Final `cargo test --workspace` passed.
- One existing real-Zellij delivery test was ignored by its environment guard.
- No ordinary-index entries remain.
- No ticket-owned source or documentation path remains modified or untracked.

## Commits

- `5dae335be378c5026fefd8d40d0a73d697f2ba9e` — core selection and tests.
- `2b4f6a0e794bb10e176e26ac83496d1f29b32602` — CLI, UX fixtures, help.
- `9b0bcc8d2209dc850c01f5ca77446cd3b89241c7` — plugin test caller.
- `f6daabf6d98c0d8bd30ca1ae540b051307ed8354` — executable flag audit row.

Each source unit was committed with `lisa commit-ticket` and exact includes.

## Open concerns

No blocking concern remains.

Generation is the operator-visible selector and is expected to be unique among
active notes for one ticket, matching completion-generation allocation.

The core uses exact key ordering only as a deterministic tie-break if legacy or
malformed history ever violates that invariant.

The real-Zellij boundary was not required for this CLI queue behavior and remains
covered by its existing opt-in environment fixture.

Lisa-managed journal, provenance, ticket phase, and admitted work paths remain in
the worktree for Lisa's completion transaction; they are not source residue.
