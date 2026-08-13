# Review — T-065-01-01: a drained board goes on ticking

**Disposition: pass.**

The one-line bug the rewritten ticket names is fixed, and the two independent
criteria it kept are done alongside it. A board that finishes now keeps its
five-second timer, keeps stamping `.lisa/scheduler.alive`, and picks up a ticket
filed onto it on the next tick — with the idle tick costing a look at the ticket
directory rather than a parse of every ticket on it.

## What changed

Two commits, three files.

**`5c8f9c8` — `crates/lisa-core/src/ticket.rs`**

- New `ticket::scan_fingerprint(dir) -> Option<u64>`. One `read_dir` plus one
  `metadata` per `.md` file; hashes each entry's name, length, and modification
  time and sums them so `read_dir`'s arbitrary order cannot change the answer.
  No file contents are read, no ticket is parsed.
- `None` means the directory could not be described (missing, or a platform
  with no modification times). Callers must read that as *changed* — failing
  toward doing the work costs a rescan, failing the other way would fence a
  scheduler off from tickets it can plainly see. That is the same fail-open
  stance the 0.4.4 journal fence taught this desk the hard way.
- One test: `a_fingerprint_answers_has_anything_changed_without_reading_a_ticket`.

**`8387fc4` — `crates/lisa-plugin/src/lib.rs`, plus a new test module
`crates/lisa-plugin/src/tests/a_drained_board_goes_on_ticking.rs`**

