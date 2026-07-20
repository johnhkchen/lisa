# Structure — T-052-02-02 fold-the-echoes

Two files change. No files are created or deleted. No public API outside the
plugin crate moves; `lisa-core`'s `ActivityEvent` is read but not modified.

```
crates/lisa-plugin/src/lib.rs   envelope + fold + projection + dump + tests
crates/lisa-plugin/src/ui.rs    entry field + tag helper + two renderers + tests
```

## A. `crates/lisa-plugin/src/lib.rs`

### A1. `LoggedActivity` gains `count` (~line 751)

```rust
struct LoggedActivity {
    /// Wall clock at emit, as a duration since the Unix epoch.
    ///
    /// On a fold this is overwritten with the latest occurrence's instant, so
    /// the line wears the age of the most recent echo, not the first.
    at: std::time::Duration,
    /// How many consecutive identical events this entry has absorbed. Always
    /// >= 1; only `log_activity_at` writes it.
    count: u32,
    event: ActivityEvent,
}
```

Field order `at, count, event` keeps the envelope fields together ahead of the
payload.

### A2. `log_activity_at` folds (~line 3419)

Current body pushes unconditionally. New shape:

```rust
fn log_activity_at(&mut self, event: ActivityEvent, now: std::time::SystemTime) {
    let at = std::time::Duration::from_secs(provenance::system_time_to_epoch(now));

    // Fold: an event identical to the newest entry is an echo, not news.
    if let Some(newest) = self.activity_log.last_mut() {
        if newest.event == event {
            newest.count = newest.count.saturating_add(1);
            newest.at = at;
            return;
        }
    }

    self.activity_log.push(LoggedActivity { at, count: 1, event });
    if self.activity_log.len() > Self::MAX_ACTIVITY_LOG {
        self.activity_log.remove(0);
    }
}
```

Boundaries this preserves:

- The early `return` is the whole of AC2: a fold never reaches the length check,
  so it cannot evict, and one folded entry occupies one slot regardless of
  `count`.
- `newest.at = at` is AC1's "carrying the latest timestamp".
- Equality is `ActivityEvent`'s existing derived `PartialEq`; nothing new derives.
- `log_activity` (~3411) is unchanged — it still just reads the clock and
  delegates, so every existing caller folds for free.

The doc comment above `log_activity_at` currently explains the `_at` convention.
It gains a paragraph naming the fold and, per Design, why the predicate is
structural equality rather than rendered identity: equal events are
indistinguishable in the feed, in the dump, and in `activity_events()`, so the
fold cannot erase a fact — a future reader tempted to loosen it to "renders the
same" needs that reasoning in front of them.

### A3. Projection copies the count (~line 9397)

The tail of `activity_event_to_ui_entry`, the only production construction site
of `ui::ActivityEntry`:

```rust
Some(ui::ActivityEntry {
    timestamp: entry.at,
    count: entry.count,
    activity,
})
```

One field copy. The function stays a total per-entry map with no access to
neighbours or to `State` — AC3/N4 verbatim. The call site at ~9031
(`.iter().filter_map(activity_event_to_ui_entry)`) does not change at all.

### A4. The Shift+D dump carries the multiplier (~line 8302)

Today:

```rust
let log_entries: Vec<_> = self.activity_events().rev().take(50).collect();
...
writeln!(out, "{:>3}. {}", i + 1, Self::format_activity_event(event))
```

The dump must iterate envelopes instead of bare events so it can see `count`:

```rust
let log_entries: Vec<_> = self.activity_log.iter().rev().take(50).collect();
...
for (i, entry) in log_entries.iter().enumerate() {
    let line = Self::format_activity_event(&entry.event);
    writeln!(out, "{:>3}. {}", i + 1, ui::with_repeat_tag(line, entry.count))
}
```

`format_activity_event` itself is untouched — it stays a pure per-variant
formatter over `&ActivityEvent`, and the multiplier is applied outside it by the
same helper the renderers use, so feed and dump cannot drift in how they spell a
fold.

`activity_events()` (~3457) stays exactly as-is: ~40 tests assert through it over
bare events, and none of them care about counts.

### A5. Test-site fallout inside `lib.rs`

- `only_phase_completed_projects_a_transition_line` (~12189) builds
  `LoggedActivity` through a local `stamped` closure; it gains `count: 1`.
- Any other `LoggedActivity` literal the compiler surfaces gets the same.
- `test_format_snapshot_activity_log_limit` (~15249) pushes 100 *distinct*
  `Info` messages, so it neither folds nor changes behaviour; it stays as a
  regression witness that the dump still shows 50–99 after the seam moved from
  `activity_events()` to `activity_log`.

### A6. New tests in `lib.rs` (module already hosts the feed tests, ~11125+)

