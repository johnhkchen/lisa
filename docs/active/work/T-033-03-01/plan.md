# T-033-03-01 Plan — deterministic stall reproduction

## Objective

Commit a native, deterministic regression test that explicitly loses a valid
post-prompt Codex acceptance event, demonstrates why the former open-loop
handoff falsely appeared owned and could stall silently, and proves the current
acknowledgment-gated scheduler reaches bounded one-shot recovery and an
actionable terminal outcome without live Codex.

## Constraints

- Complete implementation and review in this session.
- Do not update ticket `phase` or `status` frontmatter.
- Do not modify scheduler production behavior.
- Do not require Zellij, Codex, network, credentials, or sleeps.
- Preserve all unrelated worktree changes.
- Use the isolated Lisa transaction with exact include paths.
- Leave workflow artifacts for Lisa's completion commit.

## Step 1 — establish the source baseline

1. Confirm `crates/lisa-plugin/src/lib.rs` has no pre-existing worktree diff.
2. Record the current repository status for later ownership comparison.
3. Re-open the recycled-Codex test cluster to place the new test without
   disturbing neighboring tests.
4. Confirm the dependency tests pass before editing if a focused baseline is
   cheap.

Verification:

```text
git diff -- crates/lisa-plugin/src/lib.rs
cargo test -p lisa-plugin bounded_ack_wait
```

Expected result: no source diff and the existing bounded recovery test passes.

## Step 2 — add the incident regression skeleton

1. Insert
   `test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly`
   after the exact-ack ownership test.
2. Build state for ticket `T-NAME`, pane 10, and resident Codex.
3. Set `assignment_ack_timeout_secs = 1`.
4. Schedule the ticket.
5. Assert generation 1 exists with no deadline while `/clear` is pending.
6. Deliver the clear signal through `handle_cleared_signal(10)`.
7. Extract the exact armed original deadline.

Verification:

- Test compiles.
- Pane transport is `Idle` after prompt delivery.
- Seat remains unowned and pending generation 1.

## Step 3 — inject deterministic post-prompt event loss

1. Construct a matching `UserPromptSubmit` payload for `T-NAME`, generation 1.
2. Write it to the actual configured `pane-10.ack` path.
3. Assert the file exists to prove the valid event was materialized.
4. Remove the file before the scheduler scans.
5. Call `check_codex_ack_signals`.
6. Assert no acknowledgment activity was logged and no promotion occurred.

Verification:

- Signal path is absent after injection.
- Seat is still `AssignedPendingAck` with the original deadline.
- `seat_is_owned(10)` is false.

## Step 4 — encode the historical open-loop regression

1. Read actual state immediately after the dropped event.
2. Compute a local boolean from the old success facts:
   - ticket attached to pane;
   - thread running;
   - resident session present;
   - transition transport idle;
   - no acceptance event available.
3. Name the boolean
   `legacy_open_loop_would_claim_ownership_without_ack`.
4. Assert it is true with a message explaining that these transport and
   reservation facts were the old false owner.
5. Assert current explicit seat truth is still unowned pending.

Verification:

- The test fails if the scenario no longer reproduces the historical facts.
- The current ownership assertion is independent of the legacy boolean.

## Step 5 — prove the original wait is bounded

1. Evaluate acknowledgment timeouts at the exact original deadline.
2. Assert state changes to generation-2 `Recovering` with no deadline.
3. Assert transport changes to `WaitingForExit`.
4. Assert ticket `T-NAME` remains reserved on pane 10.
5. Assert ownership remains false.

Verification:

- A dropped event does not remain pending beyond its configured boundary.
- The original generation is no longer active.

## Step 6 — prove one fresh fallback launch

1. Backdate `transition_started_at` beyond `AGENT_EXIT_GRACE_SECS`.
2. Run `check_transition_timeouts`.
3. Extract generation 2's armed recovery deadline.
4. Count recovery `SessionLaunch` events for `T-NAME` carrying generation 2.
5. Assert exactly one launch.
6. Run transition evaluation again.
7. Assert the launch count remains one.
8. Assert the seat remains unowned.

Verification:

- Recovery retains the same ticket.
- The abandoned generation cannot claim the fallback.
- Repeated polling cannot launch an unbounded series of sessions.

## Step 7 — prove the fallback cannot silently stall

1. Deliberately create no generation-2 `.ack` event.
2. Evaluate acknowledgment timeouts at the recovery deadline.
3. Assert `RecoveryFailed`.
4. Assert ownership remains false.
5. Assert the ticket reservation remains on pane 10.
6. Assert the retained thread status is `Failed`.
7. Assert the error alert contains the ticket and pane.
8. Assert error activity mentions recovery failure and reset guidance.
9. Evaluate again after the deadline.
10. Assert the recovery launch count remains one.

