# Design — T-052-01-01 stamp-the-feed

Two decisions are needed: **where the emit timestamp lives** and **how the age
string is formatted**. They are independent; each is decided below against the
research.

---

## Decision 1 — Where the emit timestamp lives

### Option A — Add a time field to `lisa_core::types::ActivityEvent`

Grow every variant (or add a wrapper variant) with `at_epoch_secs: u64`.

Rejected. Research §4 found the decisive counter-evidence: `lisa-core` itself
constructs `ActivityEvent`s in `diagnostics::startup_diagnostics`, a documented
**pure function with no side effects** and no clock. Adding a time field forces
one of three bad outcomes — thread a clock into `lisa-core` (breaks the purity
that makes diagnostics testable), stamp a placeholder at construction and
overwrite later (reintroduces the exact "meaningless default" class of bug this
ticket fixes), or make the field `Option` (every consumer re-acquires the
fallback problem). It also changes the `Serialize`/`Deserialize` wire shape and
breaks `PartialEq` structural matching across ~10 diagnostics tests and ~40
lib.rs assertions. The ticket gates this option on research finding a consumer
that needs time *inside* the enum; research found none.

### Option B — Tuple in the `activity_log` Vec: `Vec<(Duration, ActivityEvent)>`

Viable. Confines time to the plugin, leaves `lisa-core` untouched.

Rejected on ergonomics. Research §5 counted ~40 test sites and 3 production
readers destructuring `activity_log` elements. A tuple makes every one of them
read `.1` or `|(_, e)|` with no name explaining which half is which, and the two
`Duration`s in scope at the render site (emit time, current time) are exactly
the pair that must not be confused — the same confusion that produced the
original bug. A tuple is the cheapest thing to write and the worst thing to read
at the site that already went wrong once.

### Option C — Wrapper struct local to lib.rs *(chosen)*

```rust
/// An activity event plus the wall-clock instant it was emitted.
struct LoggedActivity {
    /// Seconds since the Unix epoch at the moment `log_activity` ran.
    at: Duration,
    event: ActivityEvent,
}
```

`activity_log: Vec<LoggedActivity>`. Chosen.

Rationale grounded in research:

- **`lisa-core` stays clock-free** (§4). `diagnostics` keeps returning bare
  `Vec<ActivityEvent>`; lib.rs:8559 stamps them as it replays them, which is the
  correct place — that loop is running inside the plugin, which has a clock.
- **Named fields at the confusable site** (§3). `entry.at` vs `state.current_time`
  reads unambiguously where `.0` vs a local would not.
- **Blast radius is bounded and mechanical** (§5). `is_empty()`/`len()` sites are
  unaffected. The `.iter().any(|e| matches!(e, ActivityEvent::…))` sites need one
  accessor:
  ```rust
  fn activity_events(&self) -> impl Iterator<Item = &ActivityEvent>
  ```
  Each site becomes `.activity_events().any(…)` — a rename, not a rewrite, and
  `format_snapshot` (lib.rs:8099) uses the same accessor unchanged in behavior.
- **Room to grow.** If a later ticket wants a sequence number or correlation on
  the log envelope, a struct absorbs it; a tuple grows into `.2`.

### Decision 1b — The clock seam

`log_activity` keeps its signature and delegates to a stamped sibling, following
the `_at` convention already in force at lib.rs:2265→2284 and 4611→4450 (§6):

```rust
fn log_activity(&mut self, event: ActivityEvent) {
    self.log_activity_at(event, std::time::SystemTime::now());
}

fn log_activity_at(&mut self, event: ActivityEvent, now: std::time::SystemTime) { … }
```

Chosen over the `deadline.rs` `Clock` trait (§6, option 1) because `Clock` is
designed for a *snapshotting evaluator* constructed once per pass;
`log_activity` is a `&mut self` method called 60+ times per pass at arbitrary
points. The `_at` suffix is the codebase's established shape for exactly that,
and it keeps all 60+ existing call sites untouched.

Conversion uses `provenance::system_time_to_epoch` (§7) rather than open-coding,
so the saturating pre-epoch behavior is the one already reviewed and tested.
This satisfies criterion 4: no new time source, same `SystemTime::now()`/
`UNIX_EPOCH` pattern.

### Decision 1c — Threading it to the UI

