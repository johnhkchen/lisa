# Plan: T-034-03-01 deterministic split-brain regression

## Implementation objective

Commit one native plugin regression that deterministically drives the complete
split-brain field timeline through production scheduler methods.

No scheduler behavior changes are planned.

## Step 1: establish the temporary project

Add the test
`split_brain_timeline_fences_old_attempt_and_admits_one_winner` to the existing
`crates/lisa-plugin/src/lib.rs` test module.

Create a temp directory and these fixture roots:

- tickets;
- canonical work;
- attempt staging;
- signals;
- ledger and provider usage directories.

Write one Review-phase Codex ticket and scan it into a real DAG.

Verification:

- DAG contains T-SPLIT;
- ticket is open for scheduling at Review;
- all runtime paths are inside the temp directory.

## Step 2: create the two-pane field state

Construct scheduler state with deterministic timeout and scheduling settings.

Add pane 1 as the assigned predecessor.

Add pane 2 as an eligible resident Codex session.

Insert the predecessor Review thread on pane 1.

Install attempt 1 through `install_current_attempt`.

Mark pane 1 Owned.

Write predecessor review sentinel bytes into attempt 1 staging.

Verification:

- thread, slot, and authority registry agree on attempt 1;
- pane 1 is the sole reservation;
- pane 2 is Idle and eligible;
- canonical review does not exist.

## Step 3: trigger real timeout and fence

Set predecessor start/activity timestamps beyond both budget and hard-silence
limits.

Call `check_session_timeouts`.

Verification:

- lifecycle events prove LeaseRevoked before PaneFenced before SlotReleased;
- pane 1 is Fenced, unassigned, and sessionless;
- current authority is absent;
- predecessor thread is removed;
- timeout alert is present;
- first provenance row is attempt 1 / TimedOut / fenced / non-authoritative.

This step must fail if lease revocation moves behind release or the pane is
returned to reusable Idle state.

## Step 4: redispatch through the real scheduler

Call `schedule_ready_tickets`.

Capture attempt 2 from current authority.

Verification:

- attempt 2 equals attempt 1 plus one;
- attempt 2 is installed in the thread and pane-2 slot;
- pane 1 remains Fenced and cannot host the successor;
- pane 2 is the only reserved slot;
- pane 2 is AssignedPendingAck with attempt-2 generation;
- pane 2 is not Owned.

Do not call a matching ack in this step.

That omission models the missed prompt injection/acknowledgement.

## Step 5: replay the old process's signal vocabulary

Snapshot the replacement thread's last activity, pane activity, assignment
state, lease, phase, and reservation.

Write pane-1 heartbeat, ack, idle, stopped, cleared, and error signals.

The heartbeat body carries attempt 1.

The ack payload carries T-SPLIT and attempt-1 generation.

Run all corresponding consumers.

Verification:

- all signal files are consumed;
- stale ack does not promote pane 2;
- replacement thread/pane clocks do not change;
- replacement remains pending and unowned;
- replacement thread remains Running and on pane 2;
- current authority remains attempt 2;
- no replacement error alert is created;
- pane 1 stays Fenced and unassigned;
- exactly one ticket reservation exists.

This step must fail if physical-pane attribution is replaced by mutable logical
ticket attribution or if heartbeat lease validation is removed.

## Step 6: reject predecessor artifact and completion

Run `check_artifact_advances` while only attempt-1 staging contains review.md.

Call `request_completion` with attempt-1 authority.

Verification:

- no canonical review file exists;
- predecessor bytes remain private;
- replacement phase remains Review;
- no pending completion exists;
- stale completion returns false;
- current lease remains attempt 2.

This step must fail if artifact admission or completion authority stops checking
the exact current lease.

## Step 7: acknowledge the replacement

Construct a tagged Codex acknowledgement for T-SPLIT / attempt 2.

Submit it through `acknowledge_codex_assignment`.

Verification:

- the transition returns true once;
- pane 2 becomes Owned;
- pane 1 remains without assignment state;
- the count of Owned seats is exactly one;
- duplicate acknowledgement cannot create another transition.

## Step 8: admit only replacement bytes

Write a different review sentinel into attempt-2 staging.

Call `check_artifact_advances`.

Verification:

- canonical review equals attempt-2 bytes exactly;
- it does not equal attempt-1 bytes;
- pending completion exists;
- pending authority is attempt 2;
- the completion source is Artifact.

## Step 9: publish one authoritative winner

Update the fixture ticket to Done.

Call `handle_completion_result` with successful exit status and a valid 40-hex
commit ID.

Call it a second time to model a duplicate native callback.

Verification:

- the pending completion is removed;
- the replacement thread is removed;
- the replacement slot is released;
- no Owned seat remains after release;
- ledger total is two rows;
- exactly one row is authoritative Done;
- that row carries attempt 2;
- attempt 1 has only fenced, non-authoritative timeout history.

## Step 10: run focused verification

Run:

```text
cargo test -p lisa-plugin split_brain_timeline_fences_old_attempt_and_admits_one_winner
```

If it fails, diagnose the real boundary rather than weakening assertions.

Record results in `progress.md`.

## Step 11: run regression verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --all-targets -- -D warnings
git diff --check -- crates/lisa-plugin/src/lib.rs
git diff --check -- docs/active/work/T-034-03-01
```

If workspace tests expose unrelated dirty-path behavior, distinguish it from
the ticket-owned source result and document it precisely.

Do not modify unrelated files to clean the repository.

## Step 12: inspect mutation sensitivity

Review the test assertions against the acceptance checks.

Confirm each safety-critical production check has a downstream assertion:

| Boundary | Regression assertion |
|---|---|
| timeout lease revocation | lifecycle order and current authority absence |
| pane fencing | pane 1 remains Fenced and is not selected |
| monotonic redispatch | attempt 2 equals attempt 1 plus one |
| ack lease check | stale ack leaves pane 2 pending |
| heartbeat lease check | replacement clocks unchanged |
| pane signal attribution | idle/stop/clear/error cannot affect pane 2 |
| artifact lease check | no canonical predecessor review |
| completion lease check | no stale pending completion |
| provenance lease check | exactly one authoritative Done on attempt 2 |

This is a review step, not a production mutation experiment; do not temporarily
delete guards from the working tree unless a test gap is ambiguous.

## Step 13: commit the source unit

Inspect the exact source diff.

Commit only the plugin source with Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-034-03-01 \
  --message "Test deterministic split-brain fencing" \
  --include crates/lisa-plugin/src/lib.rs
```

Verification:

- command succeeds and reports a commit ID;
- commit contains exactly the plugin source path;
- plugin source is clean afterward;
- ordinary index state is unchanged;
- no ticket-owned source file remains modified, staged, or untracked.

Do not use ordinary `git add` or `git commit`.

## Step 14: complete progress and review artifacts

Update `progress.md` with:

- each completed implementation step;
- any deviations and rationale;
- focused and broad test results;
- isolated commit ID;
- owned-path repository state.

Write `review.md` with:

- outcome and scenario summary;
- files changed;
- acceptance mapping;
- test coverage;
- mutation-sensitivity assessment;
- open concerns and live-proof boundary;
- source commit and workspace integrity.

Do not edit ticket phase or status.

## Atomicity

The implementation is one meaningful source unit because the acceptance value
comes from the composed test as a whole.

Splitting setup and assertions across source commits would leave intermediate
commits with no useful regression.

## Completion criteria

Implementation is complete when:

- the narrative regression passes;
- broad verification passes or any unrelated baseline failure is documented;
- the exact source path is committed through `lisa commit-ticket`;
- `progress.md` and `review.md` are written;
- the ticket frontmatter remains untouched by this agent.
