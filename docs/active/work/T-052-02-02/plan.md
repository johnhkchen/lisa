# Plan — T-052-02-02 fold-the-echoes

Three commits, each green on its own, each carrying the tests for what it added.
Every commit goes through `lisa commit-ticket --ticket-id T-052-02-02` with exact
`--include` paths. Ticket-owned files: `crates/lisa-plugin/src/lib.rs`,
`crates/lisa-plugin/src/ui.rs`. Nothing else in the workspace is touched.

## Step 1 — Fold at the append seam

**Edit** `crates/lisa-plugin/src/lib.rs`:

- `LoggedActivity` (~751): add `count: u32` between `at` and `event`, with the
  doc lines from Structure A1. Extend `at`'s doc to say a fold overwrites it.
- `log_activity_at` (~3419): hoist `at`, add the `last_mut()` equality check that
  bumps `count` and refreshes `at` and returns; the push arm sets `count: 1`. Cap
  check unchanged and unreachable from the fold arm.
- `log_activity_at`'s doc comment: add the fold paragraph, including *why* the
  predicate is `==` and not "renders the same" (Design). This is the comment a
  future loosening has to argue with.
- `only_phase_completed_projects_a_transition_line` (~12189): `stamped` closure
  gains `count: 1`. Fix any other `LoggedActivity` literal the compiler names.

**New tests** (beside the feed tests at ~11125, using `feed_test_instant`):

1. `three_identical_events_fold_into_one_counted_line` — three identical
   `Warning`s at offsets 0/30/90 → `len() == 1`, `count == 3`, and
   `at == FEED_TEST_NOW_SECS + 90`. The timestamp assertion is the half of AC1
   that is easy to leave untested; assert the *latest*, and assert it is not the
   first instant.
2. `an_intervening_event_breaks_the_fold` — A, B, A → three entries, each
   `count == 1`, in emission order. (AC1 second half.)
3. `distinct_facts_fill_the_ring_regardless_of_echoes` — 100 distinct events,
   each emitted `2 + (i % 3)` times → `len() == 100`, first distinct fact still
   present, `count`s equal the echo multiplicities. Then one more distinct event
   → `len()` still 100 and the *oldest* is the one gone. (AC2, tested at the cap
   and one past it.)
4. `near_identical_events_never_fold` — table over pairs differing in exactly one
   field: `PhaseCompleted` by `ticket_id`; `PhaseCompleted` by `phase`;
   `Warning` by `message`; `Error` vs `Warning` with the same message. Each pair
   → two entries. (AC4.)

**Verify:** `cargo test -p lisa-plugin` green; the four new tests fail if the
fold arm is deleted.

**Commit:** `lisa commit-ticket --ticket-id T-052-02-02 --message "fix(plugin): fold consecutive identical activity events at append" --include crates/lisa-plugin/src/lib.rs`

## Step 2 — Carry the count into the projection and the feed

Structure records the hard constraint: `ui::ActivityEntry`'s new field and the
projection that builds it must land together or nothing compiles. So both files
move in this step.

**Edit** `crates/lisa-plugin/src/ui.rs`:

- `ActivityEntry` (~301): add `pub count: u32` with its doc line.
- Add `with_repeat_tag(message: String, count: u32) -> String`, `pub(crate)`,
  beside `format_age_bucket`. `count <= 1` returns `message` untouched; otherwise
  `format!("{message} (x{count})")`.
- `render_activity_log` (~1107) and `render_filtered_activity_log` (~1196): one
  rebind each, `let message = with_repeat_tag(message, entry.count);` immediately
  before the existing `output.push(format!(...))`. No `match` arm is touched.
- Seven `ActivityEntry` literals in tests (~1751, ~1758, ~1851, ~2292, ~2730,
  ~2737, ~2744): `count: 1`. Assertions untouched — that they still pass is the
  evidence that an unfolded line renders exactly as before.

**Edit** `crates/lisa-plugin/src/lib.rs`:

- `activity_event_to_ui_entry` tail (~9397): `count: entry.count`.

**New tests:**

5. (`lib.rs`) `folded_line_renders_one_entry_with_the_multiplier` — log the same
   event three times, project, render the full feed, assert exactly one activity
   line and that it ends `(x3)`. This is AC1 through the operator's eye rather
   than the struct's.
