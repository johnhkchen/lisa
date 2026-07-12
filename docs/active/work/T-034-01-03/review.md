# T-034-01-03 Review — revoke and fence before reschedule

## Review outcome

The implementation satisfies the ticket acceptance criterion.

On both hard-silence reclaim paths, Lisa now revokes the prior attempt's lease,
closes and terminally fences its pane, and only then releases the slot and
removes the thread.

The scheduler acceptance test asserts the exact safety order and then proves a
successor dispatch uses another eligible pane with a strictly higher attempt
ID.

No critical issue requires human intervention before this ticket proceeds
through Lisa's completion transaction.

## Source commit

```text
95bd8efa5360a5c6bdc5084308b068e4835459b7
fix: revoke and fence timed-out attempts
```

The commit was created through Lisa's isolated transaction and contains exactly:

```text
crates/lisa-plugin/src/lib.rs
```

The ordinary Git index remained empty.

The ticket-owned source path is clean after the commit.

Unrelated pre-existing working-tree changes were preserved and excluded.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Implemented lease-authority separation, central release revocation, terminal
pane fencing, hard-timeout integration, and scheduler regression coverage.

## Files created

Created the six RDSPI artifacts under:

```text
docs/active/work/T-034-01-03/
```

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

These artifacts are intentionally not part of the ticket-owned source commit.
Lisa owns their final isolated completion transaction.

## Files deleted

None.

## Lease model change

The prior scheduler used `current_leases` for two incompatible meanings:

- the latest attempt ever minted, retained for monotonic redispatch;
- the attempt currently authorized to own the ticket.

Those meanings diverge at revocation.

The scheduler now maintains:

```text
lease_high_water[ticket] = latest attempt minted in this process
current_leases[ticket]   = attempt currently authorized, if any
```

Dispatch mints from `lease_high_water` and installs the same new lease into
both maps before any provider lifecycle side effect.

It then stamps that lease onto the selected physical slot and logical thread.

Revocation removes only `current_leases[ticket]`.

Release retains `lease_high_water[ticket]`, so a successor remains strictly
monotonic even while no attempt is currently authorized.

This makes absence from `current_leases` truthful: no attempt owns the ticket.

## Shared release safety boundary

`release_slot_for_ticket` now revokes current authority at method entry.

That removal is idempotent. The hard-timeout path has already revoked before
fencing, while ordinary completion, error, audit, stale-slot, and manual-reset
callers receive the same fail-closed release invariant automatically.

Consequently, no release caller can expose a ticket for later scheduling while
the prior attempt still validates against scheduler current authority.

Normal successful release behavior remains intact:

- a healthy resident provider session can remain alive;
- wind-down cooldown is preserved;
- idle pane naming is preserved;
- same-provider reuse and cross-provider recycle paths are unchanged.

## Pane fencing behavior

Added a production wrapper around Zellij's `close_terminal_pane`.

Hard-silent attempts are terminated at the terminal-pane/process boundary
rather than being asked to exit cooperatively.

Added private `TransitionState::Fenced`.

This state is terminal and bounded:

- it has no transition deadline;
- it arms no timer;
- it has no retry path;
- it never returns automatically to `Idle`;
- slot selection already requires `Idle`, so it is never reused.

The fence helper also clears pane-scoped state that could outlive the closed
terminal:

- seat assignment;
- awaiting-human marker;
- attention debounce;
- deferred Enter keypresses.

It marks the slot non-resident, clears client/cooldown transition metadata, and
then issues the pane close.

Shared release subsequently clears the ticket and attempt stamps while
preserving `Fenced`.

The closed pane is not renamed because it no longer exists.

## Bounded fence outcomes

The private `FenceOutcome` enum names every helper result:

- `Fenced { pane_id }`;
- `AlreadyFenced { pane_id }`;
- `NoAssignedPane`.

An already-fenced pane is not closed twice.

A missing pane does not retain lease authority and does not create an infinite
retry. The inconsistency is logged and logical teardown proceeds.

The ordinary acceptance path finishes in persistent
`TransitionState::Fenced`.

