# Structure — T-055-01-01 guard-waits-its-turn

Three files. One modified, two created.

```
crates/lisa-cli/src/commit_transaction.rs      MODIFIED   guard wait + timeout error + unit test
crates/lisa-cli/tests/support/mod.rs           CREATED    shared four-way seal fixture
crates/lisa-cli/tests/guard_waits_its_turn.rs  CREATED    the concurrency test that consumes it
```

No `Cargo.toml` change: `tempfile` is already a `[dependencies]` entry of `lisa-cli`, `lisa-core`
likewise, and both are visible to test targets. `std::sync::Barrier` and `std::thread::scope` need
no dependency.

---

## 1. `crates/lisa-cli/src/commit_transaction.rs` (modified)

### 1a. Imports

`use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};` — `Duration` and `Instant` added to
the existing line 18 import.

### 1b. New constants, beside `COMMIT_GUARD_FILE` (~267)

```rust
/// How long a commit transaction waits for the guard before declaring it wedged.
const COMMIT_GUARD_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
/// First poll interval while the guard is held.
const COMMIT_GUARD_POLL_MIN: Duration = Duration::from_millis(5);
/// Ceiling for the doubling poll interval.
const COMMIT_GUARD_POLL_MAX: Duration = Duration::from_millis(50);
```

Doc comments carry the *why* from design (common case is a short collision; ceiling bounds wakeups).

### 1c. New free function `lock_guard_waiting`

```rust
fn lock_guard_waiting(
    guard_file: &File,
    guard_path: &Path,
    guard_wait: Duration,
) -> Result<(), CommitTransactionError>
```

Contract:

- Returns `Ok(())` with the guard held exclusively, or an `Err` with the guard **not** held.
- Loop: `guard_file.try_lock_exclusive()`.
  - `Ok(())` → return `Ok(())`.
  - `Err(e)` where `e.kind() != ErrorKind::WouldBlock` → return immediately with
    `"cannot lock commit transaction guard {path}: {e}"`. Not retried (design Q5).
  - `Err(_)` (contended) → if `started.elapsed() >= guard_wait`, return the timeout error;
    otherwise sleep `min(interval, guard_wait - started.elapsed())`, then
    `interval = min(interval * 2, COMMIT_GUARD_POLL_MAX)` and loop.
- One `try_lock_exclusive` always happens before any sleep, so an uncontended acquire pays nothing.
- A `guard_wait` of zero degrades to exactly today's single-attempt behaviour.

Timeout message, built by a private constructor so the wording lives in one place:

```rust
fn guard_wait_timeout_error(
    guard_path: &Path,
    waited: Duration,
    limit: Duration,
) -> CommitTransactionError
```

produces

```
commit transaction guard wait timed out: {path} was still held after waiting {waited}
(limit {limit}); a transient collision would have cleared by now, so treat this guard as
wedged and look for a stuck lisa process
```

with both durations rendered by the file's existing `format_lock_age(u64 ms)` (`250ms`, `30s`).
Deliberately free of `temporarily locked` / `resource temporarily unavailable` (design Q5).

### 1d. `TransactionLock::acquire` split

```rust
impl TransactionLock {
    fn acquire(root: &Path, git_dir: &Path) -> Result<Self, CommitTransactionError> {
        Self::acquire_waiting(root, git_dir, COMMIT_GUARD_WAIT_TIMEOUT)
    }

    fn acquire_waiting(
        root: &Path,
        git_dir: &Path,
        guard_wait: Duration,
    ) -> Result<Self, CommitTransactionError> { /* today's body, one line changed */ }
}
```

The only edit inside the moved body is lines ~402–407 becoming
`lock_guard_waiting(&guard_file, &guard_path, guard_wait)?;`. Everything from the marker `open`
(~409) to the final `Ok(lock)` (~536) is carried over unchanged — same order, same messages, same
one-shot marker poison. The single production call site
(`commit_ticket_with_key` ~1018, `TransactionLock::acquire(&repo.root, &repo.git_dir)?`) is
untouched.

`acquire` becomes a one-line delegate, so `#[allow(dead_code)]` is never needed; both functions have
callers (`acquire` from production, `acquire_waiting` from production-via-`acquire` and from tests).

### 1e. New unit test in `mod tests`

