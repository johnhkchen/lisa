# Review — T-045-03-01 claim is ownership proof

## Disposition

Pass.

The scheduler now consumes agent-issued claim signals and promotes a delivered seat
to `Owned` only when pane routing, active assignment state, slot lease, authoritative
current lease, retained assignment lease, and retained nonce all agree.

The ticket acceptance test observes `delivering` while the seat is unowned, confirms
there is no hook file, then observes `owned` after the exact claim alone.
Focused, package, workspace, and WASM-oriented verification pass.
All source paths are clean and the ordinary index is empty.

No critical issue blocks completion.

## Source commit

The claim source unit was committed through Lisa's isolated transaction:

`67a7f0eb976ff85d9b1d4b7ffe218412bb157c1a`

`feat(plugin): own assignments from exact claims`

Exact included paths:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

No ordinary `git add` or `git commit` was used.

## Typed signal ingestion

`crates/lisa-plugin/src/signal.rs` now imports the shared
`lisa_core::claim::AssignmentClaim` schema.

Added:

- `SignalRequest::Claims`;
- `SignalRecord::Claim { pane_id, claim }`;
- exact `.claim` ingestion in the strict pane-addressed filename family.

The consumer accepts only:

`pane-<u32>.claim`

The pane ID comes from the filename and remains separate from the body assertion.
The body is deserialized as the shared ticket/attempt/nonce record.
There is no duplicated plugin-local wire type.

Recognized claims are one-shot.
The path is removed after acquisition whether JSON parsing succeeds or fails.
An invalid pane filename remains untouched because the claim consumer does not own
it.

This matches the existing typed heartbeat/start/shell-ready behavior while keeping
raw provider hook payloads a distinct record class.

## Scheduler admission

`crates/lisa-plugin/src/lib.rs` now contains
`admit_assignment_claim(pane_id, claim)`.

The method first rejects an already-owned or ineligible state.
It then requires an active generation from the existing unowned assignment states:

- `Delivering`;
- `AssignedPendingAck`;
- `Recovering`.

It resolves the physical slot and requires both its ticket ID and attempt lease.
The following identities must all match:

- claim ticket equals slot ticket;
- claim attempt equals state generation;
- slot lease ticket equals slot/claim ticket;
- slot lease attempt equals claim attempt;
- slot lease is exactly current in `State::current_leases`;
- retained `AssignmentRef` exists for that ticket;
- retained assignment lease equals the slot/current lease;
- retained assignment nonce equals the claim nonce.

Only after every comparison succeeds does the method insert
`SeatAssignmentState::Owned`.
Every rejection returns before state mutation.

## Authority assessment

The design preserves E-034's authority boundary.
The durable pane marker checked by the CLI is transport evidence, not final
scheduler authority.
The plugin revalidates the claim against its in-memory current lease.

The retained assignment reference is also required.
This prevents an old same-attempt immutable assignment file from authorizing a nonce
that the scheduler no longer considers live.
The consumer does not scan an attempt directory or reread file existence as a
substitute for retained identity.

Pane routing cannot be bypassed by a body field.
The strict filename chooses the pane, then the slot comparison binds that pane to
the asserted ticket and lease.

A claim cannot own a merely starting or ready-but-undelivered seat because those
states do not expose an active assignment generation through the existing helper.
This preserves the required delivered-before-owned ordering.

## Consumer integration

Added `check_claim_signals` alongside the other scheduler consumers.
It requests typed claim records and calls authoritative admission.

On a successful transition it:

- bumps pane activity;
- refreshes the running thread activity through the existing helper;
- logs a claim-specific information event naming pane, ticket, and attempt.

Rejected claim evidence is consumed without bumping activity or logging a false
transition.
A corrected later claim can publish a fresh one-shot file and succeed.

The poll order is now:

1. heartbeat and awaiting-human evidence;
2. delivery of previously-ready assignments;
3. process-start and shell-ready lifecycle evidence;
4. exact assignment claims;
5. existing provider acknowledgement evidence;
6. artifact, idle, transition, error, and timeout processing.

Claims therefore run after delivery/lifecycle prerequisites and before timeout
evaluation.
An exact claim present at a deadline wins before recovery policy runs.

Added `claim` to pane lifecycle cleanup so a reset removes unconsumed predecessor
claim residue along with other pane-scoped signal files.
Cleanup is defense in depth; admission equality remains the authority fence.

## Observable delivered and owned states

No cosmetic state was added.
The existing real scheduler states remain:

- `SeatAssignmentState::Delivering`;
- `SeatAssignmentState::Owned`.

The existing UI reduction maps them to distinct output:

- yellow `delivering`;
- green `owned`.

The acceptance test uses `dashboard_thread_row` to inspect rendered scheduler output
rather than asserting only an internal enum.
This demonstrates that the published state changes reflect the underlying transition.

## Acceptance coverage

Added:

`delivered_assignment_becomes_owned_on_exact_claim_without_hook`

