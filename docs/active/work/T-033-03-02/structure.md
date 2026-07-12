# T-033-03-02 Structure — consecutive reuse live proof

## Change summary

The implementation adds one private native regression to the plugin test module
and a report-producing shell wrapper under this ticket's work directory. No
production module, public interface, configuration schema, or dependency is
changed.

## File inventory

### Modified

`crates/lisa-plugin/src/lib.rs`

- add a private multi-ticket reused-seat fixture builder inside `#[cfg(test)]`;
- add small test-only helpers for current assignment identity and matching ack;
- add the consecutive Codex reuse plus Claude control regression;
- emit stable evidence rows after assertions succeed for each assignment;
- retain all production state and transition definitions unchanged.

### Created

`docs/active/work/T-033-03-02/harness/run.sh`

- focused test runner;
- evidence-row extractor;
- independent count and outcome validator;
- optional Markdown renderer;
- concise PASS/failure receipts.

`docs/active/work/T-033-03-02/harness/README.md`

- usage and environment requirements;
- exact assertions;
- evidence schema;
- honest native/live-style boundary;
- report regeneration instructions.

`docs/active/work/T-033-03-02/run-report.md`

- generated execution metadata;
- Codex assignment table;
- Claude control table;
- count summary and acceptance verdict;
- limitations and broader test results.

### RDSPI artifacts

`docs/active/work/T-033-03-02/research.md`

`docs/active/work/T-033-03-02/design.md`

`docs/active/work/T-033-03-02/structure.md`

`docs/active/work/T-033-03-02/plan.md`

`docs/active/work/T-033-03-02/progress.md`

`docs/active/work/T-033-03-02/review.md`

These describe and hand off the work. Lisa owns their completion commit.

### Deleted

None.

## Plugin test-module organization

Place the new helper beside `pane_name_schedule_state`, where scheduler fixture
construction already lives. Place the regression beside the existing recycled
Codex ownership, dropped-ack, bounded recovery, and reused-Claude tests.

This keeps all access to private `State`, `SeatAssignmentState`, and
`TransitionState` inside the existing module and avoids production visibility
changes.

## Test fixture interface

Add a private helper with this conceptual interface:

```text
consecutive_reuse_state(
    provider: AgentClient,
    ticket_prefix: &str,
    pane_ids: &[u32],
    ticket_count: usize,
) -> (State, TempDir)
```

Responsibilities:

- create `ticket_count` sorted temporary ticket files;
- write explicit `agent:` frontmatter for the selected provider;
- scan files and construct the real `Dag`;
- configure `max_threads = pane_ids.len()`;
- set `wind_down_secs = 0`;
- set `assignment_ack_timeout_secs = 1`;
- enable permissions and slot discovery;
- add one resident provider-compatible slot for every pane ID.

The returned `TempDir` keeps ticket files alive for the full scenario.

The helper is test-only and has no stability promise outside the module.

## Test identity helper

The scenario repeatedly needs the exact pending or recovery generation and
deadline. Keep this extraction local to the regression or add a narrow test-only
function returning:

```text
(generation: u64, deadline: SystemTime)
```

It accepts either `AssignedPendingAck` or `Recovering` according to the caller's
expected leg and panics with the observed state on mismatch.

No corresponding production accessor is added.

## Matching-ack helper

Use a test-local function that accepts state, pane, ticket, generation, and a
short prompt label. It builds:

```json
{
  "hook_event_name": "UserPromptSubmit",
  "prompt": "<generation-tagged text>"
}
```

The helper calls the existing `acknowledge_codex_assignment` and returns its
boolean. Tests remain responsible for asserting the exact edge and final state.

This avoids repeating JSON construction 20 times without hiding the production
detector.

## Scenario orchestration

The single test has two clearly separated sections:

```text
Codex state
  five rounds
    schedule two reused seats
    sort active (ticket, pane) pairs
    resolve ordinary or forced-loss path
    emit rows
    complete threads and release slots
  assert totals and pane set

Claude control state
  five rounds
    schedule two reused seats
    sort active (ticket, pane) pairs
    assert WaitingForClear + Owned
    handle cleared signal
    assert Idle + Owned + no generation
    emit rows
    complete threads and release slots
  assert totals and pane set

emit one summary row
```

Sorting active pairs by ticket ID makes evidence sequence stable independent of
hash-map iteration.

## Active-assignment collection

Read active assignments from `agent_slots`, filtering `ticket_id.is_some()`.
Map each entry to `(ticket_id.clone(), pane_id)` and sort by ticket ID.

After each scheduling call, assert exactly two active assignments. This proves
both physical panes are reused per round and detects unexpected capacity or
cooldown behavior immediately.

Sequence numbers derive from the sorted ticket suffix, not iteration order.

## Codex ordinary path boundary

For every non-fault row:

- assert `WaitingForClear` plus unarmed `AssignedPendingAck`;
- deliver the cleared signal and assert an armed pending state;
- assert `seat_is_owned` false;
- record the generation;
- submit exact matching acknowledgment;
- assert promotion returns true;
- submit the same payload again and assert false if duplicate coverage is useful,
  or leave duplication to the focused predecessor test;
