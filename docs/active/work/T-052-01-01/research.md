# Research — T-052-01-01 stamp-the-feed

Descriptive map of the code that produces, carries, and renders activity-feed
ages. No solutions proposed.

## 1. The three layers a feed line passes through

An activity line crosses three crates/modules, and time is absent from the first
two.

| Layer | Type | Time carried |
|---|---|---|
| Domain event | `lisa_core::types::ActivityEvent` (types.rs:976) | **none** — no variant has a time field |
| Plugin state | `LisaPlugin.activity_log: Vec<ActivityEvent>` (lib.rs:792) | **none** — stores the bare enum |
| UI entry | `ui::ActivityEntry { timestamp: Duration, activity }` (ui.rs:301) | a field that exists and is never populated |

The break is at the boundary between layer 2 and 3:
`activity_event_to_ui_entry` (lib.rs:9065) opens with

```rust
let timestamp = Duration::ZERO;
```

and closes by moving that constant into `ui::ActivityEntry { timestamp, activity }`
(lib.rs:9181). Nothing else ever writes the field.

## 2. Why the display reads `495696h 11m`

`PluginState.current_time` (ui.rs:396) is genuine wall clock. It is built once
per state conversion at lib.rs:8973 with the plugin's standard WASI-clock idiom:

```rust
current_time: Duration::from_secs(
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs(),
),
```

`format_time_since(timestamp, current_time)` (ui.rs:445) then computes
`current_time.saturating_sub(timestamp)` and hands it to `format_duration`
(ui.rs:429), which emits `{h}h {m}m` once hours > 0. With `timestamp == ZERO`
the subtraction yields the full Unix epoch offset — ~495,696 hours as of
2026-07 — so the rendered string is arithmetically correct and semantically
meaningless. The bug is a missing write, not a broken formatter.

## 3. `format_time_since` is shared, not feed-private

Four call sites, only two of which are the feed:

| ui.rs line | Caller | Feed? |
|---|---|---|
| 597 | waiting-item park age (`pt.parked_at`) | no |
| 913 | active thread elapsed (`active.started_at`) | no |
| 948 | parked thread elapsed (`parked.parked_at`) | no |
| 1009 | `render_activity_log` (`entry.timestamp`) | **yes** |
| 1122 | `render_filtered_activity_log` (`entry.timestamp`) | **yes** |

The three non-feed callers receive real, populated `Duration`s and render
sensible composites today. Any change to `format_time_since` or
`format_duration` itself would land on them too. The acceptance criteria scope
the bucket shapes to "both the full and filtered activity renderers", i.e.
lines 1009 and 1122 only. `format_duration` additionally has a pinned test
(ui.rs:1751 `test_format_duration`) asserting `30s` / `1m 30s` / `1h 1m`.

## 4. Where events are emitted

`log_activity` (lib.rs:3327) is the single funnel — 60+ call sites throughout
lib.rs push through it:

```rust
fn log_activity(&mut self, event: ActivityEvent) {
    self.activity_log.push(event);
    if self.activity_log.len() > Self::MAX_ACTIVITY_LOG {
        self.activity_log.remove(0);
    }
}
```

One emitter is not a direct `log_activity` caller in the usual sense:
`diagnostics::startup_diagnostics` (lisa-core/src/diagnostics.rs:21) returns a
`Vec<ActivityEvent>` built as a **pure function with no clock**, which lib.rs:8559
then replays through `log_activity` in a loop. This is the load-bearing
observation for the "does the core enum need a time field?" question in the
ticket: `lisa-core` constructs `ActivityEvent`s in a context that has no access
to a clock and no business acquiring one, and `diagnostics.rs` has ~12
construction sites (lines 25–88) plus ~10 test assertions matching on bare
variants (lines 156–317). `ActivityEvent` is also
`Serialize, Deserialize, PartialEq, Eq` (types.rs:975) — a new field changes the
serialized shape and breaks structural equality for every existing comparison.

**No consumer was found that needs time inside the core enum.** Every consumer
that wants an age is on the plugin/UI side, downstream of a real clock.

## 5. Other consumers of `activity_log`

Three production readers:

- lib.rs:8099 — `format_snapshot` renders the last 50 entries via
  `Self::format_activity_event(event)`. Iterates `&ActivityEvent` directly.
- lib.rs:8816 — the UI conversion, `.iter().filter_map(activity_event_to_ui_entry)`.
- lib.rs:3329 — the `MAX_ACTIVITY_LOG` ring trim inside `log_activity` itself.

Plus a large test surface: roughly 40 assertion sites in the lib.rs test module
follow the shape

```rust
assert!(state.activity_log.iter().any(|e| matches!(e, ActivityEvent::Warning { .. })));
```

