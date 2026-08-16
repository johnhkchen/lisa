# T-070-01-07: a pty test fails where there is no terminal

## What changed

`crates/lisa-cli/src/headless.rs`, one commit (`6487a2b`).

The bug was not in the test's assertion, it was in `run_on_pty` itself. The
function spawns a thread to drain the pty into a shared `head: Arc<Mutex<Vec<u8>>>`,
and — deliberately, per the existing comment — never joins that thread, because
a lingering Zellij server can hold the pty open past the client's exit and
joining would hang. But right after `child.wait()` returns, the old code took
a single snapshot of `head` and returned it. Nothing forced the drain thread to
have actually read the child's already-buffered `stty size` output by that
point; it's a second thread, and its scheduling relative to the main thread is
not guaranteed. Under an idle machine the drain thread always won that race.
Under CI load it once didn't, and `transcript_head()` came back `None`.

**Fix: wait for output rather than sampling once**, inside `run_on_pty`, not in
the test. Added `stabilized_head(&head)`, called after `child.wait()` in place
of the old one-shot snapshot. It polls the mutex every 5ms and returns once two
consecutive polls see the same length (the drain thread has caught up — the
child has already exited, so no more bytes are coming), bounded by a 500ms
deadline so the lingering-server case that motivated "don't join" still can't
hang; it returns whatever arrived by the deadline instead. The test itself is
unchanged — it was never the wrong layer, the function it calls was.

I considered fixing this in the test instead (retry-loop over
`transcript_head()`), but `PtyRun.head` is a plain `Vec<u8>` snapshotted once
before the function returns — polling the returned value can't see anything
new arrive after the fact. The wait has to happen before the snapshot is taken,
which means inside `run_on_pty`.

## Acceptance criteria

- **Waits for output rather than sampling once**: yes, via `stabilized_head`,
  described above. Chose "wait for the shared buffer to stabilize" over
  "read the pty directly instead of a subprocess's stdout" because the
  transcript-of-a-subprocess design is what the rest of this module and its
  callers rely on (`loop_cmd.rs` reports `transcript_head()` for startup
  failures); the race was in *when* the buffer was read, not in *what* was
  read from.
- **A genuinely wrong size still fails**: verified directly — see Testing.
  `stabilized_head` only waits for the buffer to settle; it does not touch or
  filter its contents, so a pty actually opened at the wrong size still
  produces that wrong size in `transcript_head()` and the assertion still
  fails.
- **No-pty environment should skip, if one is possible for this suite**: not
  possible for this suite, so no skip was added. `run_on_pty`'s non-unix stub
  already returns a descriptive `Err`, but the tests that call it are
  `#[cfg(unix)]`-gated and don't compile on non-unix at all. On unix,
  `/dev/ptmx` is present on every realistic CI runner (GitHub-hosted and
  typical self-hosted images, including containers); if a future minimal
  sandbox genuinely lacked it, `run_on_pty` would return `Err` and the test's
  `.expect("a pty to open")` would panic with that message rather than
  silently doing the wrong thing — an honest failure, just not a skip.
- **`run_on_pty`'s other callers**: `loop_cmd.rs` also calls
  `run.transcript_head()`, but only to report what a *failed* headless client
  printed before dying (`docs/knowledge/headless-board.md`-style diagnostics),
  never as something asserted on. The same theoretical race existed there
  before this fix and is now fixed identically, since both callers go through
  the same `run_on_pty`. It was never load-bearing for that caller the way it
  was for the test — worst case was a slightly-more-often-empty diagnostic
  message — but it's fixed for free.

## Testing

- `cargo test -p lisa-cli --bin lisa` (targeted): all `headless::tests::*` and
  the two `headless_loop.rs` integration tests pass.
- Confirmed the fix actually holds under load, per the ticket's own
  reproduction instructions, substituting four `yes` loops plus a concurrent
  `cargo build -p lisa-cli --release` for four live `lisa loop` boards (didn't
  want to spin up real agent sessions from inside this session): with that
  load running, `the_terminal_lisa_opens_has_a_plausible_size` passed 10/10
  in a row.
- Confirmed the safety net: temporarily forced `set_window_size` to size the
  pty one column wider than `HEADLESS_COLS`, reran the test, and it failed
  loudly (`left: Some("  50 201"), right: Some("  50 200")`) rather than
  passing. Reverted before committing.
- `cargo fmt -p lisa-cli -- --check` and `cargo clippy -p lisa-cli --bin lisa
  -- -D warnings`: clean on `headless.rs`. (An unrelated pre-existing fmt
  diff exists in `triage_agent.rs`, untouched by this ticket — belongs to
  `T-070-01-06`, which is still open.)

## Concerns

- `T-070-01-06` (the sibling ticket, `triage_agent`'s bounded-runner flake) is
  still `phase: implement`, unresolved. The ticket's Notes suggested fixing
  both together and adding a sentence to `docs/knowledge/` about the shared
  shape ("measures whether something happened fast enough, while claiming to
  measure whether it happened at all"). I left that alone: it's a Notes
  suggestion, not this ticket's Acceptance Criteria, and touching shared docs
  or `triage_agent.rs` risks colliding with whoever finishes T-070-01-06. This
  review documents the shape here in case that ticket's reviewer wants to
  cite it.
- The 500ms stabilization deadline is a judgment call: long enough to absorb
  the CI-load lag that caused the failure, short enough that the
  don't-hang-on-a-lingering-server property (why the drain thread was never
  joined in the first place) still holds. It is a bound, not a proof — a
  pathological scheduler delay longer than 500ms would still race. I didn't
  find a way to prove no-delay-is-too-long is possible for a design that
  refuses to fully join by construction.
