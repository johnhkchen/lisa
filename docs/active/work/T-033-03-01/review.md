# T-033-03-01 Review — deterministic stall reproduction

## Outcome

Acceptance criterion met.

A committed native scheduler regression now deterministically materializes and
drops the post-prompt Codex acceptance event that the original fire-and-forget
handoff could lose. At the loss boundary, the test proves the historical
reservation/thread/transport facts would have claimed an owner without provider
acknowledgment and with no handoff recovery boundary.

Against the current scheduler, the same scenario remains explicitly unowned,
has a finite acknowledgment deadline, abandons the lost generation, launches
exactly one fresh Codex fallback for the same ticket, and terminates in a named,
alerted, reset-required failure if the fallback acceptance is also absent.

The regression uses no live Codex, Zellij, network, credentials, or sleeping.
It runs in the normal native plugin test suite and therefore in workspace CI.

## Source commit

Committed through Lisa's isolated ticket transaction:

```text
d48f3f51a3bf975bd7b2c5076033a0ac69696c13
test: reproduce dropped Codex handoff acknowledgment
```

The installed `lisa` binary did not expose `commit-ticket`, so the documented
repository CLI fallback was used:

```text
cargo run -p lisa-cli -- commit-ticket \
  --ticket-id T-033-03-01 \
  --message "test: reproduce dropped Codex handoff acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs
```

The commit owns exactly one repository path:

```text
crates/lisa-plugin/src/lib.rs
```

## Files changed

### `crates/lisa-plugin/src/lib.rs`

Added one test:

```text
test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly
```

The test is placed beside the existing exact-ack, bounded-recovery, and
recovery-ack tests. It changes no production branch, enum, interface,
configuration, hook, fixture, or dashboard behavior.

`cargo fmt` also normalized one pre-existing expression in
`start_assignment_ack_wait`. That hunk only reflows the existing checked-add
fallback expression; it does not alter values, control flow, or behavior. It
was retained so the repository-wide formatting gate passes.

### Workflow artifacts

Created under `docs/active/work/T-033-03-01/`:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

These artifacts and the untracked ticket remain for Lisa's final isolated
completion transaction. They were not included in the source commit.

### Deleted files

None.

## Regression scenario

The test builds the existing native scheduler fixture with:

- ticket `T-NAME`;
- terminal pane 10;
- resident Codex session;
- Codex requested for the new ticket;
- one-second acknowledgment timeout;
- temporary signal directory.

Scheduling reserves the reused pane and creates:

```text
AssignedPendingAck {
    generation: 1,
    ack_deadline: None,
}
```

The test asserts the acceptance clock is not armed while `/clear` transport is
still outstanding. It then invokes the real cleared-signal handler. That sends
the generation-tagged prompt, returns pane transport to `Idle`, and creates a
concrete generation-1 acknowledgment deadline.

## Deterministic event drop

The test constructs a normal `UserPromptSubmit` payload using production marker
generation for `T-NAME`, generation 1. It writes the payload to the actual
pane-scoped path:

```text
<temporary signal dir>/pane-10.ack
```

It asserts the valid event exists, deletes it, and calls the production
`check_codex_ack_signals` scanner. This is the deterministic fault injection:
the matching post-prompt event exists at the hook transport boundary but is
lost before scheduler consumption.

After scanning, the test confirms:

- the event is absent;
- no acknowledgment activity was logged;
- generation 1 remains pending with its original deadline;
- the physical seat is not owned.

No parser stub or mocked acknowledgment helper bypasses the production signal
boundary.

## Historical open-loop proof

At the exact dropped-event boundary, the test reads these real scheduler facts:

- pane 10 reserves `T-NAME`;
- the pane reports a resident session;
- pane transport is `Idle` after prompt injection;
- the `T-NAME` thread exists and is `Running`;
- no acceptance event remains.

The pre-`S-033-01` fire-and-forget scheduler had no separate seat assignment
truth or acceptance deadline. Its successful handoff was represented by the
first four facts. The test's local historical oracle asserts that this
combination, with the fifth fact, reproduces the false owner: assigned and
apparently running without evidence that Codex accepted the prompt.

That reservation prevented another scheduler claim while the absent event had
no handoff-specific deadline. The resulting condition is the original silent
stall now preserved as regression evidence.

The historical oracle is deliberately a narrow conjunction inside the test.
No deleted scheduler implementation is copied into production or a second test
state machine.

## Current bounded-recovery proof

At the same point where the legacy facts falsely imply ownership, current
explicit state is:

```text
AssignedPendingAck {
    generation: 1,
    ack_deadline: Some(D1),
}
owned = false
```

Evaluating at `D1` produces:

```text
Recovering {
    generation: 2,
    ack_deadline: None,
}
transition = WaitingForExit
owned = false
ticket = T-NAME
```

The new generation proves the original delivery was abandoned and fenced
before recovery input. The same ticket reservation remains attached; there is
no lost work item or second owner.

The test injects exit-grace passage by backdating the existing transition
timestamp and invokes the production transition evaluator. It observes one
fresh launch for `T-NAME` whose command carries generation 2 and extracts the
fresh attempt's deadline `D2`.

Calling transition evaluation again leaves the launch count at one. This
proves repeated scheduler polls cannot turn recovery into an infinite fresh
session loop.

## No silent fallback stall

The test also withholds the generation-2 acceptance event and evaluates at
`D2`. It observes:

```text
seat state = RecoveryFailed
owned = false
slot ticket = T-NAME
thread status = Failed
error alert = (T-NAME, pane 10)
operator guidance = reset the ticket
fresh recovery launches = 1
```

