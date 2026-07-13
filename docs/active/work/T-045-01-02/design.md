# Design — T-045-01-02 claim command surface

## Decision target

The command must let an agent present the exact ticket, attempt, and nonce it was
assigned, reject clearly invalid assertions before publication, and leave typed,
atomic evidence that the scheduler can authoritatively admit in T-045-03-01.

The design must preserve the E-034 lease type and authority registry.
It must build on T-045-01-01's nonce-bearing assignment filename.
It must be testable through the real `lisa` binary without Codex, Zellij, or a live
plugin process.

## Option 1 — print success after parsing arguments

The smallest command could accept `--ticket-id`, `--attempt-id`, and `--nonce`, parse
them, and return exit status 0.

Advantages:

- minimal code;
- stable CLI syntax;
- no filesystem side effects.

Disadvantages:

- does not consult any E-034 lease evidence;
- accepts prior attempts and arbitrary nonces;
- leaves nothing for the scheduler to consume;
- turns a self-assertion into apparent ownership without validation.

Decision: rejected because it does not satisfy either lease binding or rejection
coverage.

## Option 2 — trust inherited ticket and attempt environment

The command could compare arguments with `LISA_TICKET_ID` and `LISA_ATTEMPT_ID`, then
write a claim.

Advantages:

- values already exist on fresh launches;
- no marker read is required;
- command tests can inject the environment easily.

Disadvantages:

- environment belongs to process launch, not current scheduler authority;
- a resident interactive process cannot receive new environment values when its pane
  is reused;
- E-034 introduced the per-pane lease marker specifically to avoid this stale process
  identity problem;
- no environment value identifies the current assignment nonce.

Decision: rejected as incompatible with same-pane reuse and the established lease
transport.

## Option 3 — call the plugin synchronously through a new RPC

The command could invoke a Zellij pipe or another IPC endpoint and ask the plugin to
validate directly against `current_leases` and `assignment_refs`.

Advantages:

- the plugin can make the final authoritative decision immediately;
- no durable marker can be confused with live authority;
- nonce equality can be checked against the exact in-memory reference.

Disadvantages:

- Lisa currently has no plugin pipe/RPC command boundary;
- a synchronous response protocol, timeout, correlation ID, and Zellij CLI coupling
  would all be new;
- command-level tests would require a fake Zellij/plugin transport or a live session;
- the story explicitly keeps fixture proof free of live Zellij;
- it expands this small contract ticket into scheduler integration owned by the next
  story.

Decision: rejected for this slice. The asynchronous signal pattern already exists and
keeps authoritative admission in the scheduler consumer.

## Option 4 — publish a mutable current-assignment sidecar

Assignment preparation could atomically write an additional sidecar containing the
live lease and nonce. The command would compare its arguments with that sidecar.

Advantages:

- the CLI can validate the exact currently retained nonce;
- repeated preparation can replace the sidecar while old immutable files remain;
- no plugin IPC is needed.

Disadvantages:

- it introduces a second publication whose ordering must be coordinated with the
  assignment file and in-memory map;
- crash points can leave the file, sidecar, and retained map at different generations;
- the predecessor ticket intentionally made `AssignmentRef` the exact writer result;
- later scheduler admission still has to compare the claim with in-memory authority;
- it broadens the assignment writer beyond the acceptance criterion.

Decision: rejected. A second durable truth source is unnecessary when the scheduler
will perform final claim admission.

## Option 5 — validate durable lease identity and exact assignment, then signal

Add a hidden plumbing command:

`lisa claim --ticket-id <id> --attempt-id <u64> --nonce <u128>`

The command resolves the project root, reads `LISA_PANE_ID`, and uses it to locate the
scheduler-published `.lisa/signals/pane-{pane}.lease` marker.
It compares the requested ticket/attempt pair with that complete `AttemptLease`.
It then requires the deterministic T-045-01-01 path to be a regular file:

`.lisa/attempts/{ticket}/{attempt}/work/assignment-{attempt}-{nonce}.md`

After validation, it atomically publishes a typed claim payload to:

`.lisa/signals/pane-{pane}.claim`

Advantages:

- uses the E-034 marker channel built for native process identity;
- rejects a prior attempt even if its old assignment file remains;
- rejects an arbitrary nonce when its exact assignment was never published;
- produces scheduler-consumable evidence without changing scheduler state here;
- matches existing native signal transport and atomic-publication patterns;
- supports black-box command tests with only a temporary directory and environment;
- leaves final `current_leases` and `assignment_refs` equality checks to the plugin.

Costs and limits:

- the durable marker is transport evidence, not the final authority registry;
- an old nonce-bearing file can remain after repeated preparation in one attempt;
- the later scheduler consumer must reject any claim that does not equal its retained
  live reference;
- revocation cleanup remains a later ticket responsibility.

