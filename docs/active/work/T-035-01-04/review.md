# T-035-01-04 Review — bounded startup recovery

## Review outcome

The ticket acceptance criterion is met. A fresh native provider assignment now has a
finite start-observation deadline beginning at actual launch submission. If its exact
process-start signal never arrives, the assignment leaves `Starting` for the named
terminal `StartupFailed` state, never becomes `Owned`, and cannot automatically relaunch.

No critical defect was found in self-review. The implementation is committed, all
focused and workspace tests pass, and both ticket-owned source paths are clean.

## Source commit

```text
ae2fd95e72cea4e86584292e3ecf33424b3c132e
fix(plugin): bound fresh startup recovery
```

The isolated Lisa transaction includes exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

No ticket/work artifact, frontmatter, provenance, or concurrent file was included.

## Change summary

### Bounded starting state

`SeatAssignmentState::Starting` now stores:

```rust
start_deadline: Option<SystemTime>
```

This makes launch timing explicit rather than deriving it from thread creation or pane
transport timestamps.

The two shapes have distinct meanings:

- `Starting { start_deadline: None, .. }`: the seat and attempt are reserved, but a
  delayed fresh launcher has not yet been submitted;
- `Starting { start_deadline: Some(_), .. }`: the provider launch was submitted and the
  wait for the exact `.started` lease signal is bounded.

The original `generation` remains unchanged, preserving the exact attempt identity used
by `acknowledge_process_start`.

### Correct clock boundary

The existing E-033 deadline helper now arms startup as well as recycled/recovery Codex
acknowledgment. It continues to use the positive configured
`assignment_ack_timeout_secs` and adds the delayed-Enter allowance.

Immediate fresh routes arm after the scheduler submits their prepared launcher:

- empty native pane;
- `FreshExec` reuse from an idle shell.

Cross-provider recycling does not arm at reservation or `/exit`. It stays unarmed during
`WaitingForExit`, then uses the existing exit-grace launch call site to arm only after
the incoming provider command is submitted.

Same-process Claude reuse remains `Owned` and the helper is a no-op. Recycled Codex
clear-handshake behavior remains unarmed until its tagged prompt is actually delivered.

### Terminal startup failure

Added `SeatAssignmentState::StartupFailed`.

The new `fail_startup` helper performs a retained terminal transition:

- accepts only the current `Starting` state;
- writes `StartupFailed` before auxiliary work;
- marks the associated logical thread failed;
- retains the physical slot's ticket and exact attempt lease;
- retains the current lease authority;
- appends one deduplicated ticket/pane error alert;
- logs the missing-start reason and explicit reset-to-retry instruction.

It deliberately does not:

- grant ownership;
- revoke or mint a lease;
- release the slot;
- remove the thread;
- call a provider adapter;
- send `/exit`, a launcher, or any other pane input;
- arm another transition or acceptance timer.

The terminal state therefore has no automatic edge back into scheduling.

### Timeout evaluation

`check_assignment_ack_timeouts_at` now includes armed `Starting` states in its absolute
deadline collection. An unchanged expired start invokes `fail_startup`.

Existing E-033 behavior remains unchanged:

- `AssignedPendingAck` expiry begins its one fresh Codex recovery;
- `Recovering` expiry becomes `RecoveryFailed`.

Unarmed starting states, owned seats, and both terminal failure states are excluded from
future timeout actions.

The poll loop order was not changed. `.started` consumption runs before transition and
deadline processing. A valid exact start visible on the boundary poll becomes `Owned`
before timeout evaluation can inspect the old state.

### Operator-facing state

Added `ui::SeatAssignmentStatus::StartupFailed` with label `startup-failed` and terminal
red color, plus the exact internal-to-UI mapping.

This remains distinct from E-033's `recovery-failed`, avoiding ambiguity between an
initial provider-start observation failure and a recycled-Codex fallback failure.

Failed threads are excluded from the active thread table by existing UI architecture.
Operational visibility is therefore primarily the failed attention alert and actionable
activity error; the explicit assignment status is still present in UI state. Broadening
failed-row rendering is outside this ticket.

## Acceptance criteria assessment

### Native test never delivers the start signal

Met by:

```text
test_missing_fresh_start_signal_fails_within_bound_without_relaunch
```

The test schedules through the real native fresh-dispatch path and never creates a
`.started` file.

### Leaves starting within bounded deadline

Met. The fixture sets the existing configured bound to one second, extracts the exact
stored `start_deadline`, and invokes the injected-time evaluator at that deadline.

Before expiry it asserts:

- `Starting` carries the scheduler-minted attempt generation;
- its deadline is armed;
- the dashboard active row says `starting`;
- `seat_is_owned` is false.

At expiry it asserts the state is `StartupFailed`.

### Named recovery/failed state

Met. `StartupFailed` is a distinct internal state and maps to the named UI status
`StartupFailed`/`startup-failed`. The same transition fails the thread, creates a failed
attention alert, and records reset guidance.

### Never reaches Owned

