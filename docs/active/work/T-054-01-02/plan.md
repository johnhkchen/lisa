# Plan — T-054-01-02 shed-ceremony-first

Five steps. The first three are behavior-preserving and prove themselves by
leaving every existing test green without editing its expectations; the behavior
change is confined to steps 4 and 5. One commit per step, through
`lisa commit-ticket --include crates/lisa-plugin/src/ui.rs`.

## Fixtures the tests are built on

Measured against `ascii-dag 0.8`, root fanned to `n - 1` children, ids
`T-054-01-NN` with a `RDY` token:

| nodes | full | condensed |
|---|---|---|
| 6 | 99 | 69 |
| 7 | 119 | 83 |
| 8 | 139 | 97 |
| 9 | 159 | 111 |

Every column figure below comes from that table, so a fixture drifting is a test
failure with an arithmetic explanation rather than a mystery.

The 7-node board is the workhorse: **119 columns full, 83 condensed.**

- pane 200 → fits full. Board >100, pane >100 ⇒ AC5.
- pane 100 → condenses, and 83 fits ⇒ AC1 narrow half, AC2.
- pane 60 → condenses, 83 still spills ⇒ AC4 indicator.

One graph, three panes, four criteria.

---

## Step 1 — measurement

**Add** `visible_width(&str) -> usize` and `widest_visible_line(&str) -> usize`
near the other DAG helpers. Characters, SGR sequences skipped.

**Tests**

- `visible_width_counts_characters_not_bytes` — `"[T-002 WRK] → [T-003 BLK]"`
  measures its character count, strictly less than `.len()`.
- `visible_width_ignores_color` (**AC3**) — take a real rendered DAG line, build
  a colored twin by wrapping the label in `RED`/`RESET`, assert
  `visible_width(colored) == visible_width(plain)` and that the two differ as
  byte strings. This is AC3's "fully colored fixture measures identical to its
  uncolored twin" asserted on the function itself.
- `widest_visible_line_ignores_blank_trailing_rows` — ascii-dag emits trailing
  blanks; the max must come from the content.

**Verify** `cargo test -p lisa-plugin`. Nothing else can regress: no callers yet.

**Commit** `feat(plugin): measure a line by what a terminal shows, not what it stores`

---

## Step 2 — one place decides a label

**Add** `LabelStyle` and `dag_label(id, status, style)`. **Add**
`NodeInk::inked()` and the `style` field. **Refactor** `render_dag` to build
labels through `dag_label(.., LabelStyle::Full)` and to paint through
`inked()` — `Full` only, no condensed path reachable yet. Extract
`render_dag_body` so the ascii-dag invocation lives in exactly one place.

**Tests**

- `dag_label_full_matches_todays_format` — `dag_label("T-054-01-02", InProgress, Full)`
  is `"T-054-01-02 WRK"`.
- `dag_label_condensed_sheds_prefix_and_token` — `"054-01-02"`.
- `dag_label_condensed_leaves_a_prefixless_id_whole` — an id not starting `T-`
  loses no characters (guards `strip_prefix` against eating a leading char).

**Verify** the twelve existing `render_dag` tests pass **unedited except for the
new width argument**, in particular `test_dag_raw_content_unchanged_by_coloring`
(ui.rs:3037), which rebuilds the graph through ascii-dag directly and compares
byte for byte. That test passing is the proof this step changed no output.

**Commit** `refactor(plugin): give a dag node one place to say what it is called`

---

## Step 3 — thread the pane's true width

Signatures only; the width is accepted and ignored.

- `print_dashboard`: `render_dashboard_lines(state, cols, rows)` — the `.min(100)`
  moves one scope inward.
- `render_dashboard_lines(state, pane_cols, height)`: `let width = pane_cols.min(100);`
  on the first line, feeding the separator and the three text presets exactly as
  before; the `Dag` arm passes `pane_cols`.
- `render_dag_view(state, pane_cols, output)` → `render_dag(state, pane_cols, output)`.
- Twelve `render_dag` test call sites take `DAG_WIDE` (a test const, 1000) so
  they keep asserting on full labels.

**Tests**

- `text_presets_still_clamp_at_a_hundred_columns` (**AC5, second half**) —
  `render_dashboard_lines(&state, 200, 40)` on the Operations preset: the title
  separator measures 100, not 200.

**Verify** the ten `render_dashboard_lines(.., 80, ..)` tests need no edit
(`80.min(100) == 80`), including the four-preset test at 3566 whose DAG fixture
is a 3-node chain at 39 columns — comfortably full-label at 80. If any of those
ten needs editing, the clamp move was not behavior-preserving and this step is
wrong.

**Commit** `refactor(plugin): let the dag see the pane it is drawn in`

---

## Step 4 — the fit decision

Render full, measure, and on overflow re-render once condensed. Swap the legend
under `Condensed`. One `if`, no loop.

**Tests**

- `dag_wide_pane_keeps_full_labels_byte_for_byte` (**AC1**) — 7-node board at
  pane 200; ANSI-stripped body equals a direct
  `DAG::from_edges(..).render()` with `format!("{} {}", id, token)` labels,
  line for line. Same technique as the existing 3037 test, at a pane wide enough
  to prove the decision ran and chose `Full`.
- `dag_narrow_pane_condenses_and_fits` (**AC1**) — same board at pane 100:
  body contains `054-01-02`, and `widest_visible_line(body) == 83 <= 100`.
