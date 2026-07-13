# Structure — T-045-01-02 claim command surface

## Change summary

The implementation is split into a shared identity contract, one focused CLI command
module, the top-level command registration, and black-box CLI coverage.
The existing plugin assignment writer switches to the shared filename helper so the
producer and validator cannot diverge.

No scheduler ownership state changes in this ticket.
No files are deleted.
No dependency or feature changes are required.

## New file: `crates/lisa-core/src/claim.rs`

This module owns provider-neutral assignment claim identity shared by native CLI
producers and later plugin consumers.

### `AssignmentClaim`

Public serializable value with fields:

```text
ticket_id: TicketId
attempt_id: u64
nonce: u128
```

Traits:

- `Debug`;
- `Clone`;
- `PartialEq`;
- `Eq`;
- `Serialize`;
- `Deserialize`.

The serialized JSON field names remain the Rust field names.
The structure contains no pane ID because pane routing comes from the exact signal
filename.
It contains no assignment path because the path is derivable and scheduler state
retains its own exact reference.

### `ClaimRejection`

Public typed enum for semantic claim rejection.
Variants:

```text
PaneUnavailable
LeaseUnavailable
InvalidLease
WrongTicket
StaleAttempt
AttemptMismatch
WrongNonce
LeaseChanged
```

It derives stable equality and error display behavior.
`name(&self) -> &'static str` returns explicit kebab-case identifiers.
Display strings explain the failure without becoming the machine contract.

The enum does not contain filesystem I/O errors.
Those are operational failures in the CLI module, not reasons an otherwise valid
claim identity was denied.

### `assignment_file_name`

Public pure helper:

```text
assignment_file_name(attempt_id: u64, nonce: u128) -> String
```

It returns exactly `assignment-{attempt_id}-{nonce}.md`.
This is the shared bridge between T-045-01-01's writer and this ticket's validator.

### Core tests

Inline unit tests cover:

- exact filename formatting at numeric boundaries;
- every rejection variant's stable name;
- JSON round-trip of a hostile ticket ID and `u128` nonce.

These tests lock the wire contract independently of command filesystem behavior.

## Modified file: `crates/lisa-core/src/lib.rs`

Add:

```text
pub mod claim;
```

No existing re-exports change.
Both `lisa-cli` and `lisa-plugin` already depend on `lisa-core`, so this introduces no
new crate edge.

## Modified file: `crates/lisa-plugin/src/assignment.rs`

Remove the private duplicate `assignment_file_name` helper.
Import `lisa_core::claim::assignment_file_name` alongside `AttemptLease`.

`AssignmentRef`, `write_assignment`, atomic publication, temporary naming, and tests
otherwise remain unchanged.
Existing assignment tests continue to prove that the shared helper produces the
predecessor ticket's exact durable path.

This is a contract alignment change, not scheduler claim consumption.

## New file: `crates/lisa-cli/src/claim.rs`

This module owns command-side filesystem validation and atomic claim publication.
It is private to the binary crate in this ticket.

### `ClaimRequest`

Crate-visible request value with:

```text
project_root: PathBuf
pane_id: Option<String>
claim: AssignmentClaim
```

The main command dispatcher supplies the project root, raw optional
`LISA_PANE_ID`, and parsed explicit claim identity.
Keeping environment acquisition at the edge makes the validator deterministic.

### `ClaimReceipt`

Crate-visible success value with:

```text
claim: AssignmentClaim
signal_path: PathBuf
```

The dispatcher uses the claim fields for success output.
Tests observe the signal path through black-box filesystem effects rather than calling
the module directly.

### `ClaimError`

Private or crate-visible error enum with two categories:

```text
Rejected(ClaimRejection)
Operational(String)
```

Its Display implementation renders rejection as:

```text
claim rejected [reason-name]: reason display
```

Operational messages identify the failed path and action.
The top-level dispatcher adds the repository's standard `Error: ` prefix.

### Validation helpers

Focused private helpers:

- parse the raw pane environment as `u32`;
- derive `.lisa/signals/pane-{pane}.lease`;
- read and deserialize one `AttemptLease`;
- compare the requested claim lease with the observed lease;
- derive the attempt-private assignment path;
- publish serialized claim bytes through sibling temp then rename.

The lease comparison classifies a lower requested attempt as `StaleAttempt` and a
higher requested attempt as `AttemptMismatch`.
Ticket mismatch is checked first.