`guard_wait_times_out_with_a_named_error_and_does_not_hang` — see plan.md step 3 for its body.
Placed immediately after `held_lock_returns_actionable_error` (~1432), which it parallels: same
"hold the lock with a second `File` in this process" technique, applied to the guard instead of the
marker.

---

## 2. `crates/lisa-cli/tests/support/mod.rs` (created)

A directory under `tests/` is not a test target, so this compiles only into binaries that declare
`mod support;`. Header: `#![allow(dead_code)]` — each consumer uses a different subset, and clippy
runs at `-D warnings`.

Public surface, kept deliberately small and un-clever so T-055-01-02 and T-055-01-03 can extend it
without rewriting it:

```rust
/// A throwaway Git repository seeded with N sealable tickets.
pub struct SealFixture { temp: TempDir }

impl SealFixture {
    /// git init + identity; writes `docs/active/tickets/{id}.md` (phase: review) and
    /// `docs/active/work/{id}/research.md` for each id; base commit; then dirties each
    /// ticket's work directory so every seal has a non-empty diff.
    pub fn new(ticket_ids: &[&str]) -> Self;

    pub fn root(&self) -> &Path;
    pub fn git(&self, args) -> Output;          // asserts success
    pub fn git_string(&self, args) -> String;   // trimmed stdout

    /// `CompleteTicketRequest` for one ticket at a given generation.
    pub fn complete_request(&self, ticket_id: &str, generation: u64) -> CompleteTicketRequest;

    pub fn head_commit_count(&self) -> usize;
    pub fn commit_subjects(&self) -> Vec<String>;
    /// HEAD blob for a repo-relative path, or None when absent.
    pub fn show_at_head(&self, path: &str) -> Option<String>;
    pub fn assert_no_commit_lock(&self);        // `.lisa-commit.lock` gone
}

/// Release `bodies.len()` threads simultaneously through a `Barrier` and collect
/// their results in dispatch order. Panics are propagated with the thread's index.
pub fn dispatch_together<T, F>(count: usize, body: F) -> Vec<T>
where F: Fn(usize) -> T + Sync, T: Send;

/// The literal signature of the bug this fixture exists to keep out.
pub const GUARD_COLLISION_SIGNATURES: [&str; 3] =
    ["os error 35", "resource temporarily unavailable", "temporarily locked"];

/// Assert no result's rendered error contains any collision signature.
pub fn assert_no_guard_collision(results: &[Result<CommitTransactionResult, CommitTransactionError>]);
```

`dispatch_together` uses `std::thread::scope` so `&SealFixture` can be borrowed without `Arc`, and
`std::sync::Barrier::new(count)` so all four threads reach `complete_ticket` at once. Results are
returned indexed, not in completion order, so assertions are deterministic.

Why `complete_ticket` and not `commit_ticket` as the fixture's primary verb: "seal" in the story is
completion, `complete_ticket` is the path the field run failed on, and T-055-01-02's convergent
no-op is defined in terms of a completion key. `commit_ticket` remains reachable directly for any
consumer that wants it.

---

## 3. `crates/lisa-cli/tests/guard_waits_its_turn.rs` (created)

```rust
mod support;

#[test]
fn four_concurrent_seals_all_land_exactly_one_commit_each() { … }
```

Asserts, in order:

1. All four `Result`s are `Ok`.
2. No result's error text (there are none, but the check is the AC) matches
   `GUARD_COLLISION_SIGNATURES` — via `assert_no_guard_collision`.
3. `head_commit_count()` == base + 4.
4. The four commit subjects at HEAD are exactly the four `Complete {id}` messages, each once —
   "exactly one commit" per transaction.
5. Every ticket file at HEAD contains `phase: done` and `status: done`.
6. Every ticket's work artifact is present at HEAD.
7. `assert_no_commit_lock()` — the marker is gone, i.e. the guard queue drained cleanly.

Point 4 is what makes the fixture reusable by T-055-01-02: it distinguishes "four commits" from
"four results", which is exactly the axis a convergent no-op moves along.

---

## Ordering

1. `commit_transaction.rs` production change + unit test — self-contained, provable alone.
2. `tests/support/mod.rs` + `tests/guard_waits_its_turn.rs` — the fixture needs the fix to pass, so
   it lands second and is the fix's end-to-end evidence.

Each is a separate `lisa commit-ticket` unit with exact `--include` paths.
