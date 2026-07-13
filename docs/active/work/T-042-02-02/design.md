# Design: Atomic append-only completion journal

## Goal

Persist the completion aggregate strongly enough that a fresh plugin process
can recover Requested, CommandInFlight, or Confirmed without guessing from
ephemeral scheduler memory.

Every accepted transition must be durably visible as one complete journal
history or leave the prior complete history intact.

The journal must retain the completion generation identity introduced by
T-042-02-01 and the correlation needed by T-042-02-03.

The adapter must continue to mask unverified Done frontmatter, preserve attempt
fencing, and publish authoritative provenance only after confirmation.

## Grounded constraints

The pure completion reducer already defines legal transitions and correlation
matching.

The plugin is the sole owner of completion command launch and result handling.

`pending_completions` is currently the masking and result-attribution state but
is process-local.

The completion commit key is independently durable in Git, but a reachable
commit cannot prove whether a request was merely accepted or a command result
was still outstanding.

`publication.rs` provides whole-body sibling-temp replacement, not a true
append syscall.

The plugin already has serde and serde_json.

The provenance ledger has a public mixed-version compatibility contract and
does not model completion aggregate transitions.

## Option 1: extend `.lisa/provenance.jsonl`

Completion transition rows could become another untagged provenance variant.

This reuses a durable append-only file and existing parent-directory behavior.

It would require every provenance reader to distinguish a third shape.

It would couple scheduler correctness state to an analytics and execution
history schema.

The current provenance writer uses true append and does not provide atomic
whole-history publication through `publication.rs`.

A future malformed completion row could also make existing provenance tools
fail even though execution rows are intact.

This option is rejected because backward compatibility is an explicit
acceptance requirement and the two ledgers have different authority.

## Option 2: append directly with OpenOptions

A separate `completion-journal.jsonl` could use create-plus-append exactly like
provenance.

The implementation would be small and would never rewrite old bytes.

An interrupted `write_all` can expose a partial final JSON line.

Readers would need a policy for silently discarding a tail, and doing so could
discard the only durable CommandInFlight or Confirmed fact.

It also ignores the ticket's explicit sibling-temporary publication direction.

This option is rejected because a torn final record makes aggregate truth
ambiguous after restart.

## Option 3: one immutable file per transition

Each transition could be published as a new file inside a completion-journal
directory.

That is physically append-only and avoids rewriting histories for unrelated
tickets.

It requires a durable total ordering convention, collision-free sequence
allocation, directory enumeration, and handling of simultaneous writers.

Wall-clock or nonce filenames do not themselves prove semantic order.

A per-aggregate counter requires reading prior state and solving the same
serialization problem.

Operational inspection and repository tracking become noisier than one JSONL
history.

This option is viable but rejected as unnecessary complexity for the plugin's
single event-loop writer.

## Option 4: atomic whole-history JSONL publication

Use a dedicated `.lisa/completion-journal.jsonl`.

For each transition:

1. read the prior complete bytes;
2. parse and fold every existing line;
3. validate the proposed transition against the reconstructed aggregate;
4. serialize one compact record and append it in memory;
5. write the complete new history to a nonce-named sibling temporary; and
6. atomically rename the temporary over the journal.

The logical data model is append-only because accepted old records are never
changed or removed.

Filesystem visibility is atomic because readers see either the old complete
history or the new complete history.

The plugin event loop is the only journal writer, so no additional process lock
is required in this ticket.

This is the chosen option.

## Journal location and ownership

The production path is `/host/.lisa/completion-journal.jsonl` inside WASI,
corresponding to `.lisa/completion-journal.jsonl` in the project.

It is separate from `.lisa/provenance.jsonl`.

Like provenance, it is durable repository-visible evidence rather than an
ignored signal/session artifact.

The journal module creates the parent directory if necessary before the first
publication.

Unique nonce temporaries prevent stale failed attempts from sharing one
temporary name.

## Record schema

Every line has required `schema_version: 1` and a tagged `state`.

Every transition carries the generation key as component fields:

- `completion_id`;
- `attempt_id`;
- `generation`.

Storing components avoids adding a parser to CompletionGenerationId and avoids
depending on reversible interpretation of Display at the persistence boundary.

Requested additionally records:

- prior ticket Phase; and
- prior TicketStatus.

Those values let restart-time DAG construction mask Done frontmatter exactly as
the live PendingCompletion record does.

CommandInFlight records the exact `correlation_id`.

Confirmed records the matching correlation and verified `commit_id`.

Rejected is also represented with correlation when present, reason text, and
retryability.

Rejected is not the headline acceptance state, but retaining it prevents a
failed command from reconstructing forever as in-flight and preserves the
existing retry behavior.

The record enum is internal to the journal module and uses compact JSONL.

No provenance schema version changes.

## Reconstructed aggregate

The journal fold returns a map keyed by completion/ticket ID.

Each aggregate contains:

- typed CompletionGenerationId;
- typed CompletionState;
- prior Phase and TicketStatus;
- optional confirmed commit ID.

Requested reconstructs exactly as CompletionState::Requested.

CommandInFlight reconstructs with its exact CorrelationId.

Confirmed reconstructs as CompletionState::Confirmed and retains the commit
result beside it.

Rejected reconstructs as a LaunchFailed CompletionRejection with the recorded
retryability and stable reason text.

