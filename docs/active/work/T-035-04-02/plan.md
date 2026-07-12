# T-035-04-02 Plan — implement bounded shell reset and relaunch

## Step 1 — establish a clean source baseline

Inspect the path-scoped diff and ordinary index before edits.

Verification:

- `crates/lisa-plugin/src/lib.rs` has no pre-existing working-tree change;
- unrelated orchestration and documentation changes remain untouched;
- no ticket-owned source path is staged.

Commit: none; this is a read-only safety check.

## Step 2 — add state and finite-count contract

Modify `SeatAssignmentState::Starting` to store `relaunches: u8`.

Add `ResettingStartup { generation, reset_deadline }`.

Add the maximum same-pane relaunch constant.

Update all constructors and exhaustive matches to compile:

- initial fresh dispatch uses zero;
- E-033 fresh fallback uses zero;
- start deadline arming preserves the count;
- process-start admission accepts either count but transitions out of Starting;
- reset state maps to yellow startup status;
- generation acknowledgment code does not accept reset state as chat-deliverable.

Verification:

- `cargo test -p lisa-plugin --no-run` compiles;
- existing fresh state tests still assert non-ownership;
- no initial path creates more than zero relaunches.

## Step 3 — build and test the shell readiness probe

Add the pure probe builder.

Requirements:

- exact serialized successor lease payload;
- pane-scoped `.shell-ready` destination;
- unique same-directory temporary file;
- atomic rename;
- shell-quoted payload and paths;
- host path conversion compatible with the existing `/host` boundary;
- bounded command independent of ticket prompt length.

Add a unit test that executes the generated command in an isolated directory and asserts:

- final signal exists;
- body parses as and equals the exact lease;
- no temporary file remains;
- hostile ticket identity cannot execute shell syntax;
- wrong directory/path input remains safely quoted.

Verification:

- run the named probe test;
- run existing `shell_quote` and launch preparation tests.

## Step 4 — implement authority rotation and reset submission

Add `begin_startup_recovery`.

Revalidate the expired original state against:

- slot ticket;
- slot attempt;
- current authority;
- high-water predecessor.

Perform the mutation order exactly:

1. revoke predecessor;
2. mint strict successor;
3. install successor in authority/high water;
4. stamp slot and thread;
5. enter reset state and deadline;
6. remove old marker and lifecycle residue;
7. clear old pending Enter actions;
8. send Ctrl-C;
9. queue the probe and Enter;
10. log one bounded same-pane recovery.

Add a deterministic native test that starts from scheduled fresh `Starting`, injects the
deadline, and asserts:

- original attempt is no longer current;
- successor ID is exactly greater;
- physical pane is unchanged;
- state is `ResettingStartup` for successor;
- thread and slot carry successor;
- old marker is absent;
- no spare is assigned;
- no provider relaunch has yet occurred.

Verification:

- run the named timeout/rotation test;
- inspect test lifecycle ordering if events are extended.

## Step 5 — admit shell proof and relaunch bare provider

Add `acknowledge_shell_ready` and `check_shell_ready_signals`.

Admission must consume the file and require exact:

- pane;
- reset state generation;
- slot ticket and lease;
- current authority.

On admission:

- recreate successor `assignment.md`;
- recreate successor bare launch script;
- publish successor pane lease marker;
- enter replacement Starting with count one;
- send the launch indirection in the same pane;
- arm the start deadline;
- update slot provider/session/activity facts;
- log the relaunch.

Integrate the scanner into `poll_tick` before deadline evaluation.

Add a test that first submits stale proof and observes no effect, then exact proof and
asserts:

- replacement Starting count is one;
- successor marker contains exact successor;
- successor assignment exists in the successor directory;
- launch script is bare and contains no assignment prompt/reference;
- pane and ticket reservation remain unchanged.

Verification:

- run shell-ready scanner/admission tests;
- run existing bare-launch and prepared-assignment tests.

## Step 6 — preserve the start/chat ownership gates

Extend the positive same-pane test:

1. after relaunch, stale predecessor `.started` is rejected;
2. exact successor `.started` reaches `ReadyForAssignment` only;
3. a later ready-delivery action reaches `Delivering`;
4. stale predecessor ack is rejected;
5. exact successor ack reaches `Owned`;
6. exact duplicate evidence is inert.

