# T-034-01-02 Plan — mint lease on dispatch

## Objective

Implement a scheduler-lifetime attempt lease registry and stamp each admitted
dispatch onto its current ticket record, physical slot, and logical thread.
Prove that release followed by redispatch mints a strictly higher attempt.

## Step 1 — add the thread lease field

Modify `crates/lisa-core/src/types.rs`.

- Add optional `attempt_lease` to `Thread`.
- Apply serde default and omit absent values from serialization.
- Initialize it to `None` in `Thread::new`.
- Document that only scheduler dispatch stamps authority.

Verification:

- `Thread::new` compiles at all existing call sites without signature changes.
- Existing serialized fixtures without the field deserialize.
- No lease is fabricated by generic construction.

Atomic unit: public thread representation and compatibility behavior.

## Step 2 — add scheduler and seat storage

Modify `crates/lisa-plugin/src/lib.rs`.

- Import `AttemptLease` with the other core types.
- Add `current_leases` to `State` beside `threads`.
- Add `attempt_lease` to `AgentSlot` beside `ticket_id`.
- Initialize discovered and fixture-created slots without a lease.

Use compiler errors plus `rg "AgentSlot \{"` to locate every complete struct
literal. Do not rewrite unrelated fixture semantics.

Verification:

- `cargo check -p lisa-plugin` reaches scheduler behavior without missing-field
  errors.
- `State::default` starts with no current leases.
- all idle-slot construction starts unstamped.

Atomic unit: storage shape required by dispatch.

## Step 3 — mint before dispatch side effects

In `schedule_ready_tickets`, after all scheduling gates and before pane rename or
input:

- pass `current_leases.get(&ticket_id)` to `AttemptLease::mint`;
- store the successful successor in `current_leases`;
- on error, log and leave the ticket unscheduled;
- do not allocate a lease for cap-blocked, slot-blocked, paused, or
  attention-blocked tickets.

Verification:

- there is exactly one `AttemptLease::mint` call per admitted dispatch;
- a mint error cannot reach rename, send, slot reservation, or thread creation;
- unrelated ready tickets can still be considered after one mint failure.

Atomic unit: authoritative mint operation.

## Step 4 — stamp seat and thread

Still in `schedule_ready_tickets`:

- assign a clone to the selected slot when its ticket reservation is written;
- assign the local lease to the new `Thread` before insertion;
- keep Codex acknowledgment generation behavior unchanged.

Verification:

- the map, selected slot, and thread compare equal after dispatch;
- both fresh and reuse branches converge on the shared stamping code;
- no provider-specific branch mints or modifies the attempt.

Atomic unit: propagation of one authority value.

## Step 5 — clear only the physical seat on release

In `release_slot_for_ticket`:

- clear the matching slot's attempt lease with its ticket ID;
- retain the scheduler's per-ticket current/high-water entry;
- retain existing seat acknowledgment removal and cooldown behavior.

Verification:

- idle slots do not carry stale attempt authority;
- redispatch still has the first lease as predecessor;
- thread lifecycle behavior is unchanged.

Atomic unit: release cleanup and monotonic-history preservation.

## Step 6 — extend thread compatibility tests

In `lisa-core` tests:

- assert `Thread::new(...).attempt_lease` is absent;
- assert legacy JSON without lease metadata deserializes with an absent lease.

Focused command:

```bash
cargo test -p lisa-core thread_run_meta_defaults
cargo test -p lisa-core thread_deserializes_without_run_meta
```

Verification:

- both tests pass;
- existing attempt-lease tests remain green.

Atomic unit: compatibility regression coverage.

## Step 7 — add scheduler acceptance coverage

Add `dispatch_mints_and_stamps_strictly_new_attempt_lease` to plugin tests.

First dispatch:

- schedule `T-NAME`;
- clone current lease;
- assert attempt 1;
- assert slot/thread exact equality.

Release:

- call `release_slot_for_ticket`;
- remove the thread, matching production caller ownership;
- make cooldown definitively elapsed if necessary;
- assert slot stamp cleared;
- assert current predecessor retained.

