# Plan — T-055-01-01 guard-waits-its-turn

Four steps, two commits. Each step is independently verifiable.

---

## Step 1 — Constants, wait loop, and timeout error

**File:** `crates/lisa-cli/src/commit_transaction.rs`

1. Extend the import on line 18 to `use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};`.
2. Add `COMMIT_GUARD_WAIT_TIMEOUT` (30 s), `COMMIT_GUARD_POLL_MIN` (5 ms), `COMMIT_GUARD_POLL_MAX`
   (50 ms) next to `COMMIT_GUARD_FILE` (~267), each with a one-line doc comment carrying the
   rationale.
3. Add `guard_wait_timeout_error(guard_path, waited, limit) -> CommitTransactionError` beside
   `format_lock_age` (~369), reusing `format_lock_age` for both durations.
4. Add `lock_guard_waiting(guard_file, guard_path, guard_wait) -> Result<(), _>` above
   `struct TransactionLock` (~377).

**Verify:** `cargo check -p lisa-cli` — compiles; the new functions are unused at this point, so
this step is checked but not clippy-clean until step 2 wires it in. Steps 1 and 2 are therefore
applied together before running any gate.

---

## Step 2 — Split `acquire`, call the wait loop

**File:** `crates/lisa-cli/src/commit_transaction.rs`

1. Rename `TransactionLock::acquire` to `acquire_waiting`, adding a third parameter
   `guard_wait: Duration`.
2. Add a new `acquire(root, git_dir)` that delegates with `COMMIT_GUARD_WAIT_TIMEOUT`.
3. Replace the `guard_file.try_lock_exclusive().map_err(…)?` block (~402–407) with
   `lock_guard_waiting(&guard_file, &guard_path, guard_wait)?;`.
4. Change nothing else in the body — marker open, marker lock, marker-existed branches, owner
   write, and the `Ok(lock)` return stay byte-for-byte.

**Verify:**
```
cargo test -p lisa-cli --lib commit_transaction
```
All existing `commit_transaction` unit tests still pass — in particular
`held_lock_returns_actionable_error` (marker path, must be unchanged) and
`stale_commit_lock_names_age_and_absent_holder_then_recovers`. None of them contends the guard, so
none of them should get slower; if the suite time moves, something acquires the guard twice.

---

## Step 3 — Unit test for the timeout path

**File:** `crates/lisa-cli/src/commit_transaction.rs`, `mod tests`, after
`held_lock_returns_actionable_error`.

```
#[test]
fn guard_wait_times_out_with_a_named_error_and_does_not_hang()
```

Body:
1. `GitRepo::new()`, write `ticket.txt`, `base_commit()`.
2. Resolve `git_dir` via `repo.git_string(["rev-parse", "--absolute-git-dir"])`.
3. Open `git_dir/lisa-commit.guard` with a second `File` and `try_lock_exclusive().unwrap()` — a
   holder that is never released for the duration of the test (this is the "wedged" condition).
4. `let started = Instant::now();`
   `let error = TransactionLock::acquire_waiting(repo.root(), &git_dir, Duration::from_millis(250))
       .unwrap_err().to_string();`
   `let elapsed = started.elapsed();`
5. Assertions:
   - `error.contains("guard wait timed out")` — the named condition.
   - `error.contains("250ms")` — it says how long it waited (AC 3).
   - `!error.contains("os error 35")` and `!error.to_lowercase().contains("temporarily")` — it is
     not the transient message and will not be classified as transient contention.
   - `elapsed >= Duration::from_millis(250)` — it really waited the bound (AC 1: it retried, it did
     not fail on the first attempt).
   - `elapsed < Duration::from_secs(10)` — it did not hang (AC 4). Ten seconds is 40× the bound;
     generous enough for a loaded CI box, tight enough that a regression to unbounded blocking
     fails rather than hangs the suite.
   - `!repo.root().join(COMMIT_LOCK_FILE).exists()` — a failed acquisition leaves no marker behind.
6. Release: `FileExt::unlock(&holder).unwrap();`.