`activity_event_to_ui_entry(event: &ActivityEvent)` becomes
`activity_event_to_ui_entry(entry: &LoggedActivity)`, reading `entry.at` where
`Duration::ZERO` is hardcoded today (lib.rs:9068). ~15 direct test call sites
(§5) get a `LoggedActivity` wrapper around their event.

Considered and rejected: keeping the one-arg signature and adding a second
`_with_time` wrapper. It leaves the `Duration::ZERO` line alive in the codebase
as a trap for the next caller. The whole point of this ticket is that that
constant should not exist.

---

## Decision 2 — The age formatter

### Option A — Change `format_duration` / `format_time_since` in place

Rejected outright. Research §3: those two functions serve three non-feed callers
(waiting-item park age, active-thread elapsed, parked-thread elapsed) that
render correct composites today, and `format_duration` has a pinned test
(ui.rs:1751). The acceptance criteria scope buckets to the two feed renderers.
Changing the shared helper would silently reshape three unrelated UI surfaces.

### Option B — New feed-private formatter *(chosen)*

```rust
/// Format an activity entry's age in coarse human buckets.
fn format_age_bucket(timestamp: Duration, current_time: Duration) -> String
```

Called at ui.rs:1009 and ui.rs:1122, replacing `format_time_since` at those two
sites only. `format_time_since` and `format_duration` are left byte-identical,
and their three other callers are provably untouched.

Buckets, per the ticket:

| Condition | Output |
|---|---|
| `timestamp` is epoch-zero (sentinel) | `—` |
| elapsed < 60s | `just now` |
| elapsed < 3600s | `{n}m ago` |
| elapsed < 86400s | `{n}h ago` |
| otherwise | `{n}d ago` |

### Decision 2b — The epoch-zero fallback

Criterion 3 requires a bounded fallback for the exact shape that produced
495696h. Two ways to detect it:

1. **Sentinel on the input** — `timestamp == Duration::ZERO → "—"`.
2. **Cap on the output** — clamp elapsed above some threshold to `—`.

Chosen: **(1), with (2) as a secondary guard is explicitly not added.**

Sentinel-on-input is chosen because it is exact and total. Research §7 shows
epoch-zero is precisely what `system_time_to_epoch`'s `unwrap_or(0)` produces on
clock failure, and §10 notes it is reachable in production, not only in
fixtures — so it is a real sentinel value with a real meaning ("we never learned
when this happened"), not a magic number. A high-water cap would need an
arbitrary threshold, and a genuinely old entry (a long-running loop) would be
misreported as unknown. The `{n}d ago` bucket is already unbounded-but-honest
for real ages; there is nothing to cap.

The consequence worth stating: an event emitted in the first second of 1970 is
indistinguishable from an unknown timestamp. This is acceptable — the plugin
cannot run before the epoch, and the diagnostic value of `—` far exceeds the
cost of that impossible collision.

### Decision 2c — Column width

The render sites format with `{:<12}` (ui.rs:1082). Longest bucket string is
`just now` (8 chars); `{n}d ago` exceeds 12 only past 99999 days. The existing
width holds with no layout change. Not widening it, because the three non-feed
renderers share the surrounding table conventions.

---

## What this does *not* do

- Does not sort the feed by timestamp. Research §10: ordering is positional
  (`.iter().rev()`), and stamping does not change that. Out of scope.
- Does not persist timestamps across plugin restarts. The log is in-memory; a
  restart starts an empty feed.
- Does not touch `format_snapshot`'s rendering (lib.rs:8099). It renders event
  text, not ages; it moves to the new accessor with no output change.
- Does not add sub-second precision. Research §10: `current_time` is truncated to
  whole seconds at lib.rs:8973, so the emit stamp uses whole seconds too. Bucket
  boundaries are coarse enough that sub-second skew is invisible.

## Risk register

| Risk | Mitigation |
|---|---|
| Mechanical accessor rename across ~40 test sites introduces a typo | Compiler catches all of them; `just check` is the gate |
| A missed `Duration::ZERO` path leaves a line rendering `—` in production | Criterion-3 test pins the fallback; §4 confirms `log_activity` is the single funnel, so there is one place to stamp |
| Bucket edges off by one from second-truncation | Unit tests pin 59s/60s/61s and 3599s/3600s boundaries explicitly |
