# Review — T-054-02-01 pan-without-garbage

## What this does

The DAG view can now be panned sideways. `[h]`/`[l]` (and Left/Right) shift a
column offset over the graph body, clamped to exactly the number of columns the
overflow indicator already reports, reset on view switch, and completely inert
in the other three views and on any map that fits. The indicator gained the key
names, so the affordance announces itself exactly where the loss is reported.

The hazard the ticket is named for is handled by cutting with a slicer that
counts *visible* columns and carries the active color across the cut, so a node
straddling the left edge keeps its status color instead of losing it or spilling
escape garbage.

Four commits, two files, `just check` **exit 0**.

## Files

| File | Change |
|---|---|
| `crates/lisa-plugin/src/ui.rs` | modified — `DagPan`, `pan_line`, pan applied, span reported, indicator, tests |
| `crates/lisa-plugin/src/lib.rs` | modified — `dag_pan` state, reset, key branch, tests |

Nothing created or deleted. No dependency change; `ascii-dag` is untouched — a
pan is a viewport over a string it already produced, not a re-layout.

## Commits

| SHA | Message |
|---|---|
| `9416fff` | pan the map instead of losing its edge |
| `7d265e2` | let h and l walk the board sideways |
| `5cc4bd6` | say which keys reach the off-screen columns |
| `66a0023` | report the pan span on every path the dag can take |

## New in `ui.rs`

- **`pub struct DagPan { offset, span }`** — the DAG's horizontal viewport.
  `offset` travels in, `span` travels out. The render is the only thing that
  knows how wide the map came out, so it reports rather than being asked.
- **`fn pan_line(line, offset)`** — the twin of `visible_width`: the same walk,
  one counting to the end and one counting to a cut. Escapes consumed whole and
  never counted, sequences still in force re-emitted at the front, everything
  past the cut copied verbatim, and a `RESET` appended if a color would
  otherwise be left open.

Modified: `render_dag` (reports `span`, clamps, pans in the ink loop),
`render_dag_view` / `render_dashboard_lines` / `print_dashboard` (thread `pan`),
`dag_overflow_line` (names the keys).

## The design decision worth reviewing

The ticket left the slicing strategy open. I chose an **ANSI-aware cut over the
already-colored line** rather than slicing raw and re-inking. The reason is
specific to what T-054-01-02 shipped: it *deleted the status token* on the
grounds that color carries status, so in condensed mode — the only mode a pan is
reachable from — **color is the entire status channel.** Re-inking after a raw
slice matches whole labels (`colored_line.contains(ink.label)`), and a node cut
mid-label matches nothing, so it would render colorless. That turns the
predecessor's removal of a redundant label into an actual loss of information,
precisely at the edge the operator is panning toward.

Confirmed by eye before the probe was removed: at offset 8 the clipped node
renders as `1-02]` **still in its bright-yellow REV color**. Rationale in
design.md §1.

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | `[l]`/Right reveals, `[h]`/Left returns, both clamped; resets on view switch | `pan_reveals_the_clipped_columns_and_returns`, `pan_is_clamped_at_both_edges`; `pan_keys_move_the_dag_offset`, `pan_keys_clamp_at_both_edges` (all four bindings); `entering_a_view_resets_cursor_expansion_and_scroll` extended |
| 2 | Every offset over a colored board: intact escapes + correct visible text; a naive slicer fails it | `every_pan_offset_keeps_escapes_intact_and_text_correct` and `a_naive_slicer_would_fail_the_escape_walk`, over the new `mixed_status_board` |
| 3 | Inert in Operations/Activity/Present and on a fitting map, no state changes | `pan_keys_are_inert_outside_the_dag_view`, `pan_keys_are_inert_when_the_map_fits` — both assert the `false` return **and** the unmoved offset; `non_dag_views_report_no_span`, `a_fitting_map_reports_no_span`, `a_board_with_no_graph_reports_no_span` |
| 4 | Indicator names the pan keys, only when overflow exists | `overflow_beyond_condensed_carries_the_indicator` (exact string), `the_indicator_names_the_pan_keys`, `the_pan_keys_are_named_only_where_they_apply` |
| 5 | `just check` green | **exit 0** — run as `just check > log 2>&1; echo $?`, judged by exit code, never grepped output |

