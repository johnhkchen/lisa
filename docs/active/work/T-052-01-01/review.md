# Review — T-052-01-01 stamp-the-feed

The activity feed rendered every line's age as `495696h 11m` — seconds since
1970 dressed up as a duration. Two things were wrong and both are fixed: entries
never recorded *when* they happened, and the feed formatted ages as a
`{h}h {m}m` composite instead of words a person uses.

## Commits

| Commit | Message | Files |
|---|---|---|
| `ce2a527` | `fix(ui): render feed ages in human buckets` | `crates/lisa-plugin/src/ui.rs` (+135 −2) |
| `570ce29` | `fix(plugin): stamp activity events with wall clock at emit` | `crates/lisa-plugin/src/lib.rs` (+229 −134 across 4 files) |

`ccb497e` sits between them in the log. It is **not mine** — a sibling ticket
fixing an unrelated `lisa-cli` download fixture. See "Concurrency" below.

## What changed

### `crates/lisa-plugin/src/ui.rs` — the format

- **Added** `format_age_bucket(timestamp, current_time)` and
  `const UNKNOWN_AGE: &str = "—"`. Four buckets: `just now` (< 60s), `Nm ago`
  (< 1h), `Nh ago` (< 1d), `Nd ago`. Epoch-zero input short-circuits to `—`.
- **Modified** two lines: the age call in `render_activity_log` (ui.rs:1009) and
  in `render_filtered_activity_log` (ui.rs:1122).
- **Deliberately untouched:** `format_time_since` and `format_duration`. They
  serve three *non-feed* callers — waiting-item park age, active-thread elapsed,
  parked-thread elapsed — which render correct composites and are out of scope.
  Verified post-change that `format_time_since` still has exactly those three
  callers and that neither feed renderer can reach either composite formatter.

### `crates/lisa-plugin/src/lib.rs` — the data

- **Added** `struct LoggedActivity { at: Duration, event: ActivityEvent }`.
  `activity_log` is now `Vec<LoggedActivity>`.
- **Modified** `log_activity` to delegate to new
  `log_activity_at(event, now: SystemTime)`, which stamps via
  `provenance::system_time_to_epoch` and owns the `MAX_ACTIVITY_LOG` trim. All
  60+ existing `log_activity` call sites are untouched.
- **Added** `activity_events()` returning
  `impl DoubleEndedIterator<Item = &ActivityEvent>` for readers that want the
  event without the envelope.
- **Deleted** the `let timestamp = Duration::ZERO;` line in
  `activity_event_to_ui_entry`. It now takes `&LoggedActivity` and passes
  `entry.at` through to the UI entry — into a `timestamp` field that had existed
  and waited unpopulated all along.

### Why `lisa-core` was not touched

The ticket gated growing `ActivityEvent` with a time field on research finding a
consumer that needs time *inside* the enum. **None was found, and there is
positive evidence against it:** `lisa-core::diagnostics::startup_diagnostics` is
a documented pure function that builds `Vec<ActivityEvent>` with no clock and no
business acquiring one. `ActivityEvent` is also `Serialize`/`Deserialize`/`Eq`,
so a new field would change the wire shape and break structural matching across
~10 diagnostics tests. Time lives in a plugin-local envelope instead; the
plugin stamps diagnostics events as it replays them, which is where the clock
actually is.

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | Records wall clock at emit; injected clock → `just now`, then `1m ago` at 60s | `log_activity_at_stamps_the_emit_instant`, `stamped_entry_renders_just_now_then_one_minute_ago` (lib.rs). Clock advances by parameter — **no sleeps** |
| 2 | Only the four bucket shapes, both renderers; composite gone | `activity_feed_renders_only_bucket_shapes` drives both `render_activity_log` and `render_filtered_activity_log` |
| 3 | Epoch-zero entry renders a bounded fallback, never an hours figure | `format_age_bucket_renders_epoch_zero_as_bounded_fallback` — asserts `== "—"`, `!contains('h')`, and width ≤ 12 |
| 4 | No new time source; WASM builds clean | Stamp uses `provenance::system_time_to_epoch(SystemTime::now())`, the existing helper over `UNIX_EPOCH`. `cargo check -p lisa-plugin --target wasm32-wasip1` → exit 0 |
| 5 | `just check` green | exit code **0**, re-confirmed after the final commit |

