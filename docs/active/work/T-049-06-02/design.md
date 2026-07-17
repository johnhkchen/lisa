# Design — T-049-06-02 Notes for you queue

## Decision summary

Add a shared `lisa-core` notes module.
It reconstructs confirmed notes from the completion journal and subtracts exact
acknowledgments from provenance. CLI and plugin consume the same reducer.
Renderers stay formatting-only. Ack appends provenance and changes no board state.

## 1. Authoritative source

### Terminal provenance only

The terminal execution row already carries `completion_note`, so one-ledger reading
would be simple. It lacks completion generation, however, and is not the explicit
journal source required by the ticket. It is also emitted after completion teardown.

Rejected.

### Completion journal only, including acknowledgments

This would reduce one file, but would turn the CLI into a second completion-journal
writer. Informational settlement would become coupled to completion authority and
would still need a provenance row for acceptance.

Rejected.

### Confirmed journal notes minus provenance acknowledgments

Confirmed journal state supplies exact filed-note identity. Provenance supplies
append-only settlement facts. Neither the journal nor board is mutated by ack.

Chosen.

## 2. Shared implementation boundary

Independent CLI and plugin implementations would create multiple definitions of
active identity, malformed-history behavior, and sorting. Moving the full journal
module into core would be a broad migration involving plugin publication logic.

Chosen: a narrow read model in `lisa-core`. It parses the journal fields needed for
queue projection, validates complete JSONL lines, tracks exact generation state,
and returns confirmed note entries. It does not replace plugin journal authority.

## 3. Acknowledgment identity

Ticket-only identity would let an old ack hide a later note. A content hash would
add machinery and treat repeated wording ambiguously. Use the journal's exact key:
ticket/completion ID, attempt ID, and generation. Operators still type ticket ID;
the command resolves that ID to one active exact entry before appending.

## 4. Provenance shape

Add `NoteAcknowledgmentRecord` with a disjoint
`record_type: "note-acknowledged"`. Carry schema version, ticket ID, attempt ID,
generation, and acknowledgment timestamp. A dedicated append helper writes it.
The row does not copy summary text because the durable journal retains note content.

## 5. CLI syntax and copy

Use `lisa notes` to list and `lisa notes ack <ticket-id>` to settle. The explicit
nested verb prevents a bare positional value from unexpectedly writing state.

Populated output begins with `Notes for you (N)`. Each entry leads with ticket ID
and plain summary, then indents the quoted criterion and evidence citation. Empty
queues produce no heading. Ack prints a short confirmation. Missing or already
settled tickets fail without writing a row.

## 6. Status ordering

Status prints Waiting on you first when present, Notes for you second when present,
then the DAG. This makes urgency visible: Waiting needs action now; Notes are deferred.
If only notes exist, Notes is first. Empty queues render nothing.

The list formatter is shared within the CLI so `lisa notes` and status cannot drift.
Queue membership remains shared in core.

## 7. Dashboard projection

Add `NoteItem` and `PluginState.note_items` in UI. `State::to_ui_state` calls the
core reducer with its journal and ledger paths, then maps plain display data.
Render Waiting on you, Notes for you, attention, threads, then activity.

Reading durable files at projection time observes acknowledgments made by another
CLI process and guarantees restart reconstruction. Missing files mean an empty queue.
Malformed durable state produces no invented notes; journal health remains the
plugin's existing error surface.

## 8. Journal projection rules

Require nonempty JSONL to end in newline and report line-numbered parse errors.
Legacy rows without notes remain valid. Requested-only, command-in-flight, failed,
rejected, and pass-confirmed generations never appear. Confirmed note rows do.

Track state by exact generation key. A later state for that key controls visibility.
Subtract only exact acknowledgment keys. Sort by ticket, attempt, then generation.
If a ticket somehow has multiple active entries, ack fails rather than guessing.

## 9. Zero scheduling effect

The notes module imports no DAG, Thread, or seat types. Acknowledgment accepts only
journal path, provenance path, and ticket ID. It appends provenance only. No
reconciliation callback runs. Tests compare ticket bytes, DAG readiness, plugin
dependencies, threads, and seats before and after acknowledgment.

## 10. Core tests

Cover requested versus confirmed, pass versus note, exact acknowledgment, later
generation resurfacing, deterministic ordering, missing files, torn history, malformed
JSON, mixed provenance compatibility, and duplicate/unknown ack behavior.

## 11. CLI tests

Use the real binary in a project path containing spaces. Prove list summary-first
rendering, empty suppression, distinct status sections, ack provenance, durable
clearing across fresh processes, no duplicate row, and unchanged ticket/DAG state.

Update help snapshots intentionally because Notes becomes an operator command.

## 12. Dashboard tests

Pure UI tests cover populated and empty rendering, summary/detail order, and section
order. Plugin State fixtures write durable journal data, project it, reconstruct a
new State, and prove the same entry. After ack, the projection clears without a park,
seat change, dependency change, or completion change.

## 13. Compatibility

No disposition schema, seal selection, ticket frontmatter, DAG, or parking changes.
Old journal and provenance rows remain readable. The plugin WASM check validates the
new core filesystem/serde path. Notes never create tickets or amend criteria.

## 14. Rejected automatic graduation

The command will not reopen completed work, edit acceptance criteria, or author a
ticket. If the operator decides a dispute is real work, ordinary ticket authoring
remains the explicit path outside this ticket.
