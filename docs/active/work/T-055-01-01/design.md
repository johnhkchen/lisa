# Design — T-055-01-01 guard-waits-its-turn

## The decision in one line

Replace the single `try_lock_exclusive()` with a **poll-until-deadline loop** inside
`TransactionLock`, parameterised by a `Duration` that defaults to a module constant, and give
deadline expiry its own named message (`commit transaction guard wait timed out`) that reports how
long it waited.

## Question 1 — how do we wait?

### A. `fs2::FileExt::lock_exclusive()` (blocking, unbounded)
One line. Perfectly fair on Linux. **Rejected:** no timeout. The ticket forbids it explicitly —
"an unbounded wait would trade a spurious failure for a hang". A wedged guard would hang the loop
thread forever with no named state, violating E-055's N2.

### B. Blocking `lock_exclusive()` on a spawned thread + `recv_timeout` on the main thread
Gives a real timeout around a real blocking call. **Rejected:** on timeout the spawned thread is
still blocked in `flock` and still owns the `File`; it may acquire the guard *after* we have given
up and reported failure, leaving the guard held by a detached thread until process exit. That is
strictly worse than the bug being fixed. Cannot join it either — joining reintroduces the hang.

### C. Poll `try_lock_exclusive()` with backoff until a deadline — **chosen**
Loop: try; on `WouldBlock` sleep a bounded interval; repeat until `Instant::now() >= deadline`;
then return the named timeout error. Uses only APIs already in the file. No new dependency, no
thread, no async. The guard stays exclusive — every attempt is still `LOCK_EX | LOCK_NB`.

Cost: it is not FIFO-fair. With N waiters, wake order is arbitrary and a waiter can in principle be
passed over repeatedly. Accepted, because (i) the critical section is tens to hundreds of
milliseconds (Research §"critical section's shape"), (ii) N is `max_threads`, realistically 2–8,
and (iii) the deadline converts even pathological starvation into a named state rather than a hang.
The alternative that would buy fairness (a queue file, ticket-lock protocol) is a locking redesign,
which S-055-01's honest boundary rules out (N4).

### D. Retry at the caller (`commit_ticket_with_key`, or the plugin's `Retry` action)
The plugin already retries `TransientContention`. **Rejected:** that is what production was doing
when it failed — the retry budget and the reconciliation deadline are on different clocks, and the
deadline won. Retrying a whole transaction to re-attempt a lock also re-runs repository discovery
and completion-commit discovery, and widens the window rather than closing it. The wait belongs
where the contention is.

## Question 2 — what is the backoff?

Chosen: **exponential from 5 ms, doubling, capped at 50 ms**, with the final sleep clamped to the
remaining time before the deadline.

- 5 ms first sleep: the overwhelmingly common case is a collision of tens of milliseconds, so the
  first or second poll wins and the added latency is negligible.
- 50 ms cap: bounds wasted wakeups on a long hold to ~20/second, while keeping hand-off latency
  under a twentieth of a second.
- Clamping the last sleep means the loop never overshoots the deadline by more than one poll and
  the reported elapsed time is honest.

Rejected: fixed 10 ms (up to 3000 wakeups on a 30 s wedge, for no benefit); jittered backoff
(jitter buys decorrelation for *fairness* between many waiters, and we already accepted unfairness;
adding a randomness source to a file that has none is unwarranted).

## Question 3 — what is the timeout?

Chosen default: **30 seconds** (`COMMIT_GUARD_WAIT_TIMEOUT`).

Lower bound: it must comfortably exceed a real critical section under load. Four serialized
transactions on a cold repo are still well under 2 s; 30 s absorbs a pathological git invocation
(a large `add -A`, a slow filesystem, an antivirus scan) without ever being reached in normal
operation. Upper bound: it must expire well inside the loop's reconciliation deadline so the
failure lands as a *named* state rather than as a deadline expiry — the exact failure E-055 traced.

Rejected: making it configurable in `.lisa.toml`. No evidence anyone needs to tune it, `config.rs`
is a large surface to grow for a knob with one right answer, and the ticket does not ask for it. It
is a `const` in `commit_transaction.rs`; the day someone needs it configurable, promoting a const to
a config field is a small, well-understood change.

## Question 4 — how does the test reach the timeout without waiting 30 s?

Chosen: **`TransactionLock::acquire` keeps its signature and delegates to a private
`acquire_waiting(root, git_dir, guard_wait: Duration)`.** Production calls pass
`COMMIT_GUARD_WAIT_TIMEOUT`; the in-module unit test calls `acquire_waiting` directly with
250 ms.

