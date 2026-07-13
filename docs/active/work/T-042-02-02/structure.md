# Structure: Durable completion journal

## Change inventory

Create one production source file:

- `crates/lisa-plugin/src/completion_journal.rs`

Modify one production integration file:

- `crates/lisa-plugin/src/lib.rs`

Modify no `lisa-core` source.

Modify no CLI source or command contract.

Modify no provenance schema or documentation.

Create RDSPI artifacts only under the assigned attempt-private work directory.

## New module boundary

`completion_journal.rs` owns:

- the JSONL schema;
- serialization and deserialization;
- transition validation;
- aggregate folding;
- complete-history append construction;
- atomic publication through `publication.rs`; and
- reconstruction from disk.

It does not own:

- lease admission;
- dependency checks;
- ticket scanning;
- command execution;
- Zellij context;
- provenance emission; or
- seat release.

Those remain in the State adapter in `lib.rs`.

## Module declaration

Add a private top-level module declaration in `lib.rs` beside publication:

```text
mod completion_journal;
```

Import only the module's adapter-facing types and functions needed by State.

The module imports publication primitives through `super::publication`.

No public crate API is added.

## Durable transition type

Expose a `pub(crate)` enum named `CompletionJournalTransition`.

Variants:

```text
Requested {
    key: CompletionGenerationId,
    prior_phase: Phase,
    prior_status: TicketStatus,
}

CommandInFlight {
    key: CompletionGenerationId,
    correlation: CorrelationId,
}

Rejected {
    key: CompletionGenerationId,
    correlation: Option<CorrelationId>,
    reason: String,
    retryability: Retryability,
}

Confirmed {
    key: CompletionGenerationId,
    correlation: CorrelationId,
    commit_id: String,
}
```

The runtime enum contains typed core identities and is not directly serialized.

Private record types convert it to component JSON fields.

## Record schema types

Define private serde types:

- `JournalRecord` with `schema_version` and flattened transition body;
- `JournalRecordBody`, tagged by `state` and kebab-case values;
- a serde representation for retryability if core Retryability remains
  intentionally non-serializable.

Every record variant repeats:

- completion ID string;
- attempt ID string; and
- numeric generation.

CommandInFlight, Rejected, and Confirmed carry correlation as applicable.

Requested carries prior phase and status.

Confirmed carries commit ID.

Rejected carries stable reason text and retryability.

`SCHEMA_VERSION` is private and initially `1`.

## Reconstructed aggregate type

Expose a cloneable/equatable `pub(crate) CompletionJournalAggregate`.

Fields:

```text
completion_key: CompletionGenerationId
state: CompletionState
prior_phase: Phase
prior_status: TicketStatus
confirmed_commit_id: Option<String>
```

Provide borrowing accessors where `lib.rs` should not mutate fields directly.

Provide a predicate for whether the aggregate masks scanned Done state.

Requested and CommandInFlight return true.

Confirmed and Rejected return false.

The aggregate's completion ID supplies the map key.

## Journal read interface

Expose:

```text
fn load(
    path: &Path,
) -> Result<HashMap<TicketId, CompletionJournalAggregate>, String>
```

Missing file returns an empty map.

Existing file is read as bytes.

A non-empty file must end with newline.

Each non-empty line parses as exactly one JournalRecord.

Unknown schema and invalid sequence errors include the 1-based line number.

The function folds records in file order into the aggregate map.

No input is silently skipped.

## Journal append interface

Expose:

```text
fn append(
    path: &Path,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String>
```

The function reads and validates the existing complete history.

It validates and applies the proposed transition to the reconstructed map.

It serializes one compact JSON object and newline.

It appends those bytes to the prior byte vector in memory.

It creates the parent directory when absent.

It publishes through `RustPublication`.

The PublicationPath destination is the journal path.

The TemporaryName policy is Nonce with a fixed local sibling prefix.

PublicationErrors identify completion-journal temporary write and publication
failures.

On success, return the exact new aggregate for the transition's completion ID.

## Fold implementation

Use one private `apply_transition` function shared by load and append.

For Requested:

- select current aggregate state or Eligible;
- call core reduce with Request;
- require the returned state to be Requested;
- replace aggregate key and prior mask values.

For CommandInFlight:

- require an existing aggregate;
- require exact generation-key equality;
- call reduce with CommandLaunched;
- store returned CommandInFlight state.

For Rejected:

- require exact key equality;
- from Requested, fold CommandLaunchFailed;
- from CommandInFlight, require and fold correlated CommandFailed;
- store returned Rejected state.

For Confirmed:

- require exact key equality;
- fold correlated CommandSucceeded;
- require Confirmed;
- retain the verified commit ID.

