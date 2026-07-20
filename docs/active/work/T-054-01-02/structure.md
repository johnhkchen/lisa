# Structure — T-054-01-02 shed-ceremony-first

## Files

| File | Change |
|---|---|
| `crates/lisa-plugin/src/ui.rs` | modified — the whole change |

Created: none. Deleted: none. `lib.rs:9452` already passes the true `cols` to
`print_dashboard`, so no caller outside this file moves.

## New items in `ui.rs`

All private to the module unless noted.

### `enum LabelStyle`

```rust
/// How much of a node's name the board can afford to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelStyle {
    /// `T-054-01-02 WRK` — id in its phase color, token in its status color.
    Full,
    /// `054-01-02` — prefix and token shed, id recolored to carry the status.
    Condensed,
}
```

Placed next to `NodeInk` (ui.rs ~944), which is the other type that exists only
for the DAG's post-processing.

### `fn dag_label(id: &str, status: TicketStatus, style: LabelStyle) -> String`

The single place a node's text is decided.

- `Full` → `format!("{} {}", id, status.token())` — the exact expression at
  ui.rs:998 today, moved not rewritten, so AC1's byte-identity is preserved by
  construction.
- `Condensed` → `id.strip_prefix("T-").unwrap_or(id).to_string()`.

No status token is reachable on the `Condensed` arm; AC2 holds structurally.

### `fn visible_width(line: &str) -> usize`

Characters, SGR escape sequences skipped. Mirrors the test-local `strip_ansi`
(ui.rs:4196) but counts instead of allocating: on `\u{1b}`, consume through the
terminating `m` without counting; otherwise `+1`.

### `fn widest_visible_line(rendered: &str) -> usize`

`rendered.lines().map(visible_width).max().unwrap_or(0)`. Blank trailing lines
that `ascii_dag::render()` emits contribute zero.

### `fn render_dag_body(...) -> (Vec<(usize, String)>, String)`

```rust
fn render_dag_body(
    active: &[&TicketNode],
    edges: &[(usize, usize)],
    id_to_int: &HashMap<&str, usize>,
    style: LabelStyle,
) -> (Vec<(usize, String)>, String)
```

Builds `nodes` (int id + label) via `dag_label`, borrows them into `node_refs`,
and returns the labels alongside `DAG::from_edges(&node_refs, edges).render()`.

The ascii-dag invocation lives here and nowhere else — unchanged in shape from
ui.rs:1021–1022, differing only in which strings the labels are (AC6). Labels
are returned because the ink map borrows them and they must outlive it.

### `fn dag_overflow_line(widest: usize, pane_cols: usize) -> String`

The one place the indicator's wording lives:

```
(28 columns off-screen — the map needs 128, the pane has 100)
```

`DIM` … `RESET`, no jargon. Single source so AC4's "no silent-clip path" is a
property of one call site rather than a habit.

## Modified items

### `NodeInk<'a>` (ui.rs:944)

Gains one field:

```rust
struct NodeInk<'a> {
    label: &'a str,
    ticket_id: &'a str,
    token: &'a str,          // "" in condensed mode
    phase_color: &'a str,
    status_color: &'a str,
    style: LabelStyle,       // new
}
```

and one method, so the two paints live together instead of being spelled out
inside the line loop:

```rust
impl NodeInk<'_> {
    /// The label with color inserted — and nothing else changed.
    fn inked(&self) -> String {
        match self.style {
            // id in phase color, token in status color — as today.
            LabelStyle::Full => format!(
                "{}{}{} {}{}{}",
                self.phase_color, self.ticket_id, RESET,
                self.status_color, self.token, RESET
            ),
            // The token is gone; the id carries the status instead.
            LabelStyle::Condensed => format!(
                "{}{}{}", self.status_color, self.label, RESET
            ),
        }
    }
}
```

`Full` reproduces ui.rs:1051–1057 exactly. Note the condensed arm paints
`self.label` (the shed id), not `ticket_id` (which still carries `T-`).

### `fn render_dag(state, pane_cols, output)` (ui.rs:959)

Signature gains `pane_cols: usize` in position two. Body changes in four places;
steps 1–3 (header, early returns, Done filter) and the edge build are untouched.

1. **Label/render block (998–1022) → two calls.**

   ```rust
   let mut style = LabelStyle::Full;
   let (mut nodes, mut rendered) = render_dag_body(&active, &edges, &id_to_int, style);

   // Condensing is an overflow response, never a default: a board that fits
   // keeps every character it has today.
   if pane_cols > 0 && widest_visible_line(&rendered) > pane_cols {
       style = LabelStyle::Condensed;
       (nodes, rendered) = render_dag_body(&active, &edges, &id_to_int, style);
   }
   ```

   `edges` must therefore be built *before* this block; today it is built between
   the labels and the render, so it moves up. Nothing else reorders.

2. **Ink map (1027–1042)** — same `zip`, plus `style`, and `token` becomes `""`
   under `Condensed`.