Then a second, cheap assertion in the same test that the *fast* path did not regress: after
unlocking, `TransactionLock::acquire_waiting(root, git_dir, Duration::from_millis(250))` succeeds,
and `finish()` on it returns `Ok(())`.

**Verify:** `cargo test -p lisa-cli --lib guard_wait_times_out` — passes in well under a second.

**Commit 1:**
```
lisa commit-ticket --ticket-id T-055-01-01 \
  --message "let the commit guard wait its turn" \
  --include crates/lisa-cli/src/commit_transaction.rs
```

---

## Step 4 — The shared four-way concurrency fixture

**Files:** `crates/lisa-cli/tests/support/mod.rs` (new),
`crates/lisa-cli/tests/guard_waits_its_turn.rs` (new).

`support/mod.rs` per structure.md §2. Construction detail that matters:

- `SealFixture::new(&["T-A", "T-B", "T-C", "T-D"])` writes for each id a ticket at
  `docs/active/tickets/{id}.md` with `status: open` / `phase: review` frontmatter (so
  `update_ticket_done` has something to rewrite) and `docs/active/work/{id}/research.md`, then
  `git add -A` + `git commit` for the base commit, then **rewrites each work artifact** so every
  seal has a real diff. Without that last step every transaction hits the empty-diff error, which
  is T-055-01-02's subject, not ours.
- `complete_request` builds `CompleteTicketRequest` with `completion_key:
  CompletionGenerationId::new(CompletionId::new(id), AttemptId::new("1"), generation)` and message
  `format!("Complete {id}")`.
- `dispatch_together` — `Barrier::new(count)`, `std::thread::scope`, results collected by index.

`guard_waits_its_turn.rs` runs the seven assertions listed in structure.md §3.

**Verify:**
```
cargo test -p lisa-cli --test guard_waits_its_turn
```
Run it **five times in a row**; concurrency tests that pass once prove nothing. Expected wall time
per run: well under two seconds (four serialized transactions of ~8 git invocations each).

Sanity check that the test can actually fail: temporarily revert step 2's one-line change (restore
`try_lock_exclusive()?`) and confirm the fixture goes red with `os error 35`. Restore afterwards.
This is a manual check, not a committed artifact.

**Commit 2:**
```
lisa commit-ticket --ticket-id T-055-01-01 \
  --message "prove four seals can land at once" \
  --include crates/lisa-cli/tests/support/mod.rs \
  --include crates/lisa-cli/tests/guard_waits_its_turn.rs
```

---

## Testing strategy summary

| AC | Proven by |
|---|---|
| Retries on `EWOULDBLOCK` until a bounded timeout; guard stays exclusive | Step 3 lower bound (`elapsed >= 250ms` means it looped rather than failing at once) + step 4 (four exclusive hand-offs, four commits) |
| Four concurrent transactions all seal, one commit each, no `os error 35` | Step 4, `four_concurrent_seals_all_land_exactly_one_commit_each` |
| Timeout is a distinct named error and says how long it waited | Step 3 assertions on `guard wait timed out`, `250ms`, and absence of the transient wording |
| Timeout path terminates within the bound, does not hang | Step 3 `elapsed < 10s` |
| Marker file and ownership record untouched | No edit below `commit_transaction.rs` ~409; existing `held_lock_returns_actionable_error` and `stale_commit_lock_…` pass unmodified |
| `just check` green | Final gate |

## Final gate

```
just check
```
Judged by exit code, not by reading output. `cargo test --workspace` includes the `lisa-plugin`
tests that call `complete_ticket` — they exercise the new `acquire` delegate on every run.

## Risks and how they are handled

1. **Flaky timing.** Mitigated by asymmetric bounds: tight lower (the thing being proven), loose
   upper (the thing being guarded against).
2. **Four threads shelling out to `git` in one temp repo.** Everything that touches `.git` state is
   inside the guard; only `Repository::discover` (two read-only `rev-parse` calls) runs outside it.
3. **`tests/support/mod.rs` dead-code warnings** in future consumers that use a subset —
   `#![allow(dead_code)]` at the top of the module.
4. **Test suite runtime.** The guard-timeout test costs 250 ms, the concurrency test under 2 s.
