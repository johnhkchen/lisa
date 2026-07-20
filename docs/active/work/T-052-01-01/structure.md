# Structure — T-052-01-01 stamp-the-feed

File-level blueprint for the design. Two source files change; `lisa-core` is
untouched.

## File inventory

| File | Disposition | Why |
|---|---|---|
| `crates/lisa-plugin/src/lib.rs` | **modified** | envelope type, stamped emit, threaded conversion, test-site accessor |
| `crates/lisa-plugin/src/ui.rs` | **modified** | new bucket formatter, two render-site swaps, tests |
| `crates/lisa-core/src/types.rs` | **unchanged** | design decision 1A rejected — enum stays clock-free |
| `crates/lisa-core/src/diagnostics.rs` | **unchanged** | keeps returning bare `Vec<ActivityEvent>` |
| `crates/lisa-plugin/src/deadline.rs` | **unchanged** | `_at` convention chosen over the `Clock` trait |

No files created or deleted.

---

## `crates/lisa-plugin/src/ui.rs`

### Added — `format_age_bucket`

Placed in the Helper Functions block immediately after `format_time_since`
(currently ends ui.rs:448), so the two age formatters sit adjacent and the
doc comments can distinguish them.

```rust
/// Format an activity entry's age in coarse human buckets.
///
/// The activity feed answers "how long ago" in words a person uses, not the
/// `{h}h {m}m` composite `format_time_since` produces for thread elapsed times.
/// An epoch-zero timestamp means the emit instant was never recorded and
/// renders as a bounded `—` rather than a nonsense hours figure.
fn format_age_bucket(timestamp: Duration, current_time: Duration) -> String
```

Internal shape (not code, the decision order):

1. `timestamp == Duration::ZERO` → `UNKNOWN_AGE` (`"—"`)
2. `elapsed = current_time.saturating_sub(timestamp)`, in whole seconds
3. `< 60` → `"just now"`
4. `< 3600` → `"{}m ago"` with `secs / 60`
5. `< 86400` → `"{}h ago"` with `secs / 3600`
6. else → `"{}d ago"` with `secs / 86400`

A private `const UNKNOWN_AGE: &str = "—";` sits beside it so the fallback string
has one definition shared by the function and its test.

### Modified — two render sites, one line each

- ui.rs:1009 (`render_activity_log`) — `format_time_since` → `format_age_bucket`
- ui.rs:1122 (`render_filtered_activity_log`) — same

The surrounding `format!("{}{}{} {:<12} {}{}{}", …)` at ui.rs:1082 is unchanged
(design 2c: width holds).

### Unchanged — deliberately

`format_duration` (ui.rs:429), `format_time_since` (ui.rs:445), and their three
non-feed callers at ui.rs:597, 913, 948. `test_format_duration` (ui.rs:1751)
must still pass byte-identical.

### Added — tests (in the existing `mod tests`)

| Test | Pins |
|---|---|
| `format_age_bucket_covers_the_four_shapes` | just now / Nm / Nh / Nd on representative values |
| `format_age_bucket_boundaries_are_exact` | 0s, 59s, 60s, 61s, 3599s, 3600s, 86399s, 86400s |
| `format_age_bucket_renders_epoch_zero_as_bounded_fallback` | **criterion 3** — `Duration::ZERO` against a 2026-era clock is `—`, and the result contains no `h` |
| `activity_feed_renders_only_bucket_shapes` | **criterion 2** — drives both `render_activity_log` and `render_filtered_activity_log`, asserts output has no `h ` / `m ` composite |
| `format_age_bucket_clamps_future_timestamps` | future-dated entry → `just now`, not a wrong large number (research §10) |

The criterion-2 test is the load-bearing one: it must exercise **both**
renderers, so it needs an entry set that survives the alert filter
(ui.rs:1104 — `PhaseCompleted | Error | Warning | CompletionRejected`).
`PhaseCompleted` satisfies both renderers.

### Modified — existing fixtures

`sample_state()` (ui.rs:1724) and the ui.rs:2596 fixture keep their
`timestamp:` values; they are already nonzero and their tests do not assert on
age text (research §9). No change required — noted here so the blueprint is
explicit that they are surveyed and left alone.

---

## `crates/lisa-plugin/src/lib.rs`

Ordering matters here; the four edits below are listed in the order the compiler
wants them.

### 1. Added — the envelope type

Placed immediately before the `LisaPlugin` struct definition (near lib.rs:792,
where `activity_log` is declared), private to the module:

```rust
/// An activity event paired with the wall-clock instant it was emitted.
///
/// Time lives here rather than inside `ActivityEvent` because `lisa-core`
/// constructs events in clock-free contexts — see `diagnostics::startup_diagnostics`.
struct LoggedActivity {
    /// Wall clock at emit, as a duration since the Unix epoch.
    at: Duration,
    event: ActivityEvent,
}
```

Derives `Debug, Clone` to match how `activity_log` is used today.

### 2. Modified — field type

lib.rs:792: `activity_log: Vec<ActivityEvent>` → `Vec<LoggedActivity>`.

### 3. Modified — the emit funnel

lib.rs:3327. `log_activity` keeps its exact signature (all 60+ call sites stay
untouched) and delegates:

```rust
fn log_activity(&mut self, event: ActivityEvent) {
    self.log_activity_at(event, std::time::SystemTime::now());
}

/// Emit-time seam for tests. See the `_at` convention at `dispatch_completion_at`.
fn log_activity_at(&mut self, event: ActivityEvent, now: std::time::SystemTime) {
    self.activity_log.push(LoggedActivity {
        at: Duration::from_secs(provenance::system_time_to_epoch(now)),
        event,
    });
    if self.activity_log.len() > Self::MAX_ACTIVITY_LOG {
        self.activity_log.remove(0);
    }
}
```

The ring-trim moves into `_at` so it applies on every path.

### 4. Added — the read accessor

Adjacent to `log_activity`:

```rust
/// Iterate the bare events in the activity log, oldest first.
fn activity_events(&self) -> impl Iterator<Item = &ActivityEvent> {
    self.activity_log.iter().map(|entry| &entry.event)
}
```

This is the single mechanical fix for the ~40 test assertion sites and for
`format_snapshot`.

### 5. Modified — the two production readers

- **lib.rs:8099** (`format_snapshot`): `self.activity_log.iter().rev().take(50)`
  → `self.activity_events().rev().take(50)`. Requires the accessor's iterator to
  be `DoubleEndedIterator` — `Map<slice::Iter, _>` is. Output text unchanged.
- **lib.rs:8816** (state conversion): `.iter().filter_map(activity_event_to_ui_entry)`
  stays structurally identical; the closure now receives `&LoggedActivity`.

### 6. Modified — the conversion function

lib.rs:9065. Signature and first lines:

```rust
fn activity_event_to_ui_entry(entry: &LoggedActivity) -> Option<ui::ActivityEntry>
```

- The `use std::time::Duration;` at lib.rs:9066 and the
  `let timestamp = Duration::ZERO;` at lib.rs:9068 are **deleted**.
- The `match event { … }` becomes `match &entry.event { … }`; all ~20 arms are
  otherwise unchanged.
- The tail at lib.rs:9181 becomes
  `Some(ui::ActivityEntry { timestamp: entry.at, activity })`.

### 7. Modified — test call sites (mechanical, no behavior)

Two classes:

**(a) Log assertions**, ~40 sites. Pattern:
`state.activity_log.iter().any(|e| matches!(e, ActivityEvent::…))`
→ `state.activity_events().any(…)`.
Sites include lib.rs:9303–9304 (`.zip(cases)`), 10786, 10799, 10815, 11664,
11669, 11849, 12068, 12583, 12687, 12870, 13063, 13110, 13365, 13390, 13613,
13777, 13800, 14139, 14155, 14692, 14783, 14788, 14922, 14934, 15314, and the
`.is_empty()` sites (11988, 12900, 13090, 13842) which need **no change**.

**(b) Conversion call sites**, ~15 sites calling
`activity_event_to_ui_entry(&event)`. Each needs a `LoggedActivity` wrapper.
To keep the diff small and readable, a test-module helper:

```rust
#[cfg(test)]
fn ui_entry_for(event: &ActivityEvent) -> Option<ui::ActivityEntry> {
    activity_event_to_ui_entry(&LoggedActivity { at: Duration::ZERO, event: event.clone() })
}
```

Sites at lib.rs:10830, 10898–10948, 11450, 11470, 12905, 13235–13275, 13399,
13413, 21382, 22923, 23026 call the helper instead. Note these existing tests
assert on `activity` only, never `timestamp`, so `Duration::ZERO` is a
legitimate don't-care here — it is not reintroducing the bug, because the
production path (edit 3) never produces it.

### 8. Added — tests

| Test | Pins |
|---|---|
| `log_activity_at_stamps_the_emit_instant` | **criterion 1** — inject a fixed `SystemTime`, assert `activity_log[0].at` equals that epoch |
| `stamped_entry_renders_just_now_then_one_minute_ago` | **criterion 1** — inject clock, convert to `ui::ActivityEntry`, render at `t` (→ `just now`) and at `t+60` (→ `1m ago`). No sleeps |
| `log_activity_uses_wall_clock_not_zero` | guards the regression: an entry emitted via the unstamped `log_activity` has nonzero `at` |

The second test is the end-to-end one that ties the two halves together — it is
the only test that crosses the lib.rs/ui.rs boundary, and it is what actually
proves criterion 1 as written.

---

## Ordering of changes

1. `ui.rs`: add `format_age_bucket` + `UNKNOWN_AGE` and its unit tests. **Independently
   compilable and testable** — the function is unused at this point (allow a
   temporary dead-code warning, or land it with the render swap; see plan).
2. `ui.rs`: swap the two render sites + renderer tests.
3. `lib.rs`: add `LoggedActivity`, change the field, rewrite `log_activity` /
   `log_activity_at`, add `activity_events`.
4. `lib.rs`: fix the two production readers and the conversion function.
5. `lib.rs`: mechanically fix test call sites until the workspace compiles.
6. `lib.rs`: add the three new tests.

Steps 3–5 are one compile unit — the field type change breaks everything
downstream at once and the workspace does not build until step 5 completes.
Steps 1–2 are separable from 3–6 and vice versa: the ui.rs half fixes the
*format*, the lib.rs half fixes the *data*. Both are needed for the feed to be
correct, but each compiles and tests green on its own.

## Public interface impact

None. `LoggedActivity`, `activity_events`, `log_activity_at`, and
`format_age_bucket` are all private. `ui::ActivityEntry` keeps its existing
public shape — the `timestamp` field it has carried all along simply starts
receiving a real value. No crate-boundary API changes, so `lisa-cli` is
unaffected.