- assert `Owned` and `seat_is_owned` true;
- count recovery launches for this ticket as zero;
- emit `ack-then-owned`.

The consecutive test should not duplicate every stale-ack classifier assertion.

## Codex fault path boundary

For the configured sequence `6`:

- complete the shared clear handshake and observe the armed deadline;
- read original generation and exact deadline;
- do not call the acknowledgment helper;
- evaluate at the deadline;
- assert `Recovering`, unowned, same ticket, and new generation;
- backdate the pane transition start beyond `AGENT_EXIT_GRACE_SECS`;
- call `check_transition_timeouts`;
- assert one recovery launch for the ticket and generation;
- read the armed recovery state;
- submit matching recovery acknowledgment;
- assert `Owned`, no error alert, and one launch after another timeout poll;
- emit `timeout-then-fallback`.

The fresh fallback succeeds. `RecoveryFailed` remains covered by the dependency
regression and is not an allowed row outcome here.

## Completion/release boundary

After both assignments in a round reach `Owned`:

- call `Thread::complete` for each ticket;
- call `release_slot_for_ticket` for each ticket;
- assert its seat assignment entry is removed;
- assert slot ticket is `None`;
- assert `has_session` remains true;
- assert `last_client` remains the scenario provider;
- clear any zero-length cooldown only if production comparison requires it.

With `wind_down_secs = 0`, the next scheduler call should accept the slots
without mutation beyond production release behavior.

## Claude control boundary

For every Claude row:

- scheduling must choose the compatible resident pane;
- assignment state must be `Owned` immediately;
- transport must be `WaitingForClear`;
- `active_assignment_generation` must return `None`;
- `handle_cleared_signal` must change transport to `Idle`;
- assignment and ownership must remain unchanged;
- no recovery launch may appear;
- emit `clear-then-owned-unchanged`.

The control shares the same helper and round/release cadence as Codex but does
not share transition assertions, preserving provider-specific behavior.

## Evidence record schema

Assignment record:

```text
T0330302|assignment|
provider=<codex|claude>|
sequence=<01..10>|
ticket=<ticket-id>|
pane=<pane-id>|
generation=<number|none>|
outcome=<allowed-label>|
fallback_launches=<0|1>|
final=owned|
silent_stall=false
```

The implementation emits each record on one physical line.

Summary record:

```text
T0330302|summary|codex=10|ack_then_owned=9|
timeout_then_fallback=1|claude=10|silent_stalls=0
```

The shell runner treats unknown provider or outcome values as failure.

## Harness command interface

```text
harness/run.sh [--report PATH]
```

Environment:

- `CARGO`: optional Cargo executable, defaults to `cargo`;
- `RUST_BACKTRACE`: inherited;
- repository root is derived from the script location, not caller CWD.

Exit status:

- `0`: test and all evidence validations passed;
- `2`: bad arguments or missing executable;
- underlying nonzero status: Cargo test failed;
- `1`: evidence extraction or validation failed.

The runner creates temporary raw and normalized files with `mktemp` and removes
them via `trap`.

## Report renderer organization

When `--report` is present, the script creates the parent directory and writes:

1. title and verdict;
2. run metadata table;
3. proof boundary paragraph;
4. Codex assignment table;
5. Claude control table;
6. summary bullets/table;
7. limitations.

Rows are parsed field-by-field using shell string operations or `awk`; no `jq`
dependency is introduced.

The script is the only generator for `run-report.md`. Manual additions such as
broader workspace verification go in `progress.md` and `review.md`, avoiding
report regeneration loss.

## README organization

Sections:

- purpose;
- run commands;
- what production behavior is exercised;
- exact assertions;
- evidence format;
- live-style versus live-client boundary;
- report regeneration;
- troubleshooting retained Cargo output (if the runner chooses to print it on
  failure).

## Dependency boundaries

No new crate dependencies.

No shell dependencies beyond POSIX-oriented utilities already used in project
harnesses: Bash, Cargo, `grep`, `sed`, `awk`, `mktemp`, and core file tools.

No production dependency from plugin code to work artifacts. The shell runner
invokes Cargo; Rust tests do not invoke the runner.

## Ordering constraints

1. add the Rust scenario and ensure focused test passes;
2. add the wrapper against the stable emitted schema;
3. add README matching actual command behavior;
4. run wrapper and generate report;
5. run broad verification;
6. commit exact implementation/evidence paths;
7. finish progress and review artifacts.

Changing the row schema after generating the report requires rerunning the
harness.

## Ownership and isolated transaction

The implementation commit owns exactly four paths:

```text
crates/lisa-plugin/src/lib.rs
docs/active/work/T-033-03-02/harness/README.md
docs/active/work/T-033-03-02/harness/run.sh
docs/active/work/T-033-03-02/run-report.md
```

Phase artifacts are excluded from `commit-ticket`. Unrelated dirty paths and
the ordinary Git index are not touched.

## Structure conclusion

The code boundary remains test-only, the report boundary remains script-only,
and production scheduling is the sole lifecycle authority. The resulting file
shape makes both CI regression and human evidence review straightforward.