Met. The regression asserts the seat is not owned before and after timeout. The start
admission method accepts only `Starting`, so a late signal after terminal failure cannot
promote it.

### Never relaunches without bound

Met with a stronger zero-relaunch assertion. The test records one initial
`SessionLaunch`, evaluates the timeout, then evaluates again at three later times. State,
alert count, and launch count remain unchanged.

The production failure helper has no launch, transition, release, or scheduling call.
The retained slot reservation also prevents normal dispatch from selecting the pane.

### Reuses E-033 bounded fallback

Met at the shared policy/mechanism boundary:

- positive `assignment_ack_timeout_secs` configuration;
- Enter-delay-aware absolute deadline calculation;
- injected-time deterministic evaluator;
- signal-before-timeout poll ordering;
- retained failed thread/seat and operator reset action;
- no infinite automatic retry.

E-033's Codex-specific `.ack` generation and fresh recovery launch are not reused as
startup semantics. This preserves provider neutrality and avoids routing Claude through
a Codex acknowledgment state machine.

## Test coverage

### Direct ticket coverage

- immediate fresh launch arms `Some(start_deadline)`;
- matching generation is retained while starting;
- missing `.started` reaches `StartupFailed` exactly at the deadline;
- timeout cannot imply ownership;
- thread becomes failed but remains present;
- slot ticket and exact lease remain present;
- current lease remains authoritative;
- one alert is emitted;
- reset guidance is logged;
- UI conversion exposes the named terminal status;
- repeated later evaluation is inert;
- launch count stays exactly one.

### Positive path coverage

The predecessor regression still passes and proves:

- malformed and stale start signals fail closed;
- exact current lease promotes `Starting -> Owned`;
- duplicate exact signals cannot repeat the transition;
- the positive active dashboard row changes from `starting` to `owned`.

### E-033/E-034 regression coverage

Focused passes:

```text
dropped_post_prompt_ack   1 passed
split_brain               1 passed
recycled_codex            2 passed
```

These protect bounded recycled-Codex recovery, exact acknowledgment, and attempt lease
fencing from the shared timeout-helper extension.

### Full verification

```text
cargo fmt --all -- --check    passed
cargo test -p lisa-plugin     278 passed; 0 failed
cargo test --workspace        passed
```

Workspace breakdown observed:

- `lisa-cli`: 274 passed;
- `lisa-core`: 155 passed;
- `lisa-plugin`: 278 passed;
- doc tests: passed.

`git diff --check` passed for both owned source files.

## Code review observations

### Safety invariants preserved

- `seat_is_owned` still recognizes only `Owned`.
- Exact process-start admission still requires current slot/ticket/lease agreement.
- Startup state remains excluded from Codex acknowledgment generations.
- Timeout collection compares the unchanged copied state before mutating.
- Terminal failure is written before logging or optional thread resolution.
- No generic error handler is used, avoiding slot release and redispatch.

### Scope remains narrow

No changes were made to hooks, adapters, launcher preparation, config transport, CLI,
DAG scheduling policy, ticket parsing, or completion transactions.

The two modified files are the same scheduler/UI boundary established by the predecessor
ownership-gate ticket.

## Open concerns and limitations

### Shared timeout setting name

`assignment_ack_timeout_secs` now bounds two positive acceptance events: recycled Codex
prompt acknowledgment and fresh provider start. The policy is coherent and avoids a new
configuration surface, but the name is narrower than its resulting use. A future config
cleanup could rename it with migration support; that is not needed for correctness.

### False-negative start signal requires operator action

A provider may actually be running while its hook signal is lost. Lisa intentionally
does not infer ownership in that case. The operator must inspect the failed alert/pane,
repair signaling if necessary, and reset the ticket. This is the safety tradeoff required
by P2 and the story's positive ownership contract.

### Failed assignment label is not an active-row rendering

The UI conversion contains `startup-failed`, but existing thread-table rendering omits
all failed threads. Operators see the failure through the attention banner and activity
log. Rendering retained failed assignments as slot rows could improve P4 further, but it
would also affect E-033 `RecoveryFailed` and belongs in a dedicated UI ticket.

### Honest test boundary

The regression is a deterministic native scheduler test. It does not start a real
provider in Zellij or validate installed hooks. The parent story assigns real PTY and
installed-provider evidence to later E-035 stories/tickets.

## Repository hygiene

Post-commit checks show:

- `crates/lisa-plugin/src/lib.rs` clean;
- `crates/lisa-plugin/src/ui.rs` clean;
- neither path staged in the ordinary index;
- commit contains exactly those paths;
- concurrent Lisa/provenance/story/ticket changes remain untouched.

The attempt wrote phase artifacts only under
`.lisa/attempts/T-035-01-04/1/work/`. Lisa independently admitted/published copies under
the shared work path and advanced phase frontmatter. This attempt did not manually edit
phase or status.

## Final state

- Research complete.
- Design complete.
- Structure complete.
- Plan complete.
- Implement complete.
- Source commit complete.
- Review complete.
- No critical issue remains.
- The agent remains assigned to T-035-01-04 pending Lisa's completion commit.
