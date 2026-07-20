# Plan — T-052-01-01 stamp-the-feed

Four commits. Each is independently compilable and leaves `cargo test --workspace`
green. Commits 1–2 fix the *format*; commits 3–4 fix the *data*.

---

## Step 1 — Bucket formatter + its unit tests

**Files:** `crates/lisa-plugin/src/ui.rs`

1. Add `const UNKNOWN_AGE: &str = "—";` and `fn format_age_bucket` after
   `format_time_since` (ui.rs:448).
2. Add the four unit tests from structure §ui.rs/tests:
   `format_age_bucket_covers_the_four_shapes`,
   `format_age_bucket_boundaries_are_exact`,
   `format_age_bucket_renders_epoch_zero_as_bounded_fallback`,
   `format_age_bucket_clamps_future_timestamps`.

**Concrete boundary table to encode** (all against `current_time = 1_800_000_000`):

| elapsed secs | expected |
|---|---|
| 0 | `just now` |
| 59 | `just now` |
| 60 | `1m ago` |
| 61 | `1m ago` |
| 3599 | `59m ago` |
| 3600 | `1h ago` |
| 86399 | `23h ago` |
| 86400 | `1d ago` |
| 172800 | `2d ago` |

Plus `timestamp = Duration::ZERO` → `—`, asserted to also satisfy
`!result.contains('h')`.

**Note on step ordering:** at the end of this step `format_age_bucket` has no
production caller, which trips `dead_code` under clippy's `-D warnings`. To keep
the gate green at every commit, step 1 and step 2 land as **one commit**. They
are written and verified separately but committed together.

**Verify:** `cargo test -p lisa-plugin format_age_bucket`

---

## Step 2 — Swap the two feed render sites

**Files:** `crates/lisa-plugin/src/ui.rs`

1. ui.rs:1009 — `format_time_since(entry.timestamp, state.current_time)` →
   `format_age_bucket(...)`.
2. ui.rs:1122 — same.
3. Add `activity_feed_renders_only_bucket_shapes` (criterion 2). Build a
   `PluginState` whose `activity_log` entries are all `PhaseCompleted` (so they
   survive the alert filter at ui.rs:1104), with timestamps spanning all four
   buckets. Render through **both** `render_activity_log` and
   `render_filtered_activity_log`; for each output assert:
   - it contains at least one of each expected bucket string, and
   - it contains no `NNNNh MMm` composite. Assert via regex-free scan: no
     substring matching `h ` followed by digits and `m` — simplest reliable form
     is to assert the joined output does not contain `"h "` at all *within the
     age column*. Since messages can legitimately contain `h `, pin instead on
     the exact composite the bug produced: assert `!output.contains("495696h")`
     **and** that every rendered age column value (chars 1..13 of each entry
     line, after stripping ANSI) is one of the four bucket shapes. The
     column-slice assertion is the real guarantee; the `495696h` check is the
     named-regression breadcrumb.

**Guard:** confirm `format_time_since` still has exactly three callers
(ui.rs:597, 913, 948) after this step, and `test_format_duration` is untouched.

**Verify:** `cargo test -p lisa-plugin` (ui module green, including the three
pre-existing feed fixtures at ui.rs:1724 / 2156 / 2596 which must not regress).

**Commit 1** = steps 1 + 2. Message: `fix(ui): render feed ages in human buckets`
Include: `crates/lisa-plugin/src/ui.rs`

---

## Step 3 — Envelope type, stamped emit, accessor

**Files:** `crates/lisa-plugin/src/lib.rs`

This is the step that breaks the build until step 4 completes; it is planned as
one commit with step 4.

1. Add `struct LoggedActivity { at: Duration, event: ActivityEvent }` near
   lib.rs:792, `#[derive(Debug, Clone)]`.
2. Change `activity_log: Vec<ActivityEvent>` → `Vec<LoggedActivity>` (lib.rs:792).
3. Rewrite `log_activity` (lib.rs:3327) to delegate to new `log_activity_at`,
   which stamps via `provenance::system_time_to_epoch(now)` and owns the
   `MAX_ACTIVITY_LOG` trim.
4. Add `fn activity_events(&self) -> impl Iterator<Item = &ActivityEvent>`.
5. Confirm `provenance` is already imported in lib.rs (it is — used at
   lib.rs:1635, 1885, 6446…). If the import is path-qualified, match the
   existing call style exactly.

**Verify:** `cargo check -p lisa-plugin` will fail loudly with the full list of
downstream sites. That failure list *is* the worklist for step 4 — capture it.

---

## Step 4 — Thread it through readers, conversion, and tests

**Files:** `crates/lisa-plugin/src/lib.rs`

1. lib.rs:8099 — `self.activity_log.iter().rev()` → `self.activity_events().rev()`.
2. lib.rs:9065 — `activity_event_to_ui_entry(entry: &LoggedActivity)`; delete the
   `use std::time::Duration;` and `let timestamp = Duration::ZERO;` lines; match
   on `&entry.event`; tail becomes `timestamp: entry.at`.
3. lib.rs:8816 — verify the `filter_map(activity_event_to_ui_entry)` still
   type-checks (it will; the closure argument type changed with the Vec).
