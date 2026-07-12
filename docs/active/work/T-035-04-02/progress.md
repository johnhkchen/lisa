# T-035-04-02 Progress — same-pane dquote recovery

## Status

Implementation, focused tests, plugin tests, workspace tests, formatting, and WASM target
verification are complete. The ticket-owned source unit is ready for Lisa's isolated
commit transaction.

## Completed workflow phases

- [x] Read `AGENTS.md` and the project `CLAUDE.md` source of truth.
- [x] Read `docs/knowledge/rdspi-workflow.md`.
- [x] Read ticket T-035-04-02 and predecessor implementation artifacts.
- [x] Map fresh launch, assignment, lease, signal, timeout, and fence boundaries.
- [x] Write private `research.md`.
- [x] Evaluate recovery alternatives and select positive shell execution proof.
- [x] Write private `design.md`.
- [x] Define the single-source-unit implementation structure.
- [x] Write private `structure.md`.
- [x] Sequence implementation, regression, verification, and commit work.
- [x] Write private `plan.md`.

## Implemented source unit

Modified only:

```text
crates/lisa-plugin/src/lib.rs
```

No adapter, hook template, UI source, CLI source, ticket frontmatter, or shared work
artifact was manually changed by this implementation.

## Startup state changes

Extended fresh `Starting` with a bounded relaunch count:

```text
Starting {
    generation,
    start_deadline,
    relaunches,
}
```

Initial fresh launches use `relaunches: 0`.

The one admitted same-pane replacement uses `relaunches: 1`.

Added:

```text
ResettingStartup {
    generation,
    reset_deadline,
}
```

This state means the failed predecessor is revoked, a successor is installed, and Lisa
is waiting for exact proof that the pane executed a command at a shell boundary.

ResettingStartup is non-owned and displays through the existing yellow startup status.

## Shell interrupt transport

Added `interrupt_shell_input`.

It:

- removes every deferred Enter queued for the affected pane;
- writes raw byte 3 (Ctrl-C) to the physical terminal;
- does not send `/exit`;
- does not infer readiness from the interrupt itself.

Removing the pending Enter prevents the failed launch's delayed submission from racing
the reset probe.

## Positive shell-readiness probe

Added `shell_readiness_probe`.

It creates one bounded POSIX shell command that:

- serializes the exact successor `AttemptLease`;
- writes it to a same-directory temporary file;
- atomically renames it to `pane-<id>.shell-ready`;
- shell-quotes JSON and paths;
- contains no provider prompt or provider command.

A successful signal therefore proves that the pane accepted and executed a command at a
shell boundary. Ctrl-C plus time alone is never treated as proof.

The probe uses host-facing paths derived through the existing `/host` conversion.

## Lease rotation order

Added `begin_startup_recovery` for expired original Starting only.

It validates:

- state generation;
- reserved ticket and physical pane;
- exact slot lease;
- exact current authority;
- exact high-water predecessor.

It then:

1. revokes the predecessor from `current_leases`;
2. mints its strict successor;
3. installs the successor in high water and current authority;
4. stamps the slot and thread with the successor;
5. enters ResettingStartup with an absolute deadline;
6. clears predecessor pane lifecycle files and attention state;
7. interrupts incomplete shell input;
8. submits the successor-scoped readiness probe.

The successor's normal `pane-<id>.lease` marker is intentionally not published during
reset. A provider that started without emitting `.started` cannot copy successor identity
from the normal hook marker.

## Prior-attempt cleanup and rejection

Added narrow best-effort cleanup for pane-scoped:

- lease;
- start;
- acknowledgment;
- heartbeat;
- idle;
- stopped;
- cleared;
- error;
- awaiting-human;
- shell-ready files.

Cleanup is not the authorization mechanism. Exact state, slot, ticket, attempt, and
`current_leases` equality remains required by every consumer.

Native tests additionally prove that predecessor heartbeat, shell-ready, start,
assignment acknowledgment, and artifact publication cannot affect the replacement.

## Shell-ready admission and relaunch

Added `check_shell_ready_signals` and `acknowledge_shell_ready`.

The scanner consumes each pane-scoped file once, parses `AttemptLease`, and admits only:

- the pane's current ResettingStartup generation;
- the exact slot ticket and successor lease;
- the exact current authority.

Exact admission reconstructs successor runtime state under its own private directory:

- complete atomic `assignment.md`;
- bare atomic `.lisa-launch-<pane>.sh`;
- normal successor pane lease marker.

It then submits the bare launcher in the same physical pane and enters armed replacement
Starting with `relaunches: 1`.

