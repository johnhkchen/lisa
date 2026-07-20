# Structure — T-054-02-01 pan-without-garbage

The shape of the code. Two files modified, none created or deleted.

| File | Change |
|---|---|
| `crates/lisa-plugin/src/ui.rs` | `DagPan`, `pan_line`, pan applied in the ink loop, indicator sentence, signatures threaded, tests |
| `crates/lisa-plugin/src/lib.rs` | `dag_pan` field, reset in `enter_view`, `h`/`l`/arrow branch, span stored on render, tests |

No new crate, no new module, no dependency change. `ascii-dag` is not touched —
the pan is a viewport over a string it already produced.

## 1. New in `ui.rs`

### `DagPan` — the DAG's horizontal viewport

Placed beside `ViewPreset` / `PluginState` in the public type block (~line 530),
because `lib.rs` stores one.

```rust
/// The DAG's horizontal viewport: how far the operator has panned, and how far
/// there is to pan.
///
/// `offset` travels in and `span` travels out — the render is the only thing
/// that knows how wide the map came out, so it reports rather than being asked.
/// `span` is zero in every view but the DAG, and zero on a map that fits, which
/// is what makes the pan keys inert without a second question being asked.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DagPan {
    /// Visible columns dropped from the left of every graph body line.
    pub offset: usize,
    /// `widest_visible_line − pane_cols`: the largest offset that reveals
    /// anything, and the same number the overflow indicator prints.
    pub span: usize,
}
```

`Copy` + `Default` so `&mut DagPan::default()` reads cleanly at the test call
sites that do not pan.

### `pan_line` — the ANSI-aware cut

Placed immediately after `widest_visible_line` (~line 970). They are twins: the
same walk, one counting to the end, one counting to a cut.

