# T-035-01-03 Review — gate Owned on observed start

## Review outcome

The ticket is complete. Fresh native provider launches no longer publish a physical seat
as `Owned` at dispatch time. They publish an explicit, visible, lease-scoped `Starting`
state and become `Owned` only when Lisa consumes the exact current process-start signal.

No critical issue was found in self-review. The intentionally open startup-timeout case
is owned by the dependent T-035-01-04 ticket.

## Source change summary

### `crates/lisa-plugin/src/lib.rs`

Added `SeatAssignmentState::Starting { generation }` as scheduler-owned truth for a
reserved fresh seat whose provider process has not yet positively reported startup.

Added `acknowledge_process_start`, which requires all authority layers to agree:

- the pane is currently in `Starting`;
- its expected generation equals the candidate attempt ID;
- the pane resolves to a reserved slot;
- the slot ticket matches the candidate ticket;
- the slot attempt lease exactly equals the candidate;
- the candidate remains current in `current_leases`.

Only then does the method replace the assignment with `Owned`. Any other state,
including already owned, fails closed and makes duplicate delivery inert.

Added `check_process_start_signals`, which scans for `pane-<id>.started`, parses the
payload as `AttemptLease`, removes the recognized file, and attempts exact admission.
The consume-before-admit behavior matches the existing heartbeat fencing pattern and
prevents malformed or stale files from replaying indefinitely.

Added the scanner to `poll_tick` immediately after heartbeat consumption. Positive start
observation therefore becomes scheduler truth before later health or timeout decisions.

Changed scheduling to classify fresh provider process routes. Unused panes,
cross-provider recycling, and `FreshExec` routes now enter `Starting`. In-process
clear-handshake reuse does not. The existing Codex prompt acknowledgment remains the
gate for reused native Codex, and native Claude same-process reuse remains owned.

Updated the internal-to-dashboard mapping and native tests.

### `crates/lisa-plugin/src/ui.rs`

Added `SeatAssignmentStatus::Starting`, rendered as `starting` in yellow. This uses the
existing active-thread seat-status column, so the pending condition is visible without
new dashboard structure.

## Acceptance criteria assessment

### Native scheduler test drives a fresh dispatch

Met by `test_fresh_dispatch_becomes_owned_only_after_exact_process_start`.

The test creates an empty native Claude pane, runs `schedule_ready_tickets`, and inspects
the actual scheduler-minted attempt lease and resulting assignment.

### Immediately pending, not Owned

Met. The test asserts:

- `Starting { generation: lease.attempt_id }` immediately after dispatch;
- `seat_is_owned(10) == false`;
- the dashboard row contains `starting`.

The existing fresh Codex route test also now asserts `Starting` and not owned.

### Flips only after observed start

Met. The test writes the exact current lease to `pane-10.started`, calls the real signal
scanner, and asserts `Owned`, `seat_is_owned == true`, signal consumption, and an `owned`
dashboard row.

Before the exact signal, malformed and stale-generation signals are consumed without
promotion. A duplicate exact signal after ownership also cannot perform another edge.

### Pending state surfaces via seat-status label

Met. `Starting` maps into `SeatAssignmentStatus::Starting` and renders `starting` in the
same active-thread status cell used by assigned-pending-ack and recovery labels.

### E-033 and E-034 remain green

Met. The complete plugin suite passed with 277 tests, including recycled Codex exact
acknowledgment, dropped-ack bounded recovery, fresh-generation recovery acknowledgment,
same-process Claude reuse, consecutive pane reuse, stale heartbeat rejection, and the
deterministic split-brain fencing timeline.

## Test coverage

Direct new coverage includes:

- real fresh scheduler dispatch;
- exact assignment generation;
- immediate non-owned state;
- dashboard pending and owned labels;
- malformed payload rejection and consumption;
- stale generation rejection;
- exact current lease promotion;
- duplicate signal idempotence.

Existing regression coverage exercised:

- reused Codex `AssignedPendingAck -> Owned` behavior;
- reused Codex finite recovery behavior;
- recovery generation fencing;
- native Claude reuse ownership;
- cross-provider exit/launch transitions;
- attempt lease monotonicity and current-authority checks;
- stale predecessor liveness and publication rejection;
- split-brain fencing and single authoritative completion.

Verification results:

```text
cargo fmt --all -- --check                                passed
cargo test -p lisa-plugin test_fresh_dispatch...          1 passed
cargo test -p lisa-plugin                                 277 passed
cargo test --workspace                                    passed
```

## Commit and repository hygiene

Source commit:

```text
5cd47a9343270c5a529a84d990b98d4ae12d4e0c
feat(plugin): gate fresh ownership on process start
```

The commit was created with Lisa's isolated `commit-ticket` transaction and exact
repository-relative includes for `lib.rs` and `ui.rs`. Both source paths are clean after
the commit. The ordinary Git index is unchanged and empty of staged paths. Concurrent
non-ticket working-tree changes remain outside the commit.

## Open concerns and limitations

### Missing start signal has no bounded outcome yet

`Starting` remains pending indefinitely if the provider never emits `.started`, subject
to existing broader health/session mechanisms. This is deliberate, not an omitted part
of this ticket: T-035-01-04 owns bounded named startup recovery/failure and can extend
the explicit starting state without coupling it to E-033's prompt-ack deadline.

### Honest test boundary

The test is a native scheduler/fixture test. It does not launch a real Zellij PTY or an
installed provider. The parent story assigns real-Zellij and installed-provider coverage
to S-035-02 and S-035-03 respectively.

### Signal directory scan cost

The new consumer performs another small directory scan per poll, consistent with the
existing signal consumers. Existing 32-pane signal scan coverage remains green. A future
consolidated scanner could reduce directory passes, but that is unrelated refactoring
and not required for correctness.

## Human review focus

The highest-value review points are:

1. confirm the fresh-route classification matches every branch that starts a process;
2. confirm exact lease admission is equivalent to the established heartbeat boundary;
3. confirm T-035-01-04 extends `Starting` rather than reusing Codex ack recovery fields;
4. confirm the operator-facing `starting` label is the desired wording.

## Final state

- Research artifact complete.
- Design artifact complete.
- Structure artifact complete.
- Plan artifact complete.
- Implementation complete and committed.
- Progress artifact complete.
- Review artifact complete.
- Ticket frontmatter was not manually changed.
- No shared work artifact path was directly written by this attempt.
- The agent remains assigned to T-035-01-03 pending Lisa's completion commit.