Run this table-driven for Claude and Codex where practical. At minimum, route both
providers through shared helper assertions and inspect their bare launch scripts.

Verification:

- run the named same-pane positive test;
- run T-035-04-01 fresh-state provider parity tests.

## Step 7 — add bounded terminal failures

Extend deadline collection for `ResettingStartup`.

Change expired Starting handling:

- original count zero begins recovery;
- replacement count one exhausts recovery;
- defensive greater counts exhaust recovery.

Add or refactor pane fencing so terminal startup recovery:

- reports `StartupFailed`;
- fails the thread;
- emits one error alert;
- revokes current successor;
- clears queued pane input and stale marker;
- permanently fences/closes the pane;
- does not release the reservation or schedule a spare;
- remains inert on later deadline checks.

Add tests for:

- missing shell-ready signal;
- shell-ready launch preparation failure if a deterministic fault can be injected;
- missing replacement process-start signal;
- late shell/start/ack evidence after failure;
- repeated timeout calls causing no additional reset or relaunch.

Verification:

- run all `startup` and `same_pane` test filters;
- assert named error messages include reset guidance/evidence name.

## Step 8 — prove lifecycle classification boundaries

Add one classification test with four seats or sequential fixtures:

- original incomplete-shell `Starting` enters reset and receives successor state;
- `ReadyForAssignment` never enters reset;
- `Delivering` retries bounded chat only and never rotates lease;
- `Owned` never enters startup reset and retains existing hard-silence fencing behavior.

Also rerun existing regressions for:

- E-033 bounded acknowledgment wait and fresh fallback;
- E-034 split-brain fencing and authoritative completion;
- startup missing-signal behavior updated to one reset;
- pane naming;
- Claude/Codex provider parity.

Verification commands:

```text
cargo test -p lisa-plugin startup
cargo test -p lisa-plugin same_pane
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin split_brain
cargo test -p lisa-plugin pane_title
```

## Step 9 — format and run full verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-plugin/src/lib.rs
```

If formatting changes are needed, run `cargo fmt --all`, then repeat the checks.

Record exact counts and any environment limitation in `progress.md`.

The broader real-Zellij stub harness remains the dependent T-035-02-01 implementation.
This ticket verifies its command/state contract natively and, if local Zellij execution
is already available without adding unrelated harness scope, may perform an additional
manual smoke run without claiming it as committed regression coverage.

## Step 10 — self-review the ticket-owned diff

Inspect:

- `git diff -- crates/lisa-plugin/src/lib.rs`;
- all new state transitions;
- every successor marker publication site;
- every branch capable of sending Ctrl-C;
- every `Owned` transition;
- every recovery deadline;
- all exact lease checks;
- all pane release/fence interactions.

Confirm:

- only expired original Starting sends Ctrl-C;
- no `/exit` is used for shell reset;
- successor marker appears only after shell proof;
- at most one same-pane relaunch is possible;
- Ready/Delivering/Owned behavior remains separate;
- no source path outside the ticket unit changed.

## Step 11 — commit the meaningful source unit

Run only Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-035-04-02 \
  --message "fix(plugin): recover incomplete shell startup in place" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add`, `git add -A`, or ordinary `git commit`.

After the command, verify:

- the commit contains exactly `crates/lisa-plugin/src/lib.rs`;
- that path is neither staged, modified, nor untracked;
- unrelated dirty paths remain unchanged;
- the ordinary index contains no ticket-owned entry.

## Step 12 — complete private progress and review artifacts

Write `progress.md` with:

- each implemented transition;
- test results and counts;
- commit ID and exact path scope;
- deviations from this plan;
- the T-035-02-01 real-Zellij harness boundary.

Then write `review.md` with:

- source summary;
- acceptance-criterion mapping;
- stale-attempt safety analysis;
- test coverage and gaps;
- open concerns;
- clean-source and isolated-commit confirmation.

Do not update ticket phase/status or publish to `docs/active/work`.
After `review.md`, remain on this ticket and stop for Lisa's completion transaction.
