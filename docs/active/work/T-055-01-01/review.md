# Review — T-055-01-01 guard-waits-its-turn

## What changed

| File | Change |
|---|---|
| `crates/lisa-cli/src/commit_transaction.rs` | modified — guard wait loop, named timeout error, one unit test |
| `crates/lisa-cli/tests/support/mod.rs` | created — shared four-way seal fixture |
| `crates/lisa-cli/tests/guard_waits_its_turn.rs` | created — the concurrency test |

Two commits through `lisa commit-ticket`:

- `141770a` — let the commit guard wait its turn
- `27d9617` — prove four seals can land at once

### The production change

`TransactionLock::acquire` is now a one-line delegate to a new `acquire_waiting(root, git_dir,
guard_wait: Duration)`. Inside the moved body, the six lines that turned `EWOULDBLOCK` into an
error became one call to a new `lock_guard_waiting`, which polls `try_lock_exclusive()` — still
`LOCK_EX | LOCK_NB`, still exclusive — with a 5 ms interval doubling to a 50 ms ceiling, until a
30 s deadline. The last sleep is clamped to the time remaining, so the loop cannot overshoot the
deadline by more than one poll and the elapsed time it reports is honest.

Three new constants (`COMMIT_GUARD_WAIT_TIMEOUT`, `COMMIT_GUARD_POLL_MIN`,
`COMMIT_GUARD_POLL_MAX`) sit beside `COMMIT_GUARD_FILE` with their rationale in doc comments.

Deadline expiry gets its own constructor, `guard_wait_timeout_error`:

```
commit transaction guard wait timed out: <path> was still held after waiting 30s (limit 30s);
a transient collision would have cleared by now, so treat this guard as wedged and look for a
stuck lisa process
```

Two properties are load-bearing rather than cosmetic. `guard wait timed out` appears nowhere else
in the codebase, so an operator and a test can both key on it. And the message deliberately avoids
`temporarily locked` / `resource temporarily unavailable`, because
`classify_completion_failure` (`crates/lisa-plugin/src/lib.rs` ~468) reads exactly those substrings
as `TransientContention` → retry-then-wait-for-deadline. That is the path E-055 traced into a
`rejected` completion. A guard that outlasted a full 30 s wait is not transient; it falls into
`Unrecognized` → `Park`, and because `completion_failure_ask(Unrecognized)` is `None`, the call
site (~3540) surfaces this message verbatim. So the message is written for a person and ends with
the action.

A non-`WouldBlock` error from `try_lock_exclusive` (`ENOLCK`, `EIO`, a filesystem without locking)
returns immediately with its own message rather than spinning for 30 s on a condition that will
not clear.

## Test coverage

**`guard_wait_times_out_with_a_named_error_and_does_not_hang`** (unit, in-module). Holds
`.git/lisa-commit.guard` with a second in-process `File` — `flock` is per open file description,
so this contends exactly as another process would, the same technique the pre-existing
`held_lock_returns_actionable_error` uses on the marker. Calls `acquire_waiting` with a 250 ms
bound and asserts: the named phrase is present; `250ms` is in the message; `os error 35` is not;
the word `temporarily` appears nowhere; `elapsed >= 250ms` (it looped rather than failing on the
first attempt); `elapsed < 10s` (it did not hang); no marker was left behind. Then releases the
holder and proves the fast path still acquires and `finish()`es cleanly. 0.30 s.

**`four_concurrent_seals_all_land_exactly_one_commit_each`** (integration). Four tickets, four
threads released together by a `Barrier`, each calling `complete_ticket`. Asserts all four
`Result`s are `Ok`; no result carries any of `os error 35` / `resource temporarily unavailable` /
`temporarily locked`; HEAD advanced by exactly four; each of the four commit subjects appears
exactly once; each ticket at HEAD carries `phase: done` and `status: done`; each work artifact is
at HEAD; no marker remains. Green five consecutive runs (0.43–0.50 s).

