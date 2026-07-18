# Research: T-049-08-03 notes acknowledgment

## Scope

This ticket concerns the deferred completion-note queue exposed by `lisa notes`.
The relevant behavior spans the core projection, CLI presentation and command
parsing, plus black-box tests of the built `lisa` executable.

The ticket starts in Research and requires all remaining RDSPI artifacts in the
attempt-private work directory.

The ticket-owned implementation files named by the context are:

- `crates/lisa-core/src/notes.rs`
- `crates/lisa-cli/src/notes.rs`
- `crates/lisa-cli/src/main.rs`
- the existing CLI fixture coverage in `crates/lisa-cli/tests/notes_ux.rs`
- help snapshots in `crates/lisa-cli/tests/help_surface.rs` when command help changes

## Durable inputs

The notes queue does not have a mutable queue file.

`crates/lisa-core/src/notes.rs` projects queue state from two append-only files:

- `.lisa/completion-journal.jsonl` supplies note-bearing confirmed completions.
- `.lisa/provenance.jsonl` supplies exact note acknowledgment facts.

`read_jsonl` reads either file as newline-terminated JSONL.

A missing file is treated as an empty history.

An empty file is also accepted.

A nonempty file without a trailing newline is rejected as a torn final record.

Blank interior rows, invalid UTF-8, and invalid JSON are rejected with the file
label and line number.

These validation rules apply before any selection behavior is reached.

## Note identity

`NoteKey` is the exact identity of a note-bearing completion generation.

It contains:

- `ticket_id: String`
- `attempt_id: String`
- `generation: u64`

It derives equality, hashing, and total ordering.

The derived ordering compares ticket ID, then attempt ID, then generation.

`QueuedNote` combines that key with a validated `DispositionNote`.

The provenance record repeats the same three identity fields and adds schema,
record type, and acknowledgment timestamp.

Acknowledgments therefore clear one completion generation, not a whole ticket.

## Completion projection

`confirmed_notes` deserializes a narrow projection of every completion row.

Only rows whose `state` is `confirmed` participate.

Confirmed rows without a note do not create queue entries.

Confirmed note rows are inserted into a `BTreeMap<NoteKey, DispositionNote>`.

Repeated rows for an equal exact key collapse to one entry.

Distinct generations remain distinct even when their ticket IDs match.

Because `NoteKey` sorts by attempt before generation, map iteration is stable but
does not by itself define oldest generation within a ticket when attempt IDs sort
in a different order from generation numbers.

## Acknowledgment projection

`acknowledged_keys` reads the general provenance JSONL as `serde_json::Value`.

Rows whose `record_type` is not `note-acknowledged` are ignored.

Matching rows are deserialized as `NoteAcknowledgmentRecord` and converted back
to exact `NoteKey` values.

The result is a `HashSet`, so duplicate acknowledgment facts are harmless to the
queue projection.

`collect_notes` subtracts the acknowledgment set from confirmed notes.

Every fresh CLI process reconstructs the same active queue from durable facts.

No ticket phase, ticket status, DAG edge, or scheduler state is mutated by notes.

## Current core acknowledgment behavior

`acknowledge_note` currently accepts journal path, provenance path, and ticket ID.

It calls `collect_notes`, filters to the requested ticket, and collects matches.

Zero matches return `Err("no active note for {ticket_id}")`.

One match appends an exact acknowledgment row and returns that note.

More than one match returns the ticket's obsolete jargon error saying that an
exact generation is required.

There is no core argument with which a caller can provide that generation.

The provenance append is delegated to `append_note_acknowledgment_record`.

The appended row uses the current schema constant and current epoch timestamp.

Append failures retain the provenance path in their error text.

The function returns only after the durable row is appended successfully.

## Existing core tests

Core tests create temporary journal and ledger files.

They verify that only confirmed note-bearing rows are queued.

They verify that one acknowledgment clears one exact generation durably.

They deserialize the emitted row both as the narrow record and the general
`ProvenanceLedgerRecord` enum.

They verify that an acknowledgment for an earlier attempt does not hide a later
note-bearing generation.

They verify torn and malformed journal rejection.

There is no core fixture for two simultaneously active generations.

There is no core fixture for explicit generation selection.

## CLI presentation

`crates/lisa-cli/src/notes.rs` owns both list and acknowledge output.

`durable_paths` joins the project root to the two fixed `.lisa` paths.

`note_lines` returns no lines for an empty slice.

For a nonempty slice it prints a count heading followed by three lines per note:

- ticket ID and plain summary
- the quoted criterion
- the evidence citation

The current ticket line does not show attempt or generation identity.

If two notes belong to one ticket, their visible ticket labels are identical.

`print_notes` prints the lines and one trailing blank line.

`run_list` prints `Nothing to read.` for an empty global queue.

`run_ack` calls the core function and prints `{ticket_id} acknowledged.`.

Errors bubble to `main`, which prefixes them with `Error:` and exits with code 1.

## CLI parsing

`crates/lisa-cli/src/main.rs` defines `NotesCommands::Ack` with one positional
`ticket_id` and no flags.

The top-level notes path is a global option so it is accepted before or after the
subcommand.

Dispatch calls `notes::run_ack(&path, &ticket_id)`.

The notes help currently describes ack as marking a ticket's current note read.

The help snapshot pins the full `lisa notes --help` output.

Adding a generation flag changes both the parser shape and pinned help surface.

## Built-binary fixtures

`crates/lisa-cli/tests/notes_ux.rs` launches `CARGO_BIN_EXE_lisa` in fresh
processes rather than calling Rust functions directly.

Its project fixture includes a path containing spaces.

The fixture writes a done ticket and a confirmed generation-1 note.

The lifecycle test lists, acknowledges, deserializes provenance, lists again,
and tries a duplicate acknowledgment in separate processes.

It also asserts ticket bytes and DAG readiness are unchanged.

The current duplicate acknowledgment is expected to fail with `no active note`.

The ticket explicitly changes that empty per-ticket case to a successful plain
`Nothing to read for {ticket_id}.` surface.

The global empty-list behavior remains `Nothing to read.`.

The existing lifecycle already provides the pattern for durable cross-process
assertions requested by the acceptance criteria.

The fixture helper currently hard-codes one note, attempt, and generation.

It can be generalized or supplemented to create the two-generation scenario.

## Other consumers

`status` uses the same note formatting to place deferred notes between urgent
waiting work and the DAG display.

The status tests pin this ordering and single-note text.

Generation labels must therefore be conditional: single-note tickets retain the
current ticket line, while only tickets with multiple active notes gain labels.

The generation-label rule is per ticket, not based on total global note count.

A queue containing one note each for two tickets should not label either one.

## Repository and workflow constraints

The ordinary worktree already contains changes unrelated to this ticket,
including Lisa ledgers, other ticket state, and `seal_visibility.rs`.

Those files must remain untouched by ticket commits.

Ticket source commits must use `lisa commit-ticket` with exact include paths.

Ordinary `git add` and `git commit` are prohibited for this assignment.

Phase artifacts belong only in the attempt-private work directory.

Lisa owns phase transitions, publication, and final completion.

The old multi-note error string must disappear from the whole codebase.

The implementation must preserve one provenance row per acknowledged exact key.

The public operator vocabulary is ticket ID and generation; attempt ID remains
an internal part of durable identity.