```rust
/// Drop the first `offset` visible columns from a line that may already be
/// painted.
///
/// The hazard this exists for: by the time a line is sliced it carries injected
/// SGR escapes, and a byte or char cut can land inside `\u{1b}[36m` and emit the
/// tail as literal garbage. This walks columns the way `visible_width` counts
/// them — escapes consumed, never counted — so a cut can only ever land between
/// sequences.
///
/// Paint survives the cut: sequences still in force where the cut lands are
/// re-emitted at the front, so a node straddling the left edge keeps the status
/// color that is condensed mode's only status channel. Nothing is truncated on
/// the right, so the line's own resets arrive untouched.
fn pan_line(line: &str, offset: usize) -> String
```

**Algorithm.**

1. `offset == 0` → return `line.to_string()`. The identity case is explicit, so
   an unpanned board is byte-identical to today by inspection.
2. Walk chars, `column = 0`, `active: Vec<String>`:
   - on `\u{1b}`: consume through the terminating `m` into `seq`; if `seq` is
     `RESET`, `active.clear()`, else `active.push(seq)`.
   - otherwise `column += 1`; stop once `column == offset`.
3. If the walk ran out of characters, the whole line is left of the cut →
   return `String::new()`.
4. Emit `active.concat()`, then the remainder of the iterator verbatim.
5. If `active` is non-empty and the remainder contains no reset, append `RESET`
   so no ink leaks past the line.

Step 4 is a straight copy: **escapes after the cut are never inspected**, which
is what keeps the function total over any SGR vocabulary rather than only ours.

### `dag_overflow_line` — one sentence amended

```
before: (23 columns off-screen — the map needs 83, the pane has 60)
after:  (23 columns off-screen — [h]/[l] to pan — the map needs 83, the pane has 60)
```

The affordance sits next to the loss it answers and before the arithmetic, which
is detail. The function is still called from exactly one place under exactly one
predicate, so "names the keys only when overflow exists" stays structural rather
than remembered.

## 2. Modified in `ui.rs`

### `render_dag` — where the pan lands

Signature: `fn render_dag(state, pane_cols, pan: &mut DagPan, output: &mut Vec<String>)`.

Two edits inside, both small:

**a. Compute the span and the clamped offset**, replacing the second
`widest_visible_line` call at the indicator (~1170) with one computed *before*
the ink loop so the loop can use it:

```rust
let widest = widest_visible_line(&rendered);
pan.span = if pane_cols > 0 { widest.saturating_sub(pane_cols) } else { 0 };
let offset = pan.offset.min(pan.span);
```

This deletes a duplicate measurement rather than adding one: `widest` was
already computed twice (fit decision ~1118, indicator ~1170). The fit decision
must stay where it is — it runs before the possible re-render — but the
indicator's copy folds into this.

`pan.span` is written on **every** path that reaches here, including the fitting
case where it is 0. The early returns (`(no tickets)`, `All N complete`) leave it
at whatever the caller passed; `render_dashboard_lines` zeroes it for non-DAG
views (below), and those two boards have no width to pan, so 0 is correct.

**b. Pan in the ink loop** — one line changed:

```rust
- output.push(colored_line);
+ output.push(pan_line(&colored_line, offset));
```

The loop only ever sees graph body lines, so "the body pans and the chrome does
not" needs no filter and cannot drift. Header, indicator, done-summary and
legend are pushed outside the loop and are untouched.

### `render_dag_view`, `render_dashboard_lines` — pass-through

```rust
fn render_dag_view(state, pane_cols, pan: &mut DagPan, output)
fn render_dashboard_lines(state, pane_cols, height, pan: &mut DagPan) -> Vec<String>
```

`render_dashboard_lines` zeroes the span on the three non-DAG arms — a `_ =>`
arm cannot be forgotten if it is written as an explicit `pan.span = 0;` before
the `match`, which is what this does:

```rust
// A view with no map to pan reports no room to pan. Set before the dispatch so
// no arm can forget it.
pan.span = 0;
match state.active_view { .. }
```

That single line is what makes the key gate honest in Operations, Present and
Activity without the handler needing to know which views own a graph.

### `print_dashboard` — the one public seam

```rust
pub fn print_dashboard(
    state: &PluginState,
    rows: usize,
    cols: usize,
    scroll_offset: usize,
    pan: &mut DagPan,
)
```

The modal early-return leaves `pan` untouched: a modal is drawn over the
dashboard, not instead of it, and zeroing the span there would make the pan keys
inert for one frame after a modal closes for no reason. Vertical scroll is
handled the same way — the modal path ignores `scroll_offset` too.

Order of parameters puts `pan` last, beside `scroll_offset`, so the two viewport
arguments read together.

## 3. Modified in `lib.rs`

### State

```rust
/// How far the DAG view is panned, and how far it can be — the horizontal twin
/// of `scroll_offset`. Unlike the page scroll this is per-view state: only the
/// DAG has a width that can exceed its pane.
dag_pan: ui::DagPan,
```

Placed next to `scroll_offset` (~1055). One field, not two, so offset and span
cannot be updated out of step.

### `enter_view` (~8581) — one line

```rust
self.dag_pan.offset = 0;
```

The documented single seam for "entering a view starts clean." `span` is left
alone: it is render-reported and refreshed on the frame `enter_view` itself
triggers, and zeroing it would only widen the window in which panning is inert.
A comment says so, because the asymmetry with the line above it is deliberate.

### `handle_key` (~8807) — one branch

Placed **immediately after** the `j`/`k` scroll branch: the desk block above
already returns for every key it claims, and grouping the two viewport branches
keeps the vertical and horizontal patterns readable side by side.

```rust
// Normal mode: h/l pan the DAG sideways, and only where there is map to reach.
//
// Stricter than the scroll above, which increments freely and lets the renderer
// clamp: a pan key outside the DAG view, or on a map that fits, must change
// nothing at all — so both conditions are asked here rather than absorbed by
// the clamp. `span` is what the last render reported.
if matches!(key.bare_key, BareKey::Char('h') | BareKey::Char('l'))
    || matches!(key.bare_key, BareKey::Left | BareKey::Right)
{
    if self.view_preset != ui::ViewPreset::Dag || self.dag_pan.span == 0 {
        return false;
    }
    ...
}
```

Written as one guarded branch rather than two `if`s so that the inert case is
stated once and cannot be half-applied to `h` but not `l`. Inside, `l`/`Right`
saturate up to `span`, `h`/`Left` `saturating_sub(1)`.

Returning `false` means no re-render and no state change — both halves of AC3
in one statement.

### `render` (~9452)

```rust
ui::print_dashboard(&ui_state, rows, cols, self.scroll_offset, &mut self.dag_pan);
```

Legal because `render(&mut self, ..)`. `ui_state` is built from `&self` and
dropped before the `&mut self.dag_pan` borrow is taken — if the borrow checker
disagrees, `let mut pan = self.dag_pan;` … `self.dag_pan = pan;` around the call
is the fallback, and the plan carries it as a known contingency.

## 4. Call-site churn — mechanical, compiler-verified

| Call site | Count | Edit |
|---|---|---|
| `render_dag(&state, W, &mut out)` | 25 (all tests) | `+ &mut DagPan::default(),` |
| `render_dashboard_lines(&state, W, H)` | 14 (1 prod, 13 tests) | `+ &mut DagPan::default()` |
| `render_dag_view(..)` | 1 (prod) | pass `pan` through |
| `print_dashboard(..)` | 1 (prod) | pass `&mut self.dag_pan` |

No existing test's *expectations* change — only argument lists — except
`overflow_beyond_condensed_carries_the_indicator` (`ui.rs:3667`), whose exact
`assert_eq!` on the indicator string must gain the new clause. That is the one
expectation edit in the whole ticket, and it is the criterion (AC4) being
asserted rather than an assumption being relaxed.

Two loose matchers on `contains("off-screen")` — `a_board_that_fits_says_nothing`
and `assert_no_silent_clip` — are unaffected by the wording change.

## 5. New tests

In `ui.rs`, beside the existing DAG block:

- `mixed_status_board(n)` — a new fixture. `fan_board` is uniformly `Ready`, so
  every node inks the same color; AC2's walk needs a board where the escapes
  differ per node. Cycles `Ready`/`InProgress`/`WaitingReview`/`Blocked` and
  cycles phases, so both the full-mode two-channel inking and the condensed
  one-channel inking are exercised.
- `pan_reveals_the_clipped_columns_and_returns` — AC1 at the render level.
- `pan_is_clamped_at_both_edges` — AC1's clamp half.
- `every_pan_offset_keeps_escapes_intact_and_text_correct` — **AC2, the negative
  fixture.** Walks `0..=span` on a fully colored condensed board. Per offset:
  (i) every `\u{1b}` is followed by a well-formed `[…m`; (ii) `strip_ansi(panned)`
  equals `strip_ansi(original)` with `offset` chars dropped; (iii) no line ends
  with an unclosed color.
- `a_naive_slicer_would_fail_the_escape_walk` — proves the fixture has teeth by
  running the same assertions over a deliberately naive `chars().skip(offset)`
  cut and asserting it *does* produce a broken sequence. Without this, (i)–(iii)
  could pass vacuously on a board whose escapes never straddle a cut.
- `pan_line_*` unit tests — identity at 0, past-the-end → empty, color carried
  across a mid-node cut, no leaked ink.
- `the_span_is_the_indicators_number` — pins §4 of design.md.
- `non_dag_views_report_no_span` — the `pan.span = 0` line, through
  `render_dashboard_lines` for all three other presets.
- `the_indicator_names_the_pan_keys` / extension of
  `a_board_that_fits_says_nothing` — AC4 both directions.

In `lib.rs`, beside the existing key tests:

- `pan_keys_move_the_dag_offset` and `pan_keys_clamp_at_both_edges` — AC1 at the
  key level, with `dag_pan.span` seeded as a render would leave it.
- `pan_keys_are_inert_outside_the_dag_view` — AC3, asserting both `false` return
  and unchanged offset, across the other three presets.
- `pan_keys_are_inert_when_the_map_fits` — AC3's second half, `span == 0`.
- extension of `entering_a_view_resets_cursor_expansion_and_scroll` to the
  fourth field — the reset requirement of AC1.

## 6. Ordering

The compiler enforces most of it; the two orderings that matter are that
`pan_line` exists and is tested before anything calls it, and that the signature
threading lands as one commit so the tree is never half-threaded. Sequenced in
plan.md.
