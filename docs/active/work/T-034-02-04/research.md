# Research: T-034-02-04 one authoritative provenance record

## Ticket boundary

- The ticket starts in Research and follows T-034-02-03.
- Its acceptance criterion combines two observable properties.
- Timeout and fencing history must remain attributable to the attempt that ran.
- Exactly one provenance row may represent the ticket's authoritative Done.
- That row must carry the lease of the winning attempt.
- A predecessor's late completion result must not become authoritative.
- Ticket phase and status frontmatter are managed by Lisa, not this work pass.

## Attempt identity

- `lisa_core::types::AttemptLease` is the shared execution identity.
- It contains `ticket_id` and a strictly increasing `attempt_id`.
- `AttemptLease::mint` creates attempt one or a checked successor.
- A predecessor for another ticket is rejected.
- Attempt ID overflow is rejected.
- `AttemptLease::is_current` compares the complete value with optional authority.
- Absence from `State::current_leases` rejects every candidate.
- `State::lease_high_water` survives revocation and supports monotonic redispatch.
- `State::current_leases` contains only presently authorized attempts.
- Normal dispatch stamps the same lease on the logical `Thread` and physical slot.
- Production threads therefore already retain the identity needed by provenance.

## Revocation and fencing

- `State::revoke_current_lease` removes current authority without deleting high-water.
- `State::revoke_and_fence_attempt` revokes before touching the pane.
- It finds the assigned slot, marks its transition state `Fenced`, and closes the pane.
- A fenced slot is permanently ineligible for reuse.
- `release_slot_for_ticket` also revokes defensively before exposing the ticket.
- Native tests retain a test-only lifecycle trace of revoke, fence, and release order.
- Session/per-phase timeout performs fail, provenance, revoke/fence, release, remove.
- Hard-silence stale detection performs the same sequence with `Failed` outcome.
- The current sequence writes provenance before it knows the concrete fence result.
- Provenance therefore records `timed-out` or `failed`, but not whether fencing occurred.
- The record also lacks the revoked attempt lease, so retry rows share only ticket ID.

## Provenance schema

- `crates/lisa-core/src/provenance.rs` owns the ledger record and append helper.
- The ledger is `.lisa/provenance.jsonl`.
- It is append-only and repository-visible.
- `SCHEMA_VERSION` is currently 1.
- `ProvenanceRecord` stores ticket, outcome, route, timing, usage, concurrency, and pane.
- It does not store an `AttemptLease`.
- It does not store a fence result.
- It does not distinguish attempt history from the authoritative ticket outcome.
- `RunOutcome` has `Done`, `Failed`, and `TimedOut`.
- JSON names are `done`, `failed`, and `timed-out`.
- `append_record` creates parent directories and appends one compact JSON line.
- It never rewrites or deduplicates existing history.
- The runtime never reads the ledger during scheduling.
- Unit tests deserialize rows with the current `ProvenanceRecord` type.

## Provenance publisher

- `State::emit_provenance` is the only plugin ledger publisher.
- It is private to the scheduler implementation.
- It returns early when the ledger path is unset.
- It returns early when the ticket has no active thread.
- It reads route, start time, pane, and concurrency from the thread.
- It reads provider usage from ticket-keyed usage artifacts.
- It constructs a schema-v1 record and appends it.
- Append errors are logged but never terminate the scheduler.
- The method currently accepts only ticket ID and `RunOutcome`.
- It does not inspect `thread.attempt_lease`.
- It does not inspect `State::current_leases`.
- It cannot reject a stale Done publication by itself.

## Publisher call sites

- Verified completion calls `emit_provenance(..., Done)`.
- Error-signal reclamation calls it with `Failed`.
- Session/per-phase timeout calls it with `TimedOut`.
- Hard-silence reclamation calls it with `Failed`.
- All teardown callers invoke it before removing the thread.
- The thread stamp is consequently still available at every production call.
- Error-signal failure releases without fencing the resident pane.
- Timeout and hard-silence reclamation fence the pane.
- The two kinds of `Failed` row are currently indistinguishable by fence behavior.

