# Structure: T-049-08-03 notes acknowledgment

## Change inventory

No new product modules are required.

No product files are deleted.

The implementation modifies five existing files:

1. `crates/lisa-core/src/notes.rs`
2. `crates/lisa-cli/src/notes.rs`
3. `crates/lisa-cli/src/main.rs`
4. `crates/lisa-cli/tests/notes_ux.rs`
5. `crates/lisa-cli/tests/help_surface.rs`

Phase artifacts remain in the attempt-private work directory and are not part of
the source commit units.

## `crates/lisa-core/src/notes.rs`

### Public outcome type

Introduce a public enum adjacent to `QueuedNote`:

```text
NoteAcknowledgment
  NothingToRead
  Acknowledged {
    note: QueuedNote,
    remaining: usize,
    was_oldest_of_multiple: bool,
  }
```

The type derives debug, clone, partial equality, and equality to support direct
unit assertions.

The type describes the durable operation result, not presentation strings.

`NothingToRead` means no provenance row was appended.

`Acknowledged` means the exact returned note key was appended successfully.

`remaining` is scoped to the same ticket.

`was_oldest_of_multiple` is true only for bare selection from a starting queue
of at least two notes.

### Function signature

Change:

```text
acknowledge_note(journal, provenance, ticket_id)
  -> Result<QueuedNote, String>
```

to:

```text
acknowledge_note(journal, provenance, ticket_id, generation: Option<u64>)
  -> Result<NoteAcknowledgment, String>
```

All callers must now state bare (`None`) or exact (`Some(n)`) selection.

### Match collection

Keep `collect_notes` unchanged as the projection boundary.

Collect same-ticket matches into a mutable vector.

Sort the vector by numeric generation and then exact key identity.

This establishes oldest ordering without changing global list ordering.

### Bare branch

When `generation` is `None`:

- empty matches return `NothingToRead`;
- otherwise select index zero;
- set `was_oldest_of_multiple` from the initial length.

### Exact branch

When `generation` is `Some(requested)`:

- search sorted matches for equal generation;
- if absent, build a sorted deduplicated list of visible generation numbers;
- return an error naming the ticket, requested generation, and listed values;
- when no values exist, state that no generations are listed;
- if found, select that entry;
- set `was_oldest_of_multiple` false.

### Durable append

Keep the existing `append_note_acknowledgment_record` call in this function.

Keep the existing schema version, record type, ticket, attempt, generation,
timestamp, and append-error context.

Calculate `remaining` from the pre-append same-ticket match count.

Construct `Acknowledged` only after append returns success.

### Unit tests

Update existing single-note calls to pass `None` and assert the outcome enum.

Add a helper capable of writing note rows for multiple ticket generations.

Add or extend tests for:

- bare oldest choice despite reverse lexical attempt ordering;
- one provenance row for that exact oldest key;
- explicit newer-generation choice;
- unknown explicit generation with listed values;
- bare empty result without a second provenance row;
- later-generation durability after reconstructing from disk.

The malformed-history tests remain unchanged.

## `crates/lisa-cli/src/notes.rs`

### Imports

Import the new `NoteAcknowledgment` enum with existing note functions and types.

### Conditional labels

Before rendering entries, count active notes per ticket.

Use a local ordered or hashed count map keyed by ticket ID.

For each note:

- count one retains `{ticket_id}  {summary}`;
- count greater than one renders
  `{ticket_id}  Generation {generation}  {summary}`.

Criterion and evidence rendering remains byte-for-byte unchanged.

Heading count remains total active note count.

Empty formatter behavior remains unchanged.

### Acknowledge entry point

Change `run_ack` to accept `generation: Option<u64>`.

Match on the core outcome:

- `NothingToRead` prints `Nothing to read for {ticket_id}.`;
- `Acknowledged` with `was_oldest_of_multiple` prints the oldest/remaining line;
- other acknowledged bare results print the existing ticket success line;
- acknowledged explicit results print the ticket and generation success line.