6. (`lib.rs`) `projection_preserves_the_count` — a folded entry projects with the
   envelope's `count`; projecting the same log twice yields identical counts
   (the projection reads, never accumulates — AC3/N4 asserted behaviourally).
7. (`ui.rs`) `folded_entry_renders_the_multiplier_in_both_views` — one
   `PhaseCompleted` with `count: 3` drives both renderers (it survives the
   alerts-only filter); both lines end `(x3)`.
8. (`ui.rs`) `single_occurrence_renders_without_a_tag` — same entry, `count: 1`,
   no `(x` in either view.
9. (`ui.rs`) `the_multiplier_survives_message_truncation` — a `Warning` message
   past the 40- and 50-char cuts with `count: 2`; both views contain `...` *and*
   `(x2)`. Guards the ordering mistake of tagging before truncating.

**Verify:** `cargo test -p lisa-plugin` green.

**Commit:** `lisa commit-ticket --ticket-id T-052-02-02 --message "feat(plugin): render folded activity echoes as a trailing multiplier" --include crates/lisa-plugin/src/lib.rs --include crates/lisa-plugin/src/ui.rs`

## Step 3 — The audit dump keeps the multiplier

**Edit** `crates/lisa-plugin/src/lib.rs` (~8302): iterate `self.activity_log`
instead of `self.activity_events()` and wrap each formatted line in
`ui::with_repeat_tag(line, entry.count)`. `format_activity_event` and
`activity_events()` are both left alone.

**New test:**

10. `state_dump_reports_the_fold_multiplier` — three identical events, then
    `format_snapshot()`; assert the event's dump line appears once and carries
    `(x3)`. Without this the fold would erase "it happened three times" from the
    one surface whose job is to still have the answer.

**Verify:** `cargo test -p lisa-plugin` green;
`test_format_snapshot_activity_log_limit` still passes untouched (its 100
messages are distinct, so nothing folds — it witnesses that moving the dump's
iteration seam changed no behaviour).

**Commit:** `lisa commit-ticket --ticket-id T-052-02-02 --message "fix(plugin): carry the fold multiplier into the state dump" --include crates/lisa-plugin/src/lib.rs`

## Final gate

`just check` — fmt, clippy, WASM check, workspace tests. **Judged by exit code,
not by reading output**; a grep over a pipeline has masked a real failure here
before. If clippy objects to the `if let Some(_) = ... { if ... }` nesting,
collapse to a `let`-chain or `matches!` guard rather than silencing it.

Then `git status --short` must show no ticket-owned file staged, modified, or
untracked, and Review writes `review.md` plus `review-disposition.json`, followed
by `lisa check-disposition T-052-02-02`.

## Testing strategy, stated once

All ten tests are unit tests in the existing `lisa-plugin` test modules; nothing
here needs an integration harness, because the whole feature lives between one
append function and two pure renderers. Clock-dependent assertions go through the
existing `feed_test_instant` / `FEED_TEST_NOW_SECS` fixture — **no sleeps**, in
keeping with why the `_at` seam exists at all.

Coverage maps to acceptance criteria:

| AC | Tests |
|---|---|
| 1 — three fold to one `(x3)` with latest stamp; interleave breaks it | 1, 2, 5 |
| 2 — ring holds 100 distinct facts at the cap | 3 |
| 3 — fold in `log_activity`, projection stays a pure map | 1 (placement), 6 |
| 4 — near-identical events never fold | 4 |
| 5 — `just check` green | final gate |

Not covered by design: folds across a gap, time-windowed folds, and rendered-but-
unequal events (two `SessionTimedOut` both reading "60m") — all three are
deliberate non-goals from Design, and Review restates them as known limitations
rather than gaps.

## Risks

- **Field additions ripple.** Both new fields break struct literals. This is
  compile-error-guided, so the risk is churn, not silent breakage. Expect ~8
  literal sites.
- **A test may assert a rendered line that now folds.** Any existing test that
  logs the same event twice and expects two feed lines will fail. Fix by reading
  the test's intent: if it meant "two facts", make the events distinct; if it
  meant "logged twice", assert `count == 2`. Do not weaken the fold to preserve
  a fixture.
- **`log_phase_transition` interaction.** Its `PhaseCompleted` +
  `TicketPhaseChanged` pair alternates, so it never self-folds; the T-052-02-01
  invariant is untouched. Worth re-running that ticket's tests specifically.
