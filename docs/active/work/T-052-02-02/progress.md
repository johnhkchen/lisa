# Progress — T-052-02-02 fold-the-echoes

All three planned steps complete. `just check` green by exit code. Nothing
remains open.

## Step 1 — Fold at the append seam ✅

Commit `2fe36ac` — *fix(plugin): fold consecutive identical activity events at append*
`--include crates/lisa-plugin/src/lib.rs`

- `LoggedActivity` gained `count: u32`; `at`'s doc now states that a fold
  overwrites it with the latest occurrence.
- `log_activity_at` hoists `at`, checks `activity_log.last_mut()` for an equal
  event, and on a match bumps `count` (saturating) and refreshes `at` and
  returns. The push arm sets `count: 1`; the cap check is untouched and
  unreachable from the fold arm.
- Its doc comment carries the Design rationale for structural equality over
  "renders the same line" — the argument a future loosening has to answer.
- Two `LoggedActivity` literals updated (`ui_entry_for` helper, the `stamped`
  closure in `only_phase_completed_projects_a_transition_line`).

Tests added: `three_identical_events_fold_into_one_counted_line`,
`an_intervening_event_breaks_the_fold`,
`distinct_facts_fill_the_ring_regardless_of_echoes`,
`near_identical_events_never_fold`.

**Deviation from Plan (test 3):** the plan said "one more distinct event → the
oldest is gone". Kept, and it revealed nothing surprising — noting only that the
assertion checks `activity_log[0]` is now `fact-1`, i.e. eviction still removes
the oldest *fact*, never an echo.

All 464 pre-existing tests passed unchanged at this step, which is the first
evidence that no existing test depended on duplicate entries being retained.

## Step 2 — Count reaches the projection and the feed ✅

Commit `d081e72` — *feat(plugin): render folded activity echoes as a trailing multiplier*
`--include crates/lisa-plugin/src/lib.rs --include crates/lisa-plugin/src/ui.rs`

Both files in one commit, as Structure required: adding a field to
`ui::ActivityEntry` breaks its sole production construction site, which is the
projection in `lib.rs`. Splitting them would not compile.

- `ui::ActivityEntry` gained `pub count: u32`.
- `ui::with_repeat_tag(message, count)` added beside `format_age_bucket`;
  `count <= 1` returns the message untouched.
- Both renderers gained one rebind before their existing `format!`. No `match`
  arm was touched, so the two renderers' divergent truncation widths were not
  disturbed.
- `activity_event_to_ui_entry` copies `entry.count`. Still a total per-entry map
  with no access to neighbours or to `State`.
- Seven `ActivityEntry` test literals gained `count: 1`, assertions untouched.

Tests added: `folded_line_renders_one_entry_with_the_multiplier`,
`projection_preserves_the_count` (both `lib.rs`);
`folded_entry_renders_the_multiplier_in_both_views`,
`single_occurrence_renders_without_a_tag`,
`the_multiplier_survives_message_truncation` (all `ui.rs`, sharing a new
`both_activity_views` fixture helper).

**Deviation from Plan:** `render_activity_log` was widened from private to
`pub(crate)`. The plan's test 5 wanted to assert on the *rendered* line from a
`State`, which needs the renderer reachable from `lib.rs`'s test module. There is
existing precedent — `render_threads` is already `pub(crate)` and is called that
way by `dashboard_thread_row` in `lib.rs` tests. Taken because asserting the fold
end-to-end through `to_ui_state()` is materially stronger evidence for AC1 than
asserting on the projection alone.

**Deviation from Plan:** the plan's step-2 test 9 assertion was tightened during
implementation. The first draft accepted either `... (x2)` or a line ending in
`(x2)` plus a reset escape; the disjunction let a tag-before-truncation
implementation slip through one arm. Replaced by the single claim that matters:
the view contains `... (x2)`.

## Step 3 — The audit dump keeps the multiplier ✅

Commit `9e4be61` — *fix(plugin): carry the fold multiplier into the state dump*
`--include crates/lisa-plugin/src/lib.rs`

- `format_snapshot` iterates `self.activity_log` instead of `activity_events()`
  and wraps each formatted line in `ui::with_repeat_tag`, so feed and dump cannot
  drift in how they spell a fold. `format_activity_event` is untouched.
- Test added: `state_dump_reports_the_fold_multiplier`.

**Deviation from Plan (required, not optional):** `just check` failed at this
step with `-D dead-code` on `activity_events()`. The dump was its last production
caller; with the dump on envelopes, the method is reachable only from tests. It
is now `#[cfg(test)]`, with a doc note saying why. This is an accurate
description of what it became — the assertion vocabulary of ~40 tests — not a
warning silenced. The alternative (leaving the dump on `activity_events()`) was
rejected in Design: it would erase multiplicity from the audit surface.

This is exactly the failure mode a grep over the gate's output would have hidden;
it surfaced because the gate was judged by exit code.

## Mutation checks

The new tests were verified to bite, not merely pass. Three deliberate
regressions were introduced and reverted:

| Mutation | Tests that failed |
|---|---|
| Fold arm disabled (`if false && ...`) | 5 |
| `newest.at = at` removed (no stamp refresh) | 2 |
| Tag moved to the front of the line | 4 |

The stamp-refresh mutation is the one worth naming: only two tests caught it, and
without the explicit `assert_ne!` against the first occurrence's instant in
`three_identical_events_fold_into_one_counted_line`, a fold that kept the *oldest*
stamp would have passed everything else.

## Gate

`just check` — **exit 0** (fmt, clippy `-D warnings`, WASM check, workspace
tests; 474 plugin tests). Judged by exit code.

One unrelated flake was observed and dismissed on evidence:
`lisa-cli`'s `triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
failed once with `TimedOut` during a full-load run. The test gives a `printf`
shell script a 2-second wall-clock deadline; it exceeded it while 380 tests ran
in parallel immediately after a full rebuild. It passes in isolation and on the
clean re-run, and this ticket touches only `crates/lisa-plugin`. Flagged in
Review as a pre-existing load-sensitive test, not fixed here — it is outside this
ticket's ownership.

## Final state

`git status --short crates/` is empty: every ticket-owned change is committed
through `lisa commit-ticket`, nothing staged, modified, or untracked.
