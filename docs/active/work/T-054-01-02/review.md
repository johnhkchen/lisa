# Review — T-054-01-02 shed-ceremony-first

## What this does

The DAG view now answers to its pane. It renders full labels, measures the
widest raw line, and if the board runs past the pane it re-renders once with
`T-` and the status token shed — six columns a node — carrying status in the
node's color instead. If even that spills, it says how much is off-screen
rather than letting the terminal clip in silence.

Five commits, one file, `just check` exit 0.

## Files

| File | Change |
|---|---|
| `crates/lisa-plugin/src/ui.rs` | modified — +701 / −59 |

Nothing created or deleted. `lib.rs` was already passing the true `cols` to
`print_dashboard`, so no caller outside `ui.rs` moved.

## Commits

| SHA | Message |
|---|---|
| `5f3f6be` | measure a line by what a terminal shows, not what it stores |
| `a4965f3` | give a dag node one place to say what it is called |
| `ad1ad8f` | let the dag see the pane it is drawn in |
| `06f197d` | shed the ceremony when the board runs out of room |
| `b58e7d4` | say how much map is off-screen instead of clipping quietly |

The first three are behavior-preserving; the behavior change is confined to the
last two. That split is what makes the refactor auditable — see the two
verification claims below.

## New in `ui.rs`

- `visible_width(&str)` / `widest_visible_line(&str)` — columns a terminal
  shows. Characters, not bytes (the edge glyphs `→ ┌ ─ ↓` are multi-byte and
  one column); SGR escapes consumed and not counted.
- `LabelStyle { Full, Condensed }` and `dag_label(id, status, style)` — the only
  place a node's text is decided.
- `NodeInk::inked()` and a `style` field — `Full` paints id-by-phase and
  token-by-status as before; `Condensed` paints the shed id by status.
- `render_dag_body(..)` — now the only `ascii_dag::DAG::from_edges(..).render()`
  in production code.
- `dag_status_legend()` and `dag_overflow_line(widest, pane_cols)`.

Modified: `render_dag` (takes `pane_cols`, makes the fit decision),
`render_dag_view` (takes `pane_cols`), `render_dashboard_lines` (takes the true
`pane_cols` and derives `let width = pane_cols.min(100)` on its first line),
`print_dashboard` (passes `cols` unclamped).

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | Forced-width tests; wide → full byte-identical; narrow → condensed and fits or indicator; threshold is the pane | `dag_wide_pane_keeps_full_labels_byte_for_byte`, `dag_narrow_pane_condenses_and_fits`, `condensing_triggers_on_overflow_only` (incl. the exact-fit boundary at 99/99) |
| 2 | No `T-`, no status token; classes distinguishable, mechanism asserted per class and recorded in design.md | `condensed_labels_carry_no_prefix_and_no_status_token`, `condensed_status_classes_are_distinguishable`, `condensed_ids_carry_status_not_phase`; design.md §4 |
| 3 | Width measured on raw pre-color lines | `visible_width_ignores_color` — colored fixture vs. uncolored twin, plus `assert_ne!` that the twin is genuinely colored |
| 4 | Off-pane columns always carry the indicator; negative fixture | `overflow_beyond_condensed_carries_the_indicator`, `a_board_that_fits_says_nothing`, `no_body_line_exceeds_the_pane_without_the_indicator` (4 boards × 4 panes) |
| 5 | Clamp neither gates the DAG nor changes for other presets | `dag_fit_is_not_gated_by_the_hundred_column_clamp` (119-col board, 200-col pane, full labels kept), `text_presets_still_clamp_at_a_hundred_columns` — **see the open note below** |
| 6 | ascii-dag invocation unchanged but for labels | Single call site in `render_dag_body`; existing `test_dag_raw_content_unchanged_by_coloring` still green unedited |
| 7 | `just check` green | **exit 0** — run as `just check > log 2>&1; echo $?`, judged by exit code, not grepped output |

## Test coverage

101 `#[test]` in `ui.rs`; workspace total 560, up from 548. **No existing
test's expectations changed** — the twelve `render_dag` call sites gained a
`DAG_WIDE` argument and nothing else.

Two verification claims are worth more than the count:

1. **The refactor changed no output.** `test_dag_raw_content_unchanged_by_coloring`
   rebuilds the graph through ascii-dag directly and compares byte for byte. It
   passed unedited across steps 1–2.
