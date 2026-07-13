# Design — T-045-03-02 evidence tiers: hook and artifact

## Objective

Preserve the exact claim as the scheduler's primary ownership proof while
making the two weaker positive evidence forms explicit in scheduler behavior.

A matching hook may perform the ownership edge before the claim arrives.

If neither claim nor hook has done so, admitted workflow output from the exact
current attempt may perform the edge as a bounded fallback.

No stale-attempt hook or artifact may own the replacement.

The design must stay inside the current assignment boundary and leave the new
delivered-awaiting-claim state to T-045-03-03.

## Evidence hierarchy

The hierarchy is both semantic and operational:

1. exact claim;
2. matching `UserPromptSubmit` hook;
3. admitted current-attempt private workflow artifact.

The claim is strongest because it binds ticket, attempt, pane-routed signal,
and scheduler-retained assignment nonce.

The hook is supplemental because it binds the provider payload to ticket and
generation but does not carry the assignment nonce.

The artifact is fallback because it proves the current leased attempt produced
recognized workflow output, but it is discovered after signal consumers and
does not itself identify the assignment nonce.

All three may perform the same one-time pending-to-owned transition.

Once one input owns the seat, later inputs cannot perform another transition.

The existing poll order enforces the ranking when multiple inputs are visible
in one scheduler poll.

## Option 1 — tests only around existing behavior

The matching hook and stale-evidence paths largely exist already.

One approach is to add only characterization tests and describe the artifact's
existing phase advancement as fallback ownership.

Advantages:

- no production change;
- minimal regression surface;
- existing stale artifact fencing is already strong.

Disadvantages:

- `SeatAssignmentState` remains pending after valid current-attempt output;
- phase advancement would be mislabeled as ownership without changing the
  actual ownership state;
- dashboard and timeout consumers would continue to observe the false pending
  state;
- this violates the ticket's statement that the artifact establishes fallback
  ownership.

Rejected because a test-only interpretation would not change durable scheduler
truth.

## Option 2 — treat any file under the current attempt directory as ownership

The scheduler could scan the private work directory and own if any entry exists.

Advantages:

- earliest possible artifact fallback;
- little coupling to workflow phase names;
- naturally handles new agent-created files.

Disadvantages:

- temporary, editor, or unrelated files could own a seat;
- the evidence set would be unbounded and implicit;
- it would duplicate directory scanning already performed by the workflow
  artifact checker;
- it could own from a file that is never admitted or published;
- it weakens the existing artifact publication contract.

Rejected because “valid artifact” must use the scheduler's recognized admission
boundary, not arbitrary filesystem presence.

## Option 3 — own after any successful `admit_artifact` call

The artifact checker already chooses a bounded artifact name, validates exact
lease currency, requires a regular staged file, and publishes it atomically.

After a successful leased admission, it can attempt the pending-to-owned edge.

The recognized set is bounded to:

- the phase artifact selected from `Phase::artifact_filename()`;
- `review.md` for the Implement-to-Review edge;
- the explicitly recognized living `progress.md` publication during Implement.

Advantages:

- reuses the existing current-lease and file-validity boundary;
- does not add another filesystem scan;
- makes actual ownership state match admitted durable work;
- permits early fallback from recognized Implement progress without treating
  progress as a phase edge;
- stale private output remains ignored and unpublished;
- naturally runs after claims and hooks in the poll.

Disadvantages:

- `check_artifact_advances` gains one additional responsibility;
- native unleased compatibility fixtures also return successful admissions,
  so ownership promotion needs its own leased guard;
- a successful publication can happen while no matching active pane exists,
  which must remain a no-op for ownership.

Chosen because it is the smallest enforceable boundary already present in the
scheduler.

## Option 4 — introduce an explicit evidence enum and one universal admission
method

The scheduler could define an `OwnershipEvidence` enum with claim, hook, and
artifact variants and route all inputs through one ranking engine.

Advantages:

- hierarchy would be represented in a named type;
- common state checks could be centralized;
- future observability could retain the winning evidence kind.

Disadvantages:

- claim, hook, and artifact carry different schemas and validation rules;
- centralization risks weakening nonce validation by reducing all inputs to a
  common denominator;
- ranking is temporal because ownership is a one-time state edge, not a stored
  contest among simultaneous candidates;
- the current serialized poll order already supplies deterministic priority;
- it broadens a narrow ticket into a state-machine rewrite immediately before
  T-045-03-03 changes that machine again.

Rejected for this ticket.

## Chosen production behavior

Add a small scheduler method that receives a ticket and an exact attempt lease
only after artifact admission has succeeded.

