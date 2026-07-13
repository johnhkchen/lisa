# Research — T-045-01-02 claim command surface

## Ticket boundary

T-045-01-02 is the second and final ticket in S-045-01.
It adds the command an agent invokes to assert ownership of one exact assignment.
The identity named by the ticket has three components:

- ticket ID;
- attempt ID;
- assignment nonce.

The command must accept that identity only while its E-034 attempt lease is held.
The acceptance criterion requires black-box command coverage for one accepted claim,
one stale or prior-attempt claim, and one wrong-nonce claim.
Rejected claims need stable named reasons rather than only free-form failures.

The story explicitly leaves scheduler ownership transitions to T-045-03-01.
It also leaves launcher argv construction, Zellij injection, retry policy, clean TUI
exit, nonce revocation, and real Codex/Zellij validation to later stories.
This ticket therefore establishes transport and validation at the CLI boundary but
does not promote a scheduler seat to Owned.

## Predecessor assignment contract

T-045-01-01 added `crates/lisa-plugin/src/assignment.rs`.
Its `AssignmentRef` contains:

- a complete `AttemptLease`;
- a `u128` assignment nonce;
- the exact durable assignment `PathBuf`.

`write_assignment` accepts an attempt-private work directory, lease, nonce, and
payload bytes.
It publishes the payload under:

`assignment-{attempt_id}-{nonce}.md`

The production containing directory is:

`.lisa/attempts/{ticket_id}/{attempt_id}/work`

The complete path consequently carries ticket, attempt, and nonce identity.
The filename helper is currently private to the plugin assignment module.
No shared core module currently names the assignment/claim wire contract.

The writer returns the reference only after a sibling temporary is renamed to the
durable destination.
A partial temporary is not a durable assignment reference.
The exact result is retained in `State::assignment_refs`, keyed by ticket ID.
Delivery checks the reference lease against current lease authority and uses the
stored path rather than scanning the attempt directory.

Repeated preparation can leave old nonce-bearing files inside the same attempt
directory.
Only the plugin's retained `assignment_refs` entry is live scheduler identity.
Those private files are runtime evidence, not canonical work artifacts.

## E-034 attempt leases

`lisa_core::types::AttemptLease` is the established lease type.
It serializes as JSON with `ticket_id` and positive `attempt_id` fields.
`AttemptLease::mint` creates attempt 1 or the checked successor to a previous lease.
`AttemptLease::is_current` requires exact equality with an optional current lease.
An absent current lease rejects every candidate.

The plugin keeps two maps:

- `current_leases`, containing only presently authorized attempts;
- `lease_high_water`, retaining the latest minted generation after revocation.

The same current lease is stamped onto the logical thread and physical agent slot.
Lease revocation removes the ticket from `current_leases` but deliberately retains
the high-water entry.
The in-memory `current_leases` map is the scheduler's authoritative registry.

## Durable pane lease marker

`State::write_pane_lease_marker` publishes a JSON `AttemptLease` to:

`.lisa/signals/pane-{pane_id}.lease`

The write uses the plugin's `RustPublication` helper.
It writes a same-directory attempt-tagged temporary and atomically renames it over
the durable marker.
Consumers therefore observe complete lease JSON or an older complete marker, never
a partially written marker.

The marker is published before assignment prompt or launch delivery.
Native hooks copy it into heartbeat and process-start evidence.
The marker exists because a resident interactive process cannot receive updated
environment variables when its pane is reused.
The pane ID remains stable and addresses the current scheduler-controlled marker.

The marker is identity transport, while `current_leases` remains authority.
Revoking an in-memory lease does not itself remove the marker.
The scheduler already revalidates copied hook evidence against slot and current
lease state before accepting it.
The later claim consumer can perform the same authoritative revalidation.

For the separate CLI process, the marker is the existing durable E-034 lease value
available without adding an RPC channel.
The current launch command exports `LISA_PANE_ID`, `LISA_TICKET_ID`, and
`LISA_ATTEMPT_ID`.
`LISA_PANE_ID` identifies the marker path.
Ticket and attempt environment values can become stale in a reused resident process,
which is the reason the marker channel exists.

## Signal directory conventions

`.lisa/signals` is the shared native-to-plugin signal directory.
Signal filenames use `pane-{u32}.<suffix>`.
`crates/lisa-plugin/src/signal.rs` recognizes exact suffix families and parses pane
IDs strictly as `u32`.
Lease-bearing signal bodies deserialize through the shared `AttemptLease` type.
Raw provider evidence and presence-only evidence remain different record variants.

Recognized one-shot signals are removed by their consumer after acquisition.
The `.lease` file differs: it is a scheduler-owned durable source marker copied by
hooks and is not consumed as a one-shot event.
There is no `.claim` signal family yet.
There is no typed claim payload in `lisa-core` yet.

The managed `.lisa/.gitignore` already ignores both `signals/` and `attempts/`.
Claim evidence placed under the signal namespace would remain runtime state rather
than a ticket-owned repository artifact.

## CLI command architecture

