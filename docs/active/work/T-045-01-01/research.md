# Research — T-045-01-01 atomic assignment file writer

## Ticket boundary

T-045-01-01 asks for the assignment bytes for one ticket execution to be published
atomically as an attempt-tagged, nonce-bearing file.
The observable contract is that a launcher receives a durable path whose contents are
complete, or receives no path at all.
The ticket is the first half of S-045-01.
T-045-01-02 will add a claim command using the same ticket, attempt, and nonce identity.
Later stories own launcher argv construction, Zellij injection, scheduler ownership
transitions, ticket-boundary teardown, and live Codex/Zellij validation.
Those later behaviors are not part of this source change.

## Existing assignment construction

`crates/lisa-plugin/src/adapter.rs` defines provider adapters.
`SpawnContext` carries ticket ID, pane ID, attempt ID, artifact directory, and an
optional assignment acknowledgement generation.
Each adapter can build the complete assignment text.
Each adapter can also build a bounded reference prompt to an assignment path.
The reference API accepts a `Path`, so it does not require the filename to remain
`assignment.md`.

`crates/lisa-plugin/src/lib.rs` owns scheduler orchestration.
`ticket_prompt` constructs the workflow instructions.
`State::prepare_assignment` currently creates the attempt work directory and writes
the complete text through the common atomic publication helper.
The current durable destination is always `assignment.md`.
The current temporary is `.assignment.md.tmp.{wall-clock-nanosecond}`.
The function returns the destination `PathBuf`.

Assignment preparation occurs at three production boundaries:

1. ordinary dispatch after a lease is minted;
2. same-pane startup recovery after a successor lease is selected;
3. post-exit relaunch/recovery for a pane that retains a ticket reservation.

All three call sites currently discard the returned path.
Later delivery reconstructs the path by joining the attempt work directory with the
constant `assignment.md`.
That reconstruction works only because the current durable filename is deterministic.

## Existing attempt identity

`lisa_core::types::AttemptLease` contains `ticket_id` and positive `attempt_id`.
It derives clone, equality, and hash traits.
The scheduler holds current leases in `State::current_leases`, keyed by ticket ID.
`AttemptLease::is_current` compares the complete ticket/attempt pair with current
authority.
The attempt-private work directory is
`.lisa/attempts/{ticket_id}/{attempt_id}/work` in production.
Therefore the full assignment path already sits below both ticket and attempt path
components, but its leaf does not expose or bind a nonce.

Lease authority and nonce identity are distinct concepts.
The ticket explicitly says not to alter E-034 lease fencing.
Changing `AttemptLease` to contain a nonce would alter that established authority
type and affect serialization, signals, completion, and tests across the workspace.
No current core type represents an assignment nonce.

## Existing atomic publication mechanism

`crates/lisa-plugin/src/publication.rs` centralizes sibling-temporary publication.
`PublicationPath` contains the durable destination and a typed temporary naming policy.
`TemporaryName::Nonce` appends a wall-clock nanosecond value.
`RustPublication::publish` resolves the temporary beside the destination, writes the
complete byte slice, and renames the temporary over the destination.
If rename fails, it removes the temporary and returns an operator-oriented error.
Successful publication returns the destination path.

Because the temporary and destination share a directory, the rename stays on one
filesystem.
Readers cannot observe a partially written destination through this helper.
A failed write occurs only at the temporary path.
A failed rename does not publish partial bytes.
The helper already has tests for successful replacement, residue cleanup, and failed
publication preserving prior durable state.

The helper's nonce generator is private to `publication.rs`.
It uses `SystemTime::now()` relative to UNIX epoch and returns nanoseconds as `u128`.
This is already the plugin's established opaque publication nonce convention.
The plugin is a single scheduler process, and existing publication sites use this
convention for launch files, lease markers, and assignment temporaries.

## Existing tests

