# Structure — T-049-06-02 Notes for you queue

## Core notes module

Create `crates/lisa-core/src/notes.rs`.

Public types:

- `NoteKey`: ticket ID, attempt ID, and generation.
- `QueuedNote`: exact key plus validated `DispositionNote`.

Public functions:

- `collect_notes(completion_journal, provenance) -> Result<Vec<QueuedNote>, String>`.
- `acknowledge_note(completion_journal, provenance, ticket_id) -> Result<QueuedNote, String>`.

Internal types project state-tagged completion journal rows. The reducer reads strict
newline-terminated JSONL, selects confirmed notes, reads ack facts, removes exact
keys, and sorts results. Ack collects current entries, resolves exactly one matching
ticket, appends provenance, and returns the settled entry.

Modify `crates/lisa-core/src/lib.rs` to export `pub mod notes`.

## Provenance schema

Modify `crates/lisa-core/src/provenance.rs`.

Add `NoteAcknowledgmentType`, `NoteAcknowledgmentRecord`, a mixed-ledger enum variant,
and `append_note_acknowledgment_record`. Required row fields are schema version,
`record_type`, ticket ID, attempt ID, generation, and timestamp. Keep all old rows
unchanged and extend mixed-ledger/append tests.

## CLI notes module

Create `crates/lisa-cli/src/notes.rs`.

It owns production `.lisa/completion-journal.jsonl` and provenance paths, list and ack
entry points, and a reusable formatter. Output shape:

```text
Notes for you (1)
T-046-06-03  The recorded measurement exceeds the criterion text.
       Criterion: "approximately 200 MiB"
       Evidence: docs/active/work/T-046-06-03/review.md#measurement
```

No entries means no output. Ack prints a short confirmation after durable append.

## CLI command wiring

Modify `crates/lisa-cli/src/main.rs`.

Add `mod notes`, nested `NotesCommand::Ack`, and operator `Commands::Notes` with an
optional nested command and project path. No subcommand dispatches list; ack dispatches
settlement. Place Notes after Status and provide plain descriptions/examples.

## Status integration

Modify `crates/lisa-cli/src/status.rs`.

Read the shared queue from `.lisa` paths and print it after Waiting on you but before
the DAG. Reuse the CLI formatter. Propagate durable read errors. Do not pass note data
into DAG construction or scheduling summaries.

## CLI tests

Create `crates/lisa-cli/tests/notes_ux.rs`.

Fixture helpers create a project path containing spaces, minimal ticket tree, confirmed
note journal rows, and optional waiting disposition. Tests cover list, empty output,
summary-first detail order, status distinction/order, ack row shape, durable clearing,
fresh-process non-resurfacing, duplicate rejection, and unchanged ticket/DAG state.

Modify `crates/lisa-cli/tests/help_surface.rs` to add Notes to operator and all-command
arrays, top-level snapshot, command help snapshots, counts, and comments.

## Dashboard UI

Modify `crates/lisa-plugin/src/ui.rs`.

Add `NoteItem` with ticket ID, summary, criterion quote, and evidence citation. Add
`PluginState.note_items`, initialize it empty, and add `render_notes_for_you`. Render
heading, summary lead, criterion, evidence, and blank separator. Operations ordering:

1. Waiting on you.
2. Notes for you.
3. Attention banner.
4. Threads.
5. Activity.

Add pure UI tests for populated, empty, summary-first, and section ordering behavior.

## Plugin projection

Modify `crates/lisa-plugin/src/lib.rs`.

In `State::to_ui_state`, call the shared reducer with `completion_journal_path` and
`ledger_path`, map entries to UI NoteItems, and include them in PluginState. Missing
files naturally yield empty state. On read error, conservatively emit no entry.

Do not add note state to Thread, AgentSlot, PendingCompletion, DAG, or scheduler methods.

Add a lifecycle test near existing completion note fixtures. Complete with a note,
assert dependent flow and zero parks, assert UI note, reconstruct State from durable
files, assert note after restart, ack it, and assert projection clears with ticket,
dependency, thread, and seat facts unchanged.

## Attempt artifact

Create/update `progress.md` during implementation. Review artifacts are written only
after source commits and full verification. All artifacts remain in the private attempt
directory until Lisa admits them.

## Commit units

Core unit exact likely paths:

- `crates/lisa-core/src/lib.rs`
- `crates/lisa-core/src/notes.rs`
- `crates/lisa-core/src/provenance.rs`

CLI unit exact likely paths:

- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/notes.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-cli/tests/help_surface.rs`
- `crates/lisa-cli/tests/notes_ux.rs`

Plugin unit exact paths:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/ui.rs`

Each unit uses `lisa commit-ticket` with only its exact repository-relative paths.

## Explicitly unchanged

Do not modify ticket frontmatter, disposition parsing, seal selection, DAG readiness,
parking rules, or canonical work artifacts. Do not automatically create tickets from
notes. Do not write to shared `docs/active/work/T-049-06-02`.
