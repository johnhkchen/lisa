# T-034-02-03 Design — lease-bound liveness and artifact publication

## Decision summary

Use two related lease-bearing channels.

For liveness, write the assigned `AttemptLease` to a scheduler-owned per-pane
marker. Native heartbeat hooks copy that marker into each heartbeat file. The
scheduler accepts a heartbeat only when its body, slot stamp, ticket
reservation, and `current_leases` entry all match exactly.

For workflow output, give each attempt a private staging directory under
`.lisa/attempts/<ticket>/<attempt>/work`. Prompts name that directory instead of
the canonical work directory. The scheduler admits a staged artifact only for
the exact current lease, atomically publishes its bytes to
`docs/active/work/<ticket>/<artifact>`, and only then advances phase.

Canonical artifacts remain the human-visible and completion-transaction input.

## Goals

- reject a predecessor heartbeat after a successor becomes current;
- ensure stale liveness cannot refresh the successor's clocks;
- ensure stale liveness cannot clear successor attention/question state;
- prevent predecessor bytes from winning a same-logical-artifact race;
- retain canonical work artifact paths for review and completion commits;
- support fresh and in-place reused native sessions;
- use the existing `AttemptLease` as the only authority identity;
- consume malformed/stale signals without retry loops;
- preserve legacy test fixtures only where no lease authority exists;
- prove the boundary with one combined stale/current regression.

## Non-goals

- lease-gating completion, already implemented by `T-034-02-02`;
- attempt-aware provenance, owned by `T-034-02-04`;
- changing source-code edit paths or Git transaction ownership;
- sandboxing or terminating stale agent processes beyond existing fencing;
- persisting lease high-water state across plugin restarts;
- redesigning idle, stopped, cleared, error, or acknowledgement protocols;
- modifying the dirty headless `agent_exec` file in this ticket.

## Option 1 — validate shared artifacts with the current thread lease

Observe `docs/active/work/<ticket>/<artifact>` as today, then check that the
thread carrying the observation has the current lease.

Advantages:

- minimal code and prompt changes;
- no new runtime directory;
- canonical artifact appears immediately.

Disadvantages:

- validates the observer, not the writer;
- predecessor and successor still race on identical bytes/path;
- a stale predecessor file is credited to the successor;
- cannot satisfy the ticket's attribution requirement.

Decision: rejected. Shared existence has no author identity.

## Option 2 — shared artifact plus lease sidecar

Ask the agent to write the canonical artifact and a neighboring lease receipt.

Advantages:

- keeps the current primary path;
- sidecar parsing is straightforward;
- small scheduler change.

Disadvantages:

- two independent writes are not atomic;
- stale and current attempts can interleave artifact and sidecar;
- agents, rather than the scheduler, claim authority;
- a stale process can overwrite both shared files.

Decision: rejected. A self-asserted sidecar does not create an enforceable
publication boundary.

## Option 3 — infer writer from mtime or baseline hash

Snapshot canonical file metadata at dispatch and accept only later changes.

Advantages:

- no prompt changes;
- no staging storage.

Disadvantages:

- time and content changes still do not identify the writer;
- stale writes after dispatch look newer;
- equal content can suppress a legitimate current publication;
- filesystem timestamp resolution and clock behavior complicate tests.

Decision: rejected. Freshness is not authority.

## Option 4 — attempt-scoped staging with scheduler publication

Each prompt names a path derived from the exact lease. The scheduler reads
only that lease's staging directory and owns the canonical rename.

Advantages:

- predecessor and successor cannot overwrite one another's staged artifact;
- source path itself carries ticket and attempt identity;
- only scheduler code can turn staged output into canonical output;
- current-lease validation happens before reading and publishing bytes;
- canonical consumers remain unchanged after publication;
- deterministic tests need only two staging directories.

Disadvantages:

- prompt and follow-up paths change;
- staging files remain as ignored runtime evidence;
- existing artifact tests need lease-aware paths;
- direct writes to canonical paths are no longer automatic evidence for leased
  attempts.

Decision: selected. It creates the explicit admission boundary required by the
ticket while retaining the repository-facing artifact contract.

## Attempt staging layout

Production layout:

```text
.lisa/attempts/
  <ticket-id>/
    <attempt-id>/
      work/
        research.md
        design.md
        structure.md
        plan.md
        progress.md
        review.md
```

The path is computed only from an `AttemptLease` held by the scheduler.

Ticket IDs are already repository-controlled identifiers used in paths.

Attempt IDs are numeric and generated by `AttemptLease::mint`.

`attempts/` is added to the managed `.lisa/.gitignore` template.

Staging is not passed to `complete-ticket`; only published canonical artifacts
are committed.

## Prompt contract

Extend `SpawnContext` with the exact attempt artifact directory.

The shared prompt builder receives an artifact directory and says every phase
artifact must be written there.