Rejected alternatives:
- **Env-var override** (`LISA_COMMIT_GUARD_TIMEOUT_MS`): adds undocumented public surface, and
  environment-dependent locking behaviour is a bad thing to be able to set by accident.
- **Test-only `cfg(test)` constant**: then production's real timeout is never the one exercised,
  and the constant that ships is untested.
- **Making the timeout a public parameter of `commit_ticket`**: changes the public API and every
  call site for a test's benefit.

The private-parameter shape means the shipped default is exercised by every other test in the file
(they all go through `acquire`), while the expiry path is exercised cheaply.

## Question 5 — what is the "distinct, named error"?

`CommitTransactionError` is a newtype over `String` with no variants (Research §5). Two options:

### A. Grow the error type into an enum with a `kind()`
Would give callers a programmatic handle. **Rejected for this ticket:** `CommitTransactionError` is
public API consumed by `main.rs` and `lisa-plugin`; both consume it as a string. Restructuring it is
a change with blast radius across two crates and dozens of tests, for no consumer that currently
needs it. It is also not what the acceptance criterion asks for — it asks that the *message* let an
operator tell the two conditions apart.

### B. A dedicated constructor producing a distinctly-worded message — **chosen**

```
commit transaction guard wait timed out: <path> was still held after waiting 30s
(limit 30s); a transient collision would have cleared by now, so treat this guard as
wedged and look for a stuck lisa process
```

Properties this buys:
- **Named**: the phrase `guard wait timed out` appears nowhere else in the codebase, so both an
  operator and a test can key on it.
- **Says how long it waited**: the elapsed time, formatted by the file's existing
  `format_lock_age` (`250ms` / `30s`), plus the limit it was measured against.
- **Distinct from a transient collision**: a transient collision now produces *no error at all* —
  it produces a short wait. The two conditions are no longer the same message with different luck.

Deliberately **not** containing `temporarily locked` or `resource temporarily unavailable`: those
are the substrings `classify_completion_failure` uses for `TransientContention`. A wedged guard is
precisely not transient, and classifying it as such would put the loop back on the
retry-until-deadline path that E-055 traced. Falling into `Unrecognized` → `Park` is the correct
outcome — a state that stops and names itself — and because
`completion_failure_ask(Unrecognized)` is `None`, the call site surfaces *this message* verbatim to
the operator. The message is therefore written to be read by a person, and ends with the action.

**Not changing `classify_completion_failure`.** Adding a class would be defensible, but the ticket
does not ask for it, the existing `OperatorStaleLock` ask names the wrong file
(`.lisa-commit.lock`, the marker), and `Park` with a self-describing message is already a named
terminal state. Noted in review as a follow-up candidate, not done here.

Non-`WouldBlock` errors from `try_lock_exclusive` (`ENOLCK`, `EIO`, a filesystem without locking)
get their own immediate, non-retried error — spinning for 30 s on `ENOLCK` would be a second bug.

## Question 6 — where does the four-way fixture live?

Acceptance says it is shared with T-055-01-02 and T-055-01-03. That rules out the private
`#[cfg(test)] mod tests` inside `commit_transaction.rs`.

Chosen: **`crates/lisa-cli/tests/support/mod.rs`**, a non-target directory under `tests/`, declared
by each consuming test binary with `mod support;`. This is the standard Cargo pattern for shared
integration-test code, needs no feature flag, and costs nothing to the shipped crate. It carries
`#![allow(dead_code)]` because each consumer will use a different subset.

Rejected: exposing the fixture through the crate's existing `test-support` feature (as
`capture_usage` does). That feature is enabled today only because `lisa-plugin` depends on
`lisa-cli` with it; relying on resolver-2 feature unification to make it visible to `lisa-cli`'s own
integration tests is fragile, and it would ship fixture code in the library's public surface.

The fixture drives `complete_ticket` **in-process from four threads**, released together by a
`std::sync::Barrier`, rather than spawning four `lisa` processes. `flock` is per open file
description, so same-process handles contend exactly as separate processes do (already proven by
the existing `held_lock_returns_actionable_error` test). Threads give exact per-caller
`Result`s to assert on — including "no `os error 35` on any path" — which four subprocesses would
reduce to exit codes and interleaved stderr. The barrier maximises the collision the test exists to
create.

## What this does not change

- The marker file, its owner record, its stale-recovery path, and the "marker existed → always
  error" one-shot. Untouched.
- Exclusivity: every attempt remains `LOCK_EX | LOCK_NB`.
- `finish()` / `Drop` ordering (marker removed → marker unlocked → guard unlocked), which is what
  makes waiting correct rather than merely patient.
- The public signatures of `commit_ticket`, `complete_ticket`, `CommitTransactionError`.
- `classify_completion_failure` and every other line in `lisa-plugin`.