3. **Line loop (1047–1068)** — the two `format!`s collapse into `ink.inked()`.
   The id-only fallback branch (1058–1065) stays as-is: still keyed on
   `ticket_id`, still painting the phase color, and still unreachable with
   today's ascii-dag output.

4. **After the body, before the summary** — the indicator:

   ```rust
   let widest = widest_visible_line(&rendered);
   if pane_cols > 0 && widest > pane_cols {
       output.push(dag_overflow_line(widest, pane_cols));
   }
   ```

   Reached only when the condensed re-render still spills, since any overflow
   with `Full` has already flipped `style` above.

5. **Legend (1082–1094)** — branches on `style`. `Full` keeps today's `Phases:`
   line, character for character. `Condensed` emits instead:

   ```
   Status: RDY WRK REV BLK          // each word in TicketStatus::color_code()
   ```

   Built by iterating `[Ready, InProgress, WaitingReview, Blocked]` through
   `token()` and `color_code()`, so the legend cannot drift from the paint.
   `Done` is excluded — it is filtered out of the graph and never appears.

### `fn render_dag_view(state, pane_cols, output)` (ui.rs:1544)

Passthrough, one argument wider.

### `fn render_dashboard_lines(state, pane_cols, height)` (ui.rs:1494)

Parameter renamed `width` → `pane_cols`; first line derives the clamp:

```rust
let width = pane_cols.min(100);
```

`width` continues to feed the separator (1503) and the Operations, Present and
Activity presets — unchanged for them. The `Dag` arm (1508) passes `pane_cols`.

### `fn print_dashboard` (ui.rs:2022)

Line 2031 becomes `render_dashboard_lines(state, cols, rows)`. The modal path
(2024, `cols.min(60)`) is untouched.

## Call sites to update

- `render_dag(&state, ..., &mut output)` — twelve tests: 2342, 2352, 2952, 2982,
  3024, 3043, 3079, 3105, 3152, 3218, 3281, 3331. All pass a wide value
  (`DAG_WIDE`, a test const at 1000) so they keep asserting on full labels;
  their expectations do not otherwise change.
- `render_dashboard_lines(&state, 80, ..)` — ten tests: 3437, 3455, 3471, 3566,
  3581, 3684, 4016, 4389, 4404. **No edit needed**: `80.min(100) == 80`, and the
  DAG fixture at 3566 is a 3-node chain measuring 39 columns, so it stays in
  full labels and its `contains("T-002")` assertion still holds.

## Test additions

New tests, sited with the other DAG tests (after ~3073):

| Name | Criterion |
|---|---|
| `dag_wide_pane_keeps_full_labels_byte_for_byte` | AC1 — wide pane vs. a direct ascii-dag rebuild |
| `dag_narrow_pane_condenses_labels` | AC1 — same graph, narrow pane, fits after condensing |
| `condensed_labels_carry_no_prefix_and_no_status_token` | AC2 |
| `condensed_status_classes_are_distinguishable` | AC2 — per-class color asserted on body lines |
| `visible_width_ignores_color` | AC3 — colored fixture vs. uncolored twin |
| `overflow_beyond_condensed_carries_the_indicator` | AC4 |
| `no_body_line_exceeds_the_pane_without_the_indicator` | AC4 negative fixture, over several widths |
| `dag_fit_is_not_gated_by_the_hundred_column_clamp` | AC5 — pane 200, board >100, full labels kept |
| `text_presets_still_clamp_at_a_hundred_columns` | AC5 — separator width at pane 200 |

Shared test helpers:

- `wide_board(n)` — a root fanned to `n` children with realistic
  `T-054-01-NN` ids; measured widths: 6 nodes = 99 full / 69 condensed,
  8 nodes = 139 full / 91 condensed.
- `dag_body_lines(output)` — the rendered rows between the header and the
  summary, which is where AC2 and AC4 point.

## Ordering

1. `visible_width` + `widest_visible_line` + their test. Standalone, no callers.
2. `LabelStyle` + `dag_label` + `NodeInk::inked`, with `render_dag` refactored to
   route through them at `Full` only — a pure refactor, all existing tests green.
3. Width plumbing: `print_dashboard` → `render_dashboard_lines` → `render_dag_view`
   → `render_dag`, with `pane_cols` accepted and ignored. Still green.
4. The fit decision, the condensed re-render and the legend swap.
5. The indicator line.
6. Remaining tests, `just check`.

Steps 1–3 are behavior-preserving and independently verifiable; the behavior
change is confined to 4 and 5.

## Invariants to hold

- Nothing writes a node label except `dag_label`.
- `ascii_dag::DAG::from_edges(...).render()` appears exactly once in production
  code, inside `render_dag_body`.
- The condense predicate and the indicator predicate are the same expression;
  overflow without the indicator is unreachable.
- `render_dag_body` is called at most twice per render — one `if`, no loop.