Redispatch:

- schedule the same DAG-ready ticket;
- clone successor;
- assert successor attempt is strictly greater and equals 2;
- assert slot/thread exact equality with successor;
- assert old lease fails and successor passes `is_current`.

Focused command:

```bash
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
```

Atomic unit: ticket acceptance proof.

## Step 8 — format and focused verification

Run:

```bash
cargo fmt --all -- --check
cargo test -p lisa-core attempt_lease
cargo test -p lisa-core thread_run_meta_defaults
cargo test -p lisa-core thread_deserializes_without_run_meta
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
```

If formatting check fails because of ticket changes, run `cargo fmt --all`,
inspect its touched paths, and ensure unrelated user changes were not altered.

Verification criteria:

- formatting passes;
- lease core contract remains green;
- thread compatibility remains green;
- scheduler acceptance test passes.

## Step 9 — broader regression verification

Run the workspace test suite:

```bash
cargo test --workspace
```

Then run the repository quick check if practical:

```bash
just check
```

The latter covers the WASM target plus tests according to `CLAUDE.md`. Record
any environment/toolchain limitation in `progress.md` and `review.md` rather
than hiding it.

Verification criteria:

- no existing scheduler, core, CLI, or UI tests regress;
- WASM compilation accepts the new types and state shape;
- warnings do not indicate an unused or unreachable lease contract.

## Step 10 — inspect ownership and diff

Run:

```bash
git diff -- crates/lisa-core/src/types.rs crates/lisa-plugin/src/lib.rs
git status --short
git diff --cached --name-only
```

Confirm:

- source diff matches this ticket only;
- unrelated dirty files remain untouched;
- ticket frontmatter phase/status is unchanged;
- no ticket-owned file is in the ordinary index.

## Step 11 — commit the implementation through Lisa

Use the isolated ticket transaction with exact source paths:

```bash
lisa commit-ticket \
  --ticket-id T-034-01-02 \
  --message "feat: mint attempt leases on dispatch" \
  --include crates/lisa-core/src/types.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Do not include workflow artifacts; Lisa publishes ticket/work artifacts at
completion. Do not run ordinary `git add` or `git commit`.

After the command, verify the commit exists and neither source file remains
modified, untracked, or staged.

Atomic unit: complete source implementation and its tests. The two files form
one meaningful unit because the plugin cannot stamp the thread until the public
thread field exists.

## Step 12 — progress and review handoff

Maintain `progress.md` throughout implementation with:

- completed steps;
- test commands and results;
- deviations and rationale;
- isolated commit ID/message;
- remaining work.

After implementation and verification, write `review.md` summarizing:

- files and behavior changed;
- exact acceptance-criterion proof;
- test coverage and commands;
- open concerns, especially process-local high-water retention and later
  revocation work;
- critical issues, if any;
- clean ownership/commit state.

Stop after `review.md`. Do not update ticket phase/status or publish Done.

## Risk checks

### Duplicate minting

Search for the mint helper after implementation. The dispatch path should call
it once, and prompt recovery/reuse callbacks should only consume the already
stamped assignment.

### Lost monotonic history

Search release and reset paths for `current_leases.remove`. This ticket should
not add such removal. The test must prove retention through release.

### Stale seat stamp

Every path through `release_slot_for_ticket` that clears `ticket_id` must clear
the lease in the same block.

### Backward compatibility

The new thread field must be optional and serde-defaulted. A required lease
would break persisted state and extensive fixture construction.

### Provider coupling

Attempt lease mint/stamp code must sit outside Claude/Codex launch branches and
must not reuse `assignment_generation`.

## Definition of implementation complete

- Every admitted scheduler dispatch mints via the core helper.
- `State` records the new lease for the ticket.
- the selected slot and created thread carry the exact same lease.
- release clears the slot stamp but retains the predecessor.
- redispatch produces a strictly greater attempt ID.
- focused and workspace tests pass, or limitations are explicitly documented.
- ticket-owned source is committed through `lisa commit-ticket` only.
- `progress.md` is current and `review.md` provides the final handoff.
