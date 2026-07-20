# Design — T-054-02-01 pan-without-garbage

## The decision in one line

Pan the DAG's **body lines only**, by slicing each *already-colored* line with an
ANSI-aware cutter that counts visible columns and carries the active color
across the cut; the offset lives on `State`, resets in `enter_view`, is clamped
by the renderer to the same `widest − pane_cols` number the overflow indicator
already prints, and the keys are gated so they do nothing at all outside the DAG
view or on a map that fits.

## 1. The named hazard: how to cut a colored line

This is the decision the ticket exists for. Both candidates are sanctioned by the
ticket, so it turns on merit.

### Option A — slice the raw line, then re-apply colorization

`render_dag`'s ink loop (`ui.rs:1150`) has the raw line in hand and injects color
three lines later. Slicing raw is therefore nearly free: cut `rendered.lines()`
by character, then run the existing `replace`-based inking over the shortened
line. No escape can be sheared, because at slice time there are no escapes.

**Why it loses.** The inking is whole-label substring matching:

```rust
if colored_line.contains(ink.label) { ... }
else if colored_line.contains(ink.ticket_id) { ... }
```

A node straddling the left edge has had its first characters removed, so
`054-01-02` arrives as `4-01-02`. Neither branch matches, and **the node renders
with no color at all.**

That is not a cosmetic loss. T-054-01-01 painted status into the node and
T-054-01-02 then *deleted the status token*, on the explicit grounds that "status
is carried by the freshly painted color." In condensed mode — the only mode a
pan can be reached from, since panning requires an overflow that condensing
failed to fix — **color is the entire status channel.** Option A drops that
channel precisely on the node the operator is panning toward. The predecessor
removed a redundant label; Option A would turn that removal into an actual loss
of information at the seam.

Partial re-matching (ink label suffixes as well as whole labels) would patch it,
but that is a second, fuzzier matching rule layered on one the epic already flags
as collision-prone — strictly more machinery than Option B, for a worse result.

### Option B — an ANSI-aware slicer over the colored line (chosen)

One function, the twin of `visible_width`:

```rust
/// Drop the first `offset` visible columns, keeping the paint honest.
fn pan_line(line: &str, offset: usize) -> String
```

It performs the same walk `visible_width` already performs — characters, with
`\u{1b}…m` consumed rather than counted — but instead of counting to the end it
counts to `offset`, remembering which SGR sequences are still in force, emits
those, and then passes the remainder through verbatim.

Three properties, each directly assertable:

1. **No sequence is ever cut**, because the walk never stops inside one; it
   consumes each escape whole or not at all.
2. **Color survives the cut**: sequences seen in the skipped region and not
   cancelled by a `RESET` are re-emitted at the front, so a half-visible node
   keeps its status color.
3. **No ink leaks**: if anything is still active when the line ends, a `RESET`
   is appended.

Cost: ~25 lines and one small `Vec` of active sequences per line. Risk: it is
hand-rolled escape handling — which is exactly what AC2's negative fixture is
specified to police, walking every valid offset over a fully colored board.

**Chosen: B.** It preserves the one channel condensed mode has left, it composes
with the measurement helper already in production rather than with the ink
loop's matching rules, and it is independent of *where* colorization happens —
so a later change to inking cannot silently re-break panning.

### Tracking active color, concretely

`active: Vec<String>`, not "the last escape seen". Our own vocabulary already
includes `{BOLD}{CYAN}` as two consecutive sequences; keeping a list and clearing
it on `RESET` handles compound state correctly, where remembering only the last
would silently drop `BOLD`. The list is at most a few entries because every node
resets after itself.

Only the *skipped prefix* needs this bookkeeping. Everything after the cut is
copied byte for byte, so the line's own `RESET`s arrive intact and unmoved.

## 2. Shift only — no right-edge truncation

`pan_line` drops columns from the left and leaves the right end alone. It would
have been tidy to also truncate to `pane_cols` and own the clip outright, but:

- **It would break the invariant the predecessor built.** `assert_no_silent_clip`
  (`ui.rs:~3640`) reads "if the widest body line exceeds the pane, the indicator
  must be present; **otherwise it must be absent**." Truncating every body line
  to the pane makes the antecedent unreachable, so the helper would start
  demanding the *absence* of an indicator that is correctly present, and four
  existing tests would fail for a change that alters nothing a user sees.
- At `offset == 0` shifting is a **byte-for-byte no-op**, so today's rendering is
  untouched by construction — the cheapest possible safety argument.
- The terminal's right-edge clip is unchanged from today, and the indicator
  already accounts for it.

Consequence to carry forward: `assert_no_silent_clip` is only meaningful at
offset 0. At full pan the widest body line measures exactly `pane_cols`, which
the helper would read as "fits" while the indicator is legitimately present. The
new tests therefore assert against the pan directly and do not reuse that helper
at non-zero offsets.

## 3. What pans, and what stays nailed down

**The graph body only.** The header, the overflow indicator, the
`(N done tickets hidden)` summary, and the legend do not move.

The reason is not symmetry, it is usefulness: the indicator is the line that
*names the pan keys*, and the legend is the color code the panned nodes are read
by. Sliding either off the left edge would remove the instructions and the key to
the map as a direct consequence of following the instructions.

