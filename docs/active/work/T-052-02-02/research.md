# Research — T-052-02-02 fold-the-echoes

Descriptive map of the activity-log path as it exists today. No proposals here.

## 1. The ring

`State.activity_log` (`crates/lisa-plugin/src/lib.rs:872`) is a
`Vec<LoggedActivity>`, oldest first.

```rust
// lib.rs:751
struct LoggedActivity {
    /// Wall clock at emit, as a duration since the Unix epoch.
    at: std::time::Duration,
    event: ActivityEvent,
}
```

`at` is the T-052-01-01 stamp: seconds since the epoch, captured at emit. Before
that ticket the field did not exist and the feed rendered every age from
`Duration::ZERO` (`495696h 11m`). The stamp is what makes "refresh the timestamp
on fold" a meaningful operation — the entry's age is read from this field alone.

The cap is `State::MAX_ACTIVITY_LOG = 100` (`lib.rs:1052`).

## 2. The single append seam

There are exactly two entry points, and one is a thin wrapper on the other
(`lib.rs:3411-3427`):

```rust
fn log_activity(&mut self, event: ActivityEvent) {
    self.log_activity_at(event, std::time::SystemTime::now());
}

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

Two facts matter for this ticket:

- **Nothing else touches `activity_log` mutably.** A workspace grep for
  `activity_log` finds this one `push`, this one `remove(0)`, and read-only
  iteration everywhere else. `log_activity_at` is a genuine choke point, so
  append-time bookkeeping placed here is unavoidable by construction — no caller
  can route around it.
- **The `_at` seam already exists and is the tested one.** The convention is
  documented in place (matching `dispatch_completion_at`,
  `check_assignment_ack_timeouts_at`): the caller-facing method reads the clock,
  the `_at` method takes it, so tests pin an instant without sleeping. Tests use
  it via `feed_test_instant(offset)` around a fixed `FEED_TEST_NOW_SECS =
  1_800_000_000` (`lib.rs:11125-11180`).

Eviction is `remove(0)` — an O(n) memmove on a 100-element `Vec`, once per
append past the cap. Cheap enough that nobody has replaced it with `VecDeque`.

## 3. Who reads the ring

Three readers, all read-only:

1. **The feed projection** (`lib.rs:9031-9035`), the ticket's "UI projection":
   ```rust
   let activity_log: Vec<ui::ActivityEntry> = self
       .activity_log
       .iter()
       .filter_map(activity_event_to_ui_entry)
       .collect();
   ```
   A pure `filter_map` over stored entries, run inside the render path. Per
   CLAUDE.md/ticket, render runs on a five-second cadence — this is the loop the
   ticket forbids growing an O(n) rescan inside.

2. **`activity_event_to_ui_entry`** (`lib.rs:9293-9401`), the per-entry map.
   Takes `&LoggedActivity`, returns `Option<ui::ActivityEntry>`. Its doc comment
   carries the T-052-02-01 invariant — *one transition, one line* — and names the
   variants deliberately demoted to `None`: `PluginStarted`, `ThreadExited`,
   `TicketPhaseChanged`, `TicketStatusChanged`, `DagRecomputed`, `PollSummary`,
   and `HealthStateChanged{Healthy}`. Everything else becomes
   `ui::ActivityEntry { timestamp: entry.at, activity }`.

3. **The Shift+D state dump** (`lib.rs:8302-8311`):
   ```rust
   let log_entries: Vec<_> = self.activity_events().rev().take(50).collect();
   ```
   through `activity_events()` (`lib.rs:3457-3463`), which maps the envelope away
   and yields `&ActivityEvent`. Rendered by `State::format_activity_event`
   (`lib.rs:7950+`), a per-variant `Debug`-ish line: `PhaseCompleted: T-001
   Implement`, `TicketPhaseChanged: T-001 Research -> Design`.

   **This is the ring's second job.** T-052-02-01 demoted the scheduling-skip
   line "into the state dump" and its `DeclineReason` doc (`lib.rs:757-771`)
   states the principle explicitly: *demotion, not erasure* — the question "why
   didn't X spawn?" still has an answer. So the ring is both the feed's source
   and the operator's audit record, and anything that collapses two entries into
   one collapses them for both readers.

`activity_events()` is additionally the assertion vocabulary of roughly forty
tests (`state.activity_events().any(|e| matches!(...))`).

## 4. The render surfaces

`crates/lisa-plugin/src/ui.rs` holds two renderers over
`PluginState.activity_log: Vec<ActivityEntry>` (`ui.rs:391`):

- `render_activity_log` (`ui.rs:1022`) — the full feed. `.iter().rev().take(n)`,
  newest first.
- `render_filtered_activity_log` (`ui.rs:1119`) — alerts only; keeps
  `PhaseCompleted | Error | Warning | CompletionRejected`.

Both share one line shape:

```rust
let time_ago = format_age_bucket(entry.timestamp, state.current_time);
let (icon, color, message) = match &entry.activity { ... };
output.push(format!("{}{}{} {:<12} {}{}{}", color, icon, RESET, time_ago, color, message, RESET));
```

The `match` arms build `message` and truncate free text — 40 chars in the full
feed, 50 in alerts, both with a `...` suffix. Notably the two renderers have
*duplicated, slightly divergent* arms (different truncation widths); they are not
factored through a shared formatter today.

The UI types:

```rust
// ui.rs:301
pub struct ActivityEntry {
    pub timestamp: Duration,
    pub activity: ActivityType,
}
```

`ActivityType` (`ui.rs:266`) has seven variants: `PhaseCompleted`, `Commit`,
`Error`, `Warning`, `ThreadStarted`, `Info`, `CompletionRejected`. Neither type
derives `PartialEq` — only `Debug, Clone`.

`ActivityEntry` is built as a struct literal at one production site
(`lib.rs:9397`) and seven test sites in `ui.rs` (~1751, ~1758, ~1851, ~2292,
~2730, ~2737, ~2744). Adding a field to it is a compile-error-guided edit across
those eight places, not a silent change.

## 5. What can actually echo

`ActivityEvent` (`crates/lisa-core/src/types.rs:976`) derives
`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. **Structural equality is
already available** — no new derive is needed to compare an incoming event
against the newest entry.