Verification:

- The outcome is finite, named, visible, and actionable.
- No ticket is lost.
- No silent wait or infinite retry remains.

## Step 8 — run focused verification

Run:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin recovery_ack
```

If the new test fails, inspect the exact state rather than loosening assertions.
Any design deviation must be recorded in `progress.md` before changing course.

Expected results:

- one new regression passes;
- the dependency's bounded timeout test still passes;
- successful recovery acknowledgment still passes.

## Step 9 — format and inspect the patch

1. Run `cargo fmt --all` if formatting is required.
2. Inspect only the ticket-owned source diff.
3. Confirm the diff contains one test and no production mutation.
4. Run whitespace validation.

Commands:

```text
cargo fmt --all -- --check
git diff -- crates/lisa-plugin/src/lib.rs
git diff --check -- crates/lisa-plugin/src/lib.rs
```

If the formatter changes unrelated files, do not include them. Since only one
Rust file is edited and existing formatting is clean, no unrelated formatter
changes are expected.

## Step 10 — run package and workspace verification

Run:

```text
cargo test -p lisa-plugin
cargo test --workspace
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Optionally run the repository's quick check if the targeted and workspace
commands do not cover a project-specific build constraint:

```text
just check
```

Record exact outcomes and test counts where Cargo reports them.

Failure handling:

- A failure caused by the new source must be fixed before commit.
- A pre-existing unrelated failure must be investigated enough to establish
  evidence and recorded as an open concern.
- Do not alter unrelated source to make the suite green.

## Step 11 — create and maintain progress.md

Create `docs/active/work/T-033-03-01/progress.md` at the start of Implement.
Record:

- baseline status and ownership boundary;
- source edit completed;
- focused test outcomes;
- package/workspace/quality outcomes;
- any deviations and rationale;
- isolated commit command and result;
- final cleanliness for the ticket-owned source path.

Update the artifact as work proceeds. Do not use it as a phase transition
signal; `review.md` closes implementation under the repository workflow.

## Step 12 — commit the source through Lisa

Before committing:

1. Confirm `git diff --cached --name-only` to understand the ordinary index
   without changing it.
2. Confirm `git status --short -- crates/lisa-plugin/src/lib.rs` shows only the
   ticket-owned modification.
3. Run the isolated transaction with exactly one include path.

Primary command:

```text
lisa commit-ticket \
  --ticket-id T-033-03-01 \
  --message "test: reproduce dropped Codex handoff acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs
```

Repository fallback if the installed binary lacks the subcommand:

```text
cargo run -p lisa-cli -- commit-ticket \
  --ticket-id T-033-03-01 \
  --message "test: reproduce dropped Codex handoff acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs
```

After committing:

- record the commit hash;
- confirm the exact source path is clean;
- confirm no ticket-owned source is staged or untracked;
- confirm unrelated worktree content remains present and uncommitted;
- do not commit ticket/work artifacts manually.

## Step 13 — review the committed result

1. Inspect the committed diff using the recorded hash.
2. Re-run or confirm all verification against the committed tree.
3. Assess each acceptance-criterion clause explicitly:
   - committed test;
   - deterministic post-prompt event drop;
   - old false ownership/silent-stall proof;
   - current bounded recovery proof;
   - CI execution without live Codex.
4. Check for test brittleness, time dependence, and duplicated implementation.
5. Identify remaining limitations, especially that live consecutive reuse is
   intentionally deferred to `T-033-03-02`.

## Step 14 — write review.md and stop

Write `docs/active/work/T-033-03-01/review.md` with:

- outcome summary;
- committed source change and hash;
- detailed regression scenario;
- historical and current-state assertions;
- test and quality command results;
- acceptance-criterion assessment;
- source ownership/cleanliness confirmation;
- open concerns and known limitations;
- human reviewer checklist.

Do not edit ticket phase/status and do not start the next ticket. Lisa will
detect `review.md`, prepare Done, and commit workflow artifacts through its
isolated completion transaction.

## Definition of done

- All six RDSPI artifacts exist.
- The new test explicitly creates and drops a matching post-prompt `.ack`.
- The test proves old transport/reservation facts falsely implied ownership.
- Current state remains unowned and has a finite deadline.
- Original timeout creates exactly one fresh generation and launch.
- Missing recovery acknowledgment reaches actionable `RecoveryFailed`.
- The test runs natively with no live Codex.
- Focused, package, workspace, formatting, lint, and diff checks pass or any
  unrelated failure is precisely documented.
- `crates/lisa-plugin/src/lib.rs` is committed via `commit-ticket` and clean.
- Ticket frontmatter is untouched by this agent.
