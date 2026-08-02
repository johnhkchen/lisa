# Research — T-055-01-01 guard-waits-its-turn

## What the guard is

`crates/lisa-cli/src/commit_transaction.rs` implements Lisa's isolated Git transaction. Every
ticket-owned commit (`lisa commit-ticket`) and every completion seal (`complete-ticket`) funnels
through `commit_ticket_with_key` (~1004), which acquires a `TransactionLock` before touching
anything in `.git`.

`TransactionLock` (~377–585) is **two** independent mechanisms held together:

| | file | location | purpose |
|---|---|---|---|
| guard | `lisa-commit.guard` (`COMMIT_GUARD_FILE`, ~267) | inside `.git/` | mutual exclusion between live transactions |
| marker | `.lisa-commit.lock` (`COMMIT_LOCK_FILE`, ~266) | repo root | crash evidence + ownership record (PID, acquired-at ms) |

The guard is never inspected, never read, never removed — it exists only to be `flock`ed. The
marker is the opposite: it is created, JSON-populated with `TransactionLockOwner`
(schema_version/pid/acquired_unix_ms, ~270–295), removed on `finish()`, and its *survival* across
a process boundary is the signal that a previous transaction died mid-flight.

The ticket puts the marker explicitly out of scope. This research treats it as read-only context.

## The failing line

```rust
// ~402
guard_file.try_lock_exclusive().map_err(|e| {
    CommitTransactionError::new(format!(
        "commit transaction is temporarily locked by a live holder (guard {}): {e}",
        guard_path.display()
    ))
})?;
```

One attempt. No loop, no deadline, no sleep. `fs2::FileExt::try_lock_exclusive` maps to
`flock(fd, LOCK_EX | LOCK_NB)` on Unix, so a held guard yields `EWOULDBLOCK` →
`io::ErrorKind::WouldBlock` → `Resource temporarily unavailable (os error 35)`, which is
concatenated into the message verbatim. The message calls the state "temporarily locked"; the code
treats it as terminal.

`fs2` (0.4, `crates/lisa-cli/Cargo.toml`) exposes exactly three relevant calls:
`lock_exclusive()` (blocks forever), `try_lock_exclusive()` (one shot), `unlock()`. **There is no
timeout variant.** Any bounded wait has to be built from `try_lock_exclusive` plus a clock. Note
also that `unlock` is ambiguous with the now-stable `std::fs::File::unlock`, which is why every
release site in this file is written as `FileExt::unlock(&file)` (~420, ~446, ~553, ~562); the same
disambiguation discipline applies to anything added here.

## What happens after acquisition (the critical section's shape)

Understanding how long the guard is actually held decides whether waiting is cheap.

1. Guard locked (~402).
2. Marker opened, `try_lock_exclusive` on the marker (~434). Failure here unlocks the guard and
   returns "cannot acquire commit transaction lock … lock was not stolen".
3. If the marker file *pre-existed*, `acquire` **always returns an error** (~469–523) — either
   "stale … was recovered" (holder PID absent, marker deleted, retry will succeed) or "records
   holder PID … not stolen". So marker survival is a one-shot poison, not a wait state.
4. Owner metadata written (~525).
5. Caller runs `discover_completion_commit` (only for keyed completions), reserves an
   `AlternateIndex`, and runs `run_transaction_body`: `rev-parse HEAD`, `staged_snapshot`,
   `read-tree`, `add -A -- <includes>`, overlap check, `write-tree`, `commit-tree`, `update-ref`,
   `reset -- <paths>`, second `staged_snapshot` comparison.
6. `lock.finish()` (~539) removes the marker, unlocks the marker, unlocks the guard — **in that
   order**, so the marker is gone before the next waiter can see the guard free. This ordering is
   what makes waiting safe: a queued transaction that wakes up finds `marker_existed == false`.

The critical section is ~8–10 `git` subprocess invocations. On a warm repo that is tens of
milliseconds; on a cold or large one, a few hundred. Nothing in it blocks on the network or on a
human. This is a section worth queueing for, not dying on.

`Drop for TransactionLock` (~578) calls `finish()` if anything is still held, so a panicking or
early-returning holder releases the guard. A holder that is *killed* leaves the guard released by
the kernel (flock is per-open-file-description) but the marker file behind — which is exactly the
stale-marker path in step 3.

## Who sees the error today

`commit_ticket` / `complete_ticket` are `pub` (via `crates/lisa-cli/src/lib.rs`, `pub mod
commit_transaction`). Two consumers:

- `crates/lisa-cli/src/main.rs` ~492–528 — the `lisa commit-ticket` / `lisa complete-ticket`
  subcommands. Error goes to stderr, non-zero exit.
