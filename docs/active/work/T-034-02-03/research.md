# T-034-02-03 Research — reject stale liveness and artifact writes

## Ticket boundary

The ticket starts in Research and requires the remaining RDSPI phases through
Review in one pass.

Its acceptance criterion joins two scheduler inputs that currently lack
attempt attribution:

- heartbeat/liveness evidence;
- workflow artifact publication and phase advancement.

The required regression uses a predecessor and successor attempt for one
ticket, makes both attempts target the same logical artifact, and proves only
the current lease can affect scheduler truth.

The ticket does not own completion admission or provenance. Completion was
lease-gated in `T-034-02-02`; provenance is the following ticket.

## Existing lease authority

`crates/lisa-core/src/types.rs` defines `AttemptLease` as the pair of ticket ID
and positive per-ticket attempt ID.

`AttemptLease::is_current` requires exact equality with the scheduler's
optional current lease.

`State::current_leases` in `crates/lisa-plugin/src/lib.rs` is the live authority
registry.

`State::lease_high_water` survives revocation inside the scheduler process and
provides monotonic successor IDs.

Dispatch stamps the same lease onto the logical `Thread` and physical
`AgentSlot`.

Hard-silence recovery revokes the predecessor, fences its pane, releases the
reservation, and later dispatches a successor lease.

Codex acknowledgement promotion already checks the slot stamp against
`current_leases`.

Completion requests already require either an exact current attempt lease or
the narrow manual operator authority.

## Current heartbeat production

Generated native hooks live in `crates/lisa-cli/src/templates.rs`.

`ON_HEARTBEAT_HOOK` runs after each native client tool call.

It creates `.lisa/signals` and overwrites:

`pane-<pane-id>.heartbeat`

The body is only an informational UTC timestamp.

The hook receives `LISA_PANE_ID` from the launched client process.

It does not receive or read the ticket attempt identity.

`ON_HEARTBEAT_HOOK` is installed for both Claude and Codex hook settings.

Legacy hook bodies are recognized by `lisa init` so managed upgrades can
replace known prior versions without overwriting user-owned variants.

The native Codex `agent-exec` bridge has a separate `SignalWriter` that uses
the same pane-only filename and timestamp body. The current interactive Codex
adapter launches the official TUI directly, so the generated hook is the live
path for scheduled interactive Codex seats.

## Current heartbeat admission

`State::check_heartbeat_signals` scans the shared signal directory.

It derives only a pane ID from the filename.

It deletes each heartbeat, then unconditionally calls
`bump_pane_activity(pane_id)`.

`bump_pane_activity` updates the matching slot's `last_activity_at` and the
thread selected by the slot's current ticket reservation.

The heartbeat also clears attention and awaiting-human debounce state.

No heartbeat body is parsed.

No ticket ID or attempt ID is compared.

No slot lease is checked against `current_leases`.

This means a file addressed to a pane is interpreted using whatever ticket the
scheduler currently associates with that pane, regardless of which attempt
produced the file.

## Pane reuse and attempt transport

Fresh process launch commands can export attempt identity directly.

In-place TUI reuse is different: environment variables inherited by the
resident process cannot be updated by typing a new prompt.

The scheduler already has a stable physical addressing key, the pane ID.

The scheduler also controls dispatch and knows the exact lease before any
prompt or launch side effect.

A scheduler-written per-pane lease marker can therefore bridge reused native
sessions: the hook reads the marker for its pane and copies that exact lease
into the emitted heartbeat.

When a hard-silent pane is fenced, it is never selected for a successor.

Its marker remains the predecessor identity, while the replacement pane gets a
different marker containing the successor lease.

Updating a marker during ordinary same-pane reuse is safe because there is one
resident accepted session in that pane, not two concurrent physical attempts.

## Current artifact production contract

`ticket_prompt` tells every agent to write all phase artifacts directly to:

`docs/active/work/<ticket-id>/`

`finish_up_prompt` points a parked Review agent at the same shared `review.md`.

`SpawnContext` contains ticket directory, ticket ID, pane ID, and optional
Codex acknowledgement generation. It does not carry an attempt-scoped artifact
directory.

Both native adapters call the shared prompt builder.

The current prompt gives a predecessor and successor the same output path for
each logical phase artifact.

## Current artifact admission

`State::check_artifact_advances` snapshots running threads and their optional
attempt leases.

For each phase it checks only whether the shared artifact path exists.

Research, Design, Structure, and Plan use their named phase artifact.

Implement uses `review.md` because `progress.md` is a living document.

Review existence enters the lease-gated completion request added in the prior
ticket.

The completion request receives the thread lease, but that proves only which
thread asked for completion; it does not prove which attempt wrote the shared
file that triggered the request.

Intermediate phases update ticket frontmatter before completion admission is
involved.

The scan loops until all already-present artifacts have caught up.

`check_idle_signals` independently checks the same shared artifact paths for
idle-driven advancement and for an already-present Review artifact.

`open_mark_done_modal` and Review timeout logic also inspect shared paths, but
they do not form the primary automatic publication boundary.

## Shared-path race

After a predecessor loses its lease, it can continue writing the same shared
path as its replacement.

Filesystem existence contains no writer identity.

File mtime, ownership, process ID, and pane ID are not reliable attribution
because the repository is shared and agents use ordinary editor/file tools.

Checking the current thread lease after observing a shared file would attach
the replacement's identity to unattributed bytes.

Sidecar metadata written after the shared file has the same race unless the
scheduler exclusively controls publication.

The enforceable boundary is therefore before shared publication, not after
shared existence.

## Existing publication and transaction patterns

The acknowledgement hook uses a temporary file plus rename so the scheduler
does not read partial JSON.

The completion transaction publishes repository changes through an isolated
Git index, but it runs only after Review and cannot attribute earlier phase
artifacts.

`.lisa/` already contains ignored scheduler/runtime namespaces for signals and
provider artifacts.

`.lisa/.gitignore` currently ignores `signals/`, `claude/`, and `codex/`.

An attempt staging namespace under `.lisa/` would remain outside ticket work
artifacts and the final completion transaction.

The scheduler can copy a verified current attempt's staged bytes into the
canonical work directory with a temporary file and atomic rename.

## Tests around the boundary

`test_check_heartbeat_signals_updates_activity` proves pane-only heartbeat
consumption and clock updates.

Stress coverage proves heartbeat scanning leaves unrelated signal files.

`test_codex_dag_advances_all_phases_via_artifacts` writes canonical shared
artifacts and proves artifact-only catch-up through the workflow.

Several phase tests construct direct threads without leases for historical
fixtures.

`install_current_attempt` mirrors production lease stamping for completion and
newer scheduler tests.

The prior ticket added a direct stale/current completion regression that can
serve as the shape for this ticket's combined stale heartbeat and same-logical-
artifact test.

## Constraints

The ticket frontmatter phase and status must not be edited by this agent.

Ticket-owned source changes must be committed with `lisa commit-ticket` and
exact include paths.

The ordinary index and unrelated dirty files must remain untouched.

The working tree already contains unrelated modifications, including a dirty
`crates/lisa-cli/src/agent_exec.rs`; overlapping work there cannot safely be
included in this ticket's isolated commit.

Legacy/unleased test fixtures may require a compatibility path, but production
scheduled attempts now always carry leases.

Artifact staging must preserve the canonical `docs/active/work/<ticket>/`
namespace for Lisa's completion transaction and human review.

Rejected stale evidence should be consumed so it cannot retrigger every poll.

Missing or malformed attempt identity must fail closed for leased attempts.
