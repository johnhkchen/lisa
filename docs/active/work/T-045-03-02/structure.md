# Structure — T-045-03-02 evidence tiers: hook and artifact

## Change inventory

One ticket-owned source file is modified:

`crates/lisa-plugin/src/lib.rs`

No source files are created or deleted.

No change is required in:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/codex_ack.rs`;
- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-cli`;
- UI rendering modules;
- configuration types;
- launcher or adapter modules.

Private RDSPI artifacts are created under:

`.lisa/attempts/T-045-03-02/1/work/`

Those artifacts are not ticket-owned source units and are published by Lisa.

## Existing component boundaries retained

Filesystem signal syntax remains owned by `signal.rs`.

Provider hook payload semantics remain owned by `codex_ack`.

Claim record identity remains owned by `lisa-core`.

Lease authority, pane routing, ownership state, artifact admission, and poll
ordering remain owned by `State` in plugin `lib.rs`.

The new behavior is therefore placed beside existing claim and hook ownership
admission methods.

## New scheduler method

Add one private method to `impl State` near:

- `acknowledge_codex_assignment`;
- `admit_assignment_claim`.

Proposed interface:

```rust
fn admit_artifact_ownership(
    &mut self,
    ticket_id: &str,
    candidate: &AttemptLease,
) -> Option<u32>
```

The method is private because no module outside the scheduler should bypass
artifact admission and offer ownership evidence directly.

The return value communicates whether a transition occurred and identifies the
pane for activity bookkeeping.

`None` means the evidence was stale, unmatched, premature, or redundant.

`Some(pane_id)` means the method changed exactly one pending seat to `Owned`.

## New method internals

The method is organized as fail-closed guards followed by one state mutation.

Guard group 1: candidate authority.

- require `candidate.ticket_id == ticket_id`;
- require `candidate.is_current(self.current_leases.get(ticket_id))`.

Guard group 2: physical pane attribution.

- find the slot with `slot.ticket_id == ticket_id`;
- require `slot.attempt_lease == candidate`;
- obtain the slot pane ID.

Guard group 3: assignment state.

- call `active_assignment_generation(pane_id)`;
- require the returned generation equals `candidate.attempt_id`.

The helper already returns no generation for `Owned`, startup, ready, and
terminal states.

Mutation:

- insert `SeatAssignmentState::Owned` for the pane;
- return the pane ID.

The method does not inspect the filesystem.

Its caller is responsible for invoking it only after a valid artifact has been
admitted.

## Artifact checker integration

`check_artifact_advances` remains the only production caller.

Add a local helper method or a compact repeated block to perform post-admission
effects:

1. call `admit_artifact_ownership` with the running ticket's source lease;
2. if it returns a pane, call `bump_pane_activity`;
3. append an information event naming the fallback artifact.

A small second private method is acceptable if it keeps both artifact call
sites identical:

```rust
fn record_artifact_ownership(
    &mut self,
    ticket_id: &str,
    candidate: Option<&AttemptLease>,
    artifact_name: &str,
)
```

This wrapper would:

- return immediately for `None` candidates;
- call the admission method;
- perform activity and logging only on a transition.

It must not call `admit_artifact` itself.

Keeping filesystem admission separate prevents a misleading API that appears
to validate files while only validating state.

## Implement progress path

The existing Implement block calls:

```rust
self.admit_artifact(ticket_id, source_lease, "progress.md")
```

Restructure its result match so all outcomes remain visible:

- `Ok(true)`: offer fallback ownership using `progress.md`;
- `Ok(false)`: no action;
- `Err(error)`: retain the existing rejected-publication log.

Do not change `advanced_any` in this block.

Do not change `thread.current_phase` in this block.

Do not let `progress.md` become an Implement completion edge.

## Phase artifact path

The existing phase artifact match currently treats `Ok(true)` as an empty
success arm.

Expand that arm to offer fallback ownership.

All following workflow behavior stays in place:

- compute next phase;
- dispatch completion when next is Done;
- update ticket phase otherwise;
- emit phase activity;
- update the running thread;
- repeat until no more phase advances are available.

The ownership call happens before the next-phase logic because admitted current
output is already sufficient evidence even if a later ticket-file update fails.

