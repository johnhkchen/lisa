# Research — T-049-06-01

## Ticket contract

The ticket adds a third Review disposition named `note`.
It is completion-authorizing, like `pass`, rather than completion-stopping, like `block`.
Its permitted subject is narrow: disagreement between written acceptance criteria and recorded evidence.
The durable shape must retain three pieces of reviewer-authored text:

- the disputed criterion quoted verbatim;
- the evidence citation expressed as a path;
- a plain one-sentence summary.

The parser remains fail-closed.
Missing criterion or evidence fields cannot degrade to pass.
Strict pass behavior remains unchanged: its `reason` must be JSON null.
Block behavior, including the legacy structured fallback, remains unchanged.

## Existing disposition boundary

`crates/lisa-core/src/disposition.rs` owns the filesystem parser and validated domain enum.
`parse_review_disposition` reads bytes, parses generic `serde_json::Value`, then calls `validate_document`.
All read, JSON, object-shape, type, and semantic failures produce `ReviewDisposition::Invalid`.
`ReviewDisposition` currently has `Pass`, `Block`, and `Invalid` variants.
Block carries reason, remedy owner, ask, optional steps/check, and an `unstructured` marker.
The parser removes `disposition` and `reason` before branching.
Pass accepts exactly a null reason.
Block accepts a nonblank reason and delegates optional remedy validation.
Unknown names are invalid.
Extra fields are currently tolerated for pass and block.
Parser tests live beside the implementation and write documents through a temporary file.

## Pure completion domain

`crates/lisa-core/src/completion.rs` defines adapter-independent completion state.
`DurableCompletionInputs` combines current-lease artifact admission with `ReviewDisposition`.
`reconcile` is level-triggered and emits `LaunchCompletion` only when an admitted artifact exists and the disposition authorizes completion.
The current authorization check is a direct `matches!(..., ReviewDisposition::Pass)`.
Core tests prove pass eligibility and block/invalid ineligibility.
Both commit and journal seal types are represented independently of disposition.
Therefore disposition authorization does not choose or weaken a seal tier.

## Plugin admission and scheduling

`crates/lisa-plugin/src/lib.rs` is the adapter boundary.
`admit_passing_review` first admits the private attempt artifact through the current lease.
`passing_review_disposition` parses the published artifact and currently accepts only `Pass`.
Block and Invalid become typed `CompletionRejection::DispositionBlocked` values.
Several scheduler and timeout match expressions distinguish pass, block, and invalid.
The review timeout treats an accepted disposition as no remaining review work.
The level-triggered block policy specifically selects `Block`; a note must not enter that path.

## Completion journal

`crates/lisa-plugin/src/completion_journal.rs` owns append-only completion records.
Its durable schema is currently version 4 and accepts versions 1 through 4.
Transitions are `Requested`, `CommandInFlight`, `FailureObserved`, `Rejected`, and `Confirmed`.
`Confirmed` is the point at which a seal receipt is durably associated with completion.
Commit receipts carry a commit id; journal receipts carry content hashes.
The journal is atomically replaced after folding and validating the complete prior history.
The in-memory aggregate retains the confirmed receipt but currently has no Review note field.
Older serialized fields use serde defaults where schema evolution requires compatibility.
Journal tests cover folding, seal mismatch, torn records, commit receipts, and journal receipts.

## Completion publication ordering

For a commit seal, the plugin launches `lisa complete-ticket` and later verifies the result.
For a journal seal, it hashes the prepared Done ticket and retained work artifacts, atomically publishes Done, and obtains a journal receipt.
Both paths converge on `finish_successful_completion`.
That method verifies the receipt tier and durable Done bytes, appends `Confirmed`, rebuilds the DAG, emits terminal provenance, releases the slot, and schedules ready dependents.
This convergence is the shared atomic completion boundary.

## Provenance

`crates/lisa-core/src/provenance.rs` defines mixed JSONL ledger records.
Terminal execution rows use `ProvenanceRecord`.
The plugin constructs those rows in `emit_provenance` and related helpers.
The record already carries the completion seal.
Schema compatibility is maintained with serde defaults for fields added after earlier rows.
Completion tests read the mixed ledger and assert the terminal execution outcome and seal.
There is no existing note record type or note field.

## Field fixture

`docs/active/work/T-046-06-03/operator-note-2026-07-17.md` preserves the motivating disagreement.
The disputed disk criterion used an approximately 200 MiB expectation.
The recorded Codex closing measurement was 225 MiB and the runbook had been recalibrated to 300 MiB.
The operator note says the reviewer correctly identified document drift while the evidence itself remained valid.
This is the intended criteria-versus-evidence case.
The fixture also contains an old-Zellij criterion dispute, but the ticket calls out the 75 MiB paperwork-versus-measurement gap as the representative note.

## Constraints and boundaries

The source worktree already contains unrelated modifications, including `crates/lisa-plugin/src/lib.rs`.
Ticket changes must preserve those bytes and commit only exact ticket-owned paths through `lisa commit-ticket`.
No ordinary Git index operation is permitted.
The assignment artifacts belong in the private attempt work directory.
Lisa alone updates ticket phase/status and publishes admitted artifacts.
No UI Notes queue is requested by this ticket; the durable note data is the mechanism for later queue work.
No free-form work-quality complaint field should be introduced.
Evidence is cited, not loaded or judged, by the parser.
The parser must preserve supplied strings rather than normalize the quoted criterion.