**Negative control, run twice.** Restoring the literal pre-fix
`guard_file.try_lock_exclusive().map_err(…)` makes the fixture fail on the collision assertion
with the exact field error. A weaker control — `acquire` delegating `Duration::ZERO` — fails on
`is_ok()` instead, reporting the new timeout message. Both edits were reverted;
`git diff` against commit 1 is empty.

**Regression surface.** All 16 pre-existing `commit_transaction` unit tests pass unmodified,
including both marker-path tests. The 583 `lisa-plugin` tests, several of which drive
`complete_ticket` against real repositories, pass unmodified — every one of them exercises the new
`acquire` delegate.

**`just check`: exit code 0**, captured as an exit code rather than read off grepped output.

## Coverage gaps, stated plainly

1. **No cross-process concurrency test.** The fixture contends in-process. `flock` semantics make
   this equivalent, and the negative control proves the fixture reproduces the field bug — but it
   is not literally four `lisa` binaries. A subprocess variant would be slower and would reduce
   per-caller `Result`s to exit codes and interleaved stderr.
2. **The 30 s default is never exercised end-to-end.** Testing it would cost 30 s of wall clock.
   The mechanism is proven at 250 ms and the constant is a `Duration` literal.
3. **Starvation is not tested.** Polling is not FIFO-fair; a waiter can in principle be passed
   over. With critical sections of tens to hundreds of milliseconds and `max_threads` in the
   single digits this is not reachable in practice, and the deadline converts even the
   pathological case into a named state. Testing it would require a scheduler adversary.
4. **`assert_no_guard_collision` is a tripwire, not a live discriminator.** Once the wait exists,
   raw `EWOULDBLOCK` is consumed inside `lock_guard_waiting` and cannot reach a caller by any
   route. The assertion earns its place by failing if the old code ever returns.

## Open concerns for a human

**One, worth a decision but deliberately not made here.** A wedged guard now classifies as
`Unrecognized` → `Park`. That is a correct terminal state and the operator sees the full message,
but it is not a *typed* class. Adding a `CompletionFailureClass` for it would give a proper ask —
except that the nearest existing class, `OperatorStaleLock`, tells the operator to remove
`.lisa-commit.lock`, which is the **marker**, not the guard, and would be wrong advice. Adding a
new class touches `crates/lisa-plugin/src/lib.rs`, which this ticket has no other reason to open,
and the acceptance criteria ask only that the message let an operator tell the conditions apart.
Left as a follow-up candidate.

**Two, an observation for T-055-01-02.** `TransactionLock::acquire` still returns an error on
*every* path where the marker file pre-existed (`commit_transaction.rs` ~469–523) — both the
"stale, recovered" and the "not stolen" branches. That is unchanged and out of scope here, but it
means a crashed holder still costs one failed attempt before the next one can succeed. Waiting
does not help that case, because the marker is evidence rather than a lock.

**Three, scope discipline.** The marker file, its `TransactionLockOwner` record, the stale-recovery
branches, `finish()` / `Drop` ordering, and every public signature are untouched, as the ticket
required. `crates/lisa-plugin` has no changes at all.

## Acceptance criteria

| Criterion | Status |
|---|---|
| Retries on `EWOULDBLOCK` until a bounded timeout; guard stays exclusive | Met — `lock_guard_waiting`; every attempt is `LOCK_EX \| LOCK_NB`; the unit test's `elapsed >= bound` proves it looped |
| Four concurrent transactions all seal, one commit each, no `os error 35` | Met — `four_concurrent_seals_all_land_exactly_one_commit_each`, five consecutive green runs |
| Timeout is a distinct named error saying how long it waited | Met — `guard wait timed out`, elapsed and limit both rendered by `format_lock_age` |
| A test proves the timeout path terminates within the bound and does not hang | Met — `guard_wait_times_out_with_a_named_error_and_does_not_hang` |
| Marker file and ownership record untouched | Met — no edit below ~409; both marker tests pass unmodified |
| Fixture written to be shared with T-055-01-02 and T-055-01-03 | Met — `tests/support/mod.rs`, reachable by `mod support;` from any test binary |
| `just check` green | Met — exit code 0 |
