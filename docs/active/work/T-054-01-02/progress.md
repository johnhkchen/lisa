# Progress — T-054-01-02 shed-ceremony-first

All five planned steps are complete and committed. `just check` exits 0.

## Step 1 — measurement ✅

Commit `5f3f6be` — *measure a line by what a terminal shows, not what it stores*

Added `visible_width` and `widest_visible_line` to `ui.rs`. Characters, with SGR
sequences consumed and not counted.

Tests: `visible_width_counts_characters_not_bytes`, `visible_width_ignores_color`
(AC3), `widest_visible_line_ignores_blank_trailing_rows`.

## Step 2 — one place decides a label ✅

Commit `a4965f3` — *give a dag node one place to say what it is called*

`LabelStyle`, `dag_label`, `NodeInk::inked()`, `NodeInk.style`, and
`render_dag_body` — which now holds the only `ascii_dag::DAG::from_edges(..).render()`
in production code. `render_dag` routes through them at `Full` only.

**Deviation (small, recorded before the fix):** `dag_label` was planned as
`dag_label(id, status: TicketStatus, style)`. `TicketStatus` derives `Clone` but
not `Copy`, so that signature would not compile against a `&TicketNode`. Chosen
fix: take `status: &TicketStatus`. Rejected: adding `Copy` to `TicketStatus` —
it would work and would match the sibling `Phase` enum, but it changes a public
type's derives on a ticket that is about label width, and the reference costs
nothing.

Tests: `dag_label_full_matches_todays_format`, `dag_label_condensed_sheds_prefix_and_token`,
`dag_label_condensed_leaves_a_prefixless_id_whole`.

**Verification that mattered:** all 548 existing tests passed unedited,
including `test_dag_raw_content_unchanged_by_coloring`, which rebuilds the graph
through ascii-dag and compares byte for byte. The refactor changed no output.

## Step 3 — thread the pane's true width ✅

Commit `ad1ad8f` — *let the dag see the pane it is drawn in*

`print_dashboard` → `render_dashboard_lines(state, cols, rows)`;
`render_dashboard_lines` derives `let width = pane_cols.min(100)` on its first
line and passes `pane_cols` to the `Dag` arm only. `render_dag_view` and
`render_dag` each gained `pane_cols`. Twelve `render_dag` test call sites took
the new `DAG_WIDE` const (1000).

**Deviation — the AC5 clamp test was rewritten, twice.** Planned as "the title
separator measures 100 at pane 200." Two findings killed that:

1. `render_separator` clamps at 80 itself (`ui.rs:646`), so the separator is 80
   wide at any pane and proves nothing about the 100 clamp.
2. Probing further: **the outer `.min(100)` is not observable from outside at
   all.** Every downstream consumer re-clamps — `desk_card_lines` at
   `width.min(100)` (682), `render_health_alerts` at `width.min(100)` (814).
   The Operations preset renders byte-identically at 80, 100 and 200 with the
   existing fixtures.

The clamp being belt-and-braces is *why* moving it is safe, but it means no test
can distinguish the mechanism. So the test asserts the guarantee instead, on a
fixture built to be width-sensitive: a desk card with an 85-character ask
renders differently at 80 than at 100 (`assert_ne!`), and identically at 100 and
200 (`assert_eq!`). Non-vacuity is asserted, not assumed. Carried into review.md
as an open note.

Test: `text_presets_still_clamp_at_a_hundred_columns`.

**Verification that mattered:** all ten existing `render_dashboard_lines(.., 80, ..)`
call sites needed no edit. Had the clamp move altered behavior, at least one
would have had to change.

## Step 4 — the fit decision ✅

Commit `06f197d` — *shed the ceremony when the board runs out of room*

Render full, measure, and on overflow re-render once condensed — one `if`, no
loop. Legend swaps to `dag_status_legend()` under `Condensed`, built from
`TicketStatus::token()` and `color_code()` so it cannot drift from the paint.

Tests: `dag_wide_pane_keeps_full_labels_byte_for_byte`,
`dag_narrow_pane_condenses_and_fits`,
`condensed_labels_carry_no_prefix_and_no_status_token`,
`condensed_status_classes_are_distinguishable`,
`condensed_ids_carry_status_not_phase`,
`dag_fit_is_not_gated_by_the_hundred_column_clamp`,
`condensing_triggers_on_overflow_only`, `zero_width_never_condenses`.
Fixtures `fan_board(n)` and `dag_body_lines`.

**Note for future test authors:** assertions on a full label must strip ANSI
first. Coloring splits `T-054-01-02 RDY` into two escape-wrapped halves, so the
raw substring never appears in painted output. Two tests were written against
`output.join("\n")`, failed, and were moved onto `dag_body_lines(&output)`.

## Step 5 — the overflow indicator ✅

Commit `b58e7d4` — *say how much map is off-screen instead of clipping quietly*

`dag_overflow_line`, called from one site guarded by the same
`pane_cols > 0 && widest > pane_cols` predicate that drives condensing.

```
(23 columns off-screen — the map needs 83, the pane has 60)
```

Tests: `overflow_beyond_condensed_carries_the_indicator`,
`a_board_that_fits_says_nothing`, and
`no_body_line_exceeds_the_pane_without_the_indicator` — AC4's negative fixture,
a `assert_no_silent_clip` helper run across 4 board sizes × 4 pane widths, which
fails both on an unannounced overflow and on an indicator that fires when the
board fits.

## Verified by eye

The 7-node board rendered at three panes and inspected before the probe was
removed:

- **200 columns** — full labels, 119 wide. Past the legacy 100 clamp, which is
  AC5 visible rather than merely asserted.
- **100 columns** — condensed to 83, ids cyan, `Status:` legend beneath.
- **60 columns** — condensed, plus `(23 columns off-screen — the map needs 83,
  the pane has 60)`.

## Final state

- `just check` → **exit 0** (fmt, clippy, WASM check, workspace tests).
- 560 tests pass, up from 548; no existing test's expectations were changed.
- One file touched: `crates/lisa-plugin/src/ui.rs`. Nothing left staged,
  modified, or untracked.