Large native tests live in `crates/lisa-plugin/src/lib.rs`.
`test_prepare_assignment_atomically_preserves_complete_hostile_payload` verifies that
the current writer round-trips a large hostile UTF-8 assignment and leaves no temp
residue.
The combined publication test verifies replacement of an existing assignment.
The failure-path test makes the destination a directory and verifies that rename
failure is reported and temporary residue is removed.

Those tests establish the generic atomic helper behavior, but they do not establish
the new assignment identity:

- the writer takes no lease;
- the durable filename has no attempt tag;
- no durable assignment nonce is returned;
- scheduler state does not retain the exact returned reference;
- delivery reconstructs a shared filename.

The plugin crate already has `tempfile` as a dev dependency.
No new dependency is required for unit coverage.

## State and delivery boundary

`State::seat_assignments` is keyed by physical pane and represents lifecycle status.
It is deliberately about readiness and ownership, not the assignment file itself.
`State::current_leases` is keyed by ticket and is the authority check.
There is currently no state collection for an exact assignment file reference.

`deliver_assignment_to_pane` first checks human-input state, pane reservation, exact
generation, and current lease authority.
It then reconstructs `assignment.md`, requires that path to be a file, strips the
host prefix, and passes it to the provider adapter.
This is the one reader that must consume an exact nonce-bearing reference after the
writer changes.

The scheduler can prepare an assignment before delivery and can deliver later after a
provider startup signal or grace interval.
Consequently a random durable filename cannot remain only in a local return value.
It must be retained across scheduler events.
Looking up files by glob or directory order would make identity ambiguous after a
recovery or repeated preparation.

## Path and payload constraints

Ticket IDs are directory components established elsewhere by Lisa.
Attempt IDs are positive integers.
The nonce convention is numeric and does not require shell escaping in a leaf name.
The complete path can still contain hostile characters inherited from a project root.
Assignment contents can contain quotes, dollar syntax, backticks, escapes, newlines,
and large payloads.
The Rust writer treats contents as bytes and does not invoke a shell.
Later launcher work must preserve the path as one argv element, but that is outside
this ticket.

WASM production builds use WASI filesystem APIs through `std::fs`.
The existing writer and publication helper already compile on that target.
Keeping the change within those APIs avoids a new portability surface.

## Repository and workflow constraints

The working tree contains unrelated Lisa protocol, epic, story, and ticket files.
They are not owned by this ticket and must not be staged or committed by its isolated
transaction.
Phase artifacts belong only in
`.lisa/attempts/T-045-01-01/1/work`.
Source changes must be committed with `lisa commit-ticket` and exact include paths.
The ticket frontmatter is scheduler-owned and must not be edited.

## Facts that constrain the implementation

- Atomic publication already exists and should be reused.
- The assignment reference needs ticket, attempt, nonce, and exact path identity.
- Lease fencing must remain unchanged.
- A nonce-bearing destination cannot be reconstructed from a constant.
- Scheduler state must retain the writer's exact result until delivery.
- The adapter already accepts an arbitrary assignment path.
- The source scope can remain in the plugin crate.
- No CLI command or ownership transition belongs to this ticket.
- No new crate dependency is needed.
- Unit tests can verify success and interrupted publication with temporary directories.

## Open observations

The current nanosecond nonce is uniqueness-oriented rather than cryptographic.
The ticket calls for a nonce but does not require secrecy or cryptographic entropy.
The subsequent claim contract may treat it as an opaque equality token.
The current helper's convention is therefore relevant established behavior.

Repeated preparation for one lease can create more than one immutable assignment file.
Only the reference retained by scheduler state is live.
Old attempt-private files are ignored and remain inside Lisa's ignored attempt tree.
Later revocation work owns explicit nonce invalidation and cleanup policy.

The current tests are embedded in a very large `lib.rs` test module.
A focused assignment module would isolate the file identity and atomic-write contract,
while leaving scheduler wiring in `lib.rs`.
That is consistent with existing small modules such as `publication.rs` and
`codex_ack.rs`.
