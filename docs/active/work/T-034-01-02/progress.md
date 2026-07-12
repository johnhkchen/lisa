# T-034-01-02 Progress — mint lease on dispatch

## Status

Implementation and verification are complete. Ticket-owned source is committed
through Lisa's isolated transaction. Only final review remains.

## Completed — core thread stamp

Modified `crates/lisa-core/src/types.rs`:

- added `Thread::attempt_lease: Option<AttemptLease>`;
- applied serde default and omitted absent values from serialization;
- initialized generic `Thread::new` records without authority;
- extended default and legacy-deserialization tests.

The constructor signature is unchanged, so existing callers and fixtures do
not need to fabricate leases. Scheduler dispatch is the only new stamping site.

## Completed — scheduler authority registry

Modified `State` in `crates/lisa-plugin/src/lib.rs`:

- added `current_leases: HashMap<TicketId, AttemptLease>`;
- kept it adjacent to the active thread registry;
- relied on `State::default` for empty initialization.

The registry retains the latest successful lease across physical seat release.
This is both the current dispatch authority for this slice and the high-water
predecessor required to mint a successor on redispatch.

## Completed — physical seat stamp

Modified private `AgentSlot`:

- added `attempt_lease: Option<AttemptLease>` beside `ticket_id`;
- initialized slot discovery and all direct test fixtures with no lease;
- stamped the selected slot during dispatch;
- cleared the stamp alongside `ticket_id` during release.

Updating all complete slot literals was a mechanical compatibility change. No
fixture's ticket, transition, session, cooldown, activity, or provider state was
otherwise changed.

## Completed — dispatch mint boundary

`schedule_ready_tickets` now mints after:

- active-thread filtering;
- global concurrency enforcement;
- provider-cap enforcement;
- compatible/recyclable slot selection;
- awaiting-human rejection.

It mints before:

- pane rename;
- `/clear`, `/exit`, launch, or prompt input;
- slot reservation;
- provider acknowledgment state insertion;
- thread creation.

The method calls `AttemptLease::mint` once per admitted dispatch, using the
ticket's retained registry entry as predecessor. A failure logs an error,
counts the ticket as unscheduled, and continues without dispatch side effects.

The successful lease is cloned into `current_leases` and the selected slot, then
moved onto the created thread. Codex assignment generations remain untouched.

## Completed — release behavior

`release_slot_for_ticket` clears the physical seat lease with its ticket ID.
It deliberately does not remove the per-ticket registry entry.

Thread removal remains owned by existing lifecycle callers. The scheduler test
models that behavior explicitly before redispatch.

## Completed — acceptance test

Added:

`dispatch_mints_and_stamps_strictly_new_attempt_lease`

The test uses a real temporary ticket file, scanner, DAG, state, slot, and the
real scheduling method. It proves:

- first dispatch records attempt 1;
- current registry, logical thread, and physical seat carry equal leases;
- release clears the seat ticket and lease;
- release retains the first lease as high-water predecessor;
- redispatch records attempt 2;
- attempt 2 is strictly higher than attempt 1;
- attempt 1 no longer validates against the successor;
- the redispatched thread and seat carry attempt 2.

Cooldown is set to an elapsed instant after release to make the test independent
of system clock granularity. Production cooldown behavior is unchanged.

## Focused verification completed

Formatting was applied directly to the two ticket-owned Rust files with:

```bash
rustfmt --edition 2021 crates/lisa-core/src/types.rs crates/lisa-plugin/src/lib.rs
```

Plugin compilation passed:

```bash
cargo check -p lisa-plugin
```

Result: success.

Core lease contract passed:

```bash
cargo test -p lisa-core attempt_lease
```

Result: 5 passed, 0 failed.

Thread default compatibility passed:

```bash
cargo test -p lisa-core thread_run_meta_defaults
```

Result: 1 passed, 0 failed.

Legacy thread deserialization passed:

```bash
cargo test -p lisa-core thread_deserializes_without_run_meta
```

Result: 1 passed, 0 failed.

Scheduler acceptance coverage passed:

```bash
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
```

Result: 1 passed, 0 failed.

## Diff inspection completed

The ticket-owned source diff currently contains:

- 9 added lines in `crates/lisa-core/src/types.rs`;
- scheduler/storage/test changes plus mechanical slot literal initialization in
  `crates/lisa-plugin/src/lib.rs`.

No unrelated dirty file was edited by the implementation. The repository had
pre-existing modifications and untracked files outside these two source paths;
they remain outside ticket ownership.

## Deviations from plan

The planned file shape and ordering were followed.

One mechanical implementation detail differed: instead of adding the new
`AgentSlot` field to dozens of complete test literals manually, a constrained
bulk rewrite inserted `attempt_lease: None` only where `ticket_id` was directly
followed by `has_session`. Compilation and diff inspection verified the result.

No architectural or behavioral deviation occurred.

## Full verification completed

Repository formatting passed:

```bash
cargo fmt --all -- --check
```

The full workspace suite passed:

```bash
cargo test --workspace
```

Result: 693 passed, 0 failed across CLI (270), core (155), and plugin
(268), with 0 doc-test failures.

The repository quick check passed:

```bash
just check
```

This included:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- another complete `cargo test --workspace` run.

`git diff --check` passed for both ticket-owned source files.

## Isolated commit completed

The globally installed `/opt/homebrew/bin/lisa` was stale and rejected the
documented `commit-ticket` subcommand before making any state change. The same
command was then run through the repository's current `lisa-cli` package:

```bash
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-01-02 \
  --message "feat: mint attempt leases on dispatch" \
  --include crates/lisa-core/src/types.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
557e363b757b58c398208fa128909fe1a153c99e
```

Commit subject:

```text
feat: mint attempt leases on dispatch
```

Commit inspection confirms it contains exactly:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Both source paths are clean after the commit. The ordinary Git index is empty.
Pre-existing unrelated modifications/untracked files remain untouched. The
ticket and workflow artifacts remain untracked for Lisa's final completion
transaction, as expected.

## Remaining

1. Write `review.md`.
2. Stop without editing ticket phase/status or publishing Done.

## Known boundary for later tickets

This ticket retains the latest lease after release to preserve monotonic mint
history. It does not yet revoke authority or fence the old pane before making a
ticket reschedulable. `T-034-01-03` owns that ordering and may refine the
registry into separate current-authority and high-water concepts.

No stale acknowledgment, liveness, artifact, completion, or provenance surface
is gated here. Those responsibilities remain in S-034-02.
