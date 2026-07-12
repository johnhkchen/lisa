# T-033-03-01 Design — deterministic stall reproduction

## Decision summary

Add one native scheduler unit test beside the existing reused-Codex tests in
`crates/lisa-plugin/src/lib.rs`.

The test will schedule work into a resident Codex seat, complete the clear
handshake, create the exact matching post-prompt acceptance signal, delete that
signal before Lisa scans it, and then contrast two interpretations of the
resulting scheduler facts:

- the historical fire-and-forget interpretation sees a reserved slot, running
  thread, live session, and idle transport as an owned assignment despite no
  acceptance evidence and no bound;
- the current state machine reports the seat unowned, carries an armed finite
  deadline, and transitions to one fresh recovery attempt for the same ticket.

The test will continue through the recovery deadline and assert the terminal
`RecoveryFailed` state, retained ticket, failed thread, actionable alert, and
exactly one fresh launch. This proves both waits are bounded and rules out a
silent fallback stall.

No production logic, public interface, configuration, fixture, or harness file
will change.

## Design goals

The regression evidence should be:

- deterministic: no sleeps, processes, network, Zellij, or live Codex;
- faithful: use actual scheduling, prompt delivery, signal scanning, timeout
  evaluation, and recovery code;
- historical: state precisely why the old contract falsely considered the
  handoff successful;
- bounded: show the current path cannot wait forever at either generation;
- focused: avoid re-testing unrelated acknowledgment parsing combinations;
- durable: have a descriptive test name and failure messages tied to the field
  failure rather than an implementation ticket number.

## Option 1 — only rename or extend the dependency's recovery test

`test_bounded_ack_wait_recovers_once_then_fails_actionably` already withholds
the original acknowledgment and proves most recovery invariants. It could be
renamed and given comments about the original incident.

### Advantages

- minimal added test runtime;
- no duplicated setup;
- directly exercises the complete current recovery path.

### Drawbacks

- withholding is implicit; no post-prompt event is created and dropped;
- the test is owned conceptually by `T-033-01-04` and organized around that
  ticket's implementation acceptance criterion;
- it never demonstrates why the historical open-loop facts constituted a
  false owner;
- editing it would blur implementation coverage with standing field-regression
  evidence and make future failures harder to classify.

### Decision

Rejected. Keep the dependency's exhaustive transition test intact and add a
purpose-built incident regression.

## Option 2 — add an external integration test

Create a new file under `crates/lisa-plugin/tests/` and exercise the plugin as a
black box.

### Advantages

- a clearly separate committed test artifact;
- stronger apparent separation from implementation details;
- could resemble a future live-style harness.

### Drawbacks

- scheduler `State`, seat states, transition helpers, and injected-time methods
  are crate-private;
- the plugin is designed around Zellij host callbacks rather than a public
  native simulation API;
- exposing internals would alter production interfaces solely for this ticket;
- a true black-box test would require Zellij or WASM host scaffolding, increasing
  nondeterminism and moving toward `T-033-03-02`'s live-style scope;
- duplicating the state machine in an integration harness would test the copy,
  not production behavior.

### Decision

Rejected. The native unit-test module is the established production-state seam.

## Option 3 — add a static historical source fixture

Store a pre-fix scheduler fragment or serialized historical snapshot under the
test fixtures and assert that it lacks acknowledgment and recovery fields.

### Advantages

- preserves concrete historical evidence;
- can name the exact pre-fix facts independently of current code;
- avoids introducing a test-local legacy model.

### Drawbacks

- source text is not executable behavior;
- snapshot assertions are brittle to formatting yet weak against behavioral
  regression;
- carrying old Rust code as a fixture creates unclear maintenance ownership;
- it does not prove the current scheduler recovers from the same event loss.

### Decision

Rejected. Git history already preserves the old implementation. The committed
regression should preserve the behavioral failure and recovery contract.

## Option 4 — native incident regression with an explicit dropped signal

Use `pane_name_schedule_state` to create a resident Codex pane and exercise the
same native scheduler methods as production. Materialize a valid
`pane-10.ack`, delete it before `check_codex_ack_signals`, and assert both the
historical false-owner observation and current bounded state.

### Advantages

- exercises real scheduler state and acknowledgment transport boundary;
- exactly controls event loss;
- remains free and CI-runnable;
- uses existing private helpers without widening interfaces;
- can evaluate exact deadlines without sleeping;
- makes the regression discoverable beside related contract tests;
- permits strong assertions about one recovery launch and terminal failure.

### Drawbacks

- overlaps setup and some assertions with the dependency's recovery test;
- the historical behavior can no longer be executed directly against current
  code and must be expressed as a small test-local interpretation;
