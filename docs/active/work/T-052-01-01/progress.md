# Progress — T-052-01-01 stamp-the-feed

## Status: implementation complete, gate green

| Plan step | State | Commit |
|---|---|---|
| 1 — bucket formatter + unit tests | done | `ce2a527` |
| 2 — swap the two feed render sites | done | `ce2a527` |
| 3 — envelope type, stamped emit, accessor | done | (pending, see below) |
| 4 — thread through readers, conversion, tests | done | (pending) |
| 5 — three new lib.rs tests | done | (pending) |
| 6 — full gate | done, `just check` exit 0 | — |

## What landed

**Commit 1 — `ce2a527` `fix(ui): render feed ages in human buckets`**
(`crates/lisa-plugin/src/ui.rs`)

- Added `const UNKNOWN_AGE: &str = "—"` and `pub(crate) fn format_age_bucket`.
- Swapped `format_time_since` → `format_age_bucket` at the two feed render
  sites only (`render_activity_log`, `render_filtered_activity_log`).
- `format_time_since` / `format_duration` untouched; their three non-feed
  callers (ui.rs:623, 939, 974) verified unchanged after the swap.
- Five new tests: four-shape coverage, exact boundaries, epoch-zero fallback,
  future-timestamp clamp, and both-renderers bucket-shape assertion.

**Commit 2 — pending `fix(plugin): stamp activity events with wall clock at emit`**
(`crates/lisa-plugin/src/lib.rs` + 3 test module files)

- Added `struct LoggedActivity { at, event }`; `activity_log` is now
  `Vec<LoggedActivity>`.
- `log_activity` delegates to new `log_activity_at(event, now: SystemTime)`,
  which stamps via `provenance::system_time_to_epoch` and owns the ring trim.
- Added `fn activity_events(&self) -> impl DoubleEndedIterator<Item = &ActivityEvent>`.
- `activity_event_to_ui_entry` now takes `&LoggedActivity`; the
  `let timestamp = Duration::ZERO;` line is **deleted** and the entry carries
  `entry.at`.
- Three new tests pinning criterion 1.

## Deviations from plan

1. **Test blast radius was larger than research estimated.** Research counted
   ~40 sites by grepping lib.rs only; the compiler found ~86 in lib.rs plus 9
   across three files research never opened —
   `crates/lisa-plugin/src/tests/{hostile_order_regression,operator_recovery_matrix,signal_consumer_characterization}.rs`.
   All were the same mechanical `.activity_log.iter()` → `.activity_events()`
   rename, so the plan's deviation trigger ("a site resists the mechanical
   rename") was not hit and the approach did not change. Those three files are
   now part of this ticket's owned change set.

2. **`format_age_bucket` is `pub(crate)`, not private.** Plan step 5 flagged
   this as a decide-at-implementation-time call. Chosen: the end-to-end test
   (`stamped_entry_renders_just_now_then_one_minute_ago`) is the only test that
   proves criterion 1 as written — stamp at emit *and* the rendered progression
   — and it lives in lib.rs. `pub(crate)` on the formatter is a narrower
   widening than exposing a renderer, as the plan anticipated.

3. **`Duration` is fully qualified in the new lib.rs code.** lib.rs has no
   module-level `use std::time::Duration` (it is imported function-locally in
   two places). Matched the file's dominant `std::time::…` idiom rather than
   adding a top-level import.

4. **`activity_events` returns `impl DoubleEndedIterator`, not `impl Iterator`.**
   `format_snapshot` calls `.rev()` on it (lib.rs:8099). Structure §5 predicted
   the requirement; the return type states it explicitly.

5. **Five `{:?}` sites left reading `activity_log` directly** (lib.rs:22346,
   22367, 22383; operator_recovery_matrix.rs:135; lib.rs:10846). These are
   panic-message diagnostics that debug-print the whole log. `LoggedActivity`
   derives `Debug`, so they compile unchanged and now print the stamp too —
   strictly more useful. Not routed through the accessor.

6. **The mechanical rename pass over-reached twice.** A regex-driven edit
   rewrote the body of `activity_events` into a self-recursive call, and
   rewrote the production conversion at lib.rs:8850 to iterate bare events
   instead of envelopes. The compiler caught both immediately (E0609, E0631);
   both corrected. Recorded because the failure mode — a mechanical pass
   silently changing a production path while chasing test sites — is worth
   knowing about. It also renamed one test function
   (`test_activity_event_to_ui_entry`), which was restored.

## Verification

Gate run in the working tree, judged by **exit code**, not grepped output:

```
just check   → exit 0
```

Package-scoped, before the workspace gate:

```
cargo fmt -p lisa-plugin -- --check              → 0
cargo clippy -p lisa-plugin --all-targets -D warnings → 0
cargo check -p lisa-plugin --target wasm32-wasip1 → 0     (criterion 4)
cargo test -p lisa-plugin                        → 0     (455 passed)
```

## Concurrency note (not a deviation, but worth recording)

Midway through implementation the workspace gate went red with two failures in
`lisa-cli` `runtime::tests` (managed-Zellij download). These were **not caused by
this ticket** — a sibling Lisa thread was mid-edit in
`crates/lisa-cli/src/runtime.rs`, at one point leaving it non-compiling.
Confirmed rather than assumed: a detached worktree at `HEAD` in the scratchpad
ran those tests green. The sibling's work settled and `just check` is now exit 0.
No `lisa-cli` file is included in this ticket's commits.

## Remaining

- Commit 2 through `lisa commit-ticket` with exact `--include` paths.
- Review artifacts.