## Poll order

No call ordering is added to `poll_tick`.

The relevant existing order remains:

```text
check_claim_signals
check_codex_ack_signals
check_artifact_advances
check_assignment_ack_timeouts (later)
```

Update nearby comments only if necessary to state that hook and artifact are
supplemental/fallback evidence.

The order itself is the operational evidence rank.

## Logging

Use the existing `ActivityEvent::Info` variant.

Proposed message shape:

```text
Pane <pane> established ownership of <ticket> attempt <attempt>
from current-attempt <artifact>
```

The exact wording should contain:

- `Pane <id>`;
- ticket ID;
- `attempt <id>`;
- artifact filename;
- an ownership or fallback term.

No new UI activity event variant is needed.

Claim-specific and acknowledgment-specific messages remain unchanged.

## Test placement

Add focused native scheduler tests in the existing `#[cfg(test)]` module in
`crates/lisa-plugin/src/lib.rs`.

Place them adjacent to:

- `test_dashboard_snapshot_shows_fresh_codex_handoff_states`;
- `delivered_assignment_becomes_owned_on_exact_claim_without_hook`;
- existing Codex acknowledgment state-machine tests.

This keeps all three evidence-tier cases readable together.

## Hook acceleration test

Suggested name:

```rust
matching_hook_accelerates_pending_claim_ownership
```

Use `pane_name_schedule_state` with a Codex target.

Use `schedule_ready_tickets` and `exit_then_deliver_fresh_codex` to reach a real
pending delivered state.

Construct the hook body with:

- `hook_event_name: UserPromptSubmit`;
- `codex_ack::tag_codex_assignment`;
- current ticket;
- current generation.

Write it to `pane-10.ack` and run `check_codex_ack_signals`.

Assertions:

- no claim signal exists before or after;
- the hook file is consumed;
- the seat starts non-owned;
- the seat becomes exactly `Owned`;
- an acknowledgment activity event exists.

## Artifact fallback and stale evidence test

Suggested name:

```rust
current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored
```

The fixture needs a predecessor and replacement for the same ticket.

Preferred setup reuses the existing schedule/release/reschedule sequence used by
the acknowledgment regression test.

The sequence preserves monotonic lease minting and real assignment references.

After replacement delivery:

- record replacement pane/thread activity clocks;
- write a predecessor-generation hook to the replacement pane;
- write `research.md` only in the predecessor private directory;
- run hook and artifact consumers.

Assertions for stale evidence:

- `.ack` is consumed;
- seat remains replacement `Delivering`;
- replacement activity is unchanged;
- no matching acknowledgment success event is logged;
- predecessor artifact remains in its private directory;
- canonical `research.md` is absent;
- direct `admit_artifact` with predecessor lease returns `Err`.

Then write distinct bytes to replacement `research.md` and run the artifact
checker.

Assertions for fallback:

- seat becomes exactly `Owned`;
- thread phase advances from Research to Design;
- canonical artifact contains replacement bytes;
- predecessor private bytes remain unchanged;
- pane/thread activity advances;
- one fallback information event names the artifact and attempt.

## Existing tests protected

The claim-only test must remain unchanged or receive only nonsemantic cleanup.

Artifact phase tests must retain their current outcomes.

The Implement progress test should gain no phase transition.

Stale attempt publication tests must continue to reject predecessor bytes.

Timeout tests must remain green because this ticket does not change timeout
states or reinjection rules.

## Commit unit

All production and test changes are in one cohesive source unit:

`crates/lisa-plugin/src/lib.rs`

Commit it with exactly:

```text
lisa commit-ticket
  --ticket-id T-045-03-02
  --message "feat(plugin): tier hook and artifact ownership evidence"
  --include crates/lisa-plugin/src/lib.rs
```

Do not include private artifacts, ticket files, provenance files, or unrelated
working-tree entries.

## Final tree state

After the isolated transaction:

- `crates/lisa-plugin/src/lib.rs` has no staged or modified ticket-owned bytes;
- unrelated pre-existing worktree entries remain untouched;
- private phase artifacts remain under the attempt work directory for Lisa;
- no ordinary index commands are used.