2. **The clamp move changed no behavior.** All ten existing
   `render_dashboard_lines(.., 80, ..)` call sites needed no edit. Had the move
   altered anything, at least one would have had to change.

Also verified by eye at three pane widths before the probe was removed (200 →
full labels at 119 columns, past the legacy clamp; 100 → condensed to 83 with
the `Status:` legend; 60 → condensed plus the indicator). Recorded in
progress.md.

### Gaps

- **No test drives a real Zellij pane.** The plugin's `render(rows, cols)` is a
  trait method; everything is asserted one layer down at `print_dashboard`'s
  callee. This matches how the whole existing suite is built, and the ticket
  asked for forced-width unit tests specifically.
- **Fixture widths are pinned to ascii-dag 0.8's layout.** An upstream layout
  change fails several tests at once. Mitigated by the arithmetic being explicit
  in the fixture doc comment (six columns a node), so such a failure reads as a
  layout change rather than a bug here.
- **No test asserts the condensed board is still *readable* at very small
  panes** (e.g. 20 columns, where a single node overflows). The indicator fires
  correctly — `no_body_line_exceeds_the_pane_without_the_indicator` covers pane
  20 — but there is no floor below which the view refuses to draw. Nothing in
  the ticket asks for one.

## Open concerns

### 1. The 100-column clamp is unobservable from outside — the test asserts the guarantee, not the mechanism

AC5 says the clamp "stays untouched for the other presets." I moved it one scope
inward (from `print_dashboard`'s call to `render_dashboard_lines`'s first line)
so the true pane width could reach the DAG, rather than adding a second width
parameter — two `usize` widths in one signature, distinguishable only by name,
seemed the worse legacy.

While writing the test I found that **the outer clamp has no observable effect
on any other preset**: `render_separator` clamps at 80 itself (`ui.rs:646`),
`desk_card_lines` at `width.min(100)` (682), and `render_health_alerts` at
`width.min(100)` (814). Every consumer re-clamps. Deleting the outer clamp
entirely would change nothing visible.

That redundancy is exactly why the move is safe, but it means no test can
distinguish "the clamp is where it was" from "the clamp is one scope inward."
So `text_presets_still_clamp_at_a_hundred_columns` asserts the *guarantee* —
below 100 the pane width matters, above 100 nothing changes — on a fixture built
to be width-sensitive (an 85-character ask), with an `assert_ne!` proving
non-vacuity. If a reviewer reads AC5 as pinning the literal line rather than the
behavior, this is the place to push back.

### 2. Condensed mode drops the phase channel — deliberately

The id can carry one color and status takes it. Rationale in design.md §4:
status is what the board is scanned for, phase stays legible in the Operations
and Present views, and the story pinned "status carried by the freshly painted
color." The `Phases:` legend is swapped for a `Status:` legend under condensed
mode so nothing documents a code the board has stopped speaking. Full mode is
untouched.

### 3. The ticket's worked example sheds seven columns; I shed six

The ticket's rule — "`T-` prefix and status token shed" — and its price, "six
columns of `T-` and ` RDY` per node," both mean `T-054-01-02 WRK` → `054-01-02`.
Its illustration, `[T-015-04-02 RDY]` → `[15-04-02]`, additionally elides a
leading zero, which is seven. I implemented the stated rule, not the
illustration: the extra column comes from an unstated second rule, an id's
digits are its name rather than ceremony, and shedding only the prefix keeps the
transform reversible. Recorded in design.md §3. AC2 tests absence of `T-` and of
status tokens, both of which hold. **If the illustration was the intent rather
than a typo, this is a one-line change to `dag_label` plus its tests.**

### 4. Two latent items left alone, as scoped

- The prefix-collision hazard in the ink loop (a shorter id matching inside a
  longer one), narrowed by T-054-01-01 and untouched here.
- Epic-prefix elision on single-epic boards — recorded as a design option per
  the ticket, not built. It needs a uniqueness check across the active set and a
  way to say which epic is elided.

## What a reviewer should look at first

`render_dag` in `ui.rs` — the ten lines holding the fit decision and the ten
holding the indicator. Everything else is either a helper those two call or a
test. The single predicate `pane_cols > 0 && widest_visible_line(..) > pane_cols`
appears exactly twice, and that is the whole safety argument: the only way to
exceed the pane is to have already condensed and still exceed it, and that path
ends at the indicator.
