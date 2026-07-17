# Research — T-049-06-02 Notes for you queue

## Ticket boundary

This ticket adds read and settlement surfaces for completion notes.
T-049-06-01 already added the note disposition and durable completion flow.
The queue is informational and must not alter ticket status, dependencies, seats, or completion.
The presentation targets are `lisa status` and the Zellij dashboard.
The command target is `lisa notes`, including an acknowledgment operation.
All queue state must be reconstructable after restart.

## Existing note model

`crates/lisa-core/src/disposition.rs` owns `DispositionNote`.
It contains exactly `criterion_quote`, `evidence_citation`, and `summary`.
Its constructor rejects blank values and its accessors preserve supplied text.
The note disposition requires `reason: null` and rejects unknown fields.
There is no general work-quality complaint field.
`ReviewDisposition::Note` authorizes completion exactly as Pass does.
Block and Invalid remain non-authorizing.

## Existing completion journal

`crates/lisa-plugin/src/completion_journal.rs` owns `.lisa/completion-journal.jsonl`.
The journal is append-logical but atomically republished as complete bytes.
Every row has a schema version, seal, and state-tagged body.
The current schema is 5 and readers accept versions 1 through 5.
Each generation is keyed by completion ID, attempt ID, and generation number.
For ordinary completion, completion ID is the ticket ID.

Requested rows store prior phase/status and may store a validated note.
Confirmed rows store the seal receipt and may store the same validated note.
The reducer rejects confirmation whose note differs from the request.
The aggregate retains both the generation key and note.
Only Confirmed is a filed note; requested-only and rejected attempts are not.

`completion_journal::load` reconstructs current ticket aggregates.
`State::restore_completion_journal` invokes it during plugin load.
Malformed history makes journal health fail closed.
The restored aggregate already drives restart-safe completion reconciliation.
It is the durable source required by this ticket.

## Existing provenance ledger

`crates/lisa-core/src/provenance.rs` owns `.lisa/provenance.jsonl` schemas and appends.
The ledger is heterogeneous JSONL parsed by untagged `ProvenanceLedgerRecord`.
Current shapes are execution, assignment-transition, and parking-transition.
Transition shapes have disjoint `record_type` values and required fields.
The current schema version is 6.

Terminal execution rows now carry optional `completion_note`.
They include ticket and attempt lease, but not completion generation.
They prove note-bearing execution but are not the journal's exact generation state.
Append helpers create parents and write one compact newline-terminated JSON row.
An acknowledgment fits naturally as another provenance transition.

## Existing status surface

`crates/lisa-cli/src/status.rs` implements `lisa status`.
It resolves configured directories, scans tickets, builds the DAG, and renders it.
It derives parked remedies from canonical review artifacts.
Waiting on you prints before the DAG when nonempty and prints nothing when empty.
Each entry leads with a plain ask and puts engineering reason on an indented line.
Agent-owned remedies are excluded; world-owned remedies explain self-checking.

`crates/lisa-cli/tests/parked_ux.rs` exercises the real binary.
It pins section ordering, summary/detail order, and paths containing spaces.
The Notes for you surface needs equivalent black-box coverage.

## Existing dashboard surface

`crates/lisa-plugin/src/ui.rs` owns pure rendering.
`ui::PluginState` is the complete input and already contains `waiting_items`.
`render_waiting_on_you` emits the first Operations-view content.
It is followed by attention, threads, and filtered activity.
UI tests construct PluginState directly and assert exact content and ordering.

`State::to_ui_state` in plugin `lib.rs` builds the projection.
Internal State already holds completion aggregates, journal path, and ledger path.
The UI layer has no filesystem or scheduler authority.
Notes should enter PluginState as plain projection data.

## Existing command architecture

`crates/lisa-cli/src/main.rs` owns Clap definitions and dispatch.
Operator commands are top-level variants that resolve a project path then call modules.
There is no notes module today.
`crates/lisa-cli/tests/help_surface.rs` pins the full help snapshot.
It currently names six operator commands and fourteen total commands.
Adding Notes requires intentional snapshot and count changes.

## Queue identity

Ticket ID alone is insufficient for durable acknowledgment.
A ticket can have later completion attempts or generations.
An old acknowledgment must not suppress a later note for the same ticket.
The journal's exact identity is ticket/completion ID, attempt ID, and generation.
The queue can display ticket ID while retaining that full key internally.

## Queue reduction

Only confirmed generations carrying a note are candidates.
Pass confirmations, requests, failures, and rejections contribute nothing.
An acknowledgment suppresses only its exact generation key.
Unknown acknowledgments must not create misleading provenance.
Output needs deterministic ordering for stable status and tests.

## Restart and level triggering

CLI commands always read durable files fresh.
The dashboard can be running when another process writes an acknowledgment.
Fresh reduction at projection time observes that external change.
Caching would require a new invalidation mechanism.
Restarting plugin State against the same journal and ledger must reproduce the queue.

## Scheduling boundary

The DAG reads ticket files, not notes or provenance acknowledgments.
Seats use ready tickets and ownership state.
Completion uses disposition admission and completion journal state.
Acknowledgment must edit no ticket, work artifact, or completion event.
This makes its zero scheduling effect directly testable.

## Fixture patterns

Core tests already parse mixed provenance JSONL.
Journal tests already create requested and confirmed note rows.
CLI fixtures invoke `CARGO_BIN_EXE_lisa` in temporary projects.
Plugin restart fixtures construct a second State against the same durable files.
UI tests compare exact lines and relative section positions.

## Worktree constraints

The initial worktree contains Lisa-managed journal and ticket changes plus another ticket's work.
Those are not owned by this ticket.
No ordinary index operation is permitted.
Source units must use `lisa commit-ticket` with exact includes.
Phase artifacts remain under this attempt-private directory.

## Conclusions

Confirmed completion journal state is the source of queue candidates.
Provenance is the source of acknowledgment facts.
The full completion generation key prevents stale suppression.
A shared core reducer can keep CLI and plugin behavior identical.
Rendering stays downstream of all scheduling and completion authority.