The test drives the real scheduling fixture through fresh Codex delivery and uses
the actual lease and assignment nonce retained by the scheduler.

Before claim admission it asserts:

- internal state is `Delivering`;
- `seat_is_owned` is false;
- dashboard output contains `delivering`;
- no `pane-10.ack` hook evidence exists.

It publishes a wrong-nonce claim first.
That file is consumed while state and output remain delivered/unowned.

It then publishes the exact claim and runs only the claim consumer.
The test asserts:

- the claim file is consumed;
- the hook file is still absent;
- internal state is `Owned`;
- `seat_is_owned` is true;
- dashboard output contains `owned`;
- a claim-specific activity event exists.

This directly satisfies the ticket's single acceptance criterion.

## Signal regression coverage

`signal.rs` adds a typed parsing test with a nonce larger than `u64::MAX`.
It verifies valid and malformed recognized claims are both consumed correctly.

`signal_ingestion_regression.rs` extends the every-request contract with claims and
updates the pinned poll interleaving.

`signal_consumer_characterization.rs` adds claim cases to:

- the recognized malformed one-shot matrix;
- the legacy filename rejection matrix;
- the ordered signal consumer characterization.

Its focused claim consumer test constructs a delivered current attempt with a
retained assignment reference, rejects a wrong nonce without activity, and accepts
the exact nonce with activity and log effects.

## Verification performed

Baseline passed:

```text
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
```

Focused implementation verification passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
```

Observed focused results:

- 8 signal module tests passed;
- 12 consumer characterization tests passed;
- 4 ingestion regression tests passed;
- 1 exact acceptance test passed.

Complete plugin verification passed:

```text
cargo test -p lisa-plugin
```

Result: 391 passed, 0 failed, 0 ignored.

Complete workspace verification passed:

```text
cargo test --workspace
```

The run included CLI claim producer coverage, the shared core claim schema, all 391
plugin tests, and integration suites.
The existing real-Zellij boundary remained intentionally ignored by its declared
environment gate.

Production-oriented verification passed:

```text
just check
```

The `wasm32-wasip1` plugin check and the recipe's workspace tests both passed.

## Shared-file transaction collision

There was one repository-concurrency event reviewers should know about.
T-045-02-02 began changing the same `lib.rs` launcher call sites between this
ticket's diff audit and its isolated path transaction.
Because an exact include is path-level, commit `67a7f0e` also captured those
compatible launcher call-site hunks.

No foreign path was included and no content was lost.
The neighboring ticket documented the same collision and subsequently committed its
remaining adapter implementation and launcher test as:

`5f02b0f feat(plugin): launch Codex with exact assignment reference`

The final combined tree passed the package, workspace, and WASM checks.
Reverting the already-shared call sites would have destroyed active neighboring work,
so the code was preserved and the boundary is disclosed rather than hidden.

This collision is a commit-attribution concern, not a product correctness concern.
The claim-specific hunks and tests are identifiable and all intended source paths are
now clean.

## Source cleanliness

Final audit shows clean:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`;
- neighboring `crates/lisa-plugin/src/adapter.rs`.

`git diff --cached --name-only` is empty.
No ticket-owned source file remains staged, modified, or untracked.

Visible remaining repository changes are Lisa runtime ledgers and materialized
workflow documents outside this ticket's source unit.
They were preserved and excluded.

## Deliberate limits and next work

This ticket makes claim evidence sufficient without requiring a hook.
It deliberately does not finish the entire S-045-03 evidence model.

T-045-03-02 owns:

- ranking hook evidence as supplemental fast evidence;
- current-attempt private artifact fallback;
- stale hook/artifact rejection in that hierarchy.

T-045-03-03 owns:

- a real delivered-awaiting-claim scheduler state;
- zero prompt reinjection while awaiting claim;
- a bounded named actionable timeout result.

Later E-045 work owns clean ticket-boundary TUI exit, nonce revocation, and live
Codex/Zellij field proof.

The current hook acknowledgement path remains in place until the dependent evidence
tier ticket refines its role.
This ticket proves it is no longer required for ownership but does not prematurely
replace the next ticket's hierarchy.

## Open concerns

The path-level collision means commit history does not perfectly separate the
launcher and claim hunks inside `lib.rs`.
Both tickets' artifacts disclose it, both source transactions are durable, and the
final tree is clean and fully verified.

There is no live-provider proof in this ticket.
That is intentional: the story defines this slice as fixture-proven and free, with
real Codex/Zellij validation deferred.

No functional TODO remains within T-045-03-01.

## Final assessment

The scheduler now has an exact, provider-neutral claim admission boundary.
Delivery stays visibly unowned until valid claim evidence arrives.
An exact claim under the current pane lease and retained nonce establishes ownership
without any hook signal.
Stale or wrong identity fails closed.

The acceptance criterion is met, regression coverage is proportionate to the
authority risk, the production target builds, and the source tree is clean.