A later timeout evaluation leaves the launch count at one. The fallback is
therefore bounded both in time and attempt count. Its failure is named and
visible rather than silently pending.

## Acceptance-criterion assessment

### A committed test

Met by commit `d48f3f5`. The new test lives in the native plugin test module and
is included by `cargo test -p lisa-plugin` and `cargo test --workspace`.

### Deterministically drops the post-prompt acceptance event

Met. A valid matching pane signal is written, existence-checked, and removed
before the real scanner executes. A temporary directory isolates the fault.

### Proves the old open-loop path would leave owned-without-ack/silent stall

Met. The regression asserts the exact slot, thread, session, and transport facts
the historical scheduler treated as a completed handoff while proving the
acceptance event is absent. Research cross-checked those facts against the
pre-`47e64b4` implementation in Git history.

### Proves bounded recovery now

Met. The test evaluates injected original and recovery deadlines, observes a
distinct recovery generation, counts exactly one fresh launch, and reaches
actionable `RecoveryFailed` when the second event is absent.

### Runs in CI with no live Codex

Met. The test is native, synchronous, filesystem-local, and process-free. All
time boundaries are injected from scheduler state; there are no sleeps.

## Focused test coverage

Passed:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
1 passed; 0 failed

cargo test -p lisa-plugin bounded_ack_wait
1 passed; 0 failed

cargo test -p lisa-plugin recovery_ack
1 passed; 0 failed
```

The neighboring tests ensure the incident regression agrees with the exhaustive
bounded-recovery test and does not break successful fresh-generation
acknowledgment.

## Package and workspace coverage

Passed:

```text
cargo test -p lisa-plugin
265 passed; 0 failed
```

Passed:

```text
cargo test --workspace
lisa-cli unit tests: 270 passed
atomic provider integration test: 1 passed
lisa-core unit tests: 150 passed
lisa-plugin unit tests: 265 passed
doc tests: 0 failed
total executed tests: 686 passed; 0 failed
```

This establishes that the regression compiles and executes in the same native
suite as CI and does not disturb CLI, core, provider-contract, or plugin tests.

## Quality and target coverage

Passed:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Strict Clippy covers all native plugin targets. The WASM check confirms the
test-only addition does not affect the deployable plugin target.

## Determinism assessment

The test has no wall-clock wait. It reads `D1` and `D2` from the exact scheduler
states that production created, then supplies those values to the injected-time
timeout evaluator.

Exit grace is made eligible by backdating the private transition timestamp.
The production transition method still performs the actual state change and
launch recording.

The only filesystem operations use `tempfile::TempDir`. The directory lifetime
is retained for the entire test and is removed automatically afterward.

No environment-specific path, provider installation, token, permission, shell
process, or terminal rendering is involved.

## Source ownership and worktree audit

After the isolated commit:

- `crates/lisa-plugin/src/lib.rs` is clean;
- the ordinary Git index is empty;
- no ticket-owned source remains modified, staged, or untracked;
- the source commit includes exactly the requested plugin path;
- the ticket and work artifacts remain available for Lisa completion;
- unrelated pre-existing modified and untracked paths remain uncommitted;
- this agent did not edit ticket phase or status frontmatter.

Lisa advanced ticket phase automatically while artifacts appeared, as defined
by the workflow. No manual frontmatter transition was performed.

## Deviations from plan

One anticipated fallback occurred: the installed `lisa` executable rejected
`commit-ticket`, so `cargo run -p lisa-cli -- commit-ticket` performed the same
repository implementation successfully.

One formatting-only dependency hunk was added after the initial format check
found the clean `HEAD` file was not rustfmt-normalized. Keeping that rustfmt
result made the final repository-wide formatting check green. There is no
semantic production change.

No test design, behavioral assertion, file boundary, or acceptance scope
deviation occurred.

## Open concerns and known limitations

No critical issues require human intervention.

The legacy false-owner proof is necessarily a test-local historical oracle over
current observable slot/thread facts; deleted code is not executed. This is a
deliberate boundary. Git history preserves the old implementation, while the
regression preserves its failure condition and validates current behavior.

The test covers the deterministic absent-event branch through terminal fallback
failure. Successful exact acknowledgment is covered by adjacent existing tests,
including successful generation-2 promotion.

The test does not prove real Codex hook delivery across multiple consecutive
reuses. That is explicitly the live-style scope of dependent ticket
`T-033-03-02`, which builds on this free CI evidence.

The test matches a generation marker substring in the recorded launch command,
following the existing bounded-recovery test pattern. A future change to command
escaping may require both tests to update even if generation semantics remain
correct.

## Reviewer checklist

- Confirm the dropped `.ack` is created after prompt delivery and removed before
  `check_codex_ack_signals`.
- Confirm the legacy oracle includes reservation, running thread, live session,
  idle transport, and absent acceptance evidence.
- Confirm current ownership is false before acknowledgment.
- Confirm generation changes from 1 to 2 at the first deadline.
- Confirm recovery retains ticket `T-NAME`.
- Confirm only one generation-2 fresh launch is recorded across repeated polls.
- Confirm the second missing acknowledgment reaches `RecoveryFailed`, a failed
  retained thread, an error alert, and reset guidance.
- Confirm no production behavior changed beyond rustfmt layout.
- Confirm commit `d48f3f5` contains only `crates/lisa-plugin/src/lib.rs`.
- Leave live consecutive-reuse proof to `T-033-03-02`.

## Final assessment

The original fire-and-forget stall is now standing, deterministic regression
evidence. The test demonstrates the exact false-owner condition, then proves
the acknowledgment-gated scheduler makes ownership honest and recovery finite:
one lost delivery becomes one fenced fresh attempt, and a second loss becomes a
named actionable failure rather than another silent stall.