The aggregate uses core domain values after the JSON adapter boundary; scheduler
code does not parse state strings itself.

## Fold validation

Use `completion::reduce` to validate state/event legality where possible.

Requested folds a Request event from Eligible or retryable Rejected.

CommandInFlight requires the same generation key and folds CommandLaunched.

Confirmed requires the same key and folds a matching CommandSucceeded.

Rejected from Requested folds CommandLaunchFailed.

Rejected from CommandInFlight folds a correlation-matched CommandFailed.

Key changes are accepted only when the core state allows another Request.

Confirmed cannot be followed by another request or result.

Unknown schema versions, malformed JSON, empty interior lines, missing final
newline, invalid sequences, key mismatches, and correlation mismatches fail
closed with line-specific errors.

Repeated identical transitions are not silently duplicated; the reducer must
accept every appended fact.

## State integration

Add a completion journal path to State.

Add a reconstructed aggregate map to State.

Add a journal-health flag.

An empty path retains the existing no-I/O behavior for unrelated native tests.

Production load sets the path and reconstructs before building the initial DAG.

Successful append updates the in-memory aggregate with the exact state returned
by the validated fold.

`completion_state` consults the durable aggregate first, then the legacy
pending/DAG facts used by no-journal tests and migration-free startup.

If journal load fails, the plugin logs an operator-visible error and marks the
journal unhealthy.

An unhealthy production journal refuses new completion transitions rather than
launching from ambiguous state.

## Restart-time DAG masking

Load journal state before converting scanned tickets into a DAG.

For Requested or CommandInFlight aggregates, replace scanned phase/status with
the journaled prior values.

This is the restart equivalent of `rebuild_dag` masking through
PendingCompletion.

Confirmed does not mask durable Done.

Retryable Rejected does not need to mask a transaction that has ended.

The normal `rebuild_dag` path uses PendingCompletion first and falls back to a
blocking durable aggregate when no live pending record exists.

This prevents a command that wrote Done immediately before plugin failure from
releasing dependencies on restart before its result is reconciled.

## Request and launch ordering

All existing authority, dependency, ticket-path, and command-build validation
runs before journal acceptance.

Journal Requested before making the host effect externally visible.

Journal CommandInFlight with the generation-derived correlation before calling
the Zellij run-command host API.

Then insert the enriched PendingCompletion and invoke the host command.

Recording CommandInFlight immediately before the void host call creates a
small crash window where replay may find no actual process.

That direction is safe because keyed replay is idempotent and the next ticket
owns bounded reconciliation; recording after the host call would create the
more dangerous untracked-command window.

PendingCompletion gains the exact generation key and correlation so result
handling never recomputes identity from weaker ticket-only context.

## Failure ordering

A command result failure journals retryable Rejected before removing pending
masking state.

If rejection persistence fails, retain pending state and continue masking Done.

This is fail-closed and operator-visible.

A stale-authority result is recorded as a rejected old aggregate before its
live pending entry is removed.

The current attempt remains governed by its own lease checks.

Build failures occur before Requested is written where possible.

A publication failure never invokes the completion command.

## Confirmation ordering

On apparent command success, temporarily remove live pending state and rebuild
the DAG to inspect actual durable ticket bytes.

Require valid commit ID output and durable phase/status Done.

Journal Confirmed with the exact correlation and commit ID.

Only after that atomic publication succeeds may the adapter:

- leave pending removed;
- log phase completion;
- mark the thread complete;
- emit authoritative Done provenance;
- release the seat; and
- schedule dependents.

If confirmation persistence fails, restore PendingCompletion and rebuild the
masked DAG.

This ordering makes provenance backward-compatible and causally downstream of
the new journal rather than part of its schema.

## Simulated restart test

Add focused journal-module tests for each accepted state and malformed input.

The main acceptance regression uses a temporary plugin State with a real
ticket path, current attempt lease, configured command builder, and journal
path.

Drive the production completion adapter to Requested and CommandInFlight.

Read the journal into a fresh State through the same restoration helper used by
load.

Assert the fresh aggregate equals the pre-restart typed aggregate, including
generation key, correlation, and prior mask values.

Write durable Done frontmatter and deliver a correlated successful command
result to the original state.

Restore another fresh State and assert Confirmed plus the exact commit ID.

Assert the JSONL history contains one Requested, one CommandInFlight, and one
Confirmed record and no temporary residue.

Existing provenance compatibility tests remain unchanged and passing; the
acceptance test can additionally assert the emitted execution record still
parses through ProvenanceLedgerRecord.

## Risks and limits

Whole-history replacement is O(journal size) per transition.

Completion transitions are low-volume and bounded to a few records per attempt,
so the simplicity and atomicity outweigh premature compaction.

Two concurrently running plugin processes could race read-modify-publish and
lose a record.

Lisa's supported runtime has one scheduler plugin event-loop writer; adding a
cross-process journal lock is outside this ticket.

The journal does not reconstruct threads, panes, or current attempt leases.

It reconstructs completion truth and masking inputs; the next ticket owns
bounded replay and convergence once authority is available.

## Decision

Create a plugin-local, versioned completion journal module; atomically publish
validated append-only JSONL histories through `publication.rs`; reconstruct
typed aggregate state before initial DAG construction; and make adapter launch,
result, provenance, and seat-release ordering conditional on durable journal
transitions.
