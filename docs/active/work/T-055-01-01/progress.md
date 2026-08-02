# Progress — T-055-01-01 guard-waits-its-turn

## Step 1+2 — wait loop and `acquire` split — DONE

`crates/lisa-cli/src/commit_transaction.rs`:

- `Duration` / `Instant` added to the `std::time` import.
- `COMMIT_GUARD_WAIT_TIMEOUT` (30 s), `COMMIT_GUARD_POLL_MIN` (5 ms), `COMMIT_GUARD_POLL_MAX`
  (50 ms) added beside `COMMIT_GUARD_FILE`.
- `duration_ms`, `guard_wait_timeout_error`, `lock_guard_waiting` added above `TransactionLock`.
- `TransactionLock::acquire` is now a one-line delegate; the body moved to `acquire_waiting`
  with a `guard_wait: Duration` parameter. The only change inside the moved body is the six
  fail-fast lines at the old ~402 becoming
  `lock_guard_waiting(&guard_file, &guard_path, guard_wait)?;`.

Marker file, owner record, stale-recovery branches, `finish()` and `Drop`: untouched.

Verified: all 16 pre-existing `commit_transaction` unit tests pass unchanged, including
`held_lock_returns_actionable_error` (marker path) and
`stale_commit_lock_names_age_and_absent_holder_then_recovers`.

## Step 3 — timeout unit test — DONE

`guard_wait_times_out_with_a_named_error_and_does_not_hang`, placed before the stale-lock test.
Holds `.git/lisa-commit.guard` with a second in-process `File` for the whole test, then calls
`acquire_waiting` with a 250 ms bound.

**Deviation from plan:** `unwrap_err()` needs `Debug` on the `Ok` type, and `TransactionLock`
does not derive it (a `File`-holding RAII guard has no reason to). Deriving `Debug` on production
code to satisfy a test would be the tail wagging the dog, so the test uses an explicit
`match { Ok(_) => panic!(…), Err(error) => … }`. Same assertion, no production change.

Test asserts: named phrase `guard wait timed out`; the elapsed `250ms` appears; no `os error 35`;
no `temporarily` anywhere (so the loop's classifier cannot read it as transient contention);
`elapsed >= 250ms` (it looped rather than failing on the first attempt); `elapsed < 10s` (no hang);
no marker left behind. Then unlocks the holder and proves the fast path still acquires and
finishes cleanly.

Runtime: 0.30 s.

**Commit 1:** `crates/lisa-cli/src/commit_transaction.rs`.

## Step 4 — shared four-way concurrency fixture — DONE

`crates/lisa-cli/tests/support/mod.rs` (new) and `crates/lisa-cli/tests/guard_waits_its_turn.rs`
(new), per structure.md §2–§3.

**Deviations from plan:**

1. `dispatch_together`'s bound is `F: Fn(usize) -> T + Sync + Send` (plan listed only `Sync`) —
   `std::thread::scope` requires the closure reference to be `Send` as well.
2. `SealFixture` ships a slightly wider surface than structure.md listed: `ticket_ids`,
   `ticket_path`, `work_dir`, `write`, `complete_message` and `completion_key` are public too.
   All six are things a downstream consumer needs in order to vary the fixture (a second
   generation, a hand-written diff, an id-derived assertion) without reaching into the temp dir
   by hand. `#![allow(dead_code)]` covers the ones this binary does not call.

Fixture surface as shipped:

- `SealFixture::new(&[ids])` — temp repo, identity, one `docs/active/tickets/{id}.md` (status
  `open`, phase `review`) and one `docs/active/work/{id}/research.md` per id, base commit, then a
  `review.md` written per ticket so every seal has a real diff.
- `root`, `ticket_ids`, `ticket_path`, `work_dir`, `write`, `git`, `git_string`,
  `complete_message`, `completion_key`, `complete_request(id, generation)`, `head_commit_count`,
  `commit_subjects`, `show_at_head`, `assert_no_commit_lock`.
- `dispatch_together(count, body)` — `Barrier` + `std::thread::scope`, results indexed by
  dispatch order, panics re-raised via `resume_unwind`.
- `GUARD_COLLISION_SIGNATURES` and `assert_no_guard_collision`.

`four_concurrent_seals_all_land_exactly_one_commit_each` asserts all four `Result`s are `Ok`, no
collision signature on any path, HEAD advanced by exactly 4, the four subjects each appear exactly
once, every ticket file at HEAD carries `phase: done` and `status: done`, every work artifact is at
HEAD, and no marker remains.

Verified green five consecutive runs: 0.50 / 0.47 / 0.43 / 0.43 / 0.49 s.

### Negative control — run twice, and the second run is the one that counts

**(a) `acquire` delegating with `Duration::ZERO`.** Fixture goes red, but on the `is_ok()`
assertion, not on the collision check — the message is the *new* timeout error
(`guard wait timed out … after waiting 0ms`). Worth recording: once the wait exists, `os error 35`
can no longer reach a caller by any route, because the raw `EWOULDBLOCK` is consumed inside
`lock_guard_waiting`. That is exactly what AC 2 asks for, and it also means
`assert_no_guard_collision` is a regression tripwire for a *reintroduction* of the old code, not a
live discriminator.

**(b) The literal pre-fix block restored** (`guard_file.try_lock_exclusive().map_err(…)`). Fixture
goes red on the collision check as intended:

```
transaction 0 died on the guard (os error 35): commit transaction is temporarily locked by a
live holder (guard …/.git/lisa-commit.guard): Resource temporarily unavailable (os error 35)
```

Both edits reverted; `git diff crates/lisa-cli/src/commit_transaction.rs` is empty against
commit 1, and the fixture is green again.

**Commit 2:** `crates/lisa-cli/tests/support/mod.rs`,
`crates/lisa-cli/tests/guard_waits_its_turn.rs`.

## Gate

`just check` — **exit code 0**, captured directly (`just check >log 2>&1; echo $?`), not read off
grepped output. Covers `cargo check -p lisa-plugin --target wasm32-wasip1`, `cargo fmt --all
--check`, `cargo clippy -D warnings` on all three crates, and `cargo test --workspace`
(583 plugin tests, 17 `lisa-cli` lib tests, all integration binaries).

## Working tree

`git status --short crates/lisa-cli/` is empty. Both commits landed through `lisa commit-ticket`
with exact `--include` paths; nothing ticket-owned is staged, modified, or untracked.