4. Mechanical test fixes, class (a): `.activity_log.iter()` → `.activity_events()`
   at the ~26 assertion sites. `.is_empty()` / `.len()` sites need no change.
5. Mechanical test fixes, class (b): add the `ui_entry_for` test helper and route
   the ~15 `activity_event_to_ui_entry(&event)` call sites through it.
6. Iterate `cargo check -p lisa-plugin` until clean, then
   `cargo test --workspace`.

**Watch for:** lib.rs:9303–9304 uses `.zip(cases)` over the log — it needs
`.activity_events().zip(cases)`. lib.rs:14922/14934 index into the log inside a
closure; confirm the shape after conversion.

---

## Step 5 — The three new lib.rs tests

**Files:** `crates/lisa-plugin/src/lib.rs`

1. `log_activity_at_stamps_the_emit_instant` — construct a plugin, call
   `log_activity_at(ActivityEvent::Info{…}, UNIX_EPOCH + Duration::from_secs(1_800_000_000))`,
   assert `activity_log[0].at == Duration::from_secs(1_800_000_000)`. **Criterion 1, first half.**
2. `stamped_entry_renders_just_now_then_one_minute_ago` — same emit, then
   `activity_event_to_ui_entry(&plugin.activity_log[0]).unwrap()`, then call
   `ui::render_*` (or `format_age_bucket` if it is not `pub(crate)` — if it is
   private to ui.rs, this test renders through the public renderer and asserts on
   the output string). Assert `just now` at `current_time = 1_800_000_000` and
   `1m ago` at `current_time = 1_800_000_060`. **No sleeps. Criterion 1, second half.**
3. `log_activity_uses_wall_clock_not_zero` — call plain `log_activity`, assert
   `activity_log[0].at > Duration::ZERO`. Guards against a future refactor
   re-hardcoding a default.

**Visibility note:** test 2 needs to reach `format_age_bucket` or a renderer from
the lib.rs test module. `render_activity_log` is private to ui.rs. Resolution:
mark `format_age_bucket` `pub(crate)` so the lib.rs test can call it directly
with the converted entry's `timestamp`. That is a narrower widening than
exposing a renderer, and it keeps the assertion on the exact unit under test.
Decide at implementation time; if `pub(crate)` proves unnecessary because an
equivalent assertion is reachable inside ui.rs, prefer keeping it private and
splitting test 2 across the two modules.

**Commit 2** = steps 3 + 4 + 5. Message: `fix(plugin): stamp activity events with wall clock at emit`
Include: `crates/lisa-plugin/src/lib.rs`

---

## Step 6 — Full gate

```
just check
```

Runs, in order: `cargo check -p lisa-plugin --target wasm32-wasip1` (criterion 4,
WASM builds clean), `fmt-check`, `lint` (clippy `-D warnings`),
`cargo test --workspace` (criterion 5).

**Judge by exit code, not by grepping output.** Run it and read `$?`.

If `wasm32-wasip1` is not installed locally, install the target rather than
skipping — criterion 4 names the WASM build explicitly and a skipped check
cannot be reported as a passed one.

**Commit 3** (if any fmt/clippy fixes are needed): folded into commits 1–2 by
re-running the gate before each `lisa commit-ticket`, not landed separately.

---

## Testing strategy summary

| Criterion | Pinned by | Where |
|---|---|---|
| 1 — records wall clock at emit; injected clock → `just now` then `1m ago` | `log_activity_at_stamps_the_emit_instant`, `stamped_entry_renders_just_now_then_one_minute_ago` | lib.rs |
| 2 — only four bucket shapes, both renderers, composite gone | `activity_feed_renders_only_bucket_shapes` | ui.rs |
| 3 — epoch-zero → bounded fallback, never an hours figure | `format_age_bucket_renders_epoch_zero_as_bounded_fallback` | ui.rs |
| 4 — no new time source, WASM builds clean | code review of the `provenance::system_time_to_epoch` call + `just check-wasm` exit code | gate |
| 5 — `just check` green | the gate itself | gate |

All time-dependent tests take the clock as a parameter. **Zero sleeps, zero
`SystemTime::now()` in any new test** except
`log_activity_uses_wall_clock_not_zero`, which asserts only `> 0` and so cannot
be flaky.

## Commit discipline

Two `lisa commit-ticket` calls, exact `--include` paths:

```
lisa commit-ticket --ticket-id T-052-01-01 \
  --message "fix(ui): render feed ages in human buckets" \
  --include crates/lisa-plugin/src/ui.rs

lisa commit-ticket --ticket-id T-052-01-01 \
  --message "fix(plugin): stamp activity events with wall clock at emit" \
  --include crates/lisa-plugin/src/lib.rs
```

No ordinary `git add`/`git commit`. Both files are owned solely by this ticket
(no sibling ticket in E-052 touches `lisa-plugin/src`). Confirm with
`git status --short` before Review that no ticket-owned file is left staged,
modified, or untracked.

## Deviation policy

If step 4's compiler worklist turns out materially larger than the ~40 sites
research counted, or if a site resists the mechanical rename (e.g. it needs the
timestamp, not just the event), stop and record the deviation in `progress.md`
with the specific site before adapting. Do not silently widen the change.