Gates were judged by **exit code**, not by reading output — `just check` at one
point printed a wall of passing tests above two failures.

## Test coverage

Nine new tests. `cargo test -p lisa-plugin` → 455 passed, 0 failed.

- Bucket boundaries pinned exactly at 0/59/60/61/3599/3600/86399/86400s — the
  off-by-one risk from second-truncation is covered on both sides of every edge.
- Backwards clock skew (future-dated entry) pinned to `just now` rather than a
  wrong large number.
- `log_activity_uses_wall_clock_not_zero` guards the regression at the funnel:
  an entry emitted through the ordinary path must carry a nonzero stamp. This is
  the test that would fail if someone reintroduced a default stamp.

### Gaps, stated plainly

- **No test asserts the rendered feed at two different `current_time` values
  through the renderer.** The tick-forward progression is proven at the
  formatter level (criterion 1's test) and the renderer is proven to use that
  formatter, but the two are not chained in one test. The five-second timer path
  that recomputes `current_time` is untested here — it was untested before and
  this ticket adds no plumbing to it.
- **Criterion 2's negative is proven positionally, not exhaustively.** The test
  asserts each entry's age column matches a padded bucket string exactly, plus a
  named `!contains("495696h")` breadcrumb. It does not scan for arbitrary
  `NNNNh MMm` text anywhere in a line, because activity *messages* can
  legitimately contain such text. The column-position assertion is the real
  guarantee.
- **No test covers `MAX_ACTIVITY_LOG` trimming with stamps.** The trim moved
  into `log_activity_at`; it is exercised indirectly by existing tests but not
  pinned against timestamps.

## Open concerns

1. **`—` collides with a genuine 1970 timestamp.** An event emitted in the first
   second of the epoch is indistinguishable from an unknown one. Accepted
   deliberately: the plugin cannot run before 1970, and epoch-zero is a real
   sentinel — `system_time_to_epoch` returns `0` on clock failure, which is
   exactly the shape that produced the field bug. Worth knowing the tradeoff was
   made on purpose, not overlooked.

2. **A mechanical rename pass over-reached into production code.** While
   updating ~95 test call sites, a regex edit rewrote `activity_events` into a
   self-recursive call and changed the production conversion at lib.rs:8850 to
   iterate bare events instead of envelopes. The compiler caught both instantly
   (E0609, E0631) and both were corrected before any commit. Flagged for a human
   reviewer because "bulk edit silently touches a production path while chasing
   test sites" is the kind of thing worth a second pair of eyes on the diff —
   specifically lib.rs:3364 and lib.rs:8849-8852.

3. **Test blast radius exceeded research's estimate.** Research counted ~40 sites
   by grepping lib.rs; the compiler found ~95, including three test-module files
   research never opened. All were the same mechanical rename, so the approach
   held — but the research artifact undercounted, and a reviewer comparing
   research to diff will notice the gap.

## Concurrency

Midway through, `just check` went red with two `lisa-cli` `runtime::tests`
failures. These were a **sibling Lisa thread mid-edit** in
`crates/lisa-cli/src/runtime.rs`, at one point leaving it non-compiling. I
verified this rather than assuming it: a detached worktree at `HEAD` ran those
tests green, and the sibling subsequently landed `ccb497e` fixing the fixture.
No `lisa-cli` file appears in this ticket's commits, and no sibling-owned file
was modified or staged.

`git status --porcelain -- crates/lisa-plugin` → 0 lines. Nothing ticket-owned
is left staged, modified, or untracked.