The assignment path is assembled as:

```text
project_root
  /.lisa/attempts
  /{ticket_id}
  /{attempt_id}
  /work
  /assignment_file_name(attempt_id, nonce)
```

The exact path must satisfy `is_file()`.
Any missing/non-file exact nonce path maps to `WrongNonce`.

The marker is read a second time after assignment validation.
Any missing, malformed, or unequal second value maps to `LeaseChanged`.

### Atomic publisher

Create the signal directory if needed.
Serialize `AssignmentClaim` as compact JSON.
Write to a sibling path shaped like:

```text
.pane-{pane}.claim.tmp.{process-id}-{time-nonce}
```

Rename to:

```text
pane-{pane}.claim
```

On write or rename error, attempt temporary cleanup and return `Operational`.
The durable signal is returned only after successful rename.

## Modified file: `crates/lisa-cli/src/main.rs`

Add private module declaration:

```text
mod claim;
```

Add `lisa_core::claim::AssignmentClaim` import.

Add a hidden plumbing command variant after `CaptureUsage` and before Git transaction
commands:

```text
Claim {
    path: PathBuf = "."
    ticket_id: String
    attempt_id: u64
    nonce: u128
}
```

The command's help text describes exact assignment ownership assertion.
The raw pane ID is not an explicit CLI flag; it comes from `LISA_PANE_ID`, which is
the scheduler-controlled routing context.

The dispatch arm:

1. resolves `path` using the existing helper;
2. builds `AssignmentClaim`;
3. captures optional `LISA_PANE_ID`;
4. invokes the focused command module;
5. prints exact accepted identity on success;
6. prints standard error and exits 1 on rejection or operational failure.

Add `claim` to the curated plumbing footer between usage capture and ticket commits.

## New file: `crates/lisa-cli/tests/claim_cli.rs`

Black-box tests invoke the compiled `lisa` binary.

### Fixture helper

A small local fixture builds:

```text
temporary-root/
  .lisa/signals/pane-7.lease
  .lisa/attempts/{ticket}/{attempt}/work/
```

Helpers write typed lease JSON and exact assignment files.
Commands use `--path <temporary-root>` and set `LISA_PANE_ID=7`.

### Accepted test

Invoke the current ticket/attempt/nonce.
Assert:

- success status;
- exact stdout;
- empty stderr;
- `pane-7.claim` exists;
- its JSON equals the expected `AssignmentClaim`;
- no `.claim.tmp.` file remains.

### Stale-attempt test

Write a current attempt-2 lease marker.
Write both attempt-1 and attempt-2 assignment files.
Invoke attempt 1 with its real old nonce.
Assert:

- failure status;
- empty stdout;
- stderr contains `[stale-attempt]`;
- no claim signal exists.

### Wrong-nonce test

Write a matching current lease and assignment nonce 100.
Invoke nonce 101.
Assert:

- failure status;
- empty stdout;
- stderr contains `[wrong-nonce]`;
- no claim signal exists.

These are command-level tests because all parsing, environment, path derivation,
serialization, exit-code, and publication behavior goes through the built executable.

## Modified file: `crates/lisa-cli/tests/help_surface.rs`

Update the documented regression from 12 to 13 commands.
Add `claim` to:

- the plumbing command list;
- the complete own-command list;
- the exact top-level footer snapshot.

Update comments that currently say four plumbing commands to say five.
Operator command lists and jargon checks remain unchanged.

## File ownership and commit units

Meaningful unit 1 — shared assignment/claim identity:

- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-plugin/src/assignment.rs`.

Meaningful unit 2 — command producer and black-box contract:

- `crates/lisa-cli/src/claim.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/claim_cli.rs`;
- `crates/lisa-cli/tests/help_surface.rs`.

Each unit is committed with one `lisa commit-ticket` invocation and only these exact
paths.
Attempt-local RDSPI artifacts are not included; Lisa publishes them at completion.

## Unchanged boundaries

- `crates/lisa-plugin/src/signal.rs` gains no claim consumer yet.
- `State::current_leases` and `State::assignment_refs` remain unchanged.
- `SeatAssignmentState` gains no new variant or transition.
- launch adapters and assignment text remain unchanged.
- `.lisa/.gitignore` already covers both runtime namespaces.
- no Cargo manifest changes are required.
- no canonical `docs/active/work` path is written directly.