Use a small noun suffix branch for `1 more remains` versus
`N more remain` if a count beyond one occurs.

The ticket-prescribed em dash and punctuation are string-pinned.

### Formatter unit tests

Retain the current single-note exact vector assertion.

Add a multi-note same-ticket assertion with generation labels.

Add a multiple-ticket assertion if needed to prove labels are per-ticket rather
than driven by the global heading count.

## `crates/lisa-cli/src/main.rs`

### Clap structure

Add this field to `NotesCommands::Ack`:

```text
#[arg(long)]
generation: Option<u64>
```

Document it as targeting the listed generation.

Adjust the ack subcommand summary to describe oldest/default behavior.

Update the notes examples with an explicit generation invocation.

### Dispatch

Destructure both `ticket_id` and `generation`.

Pass both to `notes::run_ack`.

Do not alter main's common error handling.

## `crates/lisa-cli/tests/notes_ux.rs`

### Fixture construction

Refactor or supplement `write_note_journal` so tests can append confirmed notes
with configurable attempt, generation, and note summary.

All rows remain valid completion-journal rows at schema version five.

Each generation gets requested and confirmed rows where appropriate to mirror
real history, while only confirmed rows affect projection.

Keep separate CLI processes through the existing `lisa` helper.

### Single-note lifecycle

Keep current list text and success text assertions.

Change the duplicate bare acknowledgment expectation:

- process succeeds;
- stdout equals `Nothing to read for T-046-06-03.`;
- stderr is empty;
- provenance row count remains one.

Retain ticket-byte and DAG-readiness invariants.

### Two-note bare lifecycle

Create active generations 1 and 2 for one ticket.

Pin list output so both summary lines show `Generation 1` and `Generation 2`.

Run bare acknowledgment in a fresh process.

Pin `Marked the oldest note read — 1 more remains.`.

List from a fresh process and assert only generation 2 remains; because only one
is active, its generation label disappears.

Run bare acknowledgment again and pin existing single-note success output.

List again and pin global empty output.

Deserialize every provenance line and assert append order `[1, 2]`, attempts,
and total row count.

### Explicit-generation lifecycle

Create two active generations again.

First invoke `--generation` with an unknown value.

Assert exit 1 and plain stderr naming both listed generations.

Assert no provenance file or no acknowledgment rows were written.

Invoke `--generation 2`.

Assert exact success output and provenance generation 2.

List from a fresh process and assert generation 1 remains durable.

Optionally acknowledge it to prove the queue remains drainable.

## `crates/lisa-cli/tests/help_surface.rs`

Update the pinned notes help snapshot:

- revised ack summary;
- explicit-generation example if added.

The top-level notes help does not display nested ack options.

Add a dedicated `notes ack --help` snapshot only if the existing snapshot harness
supports nested command argument arrays; otherwise parser behavior is covered by
the built-binary lifecycle test.

## Commit boundaries

The implementation can land as two meaningful isolated transactions.

Commit 1 owns the core selection contract and its unit tests:

- `crates/lisa-core/src/notes.rs`

Commit 2 owns CLI parsing, presentation, built-binary fixtures, and help:

- `crates/lisa-cli/src/notes.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/notes_ux.rs`
- `crates/lisa-cli/tests/help_surface.rs`

Each transaction uses `lisa commit-ticket --ticket-id T-049-08-03` with these
exact repository-relative include paths.

No ordinary-index command participates.

## Verification order

1. Run formatting checks after source edits.
2. Run `cargo test -p lisa-core notes` for core selection.
3. Run `cargo test -p lisa-cli --test notes_ux` for built-binary lifecycle.
4. Run `cargo test -p lisa-cli --test help_surface` for clap snapshots.
5. Search for the obsolete multi-note error literal.
6. Run the full workspace test suite.
7. Inspect status and diffs to ensure only ticket-owned paths remain.
