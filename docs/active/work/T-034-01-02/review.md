# T-034-01-02 Review — mint lease on dispatch

## Outcome

The scheduler now mints a fresh provider-neutral `AttemptLease` for every
admitted ticket dispatch, records it as that ticket's current/latest lease, and
stamps the exact same value on both the physical agent slot and logical thread.

Release clears the physical seat stamp while retaining the per-ticket
predecessor. Redispatch of the same ticket therefore mints a strictly higher
attempt ID. The acceptance criterion is covered by a scheduler test that drives
the real dispatch method twice.

No critical issue requires human intervention for this ticket.

## Source commit

Ticket-owned source was committed through Lisa's isolated transaction:

```text
557e363b757b58c398208fa128909fe1a153c99e
feat: mint attempt leases on dispatch
```

The commit contains exactly:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-plugin/src/lib.rs`.

The ordinary Git index remained unused and is empty. Both ticket-owned source
paths are clean after the commit. Pre-existing unrelated working-tree changes
were not included or modified.

The globally installed Lisa binary did not yet contain `commit-ticket`, so the
transaction was invoked through this repository's current `lisa-cli` package.
It returned the commit ID above and preserved the same isolated-index contract.

## Files modified

### `crates/lisa-core/src/types.rs`

Added an optional public field to `Thread`:

```rust
pub attempt_lease: Option<AttemptLease>
```

The field has serde defaulting and is omitted when absent. This preserves
deserialization of thread records created before attempt leases existed.

`Thread::new` initializes the field to `None`. Generic thread construction does
not grant authority; only a scheduler dispatch with access to the predecessor
registry can stamp a lease.

Existing core tests now explicitly assert that newly constructed and legacy
deserialized threads have no lease.

### `crates/lisa-plugin/src/lib.rs`

Added `current_leases: HashMap<TicketId, AttemptLease>` to `State`.

This map is the scheduler-owned latest lease for each ticket and supplies the
predecessor for the next dispatch. Its process-local lifetime matches the scope
of this ticket.

Added `attempt_lease: Option<AttemptLease>` to private `AgentSlot`.

This stamp identifies the ticket attempt assigned to a physical pane. Slot
discovery and existing test fixtures initialize it absent. Dispatch populates
it with the selected ticket's new lease. Release clears it with the slot's
ticket ID so an idle pane cannot appear to retain assignment authority.

Updated every complete `AgentSlot` fixture literal for the additive field. The
mechanical fixture changes do not alter existing ticket, transition, cooldown,
session, activity, or provider semantics.

## Dispatch behavior

`schedule_ready_tickets` now mints after the final admission checks:

- the ticket has no active thread;
- global concurrency capacity exists;
- provider-specific capacity exists;
- a compatible or recyclable slot exists;
- the pane is not awaiting human input.

Minting occurs before any dispatch side effects:

- pane rename;
- provider launch;
- `/clear` or `/exit` transition;
- prompt input;
- slot reservation;
- acknowledgment-state insertion;
- thread insertion.

This ordering prevents a pane from launching without a lease if minting fails.

The scheduler passes `current_leases.get(ticket_id)` to
`AttemptLease::mint`. It calls the helper exactly once for the dispatch. The
successful value is cloned into the registry and selected slot, then moved into
the new thread.

Mint failures fail closed. The scheduler logs an error, counts the ticket as
unscheduled, performs no assignment side effect for it, and continues to other
ready tickets.

Codex acknowledgment generations were deliberately not reused or changed.
Those generations describe provider delivery for reused Codex seats; attempt
leases describe provider-neutral scheduler authority. A Codex recovery prompt
therefore does not mint another ticket attempt.

## Release and redispatch behavior

`release_slot_for_ticket` now clears both:

- `AgentSlot::ticket_id`;
- `AgentSlot::attempt_lease`.

It does not remove the ticket's entry from `current_leases`. Retaining the
latest minted lease is required by the core mint helper to produce a strict
successor after thread and seat cleanup.

Existing lifecycle callers continue to own thread removal. The new test calls
release and removes the thread before invoking the scheduler again, mirroring
that boundary.

## Acceptance-criterion coverage

Added the plugin scheduler test:

```text
dispatch_mints_and_stamps_strictly_new_attempt_lease
```

It constructs a real temporary ticket file, scans it into a real DAG, installs
an eligible physical pane, and calls `schedule_ready_tickets`.

The first dispatch proves:

- the current lease belongs to `T-NAME`;
- its attempt ID is 1;
- the logical `Thread` carries the exact current lease;
- the assigned `AgentSlot` carries the exact current lease.

The release step proves:

- the pane's ticket reservation is cleared;
- the pane's attempt stamp is cleared;
- the first lease remains in scheduler state as the mint predecessor.

The redispatch proves:

- the successor attempt ID is 2;
- attempt 2 is strictly greater than attempt 1;
- attempt 1 does not validate against the new current lease;
- attempt 2 validates as current;
- the new thread and reassigned slot carry attempt 2 exactly.

The test uses Claude to demonstrate the provider-neutral scheduler path. The
mint/store/stamp code is outside every provider-specific launch branch, and the
full suite covers Codex and Claude fresh/reuse dispatch behavior through the
same path.

## Verification performed

Focused plugin compilation passed:

```bash
cargo check -p lisa-plugin
```

Focused core lease coverage passed:

```bash
cargo test -p lisa-core attempt_lease
```

Result: 5 passed, 0 failed.

Focused thread compatibility coverage passed:

```bash
cargo test -p lisa-core thread_run_meta_defaults
cargo test -p lisa-core thread_deserializes_without_run_meta
```

Result: 2 passed, 0 failed across the two invocations.

Focused scheduler acceptance coverage passed:

```bash
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
```

Result: 1 passed, 0 failed.

Repository formatting passed:

```bash
cargo fmt --all -- --check
```

The complete workspace suite passed:

```bash
cargo test --workspace
```

Result: 693 passed, 0 failed:

- lisa CLI: 270 tests;
- lisa core: 155 tests;
- lisa plugin: 268 tests;
- doc tests: 0 failures.

The repository quick check also passed:

```bash
just check
```

This independently passed the plugin's `wasm32-wasip1` check and reran all
workspace tests.

`git diff --check` passed before the isolated commit.

## Coverage assessment

Coverage is sufficient for the stated acceptance criterion and compatibility
risk:

- the prerequisite lease helper retains its five monotonicity/error tests;
- legacy thread JSON compatibility is explicitly checked;
- the real scheduler method proves first dispatch and redispatch;
- equality across authoritative, logical, and physical state is explicit;
- release cleanup and predecessor retention are explicit;
- full scheduler regressions cover the existing provider lifecycle branches;
- WASM compilation covers the deployed target.

There is no direct test that forces scheduler mint failure at `u64::MAX`. The
core helper already proves exhaustion fails, while constructing a corrupted
private registry solely for a logging assertion would add little coverage. The
dispatch branch is straightforward and fails before side effects.

There is no restart/persistence test because lease durability across plugin
restart is outside this ticket's contract.

## Open concerns and known limitations

### Revocation is not implemented here

The latest lease remains in `current_leases` after release so it can serve as
the redispatch predecessor. This ticket does not revoke authority or fence the
old pane before release. `T-034-01-03` owns that ordering and may separate
current authority from the retained high-water lease.

Until that follow-up lands, downstream surfaces do not reject activity from a
prior attempt based on this registry. This is an explicit story boundary, not a
hidden completion claim.

### State is process-local

`current_leases` derives from `State::default` and is not persisted. Plugin
restart can therefore restart attempt IDs at 1. The ticket requires strict
monotonicity across scheduler dispatch/release/redispatch in the active state;
it does not specify cross-restart durability.

If leases later become durable external authority, the high-water registry
will need persistence or reconstruction before dispatch.

### Registry retention can grow

The map retains one small lease per ticket seen by the plugin. It is bounded by
the number of tickets dispatched during the process and is negligible for the
current scheduler scale. A future long-lived daemon with unbounded ticket churn
may need pruning tied to durable high-water storage.

### Legacy/manual fixtures remain unstamped

`Thread::new` and direct `AgentSlot` fixtures can represent running-looking
state without a lease. This is necessary for compatibility and existing unit
tests, but later enforcement must treat `None` as unauthorized rather than
implicitly current.

### Lease is not surfaced

Attempt IDs are not yet included in dashboard rows, pane names, signals, or
provenance. The ticket only requires scheduler recording and thread/seat
stamping. Later stale-surface tickets can consume the values without changing
the mint contract.

## TODO ownership

- `T-034-01-03`: revoke the old lease and fence its pane before rescheduling.
- S-034-02 tickets: require the exact current lease at acknowledgment,
  liveness, artifact, completion, and provenance surfaces.
- S-034-03: prove split-brain prevention in the regression/live harness.

No TODO remains within `T-034-01-02` acceptance scope.

## Ticket and artifact state

The ticket's phase and status frontmatter were not edited by this work. The six
workflow artifacts are present in `docs/active/work/T-034-01-02/`:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

Lisa owns phase transitions, final Done publication, and the completion commit
for the ticket and work artifacts.

## Final assessment

The implementation satisfies the ticket acceptance criterion with one clear
authority source and exact stamps on the assigned physical and logical run
records. It preserves provider acknowledgment semantics, fails closed before
dispatch side effects, maintains legacy thread compatibility, passes full
native and WASM verification, and leaves revocation/fencing at the explicitly
sequenced next-ticket boundary.