## Test coverage

Workspace **560 → 583**. Two verification claims carry more weight than the
count:

1. **The signature threading changed no behavior.** After threading `DagPan`
   through four functions and 39 test call sites, all 567 tests passed with
   **zero expectation edits**. Had the threading altered any output, at least
   one assertion would have had to move.
2. **The negative fixture has teeth.** `a_naive_slicer_would_fail_the_escape_walk`
   asserts the naive cut *fails* the same walk the real slicer passes, in the
   same test. Without it, AC2 could pass on a board whose escapes never straddle
   a cut.

`assert_no_silent_clip` and its four callers were left **unedited** and still
run at offset 0 only. That is the evidence that `pan_line` shifts without
truncating the right edge — design.md §2 explains why truncating would have
broken that helper's meaning.

### Gaps

- **No test drives a real Zellij pane.** Everything is asserted one layer below
  `render()`, matching how the whole existing suite is built.
- **Fixture widths are pinned to ascii-dag 0.8's layout.** `mixed_status_board(7)`
  at a 60-column pane is 83 columns condensed; an upstream layout change moves
  several assertions at once. Inherited from the predecessor's fixtures.
- **No test asserts the *rendered* pan and the *key* pan agree end to end** —
  the key tests seed `span` as a render would leave it rather than running a
  render first. The seam between them is one assignment in `render`, and the
  staleness note below is the honest statement of its cost.

## Open concerns

### 1. `span` is one frame old, by construction

AC3 asks for "no state changes," which is stricter than the vertical pattern it
otherwise mirrors: `j` increments `scroll_offset` freely and lets the renderer
clamp. To be genuinely inert, the key handler has to know whether the map
overflows — and it has neither the pane width (`cols` exists only as an argument
to `render`) nor the graph's rendered width.

So the render reports `span` and the handler reads what the last frame left. Any
key that changes the board returns `true` and forces a render, so an operator
cannot reach a stale span by pressing keys; the worst case is one keypress
accepted after the board shrank under a poll, which the render-time clamp then
corrects. **The gate is for inertness; the clamp is for correctness.** Both are
present deliberately, and `66a0023` closed the last path where a render could
leave a stale span behind.

If a reviewer reads AC3 as requiring inertness with no cached value at all, the
alternative is recomputing the DAG on every keypress, which design.md §5 rejects
and which I would not recommend.

### 2. `pan_line` assumes SGR escapes

It consumes `\u{1b}` through the terminating `m`. That is the only escape form
this codebase emits (colors and `RESET`), and it matches `visible_width`'s
existing walk exactly, so the two cannot disagree. A cursor-movement or OSC
sequence would be mishandled — but nothing produces one, and if something ever
did, `visible_width` would already be measuring it wrong.

### 3. One column per keypress

Panning 51 columns takes 51 presses. Recorded in design.md §6 as a real cost
rather than an oversight: every alternative either invents a constant ("a page"
of what?) or adds config, and the story forbids new config while asking the
bindings to mirror the vertical pattern. A `H`/`L` jump-to-edge would be a small
follow-up needing no new state.

### 4. The indicator's count does not change as you pan

It reports `widest − pane_cols` at every offset. This is deliberate and I
believe honest: the map is 111 columns and the pane is 60 wherever the viewport
sits, so panning changes *which* columns are off-screen, never how many. A
position readout ("panned 10 of 51") was considered and rejected as beyond AC4
and the story's viewport boundary. Flagged because a reviewer may reasonably read
"off-screen" as positional.

### 5. Chrome does not pan — deliberate

Header, indicator, done-summary and legend stay anchored while the graph slides
under them. Panning them would slide away the line naming the pan keys and the
legend the nodes are read by, as a direct consequence of following the
instructions. Falls out structurally: the pan applies inside the ink loop, which
only ever sees body lines.

## What a reviewer should look at first

`pan_line` in `ui.rs` — about 40 lines, and the whole ticket turns on it. Then
the three lines in `render_dag` that compute `span`, clamp the offset, and call
it. Everything else is a pass-through parameter or a test.

The single most useful thing to check: `a_naive_slicer_would_fail_the_escape_walk`
asserts both that the naive cut breaks and that the real one does not, over the
same board and the same offsets. If that test is sound, AC2 is sound.