- the test remains coupled to private enum variants, as existing scheduler
  tests already are.

### Decision

Selected. It best satisfies the explicit event-drop and old-versus-current
proof while staying within the story's no-production-logic boundary.

## Historical comparison design

The current binary cannot execute deleted behavior, and restoring old scheduler
logic in production or as a large test copy would be misleading. The test will
instead capture the historical contract in one small test-local predicate or
snapshot based on facts visible immediately after prompt submission.

The legacy interpretation is true when all of these hold:

```text
slot.ticket_id == Some("T-NAME")
thread.status == Running
slot.has_session == true
slot.transition_state == Idle
ack signal is absent
```

Before explicit seat truth, the first four scheduler facts constituted the
successful handoff. The fifth fact demonstrates the false positive: provider
acceptance was never observed. The historical state also had no assignment
deadline, so nothing in that handoff path could initiate fresh-session
recovery.

The regression will give this boolean a precise name such as
`legacy_open_loop_would_claim_ownership` and assert it with a failure message
that explains the incident. This is deliberately a narrow historical oracle,
not a second scheduler implementation.

Current truth is asserted independently with `seat_assignment` and
`seat_is_owned`. The test must observe `AssignedPendingAck` with a concrete
deadline and `false` ownership at the same moment the legacy oracle is true.

## Dropped-event design

After `handle_cleared_signal(10)` arms generation 1, construct the exact normal
payload using `codex_ack::tag_codex_assignment` for ticket `T-NAME` and
generation 1.

Write it to `state.signal_dir.join("pane-10.ack")`, verify the file exists, then
remove it before calling `check_codex_ack_signals`. This sequence establishes
that a valid post-prompt acceptance event existed at the transport boundary but
was lost before scheduler consumption.

After scanning:

- the signal remains absent;
- no acknowledgment activity exists;
- state remains pending for generation 1;
- ownership remains false.

The test does not need fault injection in production code. Filesystem deletion
is the fault injector and is confined to the temporary directory.

## Recovery assertions

Evaluate the original exact deadline. The test will assert:

- state becomes `Recovering` with generation 2 and no deadline;
- transport becomes `WaitingForExit`;
- the same ticket remains on pane 10;
- ownership remains false.

Backdate the transition start beyond `AGENT_EXIT_GRACE_SECS` and invoke
`check_transition_timeouts`. Then assert:

- a generation-2 recovery deadline exists;
- exactly one `SessionLaunch` names `T-NAME` and contains the generation-2
  marker;
- a repeated transition check does not add a second launch;
- the seat remains unowned.

Do not create a recovery acknowledgment signal. Evaluate the recovery deadline
and assert:

- state becomes `RecoveryFailed`;
- the ticket reservation remains attached;
- the thread becomes `Failed`;
- the pane/ticket error alert exists;
- activity tells the operator to reset the ticket;
- a later timeout evaluation still leaves exactly one recovery launch.

This terminal half is intentional. Proving only pending-to-recovering would
leave open the possibility that the fresh fallback itself silently stalls.

## Test naming and placement

Place the test after
`test_recycled_codex_ownership_requires_matching_ack_exactly_once` and before
the dependency's generic bounded-recovery test.

Use a behavior-oriented name:

```text
test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly
```

The name contains the injected fault, historical regression, and expected
current outcome. It also supports a focused CI command using a unique substring.

## Verification design

Run, in increasing scope:

1. `cargo test -p lisa-plugin dropped_post_prompt_ack`
2. `cargo test -p lisa-plugin bounded_ack_wait`
3. `cargo test -p lisa-plugin recovery_ack`
4. `cargo test -p lisa-plugin`
5. `cargo test --workspace`
6. `cargo fmt --all -- --check`
7. `cargo clippy -p lisa-plugin --all-targets -- -D warnings`
8. `git diff --check -- crates/lisa-plugin/src/lib.rs`

The focused neighboring tests guard against contradictory or duplicated
recovery expectations. Package and workspace suites establish CI compatibility.
Formatting, Clippy, and diff checks protect source quality.

## Commit design

The single meaningful source unit is the new regression in
`crates/lisa-plugin/src/lib.rs`. Commit only that path with:

```text
lisa commit-ticket --ticket-id T-033-03-01 \
  --message "test: reproduce dropped Codex handoff acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs
```

If the installed `lisa` binary lacks the command, use the repository CLI via
`cargo run -p lisa-cli -- commit-ticket` with the same ticket, message, and exact
include. Do not use the ordinary index.

## Chosen design outcome

One explicit dropped-signal regression will connect field failure to the
current recovery contract. It preserves the historical false-owner condition
without resurrecting old production code and proves the modern scheduler has a
finite, one-shot, actionable outcome with no live provider dependency.