Some variants carry fields the feed does not render, so *equal* and *renders
identically* are not the same relation:

| Variant | Fields not in the rendered line |
|---|---|
| `ThreadSpawned` | `pane_id` (feed prints ticket + `Ready`) |
| `ArtifactCreated` | `phase` (feed prints ticket + path) |
| `SessionLaunch` | `pane_id`; `command` truncated at 120 |
| `SessionTimedOut` | `elapsed_secs` shown only as `elapsed_secs / 60` |
| `HealthStateChanged` | `old_health` |

So two unequal events can render one identical feed line (two `SessionTimedOut`
at 3610s and 3620s both read "60m"), and the demoted variants all render *no*
feed line at all — `DagRecomputed{5}` and `TicketPhaseChanged{...}` are equally
invisible in the feed while being entirely different facts in the dump. Any fold
predicate has to pick a side of this gap; Design owns that choice.

## 6. Existing tests that constrain the change

- `log_activity_at_stamps_the_emit_instant`, `log_activity_uses_wall_clock_not_zero`,
  `stamped_entry_renders_just_now_then_one_minute_ago` (`lib.rs:11133-11196`) —
  the T-052-01-01 stamp guarantees.
- `only_phase_completed_projects_a_transition_line` (`lib.rs:12188+`) builds
  `LoggedActivity` as a struct literal via a local `stamped` closure; a new
  envelope field lands here.
- `test_format_snapshot_activity_log_limit` (`lib.rs:15249`) pushes 100
  `Info{message: format!("event-{i}")}` and asserts the dump shows 50–99. All
  messages are distinct, so no fold predicate based on content can disturb it.
- `activity_feed_renders_only_bucket_shapes`, `test_render_activity_log`,
  `named_completion_rejections_become_distinct_correlated_activity_events`
  (`ui.rs`) — the renderer fixtures listed in §4.
- `feed_phase_lines(&state)` (`lib.rs` ~12030-12060) is an existing test helper
  that counts *through the projection* rather than over `activity_events()`,
  established by T-052-02-01. It is the right shape to reuse for feed-line
  assertions.

## 7. Constraints carried into Design

- Fold must live in `log_activity`/`log_activity_at` or its immediate seam
  (AC3, N4); the projection stays a pure per-entry map.
- The render path is five-second cadence — no O(n) work added there.
- The ring backs the Shift+D audit dump as well as the feed, and this codebase's
  recent stance (T-052-02-01) is *demote, never erase*.
- `MAX_ACTIVITY_LOG = 100` must come to mean 100 distinct facts.
- Fold must refresh `at`, so a folded line wears its latest occurrence's age.
- `just check` = fmt, clippy, WASM check, workspace tests.