`crates/lisa-cli/src/main.rs` owns Clap parsing and command dispatch.
The public operator commands appear first in the enum.
Machine-oriented commands are hidden from Clap's generated command list while still
resolving directly.
Current plumbing commands are:

- `agent-exec`;
- `capture-usage`;
- `commit-ticket`;
- `complete-ticket`.

Each branch resolves project-relative paths, constructs a request when needed, calls
a focused module or library boundary, prints success to stdout, and prints
`Error: {message}` to stderr with exit status 1 on failure.
`crates/lisa-cli/src/lib.rs` exports only reusable boundaries needed across crates or
feature-gated tests.

The claim command does not exist in the command enum, curated plumbing help footer,
or command-count regression.
There is no current parser for a `u128` CLI nonce because no command accepts one.
Clap supports numeric parsing for `u64`, `u128`, and `u32` fields directly.

## Command-level test conventions

`crates/lisa-cli/tests` contains black-box integration tests.
They launch the compiled executable through `env!("CARGO_BIN_EXE_lisa")` and inspect
the real exit status, stdout, stderr, and filesystem effects.
The CLI crate already has `tempfile` as a development dependency.
No external command assertion crate is present.

`help_surface.rs` pins all Lisa-owned subcommands, the exact top-level help snapshot,
the distinction between operator and plumbing commands, and jargon rules.
Adding any command requires updating its total count and the relevant categorized
command list.
Adding a plumbing command also changes the curated top-level footer snapshot.

Other integration tests create temporary repositories or fixture paths and invoke
the binary with explicit arguments.
Environment variables can be set directly on `std::process::Command`, allowing a
test to model `LISA_PANE_ID` without a live Zellij process.

## Validation inputs available to the command

The agent can present ticket ID, attempt ID, and nonce as explicit command values.
The process can locate the project root from an optional path argument or its current
working directory.
The inherited pane ID identifies the durable lease marker.
The exact assignment identity maps deterministically to the predecessor ticket's
attempt-private filename contract.

An exact lease comparison distinguishes:

- the held current attempt represented by the pane marker;
- a prior attempt for the same ticket;
- a claim for another ticket;
- malformed or missing lease evidence.

An exact regular-file check distinguishes a published nonce-bearing assignment path
from an arbitrary nonce for which no assignment was published.
The command cannot inspect the plugin's in-memory `assignment_refs` map directly.
Scheduler admission in the dependent ownership ticket remains responsible for
comparing emitted claim evidence with both `current_leases` and the retained live
assignment reference.

## Publication and race properties

Existing command-side code does not expose the plugin's `RustPublication` helper;
the plugin crate is a WASM-oriented binary/library boundary rather than a CLI
dependency.
The standard library provides the same write-then-rename operations used elsewhere.

A claim signal is ownership evidence consumed asynchronously by the plugin.
A direct write could expose partial JSON during a scheduler scan.
The existing signal and assignment protocols use sibling temporary files followed
by rename to avoid this state.

Lease identity can change while a separate command runs.
The command can observe durable marker values before emitting its signal, but only
the single-threaded plugin can compare a claim with its authoritative in-memory state
at admission time.
This is the same producer/consumer split used by heartbeat and start signals.

## Serialization boundaries

`lisa-core` already depends on Serde and defines shared cross-process records such as
attempt leases, completion identities, capture records, and provenance records.
The CLI and plugin both depend on `lisa-core`.
A shared claim payload in core can be serialized by the command and deserialized by
the later plugin consumer without a dependency inversion.

Named rejection reasons need both stable machine names and useful display text.
The codebase commonly uses typed enums for completion rejections and string errors at
CLI boundaries.
Serde's kebab-case or snake-case attributes can make wire names explicit rather than
deriving them from prose.

## Repository state and workflow constraints

The worktree contains unrelated runtime ledger changes and untracked epic, story, and
ticket files.
They belong to Lisa or other work and must remain untouched by this ticket's commits.
The ticket file itself is scheduler-owned and must not be edited.
All phase artifacts belong only in:

`.lisa/attempts/T-045-01-02/1/work`

Ticket-owned source units must be committed through `lisa commit-ticket` with exact
repository-relative include paths.
The ordinary Git index must remain untouched.

## Constraints surfaced by the codebase

- E-034 lease serialization must remain unchanged.
- The in-memory current lease map remains final scheduler authority.
- The CLI can read the scheduler-published pane lease marker.
- Pane identity is available from `LISA_PANE_ID` for native launches.
- Assignment identity already has a deterministic attempt/nonce filename.
- Assignment files and claim signals live outside canonical repository artifacts.
- The claim payload must be usable by a later scheduler consumer.
- Command-level tests can model the full filesystem boundary without Codex or Zellij.
- Wrong-nonce coverage needs a real matching lease so failure order is observable.
- Prior-attempt coverage needs stale assignment residue so lease rejection, rather
  than mere file absence, is what the test proves.
- No ownership promotion belongs in this ticket.
- No live provider process, Zellij session, or metered model call is needed.