- `condensed_labels_carry_no_prefix_and_no_status_token` (**AC2**) — over the
  body lines only, assert no `"T-"` and none of `RDY WRK REV BLK DON`. Asserted
  on the graph body, which is what the criterion names ("condensed node text");
  the swapped `Status:` legend defines the color code and sits outside the body.
- `condensed_status_classes_are_distinguishable` (**AC2**) — a board carrying one
  Ready, one InProgress, one WaitingReview and one Blocked ticket, condensed;
  for each, assert the body contains `{status.color_code()}{condensed_id}{RESET}`,
  and assert the four colors are four distinct strings. Per class, as AC2 asks.
- `condensed_ids_carry_status_not_phase` — two Blocked tickets in different
  phases condense to the same color; a Blocked and a Ready ticket in the *same*
  phase condense to different colors. This is the test that would fail if the
  recolor silently kept sourcing `phase.color_code()`.
- `dag_fit_is_not_gated_by_the_hundred_column_clamp` (**AC5**) — through
  `render_dashboard_lines(&state, 200, 60)` with the DAG preset and the 7-node
  board: 119 > 100 but < 200, and the output still carries `T-054-01-` labels.
  Drives the real entry point, so it fails if the clamp is reintroduced anywhere
  on the path.
- `condensing_triggers_on_overflow_only` — the 6-node board (99) at pane 120
  keeps full labels; at pane 90 it condenses. The flip threshold is the pane
  width itself, no knob (**AC1**).
- `zero_width_never_condenses` — `pane_cols == 0` keeps full labels and emits no
  indicator.

**Verify** `cargo test -p lisa-plugin`.

**Commit** `feat(plugin): shed the ceremony when the board runs out of room`

---

## Step 5 — the overflow indicator

Add `dag_overflow_line` and its single call site, guarded by the same predicate
that drives condensing.

**Tests**

- `overflow_beyond_condensed_carries_the_indicator` (**AC4**) — 7-node board at
  pane 60: condensed is 83, so the output carries the indicator naming both
  numbers (`83`, `60`) and the column shortfall.
- `no_body_line_exceeds_the_pane_without_the_indicator` (**AC4, the negative
  fixture**) — a helper `assert_no_silent_clip(output, pane)` that measures every
  body line and requires the indicator whenever any exceeds `pane`. Run it
  across a matrix: boards of 3/6/7/9 nodes × panes of 20/60/100/200. This is the
  criterion stated as a property rather than as one example, and it is the test
  that fails if a future style is added without a matching indicator path.
- `a_board_that_fits_says_nothing` — 7-node board at pane 200 and the condensed
  fit at pane 100 both carry no indicator. Guards against a permanently-on
  indicator passing the AC4 tests by brute force.

**Verify** `just check` — fmt, clippy, `cargo check --target wasm32-wasip1`, and
`cargo test --workspace`. Judged by exit code, not by reading output (**AC7**).

**Commit** `feat(plugin): say how much map is off-screen instead of clipping quietly`

---

## Testing strategy

Everything is a unit test in `ui.rs`'s `mod tests`. There is no integration
surface to add: the DAG renders to a `Vec<String>` and the only I/O is
`println!` in `print_dashboard`, which the existing suite already routes around
by calling `render_dashboard_lines` directly.

Two levels, deliberately:

- **Against `render_dag` directly**, with an explicit `pane_cols`, for the fit
  and label behavior. Forced-width, as AC1 requires, with no dependence on a
  terminal.
- **Against `render_dashboard_lines`**, for the two plumbing criteria — that the
  clamp still governs the text presets and no longer gates the DAG. Asserting
  those on `render_dag` would prove nothing about the path the pane's width
  actually takes.

Coverage of the acceptance criteria:

| AC | Step | Tests |
|---|---|---|
| 1 forced-width, both directions, no knob | 4 | `..._keeps_full_labels_byte_for_byte`, `..._condenses_and_fits`, `condensing_triggers_on_overflow_only` |
| 2 no prefix/token, classes distinguishable | 2, 4 | `dag_label_condensed_*`, `condensed_labels_carry_no_*`, `condensed_status_classes_*`, `condensed_ids_carry_status_not_phase` |
| 3 measured pre-color | 1 | `visible_width_ignores_color` |
| 4 indicator, no silent clip | 5 | `overflow_beyond_condensed_*`, `no_body_line_exceeds_the_pane_without_the_indicator`, `a_board_that_fits_says_nothing` |
| 5 clamp neither gates nor moves | 3, 4 | `text_presets_still_clamp_*`, `dag_fit_is_not_gated_*` |
| 6 ascii-dag invocation unchanged | 2 | `test_dag_raw_content_unchanged_by_coloring` (existing), plus the single-call-site invariant |
| 7 `just check` green | 5 | exit code |

## Risks

- **The clamp move (step 3) is the one place behavior could shift for a view
  this ticket does not own.** Mitigated by the ten unedited call sites: if the
  move were not behavior-preserving, at least one would have to change.
- **A fixture width drifting with an ascii-dag patch release** would fail several
  tests at once. The dependency is pinned at `0.8` and the table above makes the
  arithmetic (six columns per node) explicit, so the failure would read as a
  layout change rather than as a bug in this code.
- **Over-eager condensing** on a board that fits is the regression a user would
  notice first. `condensing_triggers_on_overflow_only` and the untouched
  four-preset test at 3566 both guard it.

## Deviation policy

Any departure from this sequence gets written into `progress.md` with its
reason before the code changes, not after.