The method returns the pane ID when it performs the transition.

It must require:

1. the lease ticket equals the artifact ticket;
2. the lease is exactly current in `State::current_leases`;
3. an agent slot is reserved for that ticket;
4. the slot carries the exact same attempt lease;
5. the pane has an active delivered assignment generation;
6. that generation equals the lease attempt ID;
7. the pane is not already owned.

The active generation helper already excludes startup and ready states.

The method then inserts `SeatAssignmentState::Owned` and returns the pane ID.

Returning the pane ID lets the caller update liveness and log the fallback
without repeating the lookup.

## Artifact call sites

There are two admission sites inside `check_artifact_advances`.

The first publishes `progress.md` while the thread is in Implement.

If that exact leased publication succeeds, it is recognized work and may be
fallback ownership evidence.

Its success must not set `advanced_any` or change the workflow phase.

The second admits the artifact selected for the phase edge.

Only the `Ok(true)` branch may offer fallback evidence.

`Ok(false)` means no recognized artifact exists.

`Err` means the candidate is stale or publication failed.

Neither branch may own.

The fallback call belongs immediately after `Ok(true)` and before phase changes.

This ensures ownership reflects admitted output even if the later ticket file
update fails for an unrelated reason.

## Activity behavior

A successful artifact fallback should refresh the pane and thread activity
clock, matching successful claim and hook consumers.

It should append an information event that names:

- pane ID;
- ticket ID;
- attempt ID;
- artifact filename.

Rejected or duplicate evidence should not bump activity or log false success.

Claim and hook event text remain unchanged.

## Hook behavior

No production change is needed to matching-hook admission.

`acknowledge_codex_assignment` already requires the exact current pane lease,
active generation, and matching tagged payload.

`check_codex_ack_signals` already bumps activity only on success.

Its poll position is already after claims and before artifacts.

The test will make that supplemental role explicit rather than renaming or
removing the compatibility path.

## Stale hook behavior

A predecessor hook can take two forms in tests.

It can be routed to a predecessor pane that no longer holds the ticket.

It can be routed to the replacement pane but carry the predecessor generation.

The second form exercises the strict generation check directly and is more
focused for this ticket.

The `.ack` record should still be consumed once.

It must not own, bump replacement activity, or produce an acknowledgment event.

## Stale artifact behavior

The predecessor private directory may legitimately remain on disk.

The artifact checker derives its candidate directory from the running thread's
current lease.

Therefore a predecessor artifact sitting in its old attempt directory is not
even selected as the current phase artifact.

Direct admission with the predecessor lease also fails the current lease guard.

The evidence-tier test should demonstrate both observable outcomes where
practical:

- predecessor output remains private and canonical output is absent;
- direct predecessor admission is rejected;
- the replacement remains pending.

Then the same logical artifact under the replacement private directory should
be admitted and own the seat.

## Test arrangement

Use the existing scheduled Codex fixture rather than a synthetic state-only
fixture.

It installs a scanned ticket, slot, thread, lease, assignment reference, and
real `Delivering` state.

One test case will cover hook acceleration:

1. schedule `T-NAME`;
2. reach `Delivering`;
3. confirm no claim exists;
4. write a matching pane hook;
5. run the hook consumer;
6. assert one-shot consumption and `Owned`.

A second test case will cover artifact fallback and stale evidence:

1. create a predecessor attempt;
2. release and reschedule to mint the replacement;
3. reach replacement `Delivering`;
4. write predecessor-generation hook to the replacement pane;
5. write `research.md` in the predecessor private directory;
6. run hook and artifact consumers;
7. assert replacement remains pending and stale output is private;
8. write `research.md` in the replacement private directory;
9. run artifact admission;
10. assert `Owned`, phase advancement, canonical current bytes, and a fallback
    activity event.

## Compatibility

Claim-only ownership remains unchanged and is processed first.

Claude scheduling remains unchanged because already-owned seats make the
fallback method a no-op.

Unleased historical artifact fixtures continue advancing phases but cannot gain
pane ownership without a lease.

Artifact publication and phase advancement retain their existing error paths.

No signal schema, CLI surface, assignment file, or UI status enum changes.

The existing timeout behavior remains intact for T-045-03-03 to replace.

## Verification

Run the focused evidence-tier test first.

Run the existing claim-only test to protect primary evidence.

Run stale-attempt and artifact-advance focused tests.

Run the complete `lisa-plugin` test suite.

Run `cargo test --workspace` if the focused plugin suite succeeds.

Inspect the exact diff and ensure only `crates/lisa-plugin/src/lib.rs` is a
ticket-owned source path.
