# Structure: T-039-02-01

## File inventory

### Modified

`crates/lisa-plugin/src/lib.rs`

- Add one `#[cfg(test)]` child-module declaration inside `mod tests`.
- Do not edit `State::poll_tick`.
- Do not edit any `check_*_signals` method.
- Do not alter production types, helpers, constants, or visibility.

### Created

`crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

- Hold the complete named characterization suite.
- Import private test-module and crate members with `use super::*`.
- Define only test fixtures and `#[test]` functions.
- Keep all filesystem fixtures within `tempfile` directories.

### Attempt-private artifacts

- `.lisa/attempts/T-039-02-01/1/work/research.md`
- `.lisa/attempts/T-039-02-01/1/work/design.md`
- `.lisa/attempts/T-039-02-01/1/work/structure.md`
- `.lisa/attempts/T-039-02-01/1/work/plan.md`
- `.lisa/attempts/T-039-02-01/1/work/progress.md`
- `.lisa/attempts/T-039-02-01/1/work/review.md`

Lisa owns publication of these files and ticket frontmatter transitions.

## Test module organization

The child module will contain:

1. A small `consumer_state` fixture.
2. A helper for adding a pane slot and running thread.
3. A helper for writing a current attempt lease.
4. A poll-order test.
5. A legacy-name and rejected-record deletion matrix.
6. A heartbeat effect test.
7. A process-start effect test.
8. A shell-ready characterization test.
9. An acknowledgement effect test.
10. An awaiting effect test.
11. An idle legacy/effect test.
12. A transition presence/effect test.
13. An error effect test.

Tests may reuse `install_current_attempt` and existing scheduling fixtures from
the parent test module where doing so keeps state construction faithful.

## Poll-order test boundary

The test will isolate the text of `fn poll_tick` from the rest of `lib.rs`.
It will search for these call strings in this exact sequence:

1. `self.check_heartbeat_signals();`
2. `self.check_awaiting_signals();`
3. `self.check_process_start_signals();`
4. `self.check_shell_ready_signals();`
5. `self.check_codex_ack_signals();`
6. `self.check_idle_signals();`
7. `self.check_transition_signals();`
8. `self.check_error_signals();`

The cursor advances after every match. A missing or reordered call fails with
the consumer name. The slice ends before `fn update`, preventing matches in
tests or unrelated methods from satisfying the assertion.

## Contract-matrix boundary

The negative/legacy matrix uses one temp signal directory per case so directory
iteration order cannot influence results. Each case records:

- consumer label;
- current recognized filename;
- rejected body or inapplicable state;
- invocation function;
- expected deletion of the recognized record;
- legacy ticket-named filename;
- expected preservation of an unrecognized record.

Idle is the intentional exception: `T-LEGACY.idle` is recognized and deleted.
Its current pane form is also recognized and deleted even with no assigned slot.

Rust function pointers or a compact match over consumer labels can drive the
matrix. Explicit match arms are preferred when setup differs, keeping failures
diagnostic and avoiding a product abstraction in test code.

## Positive fixture boundary

### Heartbeat

- Slot pane 7 owns `T-SIGNAL`.
- Running thread pane 7 exists.
- `install_current_attempt` stamps slot, thread, and current authority.
- Awaiting and notified markers start populated.
- A matching lease body is written.
- Assertions cover deletion, clocks, and marker clearing.

### Process start

- Slot pane 7 owns a current lease.
- Seat assignment starts in `Starting` for the lease generation.
- A stale record is consumed without promotion.
- A matching record promotes to `ReadyForAssignment`.

### Shell ready

- Use the existing scheduling/recovery fixture to produce the genuine
  `ResettingStartup` successor state.
- A predecessor payload is consumed without relaunch.
- A successor payload is consumed and produces replacement `Starting`.
- If reuse of the larger fixture is too coupled, directly characterize malformed
  deletion here and rely on the existing recovery test for the positive edge.

### Ack

- Slot and current authority use a fixed attempt lease.
- Seat assignment is pending/delivering for that generation.
- A stale tagged payload is consumed without ownership.
- An exact tagged provider payload produces `Owned`.

### Awaiting

- No slot or thread is required.
- Arbitrary body content demonstrates presence-only admission.
- Effect is insertion into `awaiting_human`.
- `last_activity_at` remains unchanged when a slot is supplied.

### Idle

- A minimal ticket DAG contains `T-LEGACY` in Research.
- A running thread exists for the ticket.
- No research artifact exists.
- Legacy `T-LEGACY.idle` is consumed.
- The effect is one idle-without-artifact alert.
- This avoids ticket phase mutation while exercising the legacy route.

### Transition

- Pane slot is in `Idle`, avoiding native host input calls.
- Arbitrary stopped body is accepted by presence.
- File deletion and activity refresh are asserted.
- Malformed pane-prefixed stopped names demonstrate delete-before-parse.
- Ticket-named transition records remain untouched.

### Error

- A running thread owns pane 7.
- Arbitrary, non-JSON body demonstrates presence admission.
- Consumption removes the record and running thread.
- Slot release and error alert are asserted.

## Public interface

No public interface is introduced. No production visibility is widened. The
suite's stable interface is its test-name prefix and module path, allowing the
next ticket to run it unchanged with a Cargo test filter.

## Ordering of implementation

1. Add the module file and declaration.
2. Implement order and negative matrix tests.
3. Implement positive consumer effect tests.
4. Format and run the narrow suite.
5. Adjust test fixtures only, never production behavior.
6. Run full gates.
7. Commit both exact source paths as one meaningful test unit.
8. Complete progress and review artifacts.

## Ownership and commit boundary

The meaningful source unit consists of exactly:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

The commit command must name both exact paths. Lisa-owned changes to
`.lisa/provenance.jsonl` and the active ticket must remain outside the isolated
transaction. Attempt artifacts are published separately by Lisa.