The route is resolved again from the current ticket, preserving Claude/Codex, model, and
configured Lisa binary behavior.

## Replacement ownership boundary

The replacement retains T-035-04-01's full positive sequence:

```text
ResettingStartup
  -- exact shell-ready --> Starting(relaunches 1)
  -- exact process-start --> ReadyForAssignment
  -- bounded chat reference --> Delivering
  -- exact UserPromptSubmit --> Owned
```

The native provider-parity regression runs this sequence for both Claude and Codex.

It verifies stale predecessor start and acknowledgment evidence are rejected.

It verifies process start reaches only ReadyForAssignment.

It verifies only exact successor chat acknowledgment reaches Owned.

## Bounded failure behavior

Extended deadline evaluation:

- expired original Starting begins the sole shell reset;
- expired ResettingStartup names missing positive shell readiness and fences;
- expired replacement Starting names missing replacement process start and fences;
- replacement Starting never enters another reset;
- existing Delivering keeps one bounded chat retry;
- existing reused-Codex recovery remains separate;
- Owned hard-silence behavior remains the E-034 path.

Added terminal startup recovery failure handling that:

- enters red `StartupFailed`;
- fails the logical thread;
- adds one deduplicated error alert;
- revokes successor authority;
- clears pending pane input and lifecycle markers;
- permanently marks/closes the pane as Fenced;
- retains the ticket reservation for explicit operator reset;
- does not release to automatic scheduling or consume a spare.

## Test changes

Added:

```text
shell_readiness_probe_publishes_exact_attempt_atomically
same_pane_replacement_requires_start_and_chat_ack_for_both_providers
missing_replacement_start_fences_without_second_relaunch
test_missing_shell_readiness_fences_without_relaunch
```

The former zero-relaunch missing-start regression was replaced by the stronger bounded
reset contract.

The shell probe test executes the real generated command through `sh` in an isolated
directory with hostile ticket/path quoting and verifies exact atomic lease bytes.

The provider-parity test proves:

- strict successor lease;
- same physical pane;
- no spare consumption;
- stale heartbeat rejection;
- stale artifact rejection;
- stale shell proof rejection;
- successor assignment and bare launcher preparation;
- successor marker timing;
- start/chat evidence separation;
- exact successor ownership.

The failure tests prove missing reset and replacement-start evidence are finite, named,
fenced, non-owned, and cannot submit an additional relaunch.

## Focused verification

Passed:

```text
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically
cargo test -p lisa-plugin same_pane
cargo test -p lisa-plugin missing_replacement_start
cargo test -p lisa-plugin missing_shell_readiness
```

Each filter passed its selected regression with zero failures.

## Plugin verification

Passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin
```

Result: 283 passed, 0 failed.

This includes existing E-033 acknowledgment, E-034 split-brain fencing, provider parity,
pane naming, delivery retry, hard-silence, and completion-authority tests.

## Workspace and target verification

Passed:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Workspace suites included:

- 274 CLI unit tests plus its provider-contract integration test;
- 155 core tests;
- 283 plugin tests;
- doc tests.

All completed with zero failures.

## Plan deviations

The structure considered factoring the existing E-034 fence helper. The implementation
instead kept E-034 untouched and added a narrow terminal startup-recovery fence path.
This avoids changing E-034's remove-assignment/release semantics while allowing
StartupFailed to remain visible on a retained failed reservation.

The plan proposed a separate classification test. Existing plus new tests provide the
same explicit classification:

- only expired original Starting calls startup reset;
- ReadyForAssignment is handled only by delivery;
- Delivering uses bounded chat retry;
- Owned uses existing hard-silence fencing.

No committed real-Zellij harness was added. Active dependent ticket T-035-02-01 is
explicitly titled `deterministic-delivery-boundary-regression`, depends on this ticket,
and owns the automated isolated Zellij stub. This implementation supplies the complete
probe/signal/state contract that harness will exercise. The native probe test does execute
the actual shell command, but it is not represented as real-Zellij coverage.

## Source transaction

Completed through Lisa's isolated transaction:

```text
lisa commit-ticket --ticket-id T-035-04-02 \
  --message "fix(plugin): recover incomplete shell startup in place" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
a0726e2a5b3d6a4ad319447b3458bcbb30acf2b1
```

`git show --name-only` confirms that the commit contains exactly
`crates/lisa-plugin/src/lib.rs`.

The ticket-owned source path is clean and the ordinary index contains no paths. Unrelated
pre-existing orchestration/documentation changes remain untouched.