Convert every reducer rejection into contextual journal validation text.

## State fields

Extend `State` in `lib.rs` with:

```text
completion_journal_path: PathBuf
completion_journal_healthy: bool
completion_aggregates: HashMap<TicketId, CompletionJournalAggregate>
```

Document that an empty path disables journal I/O only for pre-load native test
fixtures.

Default leaves the path empty and health false.

Production load sets the path then explicitly restores it.

## Pending completion enrichment

Extend `PendingCompletion` with:

```text
completion_key: CompletionGenerationId
correlation: CorrelationId
```

These fields become the sole identity used by result transitions.

Retain prior phase, prior status, source, and authority.

Update direct test literals with explicit identities.

## State journal helpers

Add a private restore helper:

```text
fn restore_completion_journal(&mut self)
```

It loads the aggregate map, marks health true on success, and logs a visible
error plus health false on failure.

Add a private transition helper:

```text
fn journal_completion_transition(
    &mut self,
    transition: CompletionJournalTransition,
) -> Result<(), String>
```

An empty path returns success for legacy unit fixtures.

A non-empty unhealthy journal returns an error without I/O.

Successful append inserts the returned aggregate in the map.

No caller updates the map before durable publication succeeds.

## Aggregate-state integration

Modify `completion_state` to use this precedence:

1. reconstructed aggregate state;
2. live pending map -> Requested for no-journal tests;
3. durable DAG Done -> Confirmed;
4. Eligible.

Return cloned core state because reducer/reconciler consumes owned or borrowed
values at existing boundaries.

This exposes CommandInFlight with its exact reconstructed correlation.

## DAG masking helper

Add a private helper that applies a transaction mask to a scanned Ticket.

Live PendingCompletion values have first precedence.

Otherwise, a durable aggregate whose state masks Done supplies its prior phase
and status.

Call this helper in `rebuild_dag` for every scanned ticket.

Apply equivalent masking to initial load scan results before constructing the
initial DAG.

Do not mutate ticket files.

## Load ordering

In `State::load`:

1. resolve configured host paths;
2. set `.lisa/completion-journal.jsonl`;
3. restore journal state;
4. subscribe/request permissions as today;
5. scan tickets;
6. mask scanned Done using reconstructed aggregates;
7. build diagnostics and DAG;
8. continue existing reconciliation and initialization.

The journal restore does not fabricate threads or leases.

## Effect execution ordering

In `execute_completion_effect` retain every existing validation.

Build command argv/context before durability transitions.

Construct correlation from the exact generation key.

Append Requested with prior phase/status.

Append CommandInFlight with correlation.

If either append fails, log a structured launch rejection and do not invoke the
host command.

After both succeed, insert enriched PendingCompletion.

Record the inert effect for native tests at the same accepted boundary.

Invoke `run_command_with_env_variables_and_cwd` only afterward.

## Result handling ordering

Read key and correlation from PendingCompletion.

For stale authority or failed command result, append Rejected before removing
pending state.

If append fails, retain pending, rebuild masked DAG if necessary, log the
failure, and return.

For success, retain the existing durable Done verification.

Append Confirmed after verification and before completion activity,
provenance, thread completion, release, or scheduling.

If append fails, restore pending and its DAG mask.

Only confirmed publication permits downstream success effects.

## Tests in the new module

Add tests for:

- Requested -> CommandInFlight -> Confirmed reconstruction after each append;
- exact key, correlation, prior values, and commit preservation;
- one compact complete JSON line per transition;
- no sibling temporary residue;
- malformed/torn/unknown-version rejection;
- illegal transition, key mismatch, and correlation mismatch rejection;
- rejected retry state followed by a new Requested transition; and
- hostile but valid destination-directory bytes.

## Plugin acceptance regression

Add a native test in `lib.rs` near completion result tests.

Build a temporary State with real ticket/work paths and current lease.

Configure the completion command builder sufficiently to cross the production
launch boundary without executing a real host command in native tests.

Dispatch one completion and assert journal lines Requested and CommandInFlight.

Create a fresh State, point it at the same journal, call the restoration helper,
and assert exact aggregate equality.

Make ticket frontmatter durably Done and deliver a valid commit-ID result.

Restore a second fresh State and assert Confirmed plus commit ID.

Parse emitted provenance through the existing backward-compatible enum without
adding a new variant.

## Commit units

Commit the standalone journal module first if it compiles independently after
module registration and tests.

Commit adapter integration as the second meaningful unit only after the
concurrent `lib.rs` owner has committed and the path is clean.

Each unit uses `lisa commit-ticket` with exact includes.

No active ticket, provenance data, shared work artifact, attempt-private
artifact, or unrelated plugin docs path is an include.