There is also precedent: T-054-01-02 scoped its *measurement* to the graph body
for the same class of reason ("folding chrome into the max would let a long
legend trigger a condense that does nothing for it"). Measuring the body and
panning the body are the same scope, which keeps `span` exactly meaningful.

Mechanically this falls out for free — the pan applies inside the ink loop, which
only ever sees body lines.

## 4. The pan span is the indicator's number

`render_dag` already computes `widest = widest_visible_line(&rendered)` and, on
overflow, prints `widest − pane_cols` as "N columns off-screen." That subtraction
**is** the clamp the ticket specifies (`widest_line − cols`). So:

```rust
let span = if pane_cols > 0 { widest.saturating_sub(pane_cols) } else { 0 };
let offset = pan.offset.min(span);
```

One number with one meaning: how much map lies outside the pane, how far the
operator may pan, and what the indicator reports. `pane_cols == 0` — a caller
that does not know the pane — yields span 0 and no panning, matching the
established "the honest answer to not knowing is to change nothing."

`offset.min(span)` mirrors `scroll_offset.min(max_scroll)` in `print_dashboard`
exactly: **the renderer clamps, because the renderer is where the bound is
discovered.**

### A wording question this raises, answered

After panning right, does "23 columns off-screen" become a lie? No. The map is
83 columns and the viewport is 60 at every offset, so 23 columns are unseen at
every offset — panning changes *which* 23, not how many. The sentence describes
the mismatch between map and pane, which is invariant under panning, so N3 ("the
indicator reports what the render actually did") is satisfied without making the
line position-aware.

## 5. Meeting AC3's "no state changes" — the feedback problem

AC3 is stricter than the vertical pattern it otherwise asks us to mirror. `j`
increments `scroll_offset` unconditionally and lets the renderer clamp; the state
moves even when the view cannot. AC3 forbids the horizontal equivalent: on a
fitting map, or in another view, the offset must not change at all.

The view half is trivial (`self.view_preset`). The fits half is not: the key
handler knows neither the pane width (`cols` exists only as an argument to
`render`) nor the graph's rendered width (computed inside `render_dag` and
discarded).

**Option A — recompute in the handler.** Cache `last_pane_cols` on render, then
call a `dag_pan_span(state, cols)` helper from the handler. There is precedent
for real work in the handler (the desk rebuilds its whole card list per
keypress). But it re-renders the DAG on every keypress to recover a number the
last render already had, and it needs a second entry point into the DAG that
must not drift from the first.

**Option B — the render reports what it did (chosen).** The span the renderer
already computes stops being thrown away.

```rust
pub struct DagPan { pub offset: usize, pub span: usize }   // offset in, span out
```

threaded down the path T-054-01-02 already cut for `pane_cols`
(`print_dashboard` → `render_dashboard_lines` → `render_dag_view` →
`render_dag`) as `&mut DagPan`, and stored back on `State` at `lib.rs:9452`
— legal because `render(&mut self, ..)` is mutable.

Chosen: **B**. It is the same instinct as the indicator itself: the view reports
what the render actually did rather than anyone guessing. One struct, no second
DAG entry point, no per-keypress re-render.

A `&mut` parameter over a tuple return because `render_dashboard_lines` already
returns `Vec<String>` to ten call sites; `(Vec<String>, usize)` would make every
one of them destructure a bare `usize` whose meaning lives in the signature.
`&mut DagPan` names itself and cannot be confused with the `usize` widths beside
it — the confusability that Option A of the predecessor's design was rejected for.

**Staleness, stated plainly:** `span` is at most one frame old. Any key that
changes the board returns `true` and forces a render, so the operator cannot
reach a stale span by pressing keys. If the board shrinks under a poll, the worst
case is one keypress accepted that the renderer then clamps — which is exactly
what the render-time clamp is for. The gate is for AC3's inertness; the clamp is
for correctness. Both, deliberately.

**The gate:**

```rust
if self.view_preset == ui::ViewPreset::Dag && self.dag_pan.span > 0 { ... }
```

`span` is 0 in every non-DAG view by construction, so the first condition is
technically redundant — but each condition states one clause of AC3, and a test
can point at each. Redundancy that makes a criterion legible is worth one `&&`.
Inert means `return false`: no state change, no re-render.

## 6. Bindings and step size

`h`/`Left` and `l`/`Right` — all four verified unbound (`lib.rs` grep: zero
hits). No modal or desk block claims a horizontal key, so the new branch sits
beside the `j`/`k` branch and no existing handler moves.

**Step: one column**, mirroring one line vertically. Panning 23 columns takes 23
presses, which is genuinely tedious, and the honest reason not to fix it here is
that every alternative invents a constant (a "page" of what?) or a config knob,
and the story forbids new config while asking bindings to mirror the vertical
pattern. Recorded as a real cost, not an oversight; if it bites in the field, a
`H`/`L` jump-to-edge is a small follow-up that needs no new state.

`h` decrements with `saturating_sub`; `l` increments and is bounded by the gate
plus the render clamp.

## 7. Reset on view switch

One line in `enter_view` (`lib.rs:8581`), the documented single seam:
`self.dag_pan.offset = 0;`. Both view keys (`p`, `v`) route through it, and the
existing `entering_a_view_resets_cursor_expansion_and_scroll` test extends to
cover the fourth field.

`span` is not reset — it is render-reported, refreshed on the next frame, and
entering a view renders immediately. Resetting it would only widen the window in
which it is 0.

## 8. Rejected outright

- **Panning the whole DAG view including chrome.** Removes the instructions and
  the legend as a consequence of following the instructions (§3).
- **Right-edge truncation inside `pan_line`.** Breaks a working invariant helper
  to change nothing visible (§2).
- **A position report in the indicator** ("panned 10 of 23"). Useful, but AC4
  asks for the keys and the story's boundary says viewport, not chrome. The
  sentence stays honest at every offset without it (§4).
- **Follow-the-cursor, auto-centering, mouse drag, minimap.** N4 and the story's
  out-of-scope list.
- **Re-laying-out the graph at the panned width.** ascii-dag owns layout; a pan
  is a viewport over a fixed render, so the map does not reshape under the
  operator's hands.
- **Making the pan global like vertical scrolling.** Only the DAG has a width
  that exceeds its pane; the text presets clamp themselves at 80–100 columns
  three separate ways.

## 9. Consequences

`pan_line` joins `visible_width` as the second production function that knows
what a terminal shows versus what a string stores — together they are a small,
honest column-arithmetic vocabulary any future width work can reuse. `DagPan`
establishes the return path for "what the render actually did," which is the
shape the epic's N3 constraint keeps asking for. And the last silent failure in
this view closes: the board condenses, then says how much is off-screen, then
hands over the keys to go and look at it.