Placed beside the existing stamp tests so the fold and the timestamp guarantees
it depends on read together. All use the established `feed_test_instant(offset)`
helper against the fixed `FEED_TEST_NOW_SECS` clock — no sleeping.

1. `three_identical_events_fold_into_one_counted_line` — AC1a. Three identical
   `Warning`s at offsets 0/30/90. Assert `activity_log.len() == 1`, `count == 3`,
   and `at == FEED_TEST_NOW_SECS + 90` (latest, not first).
2. `folded_line_renders_one_entry_with_the_multiplier` — AC1a through the eye
   the ticket cares about: project and render, assert the feed shows exactly one
   line and it ends `(x3)`.
3. `an_intervening_event_breaks_the_fold` — AC1b. Identical A, then B, then A.
   Assert three entries, all `count == 1`, in order.
4. `distinct_facts_fill_the_ring_regardless_of_echoes` — AC2. Emit 100 distinct
   events, each repeated several times; assert `activity_log.len() == 100`, that
   the oldest distinct fact is still present (nothing evicted), and that the
   counts are the echo multiplicities.
5. `near_identical_events_never_fold` — AC4. Table of pairs differing in exactly
   one rendered field: `PhaseCompleted` differing by `ticket_id`;
   `PhaseCompleted` differing by `phase`; `Warning`/`Error` differing by
   `message`. Each pair must produce two entries.
6. `projection_preserves_the_count` — AC3 seam check: a folded entry's
   `ui::ActivityEntry.count` equals the envelope's, and the projection is still
   a plain `filter_map` (asserted behaviourally: projecting the same log twice
   yields the same counts, i.e. no accumulation or mutation).

## B. `crates/lisa-plugin/src/ui.rs`

### B1. `ActivityEntry` gains `count` (~line 301)

```rust
pub struct ActivityEntry {
    pub timestamp: Duration,
    /// Consecutive identical occurrences folded into this entry (>= 1).
    /// Rendered as a trailing `(xN)` when greater than one.
    pub count: u32,
    pub activity: ActivityType,
}
```

Adding a field to a struct built by literal is a compile-error-guided edit; there
is no silent-default path. That is intended — every construction site should
state its multiplicity.

### B2. `with_repeat_tag` helper (module-level, beside `format_age_bucket`)

```rust
/// Hang the fold's multiplier on a rendered line.
///
/// `count == 1` returns the message untouched, so an unfolded line is
/// byte-identical to what it rendered before folding existed.
pub(crate) fn with_repeat_tag(message: String, count: u32) -> String
```

`pub(crate)` because `lib.rs`'s dump (A4) calls it too. Applied *after* the
per-variant truncation in both renderers: the multiplier is the one part of the
line an operator cannot reconstruct, so `...` must never eat it.

### B3. `render_activity_log` (~1022) and `render_filtered_activity_log` (~1119)

Each has exactly one emission point after its `match`. One line changes in each:

```rust
let message = with_repeat_tag(message, entry.count);
output.push(format!(
    "{}{}{} {:<12} {}{}{}",
    color, icon, RESET, time_ago, color, message, RESET
));
```

The `match` arms are not touched — no per-variant edits, no new truncation
divergence between the two renderers. `message` is already an owned `String` out
of every arm, so this is a rebind, not a clone.

### B4. Test-site fallout in `ui.rs`

Seven `ActivityEntry` literals (~1751, ~1758, ~1851, ~2292, ~2730, ~2737, ~2744)
gain `count: 1`. Their assertions are unchanged and must stay passing untouched —
that is the evidence that `count == 1` renders exactly as before.

### B5. New tests in `ui.rs`

7. `folded_entry_renders_the_multiplier_in_both_views` — one `PhaseCompleted`
   entry with `count: 3` (it survives the alerts-only filter, so one fixture
   drives both renderers, matching the `activity_feed_renders_only_bucket_shapes`
   pattern). Assert the line ends `(x3)` in the full feed and in alerts-only.
8. `single_occurrence_renders_without_a_tag` — the same entry with `count: 1`
   renders no `(x` anywhere.
9. `the_multiplier_survives_message_truncation` — a `Warning` whose message
   exceeds the 40/50-char cut with `count: 2`; assert the line contains both
   `...` and `(x2)`.

## Ordering

1. **A1 + A2 + A5** — envelope, fold, `LoggedActivity` literals. Compiles and
   passes on its own; folding is live but invisible (nothing reads `count` yet).
2. **B1 + B2 + B3 + B4** — UI field, helper, both renderers, `ActivityEntry`
   literals. The feed now shows multipliers.
3. **A3** — projection copies the count, joining 1 and 2 end to end.
4. **A4** — dump multiplier.
5. **A6 + B5** — the acceptance tests, last, so they are written against the
   assembled behaviour rather than alongside it.

Steps 1–2 each leave the workspace green, but the feature is only observable
after 3. Plan sequences the commits accordingly.