## Completion admission

- `State::request_completion` is the single completion request boundary.
- It rejects a request when another completion for the ticket is pending.
- Attempt authority must exactly match `current_leases[ticket]`.
- Operator authority is allowed only for a Manual source without an active thread.
- Dependencies must be Done.
- Ticket path and pre-completion state are captured in `PendingCompletion`.
- `PendingCompletion` retains source and `CompletionAuthority`.
- Attempt-originated pending records therefore preserve the winning lease candidate.
- The native `complete-ticket` command performs the isolated completion commit.
- The scheduler masks externally written Done while the command is pending.

## Completion result publication

- `handle_completion_result` is the only successful completion publisher.
- Missing pending state makes duplicate or unsolicited command results no-ops.
- A failed command removes pending state and leaves the ticket recoverable.
- A successful result must contain a 40- or 64-character hexadecimal commit ID.
- The scheduler then verifies durable Done frontmatter.
- Only after verification does it complete the thread and emit Done provenance.
- It then releases the slot, removes the thread, and schedules dependents.
- This ordering implements T-031's commit-gated completion guarantee.
- The publisher logs the pending authority but does not revalidate it.
- A command callback is asynchronous relative to scheduler timer processing.
- A thread can currently reach session timeout while its completion is pending.
- Timeout reclamation does not exclude `pending_completions`.
- It can revoke attempt N, remove its thread, and dispatch attempt N+1.
- The earlier command result can then arrive with pending authority N.
- The result handler currently accepts that stale pending record if commit output is valid.
- This is the remaining late-result authority gap.

## Existing duplicate defenses

- A ticket has at most one `PendingCompletion` entry.
- A successful handler removes that entry before publishing provenance.
- A repeated callback finds no pending record and returns.
- Existing regression coverage proves duplicate command results append one Done row.
- Existing stale-request coverage proves attempt N cannot request after N+1 is current.
- Neither test covers revocation between request admission and result publication.
- Direct calls to `emit_provenance` in provenance tests can append repeated Done rows.
- Production Done has only the verified-result call site.

## Usage and route attribution constraints

- Usage artifact names remain ticket-scoped, not attempt-scoped.
- This ticket does not own usage capture or staging.
- Route and spawn facts are already snapshotted per `Thread`.
- Attempt identity can be added without changing adapter protocols.
- Fencing is a scheduler fact and should not be inferred downstream from outcome text.
- The append-only ledger must preserve predecessor rows rather than rewrite them.
- Schema evolution is explicitly supported through `schema_version`.

## Documentation and tests

- `docs/knowledge/provenance-ledger.md` documents schema version 1 and field semantics.
- Its example and field table omit attempts, fencing, and authority.
- Core tests cover serialization, enum names, usage extraction, and append behavior.
- Plugin tests cover all four provenance call patterns and completion deduplication.
- The timeout test already constructs and stamps a real attempt lease.
- Several older provenance fixtures construct unleased threads directly.
- Those fixtures are test compatibility artifacts, not production dispatch behavior.
- Lease-attributed schema enforcement will require those fixtures to install leases.

## Constraints surfaced by the code

- Append history must remain append-only.
- Timeout/failure rows must not be suppressed merely because authority was revoked later.
- Done is different: it is a ticket-level publication and requires current authority.
- Fence state must be captured after the fencing operation has produced a result.
- The thread must remain present until the record is built.
- Completion timeout and callback processing must not create two current candidates.
- The isolated commit transaction should remain unchanged.
- Ordinary Git staging must not be used for this ticket.
- Ticket-owned source changes must be committed through `lisa commit-ticket`.

## Research conclusion

- The necessary data already exists on `Thread`, `PendingCompletion`, and fence results.
- The current schema discards that data at the ledger boundary.
- The current Done request gate is strong at admission but not at callback publication.
- The current pending state already supplies a natural lease-stability boundary.
- The implementation surface is limited to the provenance schema, scheduler publisher,
  timeout/stale teardown ordering, focused tests, and ledger documentation.