The prompt explicitly states that Lisa publishes admitted artifacts into the
canonical `docs/active/work/<ticket>` directory.

Fresh launch, same-provider reuse, cross-provider recycle, recovery launch,
and clear-timeout prompt delivery all derive the directory from the slot's
stamped lease.

If a prompt cannot resolve a stamped lease, the scheduler must not silently
invent a current staging path.

Review follow-up uses the same attempt directory, so it cannot accidentally
direct a stale Review attempt back to the canonical shared path.

## Artifact admission algorithm

For a running leased thread and expected artifact name:

1. clone the thread's candidate lease;
2. require exact equality with `current_leases[ticket]`;
3. build the candidate staging path from that lease;
4. require a regular readable staged file;
5. create the canonical ticket work directory;
6. write the bytes to a canonical-directory temporary file;
7. atomically rename the temporary file to the logical artifact path;
8. only after successful publication, perform phase advancement or completion.

The single-threaded scheduler cannot revoke the lease between the check and
rename without re-entering its event loop.

Publication failures log an error and leave the phase unchanged for retry.

Stale staging output remains isolated and is never copied.

For historical fixtures with neither a thread lease nor a scheduler current
lease, canonical existence may remain a compatibility path. It is unavailable
once any live authority exists for that ticket.

## Shared consumers

`check_artifact_advances` becomes the primary publisher.

Its catch-up loop can publish one current attempt's multiple completed phase
artifacts in sequence.

`check_idle_signals` calls the same admission helper before using an artifact,
covering the independent idle-driven path.

Review-to-Done still calls `request_completion` with the same thread lease.

The completion gate remains a second defense: publication proves artifact
attribution, and completion admission proves transition authority.

## Heartbeat marker format

Use JSON serialization of `AttemptLease`:

```json
{"ticket_id":"T-034-02-03","attempt_id":2}
```

At dispatch or recovery lease replacement, the scheduler atomically writes:

`.lisa/signals/pane-<pane-id>.lease`

The heartbeat hook copies that marker into a temporary heartbeat file and
renames it to `pane-<pane-id>.heartbeat`.

The hook stays independent of stdin and does not invoke `lisa`.

The marker solves native reused-session environment immutability because the
resident hook reads scheduler state at emission time.

Fenced panes retain their predecessor marker. A replacement on another pane
gets the successor marker.

## Heartbeat admission algorithm

For each heartbeat file:

1. parse pane ID from its filename;
2. read its body;
3. delete the signal regardless of validity;
4. deserialize an `AttemptLease`;
5. find the exact physical slot;
6. require slot ticket and slot lease to match the candidate;
7. require candidate equality with `current_leases[ticket]`;
8. only then bump slot/thread activity and clear debounce state.

Malformed, absent, cross-ticket, predecessor, future, unstamped, and revoked
heartbeats are inert.

## Marker write failure

The marker must exist before pane lifecycle input begins.

If dispatch cannot publish it, revoke the just-installed current lease, retain
its high-water value, log an error, and leave the ticket unscheduled.

This fails closed and ensures a retry receives a strictly newer lease.

Recovery marker replacement follows the same rule before delivering the fresh
attempt prompt.

## Compatibility and upgrades

Add the previous generic heartbeat hook body to the legacy managed-hook list.

This lets `lisa init` upgrade untouched generated hooks to the lease-bearing
version.

User-modified hook bodies remain protected by the existing ownership logic.

The repository's currently dirty installed hook files are not included in this
ticket's source commit.

Unleased native test fixtures can retain canonical-file behavior, but all
production dispatches have stamped leases after `S-034-01`.

## Test design

The principal regression creates predecessor and successor leases for one
ticket and assigns them different panes.

Both attempts write `research.md` in their own staging directories with
distinct content.

Feed a predecessor heartbeat after the successor is current and record the
successor thread/slot clocks.

Run heartbeat admission and assert the successor clocks are unchanged.

Run artifact admission with only stale staged output and assert:

- no canonical `research.md` exists;
- phase remains Research;
- stale content is not visible as canonical output.

Then write the successor's same logical artifact and current heartbeat.

Assert the current heartbeat updates clocks, the canonical file contains only
current bytes, and the phase advances exactly once.

Additional unit coverage verifies hook marker copying, prompt staging paths,
malformed heartbeat rejection, and atomic publication behavior.

## Risks

An agent can ignore its prompt and directly edit canonical work paths. Those
bytes are visible in the working tree, but leased automatic admission never
uses canonical existence as evidence; only current staging can publish and
advance.

The headless `agent_exec` writer still emits timestamp heartbeat bodies. It is
not the live interactive adapter path and is deliberately deferred because the
file has unrelated uncommitted work. Under the new fail-closed consumer, those
heartbeats are ignored until that bridge adopts lease bodies.

Staging cleanup is not required for correctness. A future maintenance ticket
may prune old attempt directories after authoritative completion.