- `poll_tick()` no longer returns without re-arming when `check_all_done()`
  fires. The `arm_timer(POLL_INTERVAL_SECS)` at the end is now unconditional.
  This is the whole bug (`lib.rs:9578-9582` at the ticket's HEAD).
- A drained tick short-circuits after `stamp_scheduler_alive()` and
  `note_other_schedulers()`: if the ticket-directory fingerprint is unchanged it
  re-arms and returns. The first look that differs clears `terminated` and falls
  through to an ordinary full poll.
- New `State::ticket_dir_changed()`, and three new `State` fields:
  `last_ticket_fingerprint`, `last_scan_errors`, `completion_announced`.
- `rebuild_dag()` now scans through `scan_tickets_with_diagnostics` and records
  the fingerprint **before** the scan, never after.
- New `State::report_scan_errors()` and the free `scan_error_message()`: an
  unparsable ticket file is logged to the dashboard activity feed as
  `Skipped <file> — Lisa can't read it: <error>`, on the edge rather than the
  level, with `Every ticket file reads cleanly again` when the last one is fixed.
  `load()` seeds `last_scan_errors` from the startup scan so the first poll does
  not repeat what start-up just said.
- The `complete` notification and the `AllTicketsDone` feed line now fire once
  per *drain* rather than once per *tick*, guarded by `completion_announced`,
  which resets whenever the board is working again.
- Finished-screen copy: `All tickets done. Lisa is still watching for new
  tickets. Press [q] to quit.` The old line promised the loop was over, and now
  it is not. The stale comment above it (“the poll timer is not re-armed once a
  loop drains”) is corrected.

Nothing else moved. `keep_working()` is untouched.

## The acceptance criteria, one at a time

**“A board that has completed keeps ticking, or stops in a way something can
wake. Say which, and why the idle cost is acceptable if it keeps ticking.”**

It keeps ticking, at the same five seconds as a live board. Deliberately the
same, not slower: `SchedulerAlive` writes `poll_interval_secs` into the stamp,
and every reader (`seats.rs:285`, `loop_cmd.rs:83`,
`schedulers::live_window_secs`) sizes its liveness window at six of them. A
slower idle cadence would silently widen the window in which a *dead* scheduler
still reads as live — which is the failure that put two schedulers on this board.
One cadence keeps that window at its present 30 seconds and keeps pickup latency
at five seconds, and it removes a whole class of cadence/window bugs.

The idle cost is one `read_dir` plus one `stat` per `.md` file, and a ~140-byte
stamp write, every five seconds. On this board that is 207 stats and no file
reads, against the 207 opens, 896 KB of reads, 207 frontmatter parses and one DAG
build that a naive re-arm would have cost at the same cadence.

**“`scheduler.alive` keeps being stamped for as long as the plugin is
resident.”**

`stamp_scheduler_alive()` leads the tick and the short-circuit is placed after
it, so a drained scheduler stamps on exactly the same schedule as a working one.
The test deletes the stamp file, ticks an idle board, and asserts it is back.
This is the stale-stamp half of `S-063-01`; `T-065-01-03` fixed the refusal's
evidence, this fixes the reason the evidence went stale.

**“A ticket filed onto a drained board is picked up without a keystroke.”**

`a_ticket_filed_onto_a_drained_board_is_picked_up_without_a_keystroke`: drain a
real board of real ticket files in a temp directory, tick it (assert
`terminated`, timer armed, stamp written), tick it again with nothing changed
(assert no rebuild, stamp rewritten, drain not re-announced), then write
`T-FILED-LATER.md` into the directory and tick once more. The ticket is in the
DAG, a thread exists for it, and `agent_slots[0].ticket_id` is it. No keystroke,
no modal, no restart.

That is the reproduction at the highest level the workspace can test: `State` +
`poll_tick()` against real files on disk, which is what the scheduler is. It
does not drive a live zellij session, and I did not run one — starting a real
`lisa loop` from inside a ticket would put a second scheduler on this board,
which is the incident this story exists to prevent.

**“Rescan cost is bounded. Say what triggers a rebuild and why that is enough.”**

A full rebuild is triggered by:

1. **any tick while the board is live** — unchanged from today, and it has to
   stay: the DAG is the scheduler's working set, phase advancement is file-driven,
   and `mask_completion_transaction()` folds in-memory completion state into the
   scanned tickets, so a DAG that skipped a tick could disagree with an in-flight
   completion rather than merely be late.
2. **a ticket-directory fingerprint that differs, while the board is drained** —
   a new file, a removed file, or any edit to an existing one.

That is enough because those are the only two states there are, and (2) covers
every way the board's contents can change: a filed ticket, a deleted one, a
`lisa reset-ticket` rewrite, a hand edit. The fingerprint is taken *before* the
scan that builds the DAG, so a ticket that lands while a scan is running shows
as a difference on the next tick instead of being recorded as already seen.
A directory that cannot be described reads as changed, so no error can turn into
a permanently deaf scheduler.

**“An unparsable ticket is reported where a person can see it.”**

The dashboard activity feed — the same place `load()`'s startup diagnostics
report the same fact, so a file that breaks at ten in the morning now reads the
same as one that was already broken at start-up. Reported once when it breaks
and once when it is fixed, because a rescan runs every five seconds and a feed
that repeats one broken file forever is a feed nobody reads. Test:
`a_ticket_file_that_will_not_parse_is_reported_once_and_its_repair_too`.

**“The quit-modal `keep_working()` path keeps working and stops being the only
way a drained board resumes.”**

`keep_working()` is unchanged, and `the_quit_modal_still_resumes_a_drained_board`
pins it: drain, file a ticket, press Enter, get a thread. Its `if
pending_timer_count == 0` guard now simply never fires — the timer is already
armed — so it no longer double-arms, which is what that guard was for. It is no
longer the only way back.

## Tests

`just check` — `check-wasm`, `fmt-check`, `lint`, `cargo test --workspace` —
**exit 0**. 662 plugin tests, 561 core tests, all green. Four new tests:

| Test | What it pins |
| --- | --- |
| `ticket::a_fingerprint_answers_has_anything_changed_without_reading_a_ticket` | add / remove / same-length edit / non-ticket file / missing directory |
| `a_ticket_filed_onto_a_drained_board_is_picked_up_without_a_keystroke` | the incident itself, end to end |
| `the_quit_modal_still_resumes_a_drained_board` | no regression on the old path |
| `a_ticket_file_that_will_not_parse_is_reported_once_and_its_repair_too` | edge-triggered reporting, and the board around it unaffected |

## Concerns

1. **The fingerprint's same-length-edit case leans on modification-time
   resolution.** Adding or removing a ticket changes the entry set and is
   detected regardless; an in-place edit that leaves the byte count identical is
   detected only because the mtime moved. APFS, ext4 and tmpfs all keep
   nanoseconds, so the test is stable here, but on a filesystem with one-second
   mtime granularity a same-length edit inside the same second would be missed
   until the next change. It fails toward a late rebuild on a drained board, not
   a wrong one, and it cannot affect a live board, which rebuilds every tick.

2. **`entry.metadata()` and `modified()` inside zellij's WASI sandbox are
   reasoned, not measured.** If either is unavailable there, `scan_fingerprint`
   returns `None`, which reads as *changed*, and a drained board degrades to a
   full rescan every five seconds — correct behaviour, unbounded cost. The
   failure mode is the one worth having; I have not observed it either way, and
   the first real drained loop on this build will settle it. Both calls sit on
   `fd_filestat_get`, which `path.is_dir()` in the existing `scan_tickets` already
   depends on.

3. **An idle tick is deliberately narrow, and two sweeps do not run on a drained
   board.** `retire_resting_sessions()` and `sweep_usage_captures()` are in the
   full tick only, so a session still resting when the last ticket completes is
   not retired and a usage capture that lands afterwards is not swept until
   something wakes the board. That is exactly today's behaviour — today *nothing*
   runs after a drain — so it is not a regression, but it is now a choice rather
   than an accident, and folding those two into the idle path is a reasonable
   follow-up.

4. **A seat still mid-transition when the board drains stays frozen until the
   wake.** `check_all_done()` only asks whether any thread is `Running`, so a pane
   still exiting can be left with its transition clock stopped. On wake, the full
   tick's `check_transition_timeouts()` force-advances it — the deadline is long
   past by then — so it self-heals in one tick. Same property `keep_working()`
   has always had.

5. **The wake is not tested against a mid-tick write.** The ordering that makes
   it safe — fingerprint before scan — is stated in the code and reasoned above,
   but I could not inject a write between `rebuild_dag()`'s scan and the end of
   `poll_tick()` without a test hook I did not want to add to production code.

6. **Two pre-existing clippy lints outside this ticket's files.** With
   `--all-targets` (which `just lint` does not use), `lisa-core/src/completion_journal.rs:1339`
   trips `unused_mut` and `lisa-plugin/src/tests/operator_recovery_matrix.rs:503`
   trips `needless_borrows_for_generic_args` under the installed clippy. Both are
   in test code, both are at HEAD, and neither is touched here. The enforced gate
   passes.