## Timeout path ordering

`check_session_timeouts` retains the prior reclaim eligibility rules:

- global or per-phase budget exceeded;
- hard silence for at least `2 * stuck_threshold_secs`;
- pane is not awaiting a human response.

For eligible attempts, the relevant order is now:

1. mark logical thread failed;
2. emit timed-out provenance;
3. remove current lease authority;
4. close and mark physical pane `Fenced`;
5. invoke `release_slot_for_ticket`;
6. remove the logical thread;
7. publish timeout alert and structured activity.

The safety-critical subsequence is therefore exactly:

```text
LeaseRevoked -> PaneFenced -> SlotReleased
```

`detect_stale_threads`, the pure doubled-stuck-threshold path, uses the same
revoke/fence helper before release.

It retains its existing failed provenance and stale error reporting.

## Acceptance-criterion coverage

The strengthened scheduler test is:

```text
test_check_session_timeouts_expired
```

It installs a real attempt 1 consistently in:

- high-water history;
- current authority;
- the hard-silent logical thread;
- the assigned physical pane.

It also creates a second eligible pane for later redispatch.

After running the real timeout scan, the test asserts the test-only lifecycle
trace equals exactly:

```text
LeaseRevoked(T-001)
PaneFenced(T-001, pane 1)
SlotReleased(T-001)
```

The trace storage exists only under `cfg(test)`; production state has no event
journal overhead.

The same test verifies final timeout state:

- old lease fails `is_current` because current authority is absent;
- attempt 1 remains only as the high-water predecessor;
- pane 1 has no ticket, attempt stamp, resident session, or cooldown;
- pane 1 remains `TransitionState::Fenced`;
- the old logical thread is absent;
- timeout alert and `SessionTimedOut` activity remain present.

The test then calls the real scheduler and verifies:

- pane 1 remains fenced and unselected;
- pane 2 receives the ticket;
- the successor lease is attempt 2;
- attempt 2 is strictly greater than attempt 1;
- attempt 1 is not current;
- attempt 2 is current;
- high-water, current, slot, and thread all equal attempt 2.

This directly proves the old pane and lease cannot re-own the ticket through
the scheduler rescheduling path.

## Additional regression coverage

`dispatch_mints_and_stamps_strictly_new_attempt_lease` now proves the refined
authority model during ordinary release:

- first dispatch populates high-water/current/stamps with attempt 1;
- release removes current authority;
- release retains only high-water attempt 1;
- redispatch populates every location with attempt 2.

`test_detect_stale_threads` now uses a real lease and proves the independent
pure-stale reclaim path also:

- removes current authority;
- retains high-water history;
- leaves its pane terminally fenced.

Existing tests continue to cover:

- over-budget but active sessions are not reclaimed;
- awaiting-human sessions are not reclaimed;
- timeout reclaim resumes after the awaiting marker clears;
- stale detection respects configured thresholds;
- normal release preserves resident session reuse;
- cooldown behavior;
- Claude and Codex reuse;
- cross-provider recycling;
- acknowledgement recovery;
- completion ordering and dependency scheduling.

## Verification performed

Focused dispatch coverage passed:

```text
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
1 passed; 0 failed
```

Focused acceptance coverage passed:

```text
cargo test -p lisa-plugin test_check_session_timeouts_expired
1 passed; 0 failed
```

Focused stale coverage passed:

```text
cargo test -p lisa-plugin test_detect_stale_threads
2 passed; 0 failed
```

The complete plugin suite passed:

```text
cargo test -p lisa-plugin
268 passed; 0 failed
```

The complete workspace suite passed:

```text
cargo test --workspace
```

Results:

- Lisa CLI: 270 passed;
- atomic provider contract integration: 1 passed;
- Lisa core: 155 passed;
- Lisa plugin: 268 passed;
- doc tests: 0 failures.

Total: 694 passed, 0 failed.

The repository quick check passed:

```text
just check
```

The deployed plugin target passed explicitly:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Plugin library Clippy passed with warnings denied:

```text
cargo clippy -p lisa-plugin --lib -- -D warnings
```

Formatting and whitespace checks passed:

```text
cargo fmt --all -- --check
git diff --check
```

## Test coverage assessment

Coverage is sufficient for the acceptance criterion and the principal
regression risks.

The strict order is asserted directly rather than inferred only from final
state.

The real scheduler is exercised after timeout, not a hand-written successor
mint.

Both hard-silence callers are covered.

Normal provider reuse and completion behavior is covered by the full existing
suite.

The production pane-close API is compile-checked on the deployed WASM target.

## Coverage gaps

There is no live Zellij integration test that observes the operating system
process exiting after `close_terminal_pane`.

Native tests intentionally replace the host call with state/order observation;
the Zellij API is compile-checked for WASM. A live process-level regression
belongs to S-034-03's harness/live proof.

There is no automatic replacement-pane test because replacement is not
implemented in this ticket.

The `AlreadyFenced` and `NoAssignedPane` outcomes are simple idempotent/error
boundaries and are not exercised by dedicated tests. The acceptance path and
both real hard-silence callers exercise `Fenced`.

## Compatibility assessment

All source changes are private to `lisa-plugin`.

No public Rust API changed.

No serialized type or configuration key changed.

No CLI, hook, adapter, layout, signal format, pane-name format, or provenance
schema changed.

Normal release still supports resident session reuse.

Legacy/manual fixtures without leases remain valid; revocation becomes a
no-op, while a hard timeout still fences their assigned pane.

## Open concerns and known limitations

### Fencing permanently consumes a slot

Closing a terminal pane guarantees the old process cannot continue, but the
current slot-discovery implementation runs once and does not recreate panes.

A fenced slot therefore remains unavailable for the rest of the plugin run.

If another idle pane exists, the ticket can redispatch immediately, as the
acceptance test demonstrates.

If no other pane exists, the ticket remains startable but unscheduled until an
operator or future recovery mechanism restores capacity.

This is a named bounded failure mode and intentionally avoids automatic retry
or unsafe reuse. Replacement-pane lifecycle is a potential future ticket.

### High-water state is process-local

`lease_high_water` is not persisted. A plugin restart can restart attempt IDs
at 1.

The current story establishes monotonicity within active scheduler state.
Durable authority across plugin restart remains unspecified and should be
resolved before leases are treated as cross-process durable identities.

### Surface rejection is still later work

This ticket establishes scheduler authority and physical fencing at the timeout
boundary.

S-034-02 still owns exact-current admission at acknowledgement, liveness,
artifact, completion, and provenance surfaces.

Until those gates land, the current map is authoritative scheduler state but
individual stale signal payloads do not yet carry/validate attempt leases.

### Pane close is fire-and-forget

Zellij's close command does not provide a completion callback to this code.

The scheduler treats the pane as fenced immediately and never reuses its ID.
This is safe for ownership even if host teardown is asynchronous, because no
successor input is sent to the fenced pane.

## Human review focus

A reviewer should confirm these intentional operational choices:

1. hard-silent attempts warrant closing the terminal pane rather than a
   provider-specific graceful exit;
2. permanent capacity reduction is the desired bounded behavior until pane
   replacement exists;
3. current-authority and high-water maps should remain separate state concepts;
4. S-034-02 will consume `current_leases` as the exact authority source.

## Ticket and artifact state

The agent did not edit ticket phase or status frontmatter. Lisa detected phase
artifacts during execution and owns all resulting phase transitions.

All six workflow artifacts are present.

Lisa owns final Done publication, the ticket/work-artifact completion commit,
and seat release.

## Final assessment

The implementation closes the timeout split-brain window at the scheduler
boundary.

Prior authority is absent before release, the old physical writer is closed and
permanently disqualified before release, and any successor dispatch is minted
from retained high-water history with a strictly greater attempt ID.

The acceptance criterion is met with direct ordering evidence, bounded state,
end-to-end scheduler redispatch coverage, full regression tests, and deployed-
target compilation.