- `crates/lisa-plugin/src/lib.rs` ~12207, ~23626 — the loop's completion path. The error string is
  fed to `classify_completion_failure` (~468–499), which is **string-matching**:

  ```rust
  } else if (detail.contains("index.lock") && detail.contains("another git process"))
      || detail.contains("resource temporarily unavailable")
      || detail.contains("temporarily locked")
  { CompletionFailureClass::TransientContention }
  ```

  Today's guard error therefore classifies as `TransientContention` → `Retry` while
  `failure_count < MAX_COMPLETION_FAILURES`, then `WaitForDeadline`. E-055's field trace records
  the deadline passing before a retry landed, which is how a "transient" failure became a
  `rejected` completion with `retryability: action-required`.

  Also relevant: `completion_failure_ask(Unrecognized) == None`, and the call site
  (~3540) does `completion_failure_ask(class, ticket_id).unwrap_or_else(|| failure.clone())` —
  an unclassified failure surfaces **its own message** to the operator. That means the wording of
  a new terminal error is itself the operator-facing ask. It also means the classifier's
  `OperatorStaleLock` ask ("Remove `.lisa-commit.lock`") is about the *marker*, and would be wrong
  advice for a wedged *guard*.

## Existing test ground

Unit tests live in-module at the bottom of `commit_transaction.rs` (~1173–1874) with a `GitRepo`
harness (`new`, `write`, `git`, `git_string`, `base_commit`, `assert_no_commit_lock`, `request`).
Relevant precedents:

- `held_lock_returns_actionable_error` (~1401) holds the **marker** with a second `File` handle in
  the same process and asserts the error text. This proves that same-process `flock` contention is
  observable — two `open()` calls in one process are two open file descriptions, so they conflict.
  A guard-timeout test can use the same trick.
- `stale_commit_lock_names_age_and_absent_holder_then_recovers` (~1435) spawns and reaps a child
  to obtain a genuinely absent PID.

Integration tests live in `crates/lisa-cli/tests/*.rs`. They are separate binaries; they may use
`lisa_cli::commit_transaction::*` and `lisa_core::*` (both are `[dependencies]` of `lisa-cli`, and
`[dependencies]` are visible to test targets). Some drive the built binary via
`env!("CARGO_BIN_EXE_lisa")`. **There is no shared helper module today** — no `tests/common/mod.rs`
pattern exists in this repo yet; `tests/fixtures/` holds only data files and shell harnesses.

## Constraints and assumptions surfaced

1. **No async, no runtime.** `lisa-cli` is synchronous and shells out to `git`. A bounded wait must
   be `std::thread::sleep` + `Instant`, not a timer abstraction.
2. **`max_threads = 2` in this repo's `.lisa.toml`; the field run was at 4.** The fixture in the
   acceptance criteria is four-way, above what this repo itself runs — so the fixture must create
   its own concurrency, not rely on the scheduler.
3. **The guard must stay exclusive** (N4). Shared locks, lock stealing, or dropping the guard when
   contended are all out.
4. **The marker mechanism is untouchable** (ticket, explicit). Any change must leave lines ~409–534
   byte-identical in behavior, including the "marker existed → always error" one-shot poison.
5. **Error type is a newtype over `String`** (`CommitTransactionError(String)`, ~48). There are no
   variants and no `kind()`. "A distinct, named error" has to be expressed as a distinct,
   recognizable *message* unless the type grows — and the plugin's classifier reads messages, so
   message wording is load-bearing.
6. **`just check` is the gate**: `cargo check -p lisa-plugin --target wasm32-wasip1`,
   `cargo fmt --all --check`, `cargo clippy -p {plugin,core,cli} -- -D warnings`,
   `cargo test --workspace`. Clippy at `-D warnings`, so no dead code, no unused imports.
7. **Wall-clock in tests is a flake risk.** A timeout test must assert a generous upper bound
   (does not hang) and a tight lower bound (did actually wait), not an exact duration.
8. **Downstream reuse.** T-055-01-02 (empty diff → convergent no-op, same file) and T-055-01-03
   (`lisa unblock` route out, different files) both depend on this ticket and both need the
   four-way fixture. They run *after* this ticket completes, so there is no concurrent-edit clobber
   risk with them — but the fixture must be reachable from a second and third test binary, which
   rules out putting it inside the private `#[cfg(test)] mod tests` of `commit_transaction.rs`.
9. `complete_ticket` mutates the ticket file (`update_ticket_done`) *before* the lock is taken, and
   restores the original bytes if the transaction fails. Concurrent sealers each touch their own
   ticket file, so this is safe under a four-way fixture provided each thread owns distinct paths.