Decision: selected. It is the smallest command boundary that performs useful local
rejection, binds to the E-034 identity channel, and emits evidence fit for final
scheduler admission.

## Shared claim contract

Create `lisa_core::claim` so the CLI producer and later plugin consumer use the same
wire types.

`AssignmentClaim` contains:

- `ticket_id: TicketId`;
- `attempt_id: u64`;
- `nonce: u128`.

It intentionally omits pane ID.
The pane is the trusted routing component of the strict
`pane-{u32}.claim` filename, consistent with other signal families.
The payload is JSON and derives exact equality for tests and later admission.

The core module also owns `assignment_file_name(attempt_id, nonce)`.
Both the plugin writer and CLI validator call this helper, preventing the two halves
of S-045-01 from drifting on punctuation or numeric formatting.

## Named rejection reasons

Define a typed `ClaimRejection` enum with a stable `name()` value and descriptive
Display text.
The relevant reasons are:

- `pane-unavailable`: no valid `LISA_PANE_ID` identifies the lease marker;
- `lease-unavailable`: the pane marker cannot be read as a regular file;
- `invalid-lease`: the marker is not valid `AttemptLease` JSON;
- `wrong-ticket`: the marker belongs to another ticket;
- `stale-attempt`: the requested attempt is lower than the held attempt;
- `attempt-mismatch`: the requested attempt is higher or otherwise non-current;
- `wrong-nonce`: no exact durable assignment file exists for the requested nonce;
- `lease-changed`: the marker changed while the command was validating.

Operational publication failures are command errors rather than semantic claim
rejections, but their messages still identify the failed path and operation.
The CLI renders a semantic failure as:

`claim rejected [<stable-name>]: <description>`

Tests and future callers can key on the bracketed name without parsing prose.

## Validation order

The command validates in this order:

1. parse `LISA_PANE_ID` as `u32`;
2. require the exact pane lease marker to be a regular file;
3. deserialize its full `AttemptLease`;
4. compare ticket identity;
5. compare attempt identity, distinguishing prior from non-current future values;
6. derive the exact assignment path with the shared filename helper;
7. require that path to be a regular file;
8. reread the pane marker and require it to equal the first observed lease;
9. serialize and atomically publish the claim signal;
10. print a concise accepted identity and exit 0.

Ticket and attempt checks precede nonce checks deliberately.
A prior attempt may still have a valid immutable assignment file, but the command must
name its lease failure rather than misleadingly call its nonce wrong.

The second marker read detects replacement during validation.
It narrows the producer-side race but does not replace final scheduler admission.
The plugin remains responsible for checking the claim after acquisition against
`current_leases`, the pane's stamped lease, and `assignment_refs`.

## Atomic claim publication

The command creates `.lisa/signals` when necessary.
It writes complete JSON to a hidden same-directory temporary containing pane ID,
process ID, and a time nonce.
It renames that temporary over `pane-{pane}.claim` only after the full write succeeds.
If rename fails, it removes the temporary and reports an operational error.

The scheduler will therefore see the previous complete claim or the new complete
claim, never torn JSON.
Overwriting a previous pane claim is acceptable because claims are keyed to the
currently routed physical seat, and final admission is identity-gated.

## Success output

On success, stdout contains one stable, human-readable line naming ticket, attempt,
and nonce.
Stderr remains empty.
The output is confirmation for the calling agent; the durable JSON signal is the
machine-consumed evidence.

On rejection, stdout remains empty, stderr carries the named reason through the
standard `Error: ...` wrapper, and exit status is nonzero.

## Command-level test design

Add one black-box integration test module invoking `CARGO_BIN_EXE_lisa`.
Each fixture uses a temporary project root and sets `LISA_PANE_ID`.

The accepted case creates:

- a matching pane lease marker;
- the exact nonce-bearing assignment file.

It asserts exit 0, exact stdout, empty stderr, no temporary residue, and a `.claim`
payload equal to `AssignmentClaim`.

The stale case creates both current and predecessor assignment files, leaves the pane
marker at the current attempt, and invokes the command for the predecessor.
It asserts nonzero exit, empty stdout, the `stale-attempt` reason, and no claim file.
Keeping the stale assignment present proves lease validation causes the rejection.

The wrong-nonce case uses the matching current lease and one valid assignment nonce,
then invokes another nonce.
It asserts nonzero exit, the `wrong-nonce` reason, and no claim file.

The existing help-surface regression is updated to count and categorize `claim` as a
hidden plumbing command and to pin its curated footer text.

## Rejected scope

This ticket does not:

- add `SignalRequest::Claims` to the plugin;
- transition `SeatAssignmentState` to Owned;
- change Codex or Claude launch commands;
- inject claim syntax into assignment text;
- define delivered-awaiting-claim retry policy;
- delete old assignment files;
- revoke a nonce at completion;
- add dashboard output;
- run a live Zellij or Codex process;
- change `AttemptLease` serialization or minting.
