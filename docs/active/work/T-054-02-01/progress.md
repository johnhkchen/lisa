# Progress — T-054-02-01 pan-without-garbage

## Status: Implement complete, `just check` exit 0

Three commits rather than the planned five. One planned deviation, two
discoveries during testing, all recorded below.

| # | SHA | Message |
|---|---|---|
| 1 | `9416fff` | pan the map instead of losing its edge |
| 2 | `7d265e2` | let h and l walk the board sideways |
| 3 | `5cc4bd6` | say which keys reach the off-screen columns |

Workspace tests: **560 → 582**. No existing test's expectations changed except
the one the criterion required (the indicator's exact string, AC4).

## Deviation 1 — five commits became three

**Recorded before the code changed.** Plan Step 1 was "the slicer, alone" — add
`pan_line` and `DagPan` with no callers, commit, then wire them up. That commit
cannot exist in this project: `just lint` runs
`cargo clippy -- -D warnings`, and a helper with no production caller is
`dead_code`, which is a hard error under that flag. I confirmed this rather than
assuming it — the standalone commit failed clippy twice, first on `DagPan` and
then, after moving the struct out, on `pan_line` itself.

The options were `#[allow(dead_code)]` for one commit, or folding the helper in
with its first caller. I took the second: an `allow` that exists only to survive
a commit boundary is scaffolding left in the tree, and the plan's own principle
was "every step ends green."

So Steps 1–3 merged into commit 1 (slicer + `DagPan` + threading + application +
render-side tests). Steps 4 and 5 landed as planned. The property the plan was
protecting — *the signature threading changed no behavior* — survived intact and
was verified directly: after threading, all 567 tests passed with **zero**
expectation edits, which is the check plan.md called load-bearing.

## Deviation 2 — the "nothing left" rule measures visible content, not bytes

Found by `pan_line_past_the_end_is_empty`, which failed on the first run.

Cutting a line at exactly its last visible column leaves the iterator holding
the trailing `RESET`. The remainder was non-empty as a byte string, so the
function emitted `{GREEN}{RESET}` — color codes with no glyphs between them.
Harmless on screen, but it makes "past the end is empty" false and would have
put stray escapes on every fully-panned-past line.

Fix: the guard tests `visible_width(&remainder) == 0` rather than
`remainder.is_empty()`. The rule is the same one the whole ticket rests on —
*escapes are not ink* — applied one place I had not thought to apply it.

## Deviation 3 — the negative fixture needed a wider net than "intact escapes"

The more interesting finding, and it changed a test rather than the code.

`a_naive_slicer_would_fail_the_escape_walk` failed on first run: a naive
`chars().skip(offset)` cut **passed** my escape-intactness check at every offset.
Two reasons, both worth recording:

1. A naive cut usually does not leave a *sheared* sequence. It drops the lone
   `\u{1b}` and leaves `[32m` — literal text, with no escape character in it at
   all, so a validator that only inspects sequences beginning with `\u{1b}` sees
   a clean line. The garbage is real and on screen; it is simply not detectable
   as a broken escape.
2. The escapes sit well inside the line (nodes are indented), so at small offsets
   a naive cut lands in leading whitespace and is accidentally correct.

The ticket asks the fixture to assert "intact ANSI sequences **and** the correct
visible text." The second property is the one that catches this, and my first
draft only used it as a separate assertion rather than as the failure detector.
Fixed by extracting `pan_is_faithful(panned, original, offset)` — intact
sequences **and** closed ink **and** exactly the expected visible text — and
using it for both directions: the real slicer must satisfy it at every offset,
and the naive slicer must fail it somewhere. Both are now asserted in the same
test, so the fixture cannot pass vacuously.

Also replaced the ink-leak assertion: `line.ends_with(RESET)` was wrong, because
a body line legitimately ends with `]` after its reset. `ink_is_closed` counts
opens against resets instead.

## Verified by eye

A temporary probe rendered `mixed_status_board(9)` at a 60-column pane (map 111
columns, span 51) at offsets 0, 8 and 100, then was removed — `git diff` on
`ui.rs` after removal was empty, confirming the tree matches commit 3.

What it showed, beyond what the tests assert:

- At offset 8 the leftmost node is cut mid-label to `1-02]` and **keeps its
  bright-yellow REV color.** This is design.md §1's whole argument made visible:
  the slice-raw-then-re-ink alternative would have rendered that node colorless,
  and in condensed mode color is the only status channel it has.
- The header, the indicator and the `Status:` legend stay put while the graph
  slides under them.
- Offset 100 clamps to 51 and the map ends exactly at the pane's right edge.

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | `[l]`/Right reveals, `[h]`/Left returns, both clamped; resets on view switch | `pan_reveals_the_clipped_columns_and_returns`, `pan_is_clamped_at_both_edges` (render); `pan_keys_move_the_dag_offset`, `pan_keys_clamp_at_both_edges` (keys); `entering_a_view_resets_cursor_expansion_and_scroll` extended to the pan |
| 2 | Every valid offset over a colored board: intact escapes, correct visible text; a naive slicer fails it | `every_pan_offset_keeps_escapes_intact_and_text_correct` + `a_naive_slicer_would_fail_the_escape_walk`, both over `mixed_status_board` |
| 3 | Inert in other views and on a fitting map, no state changes | `pan_keys_are_inert_outside_the_dag_view`, `pan_keys_are_inert_when_the_map_fits` — each asserts **both** `press` returning `false` and the offset unmoved; `non_dag_views_report_no_span`, `a_fitting_map_reports_no_span` |
| 4 | Indicator names the pan keys, only when overflow exists | `overflow_beyond_condensed_carries_the_indicator` (exact string), `the_indicator_names_the_pan_keys`, `the_pan_keys_are_named_only_where_they_apply` |
| 5 | `just check` green | **exit 0**, judged by exit code |

## Notes for Review

- `assert_no_silent_clip` was left untouched and is still only exercised at
  offset 0, as design.md §2 requires. Its four callers passed unedited, which is
  the evidence that `pan_line` shifts without truncating the right edge.
- One-column-per-keypress is the shipped behavior and a recorded cost
  (design.md §6), not an oversight.