along with `state.activity_log.is_empty()` / `.len()` checks and a `.zip(cases)`
at lib.rs:9304. Any change to the *element type* of the `activity_log` Vec has
this blast radius; changes that keep the element type do not. `is_empty()` and
`len()` survive an element-type change; `.iter()` + `matches!(e, ActivityEvent::…)`
does not, because `matches!` does not auto-deref.

`activity_event_to_ui_entry` has ~15 direct test call sites (lib.rs:10830,
10898–10948, 11450, 11470, 12905, 13235–13275, 13399, 13413, 21382, 22923,
23026), all currently calling it with a single `&ActivityEvent` argument.

## 6. Existing clock-injection precedent

The codebase already has two established seams for testing time without sleeps:

1. **A `Clock` trait** — `deadline.rs:7`:
   ```rust
   pub(crate) trait Clock { fn now(&self) -> SystemTime; }
   impl Clock for SystemClock { … SystemTime::now() }
   impl Clock for SystemTime  { … *self }
   ```
   `DeadlineEvaluator::new(clock: impl Clock)` (deadline.rs:31) snapshots once.
   Tests pass a bare `SystemTime` as the clock (deadline.rs:316).

2. **The `_at` suffix convention** — a public method takes no clock and
   delegates to a sibling that takes `now: SystemTime`:
   ```rust
   self.dispatch_completion_at(input, std::time::SystemTime::now())   // lib.rs:2265 → 2284
   self.check_assignment_ack_timeouts_at(std::time::SystemTime::now()) // lib.rs:4611 → 4450
   ```

Both are available; the second is the lighter-weight one and is used for
`&mut self` methods, which is what `log_activity` is.

## 7. The wall-clock idiom in force

116 `SystemTime::now()` sites in lib.rs. Conversion to epoch seconds is
centralized in `lisa_core::provenance::system_time_to_epoch` (provenance.rs:491):

```rust
time.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
```

used at lib.rs:1635, 1885, 6446, 6537, 6605, 6689, 6794. Note its saturating
behavior: a pre-epoch `SystemTime` yields `0`, the exact value that produces the
495696h symptom. The state conversion at lib.rs:8973 open-codes the same
computation with `unwrap_or_default()` rather than calling the helper.

Acceptance criterion 4 ("no new time source") is satisfied by either idiom —
both bottom out in `SystemTime::now()` / `UNIX_EPOCH`. WASI provides
`SystemTime::now()` under `wasm32-wasip1`; this is already load-bearing at
lib.rs:8973 in the shipped plugin.

## 8. Render cadence

The five-second timer (lib.rs:~80 `MAX_ACTIVITY_LOG`/timer constants, lib.rs:8637
timer handler) re-enters the state conversion each pass, which recomputes
`current_time` from `SystemTime::now()`. Ages therefore re-derive on every
render with no additional plumbing — a stored emit timestamp is sufficient;
no per-entry refresh mechanism is needed.

## 9. Test fixtures that touch feed timestamps

- ui.rs:1724 — `sample_state()` builds two `ActivityEntry` values with
  `timestamp: Duration::from_secs(30)` / `from_secs(60)` against
  `current_time: Duration::from_secs(120)` (i.e. ages of 90s and 60s).
- ui.rs:2156 — `completion_rejections_render_distinct_kinds_and_correlations_in_both_activity_views`
  builds five entries at `timestamp: index` seconds, `current_time: 60`, and
  asserts on kind labels and correlation IDs, not on the age string.
- ui.rs:2596 — a third fixture constructing three `ActivityEntry` values.
- ui.rs:2132 — `test_render_activity_log` asserts only non-emptiness.

None of the existing feed tests assert on the rendered age text, so the age
column is currently unpinned in either direction.

## 10. Constraints and assumptions surfaced

- **Epoch-second granularity.** `current_time` is truncated to whole seconds at
  lib.rs:8973. Any stored emit timestamp compared against it should use the same
  granularity or sub-second skew will appear as off-by-one at bucket edges.
- **Ordering is positional.** The feed renders `.iter().rev()` — insertion order,
  not timestamp order. Stamping entries does not change ordering semantics and
  nothing sorts by time.
- **`MAX_ACTIVITY_LOG` trim is `remove(0)`** — O(n) front-pop on a Vec; a wider
  element type slightly increases the memmove cost. Not a correctness concern at
  the current bound.
- **Epoch-zero is reachable in production, not just in tests.** `unwrap_or(0)` /
  `unwrap_or_default()` on a clock error yields a zero timestamp, which is why
  criterion 3 asks for a pinned bounded fallback rather than trusting that
  stamping alone removes the failure mode.
- **`Duration` is unsigned.** `saturating_sub` means a future-dated entry
  (clock skew backwards) renders as zero elapsed, not a negative age — it would
  land in the "just now" bucket rather than producing a wrong large number.
