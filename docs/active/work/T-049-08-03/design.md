# Design: T-049-08-03 notes acknowledgment

## Goal

Every active notes state created by Lisa must have an operator-facing command
that advances it without editing provenance by hand.

Bare acknowledgment must drain a same-ticket queue one item at a time.

Explicit acknowledgment must address the generation identity shown by listing.

The design must preserve exact, append-only acknowledgment provenance.

## Behavioral decisions

`lisa notes ack T-ID` selects the active note with the smallest generation.

When bare acknowledgment starts with multiple active notes for that ticket, it
prints the ticket-prescribed sentence with the count remaining afterward:

`Marked the oldest note read — 1 more remains.`

The count and noun inflect for values other than one.

When bare acknowledgment starts with one active note, it retains the current
success output:

`T-ID acknowledged.`

Repeating bare acknowledgment after the queue is empty succeeds and prints:

`Nothing to read for T-ID.`

`lisa notes ack T-ID --generation N` selects only generation `N`.

Explicit success names the generation:

`T-ID generation N acknowledged.`

An explicit generation that is not active fails without writing provenance.

Its error says the requested generation is not listed and names all active
generations for the ticket, for example:

`Generation 9 is not listed for T-ID. Listed generations: 1, 2.`

When there are no active generations, the same explicit operation reports that
nothing is listed rather than emitting an empty suffix.

## Definition of oldest

The numeric completion generation is the operator-visible order key.

The oldest active note is the match with the minimum generation.

This is intentionally not the first `BTreeMap<NoteKey, _>` item.

`NoteKey` orders attempt ID before generation, so lexical attempt ordering can
disagree with numeric generation ordering.

Exact `NoteKey` ordering is retained only as a deterministic tie-break if
malformed or legacy history contains two attempt keys with the same ticket and
generation.

That tie-break still appends one exact provenance fact and makes forward
progress.

## Listing format

Generation labels appear only when a ticket has more than one active note.

The ticket summary line becomes:

`T-ID  Generation N  Summary text.`

The words are plain and match the `--generation` concept in command help.

Criterion and evidence lines remain unchanged.

A ticket with one active note retains its exact current summary line.

The decision is based on count per ticket, not total queue length.

This preserves existing status and list snapshots for unrelated multi-ticket
queues in which every ticket has only one note.

## Core API options

### Option 1: Keep the current signature and select oldest internally

This would repair bare acknowledgment but cannot implement `--generation`.

Adding a second exact-generation function would duplicate collection, error,
and append logic.

It would also make consistent empty and remaining-count behavior harder for the
CLI to express.

Rejected because the two commands are variants of one selection operation.

### Option 2: Let the CLI collect and select, then expose append publicly

The CLI could call `collect_notes`, select a key, and append provenance itself.

This moves durable exactness and timestamp construction out of the core module.

Other future callers could select differently or construct incomplete records.

Rejected because the core currently owns the queue-to-provenance transition.

### Option 3: Extend `acknowledge_note` with optional generation and outcome

The core receives `Option<u64>` and owns both selection modes.

It returns enough structured state for the CLI to choose plain output without
re-reading files after the append.

The result distinguishes an empty bare queue from a successful append.

Explicit unknown generations remain core errors because validity depends on the
same projected snapshot used for selection.

Chosen because it keeps selection and durable mutation in one boundary.

## Outcome shape

Add an acknowledgment result enum in `lisa-core::notes`:

- `NothingToRead`
- `Acknowledged { note, remaining, was_oldest_of_multiple }`

`note` is the exact `QueuedNote` whose provenance row was written.

`remaining` is the number of other active notes for that ticket in the snapshot.

`was_oldest_of_multiple` records that the caller used bare selection while the
ticket had more than one match.

The CLI does not need to infer why a note was selected from counts or flags.

An alternative is a struct wrapped in `Option`, but an enum makes the no-write
case explicit and prevents callers from looking for a fabricated note.

## Selection algorithm

Collect all active notes once.

Filter entries to the requested ticket.

For bare selection:

1. Return `NothingToRead` when the filtered list is empty.
2. Sort by `(generation, exact NoteKey)` or use `min_by_key` with that key.
3. Select the first entry.
4. Record whether the starting count exceeded one.

For explicit selection:

1. Find the entry whose generation equals the requested value.
2. If absent, derive sorted, deduplicated active generation labels.
3. Return a plain error that names those labels.
4. If present, select that exact entry.

For either successful selection, append the existing record shape unchanged.

Return the selected note and `starting_count - 1` only after append succeeds.

## Empty behavior

Bare empty acknowledgment is a successful no-op.

This follows the ticket's no-blank-surfaces rule and makes repeated draining safe.

It writes no provenance row because there is no exact completion key to settle.

Explicit unknown acknowledgment remains an error because the user requested a
specific identity that is not active.

This distinction makes typos visible while keeping the natural repeat-until-empty
workflow idempotent.

## CLI parsing

Add `generation: Option<u64>` to `NotesCommands::Ack`.

Expose it as `--generation <GENERATION>`.

Clap parses only nonnegative integers into `u64`.

Dispatch passes the optional value directly to `run_ack`.

Help text changes from “current note” to language covering oldest or exact note.

The top-level example remains valid; an explicit-generation example can be added
to make the newly available escape hatch discoverable.

## Error boundaries

History read, parse, and append failures remain `Err(String)`.

The main command continues to prefix errors and exit 1.

Bare empty acknowledgment is no longer represented as an error.

Unknown explicit generation uses operator vocabulary only:

- requested generation
- ticket ID
- listed generations

It does not mention active-note multiplicity, exact keys, attempts, provenance,
or acknowledgment requirements.

The obsolete multi-note error literal is deleted rather than retained unused.

## Test strategy

Core unit tests pin selection independent of CLI formatting.

They cover oldest selection, exact selection, no-op empty selection, unknown
generation, and exact provenance identities.

CLI formatter tests pin conditional per-ticket generation labels.

Built-binary fixtures provide the acceptance-level contract.

One two-generation lifecycle invokes a fresh process for list, first bare ack,
second bare ack, final list, and empty bare ack.

It checks both provenance rows and their generations in append order.

A separate two-generation fixture acknowledges the newer generation explicitly,
checks that the older remains, and rejects an unknown generation while naming
both currently listed generations before mutation.

Existing single-note lifecycle assertions remain to protect old behavior.

The help snapshot pins the new flag and its discoverability.

Focused package tests run before the full workspace suite.

## Compatibility

The provenance schema and JSON field names do not change.

No migration is needed because queue state is reconstructed.

Existing acknowledgments continue to suppress only their exact keys.

Single-note list and bare-ack success output remain stable.

Global empty list output remains stable.

Only the previous duplicate-bare-ack failure changes to a successful plain no-op.

Status inherits conditional labels through the shared formatter.

No scheduler, ticket, phase, or DAG API changes.
