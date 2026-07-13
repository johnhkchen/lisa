# Research: attempt transition/failure provenance schema

## Assignment boundary

T-040-02-01 owns the shared schema only.
The ticket asks for a new attempt-scoped pre-ownership assignment-transition
or failure row, a schema-version bump, and compatibility tests.
The downstream writer is T-040-02-02.
The downstream CLI reader is T-040-02-03.
Both depend directly on this ticket, so the public field shape established here
is their compile-time and serialized-data contract.

## Existing schema owner

`crates/lisa-core/src/provenance.rs` owns the provenance schema.
It is exported through `lisa_core::provenance` by `lisa-core/src/lib.rs`.
The module contains schema types, time conversion, usage extraction, and JSONL
append I/O.
It intentionally has no scheduler knowledge.
The plugin decides when a record is emitted and supplies all field values.
That separation permits both the plugin and CLI to share schema types without
creating a dependency on the WASM plugin.

`SCHEMA_VERSION` is currently `2`.
The constant is stamped onto every newly created terminal execution record.
The version was last bumped when attempt authority fields were added.
The current module documentation describes one terminal execution record per
attempt and therefore does not yet describe the pre-ownership case.

## Existing terminal execution row

`ProvenanceRecord` is the existing public row type.
It derives `Serialize` and `Deserialize` directly.
Its required fields are:

- `schema_version`;
- top-level `ticket_id`;
- nested `attempt_lease`;
- terminal `outcome`;
- `authoritative` and `fenced` flags;
- requested and actual routes;
- start, end, and wall-clock timestamps;
- nullable token and cost fields;
- concurrency at spawn;
- pane id.

`AttemptLease` is the provider-neutral attempt identity.
It contains `ticket_id` and a numeric `attempt_id`.
The existing tests require the top-level ticket and lease ticket to agree for
records written by the append path, although deserialization itself does not
run semantic validation.

`RunOutcome` is limited to `done`, `failed`, and `timed-out`.
Those values describe execution teardown after a run exists.
They do not identify a pre-ownership scheduler state or explain why provider
ownership was never established.

`Route` holds method, provider, and optional model.
The new assignment evidence requires provider identity but does not represent a
completed route with requested/actual execution semantics.
`AgentClient` is the shared provider/client vocabulary and serializes as
`claude` or `codex`.

## Existing JSONL behavior

`append_record` accepts only `&ProvenanceRecord`.
It creates parent directories, serializes compact JSON, adds one newline, and
opens the ledger in create-and-append mode.
It never rewrites existing rows.
The append tests verify two records, cross-ticket attempt attribution, hostile
path handling, and preservation after an I/O failure.

The ticket does not ask to wire the new row into the plugin.
It also does not require a generalized append function for the downstream
writer, though any serializable row can follow the same single-line contract.
Changing the established `append_record` signature would affect existing plugin
call sites outside this ticket's schema-only scope.

## Existing readers

The plugin's provenance unit tests define a local `read_ledger` helper returning
`Vec<ProvenanceRecord>`.
Those tests currently operate on ledgers containing only execution records.
No production mixed-ledger reader exists yet.
T-040-02-03 will introduce the CLI reconstruction reader.

Backward compatibility therefore has two relevant meanings.
First, a literal schema-v2 execution JSON object must still deserialize into
`ProvenanceRecord` without new required fields.
Second, a shared mixed-row representation must be able to deserialize that same
v2 object from a ledger that may also contain new assignment rows.

Adding required assignment fields directly to `ProvenanceRecord` would violate
the first meaning.
Adding optional fields would blur two lifecycle events and leave downstream
readers to infer a row kind from null combinations.
A separate row type preserves the established execution representation.

## Scheduler state vocabulary

The pre-ownership state machine currently lives privately in
`crates/lisa-plugin/src/lib.rs` as `SeatAssignmentState`.
Relevant successful/intermediate states include `Starting`,
`ReadyForAssignment`, `Delivering`, `AssignedPendingAck`, `Recovering`, and
`Owned`.
Named terminal states include `DeliveryFailed`, `RecoveryFailed`, and
`StartupFailed`.

The terminal mutators are `fail_assignment_delivery`,
`fail_assignment_recovery`, `fail_startup`, and a related
`fail_startup_recovery` path.
They retain the reservation, fail the thread when present, add an error alert,
and log an activity event.
They currently append no durable provenance row.

The scheduler enum is private, carries deadlines and retry counters on some
variants, and is scheduling authority.
Moving or reusing it in `lisa-core` would couple durable evidence to mutable
scheduler internals.
The ledger instead needs a stable named state suitable for storage and display.

## Timing information

Existing execution records use UTC epoch seconds and expose `started_at`,
`ended_at`, and `wall_clock_secs`.
The core helper `system_time_to_epoch` already defines conversion and saturates
pre-epoch values to zero.
Assignment slots track `transition_started_at`, while failure methods receive
the terminal observation time implicitly.
Thus downstream wiring can supply a transition start and transition end without
inventing a new time unit.

## Repository and workflow constraints

The working tree already contains Lisa-owned modifications to the active ticket
files.
They are outside this ticket's source ownership and must remain untouched.
Phase artifacts belong only under the private attempt work directory.
Ticket source changes must be committed using `lisa commit-ticket` with exact
repository-relative include paths.
Ordinary Git staging and commits are forbidden for this assignment.

## Test conventions

The provenance module keeps unit tests beside the schema.
Tests use literal JSON, `serde_json::to_string`, `from_str`, and temporary files.
The current compact-line test asserts the literal schema version, required
fields, absence of newlines, and equality after round trip.
The acceptance criteria call for equivalent round-trip coverage for the new row
and explicit forward/backward compatibility with an existing v2 row in the same
ledger.

## Scope constraints

This ticket does not modify scheduler retry behavior.
It does not choose failure emission sites.
It does not append live rows.
It does not add the CLI output surface.
It must provide enough public schema vocabulary that those two consumers can be
implemented independently without editing this file concurrently.
