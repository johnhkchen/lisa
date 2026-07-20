# Design — T-054-01-02 shed-ceremony-first

## The decision in one line

Give `render_dag` the pane's true width, render full labels, measure the widest
raw line as characters, and on overflow re-render once with `T-` and the status
token shed — carrying status by **recoloring the id** and swapping the phase
legend for a status legend so the new color code is spelled out; if even that
spills, append one plain line saying how many columns are off-screen.

## Four decisions, taken in order

### 1. How the true width reaches the DAG

Today `print_dashboard` clamps and forgets: `render_dashboard_lines(state, cols.min(100), rows)`.
The DAG needs the number the clamp threw away, and the other three presets need
the clamp to keep working exactly as it does.

**Option A — add a fourth parameter.** Leave line 2031 alone and pass the true
`cols` alongside the clamped `width`.

Against: two width parameters in one signature, distinguishable only by name,
and every one of the ten test call sites has to grow an argument anyway. The
next person to add a preset has to guess which width their view wants, and the
compiler will not help them — both are `usize`. Literal compliance with "the
clamp stays untouched" bought with a permanently confusable signature.

**Option B — clamp one scope inward (chosen).** `render_dashboard_lines` takes
the true `pane_cols` and derives the clamped width on its first line:

```rust
fn render_dashboard_lines(state: &PluginState, pane_cols: usize, height: usize) -> Vec<String> {
    // Text presets have always laid out against at most 100 columns. The DAG
    // measures against the pane itself, so the clamp lands here rather than at
    // the call site.
    let width = pane_cols.min(100);
```

`print_dashboard` then passes `cols` unmodified. Every existing call — including
all ten tests passing `80` — behaves identically, because `80.min(100) == 80`.
The separator and the three text presets still receive the clamped number; only
the DAG dispatch gets `pane_cols`.

Read AC5 on its terms: the clamp "does not gate the DAG fit decision and stays
untouched **for the other presets**." That is a statement about which views the
clamp still governs, and under Option B it governs exactly the same three, to
the column. A test pins it: at `pane_cols = 200` the title separator is still
100 wide while the DAG runs past it.

Chosen: **B**. One width in, one clamp, documented where it applies.

### 2. Where the fit decision lives

Inside `render_dag`, not in `render_dag_view` and not in a wrapper. The decision
needs the node list, the edge list and the id map — everything `render_dag`
already has and nothing it does not. Hoisting it out would mean rebuilding the
graph twice at two altitudes.

Shape: extract the label-building and rendering into one helper parameterised by
style, then call it once or twice.

```rust
enum LabelStyle { Full, Condensed }

// -> (labels, rendered)
fn render_dag_body(active, edges, id_to_int, style) -> (Vec<(usize, String)>, String)
```

```rust
let mut style = LabelStyle::Full;
let (mut nodes, mut rendered) = render_dag_body(.., style);
if pane_cols > 0 && widest_visible_line(&rendered) > pane_cols {
    style = LabelStyle::Condensed;
    (nodes, rendered) = render_dag_body(.., style);
}
```

"Re-render **once**" is structural here, not a comment: there is one `if`, no
loop, and only two styles exist. There is no path that renders three times.

`pane_cols == 0` means "no constraint" — never condense, never indicate.
A zero width is a caller that does not know the pane, not a pane one column
wide, and the honest response to not knowing is to change nothing.

### 3. What the condensed label is

The rule the ticket states twice is "`T-` prefix and status token shed", and the
_Advances_ line prices it at "six columns of `T-` and ` RDY` per node." Six is
exactly what the measurements show: 99→69 over five nodes, 239→167 over twelve.

The ticket's worked example goes one column further — `[T-015-04-02 RDY]` →
`[15-04-02]` also drops the leading zero of the epic segment. **Not adopted**,
and recorded here as the ticket asks:

- It is a second rule that appears only in an illustration, while the rule in
  prose and the six-column budget in _Advances_ both agree on `T-` alone.
- Zeroes in an id are id, not ceremony. `054-01-02` is the ticket's name with a
  prefix removed; `54-01-02` is a name being edited. The two-line rule stays
  reversible — prepend `T-` and you are back — which the zero-eliding version
  is not once a three-digit epic exists.
- It buys one column out of six. Poor trade for an irreversible transform.

So: `strip_prefix("T-")` (falling back to the id unchanged when the prefix is
absent — a non-`T-` board loses no characters rather than a wrong one), and no
token. `T-054-01-02 WRK` → `054-01-02`.

AC2 is then structural: the condensed label is built from the id alone, so
there is no expression in the condensed path that could emit a status token.

### 4. How status survives — recolor, not glyph

**Option A — a one-glyph marker.** Keep the id in its phase color, append a
status glyph: `[054-01-02 ●]`.

Against: it costs back two of the six columns just won, on every node, in the
one mode that exists because columns ran out. Five statuses need five
distinguishable glyphs — a second vocabulary to learn, on top of the phase
glyph vocabulary the legend already lists. And it does not actually avoid the
color question: five ASCII-safe glyph shapes that read at a glance are harder to
find than five colors already mapped and already tested.

**Option B — recolor the id by status (chosen).** The id is painted
`status.color_code()` instead of `phase.color_code()`. Zero columns.

This is the line the story pins ("status carried by the freshly painted color")
and the one T-054-01-01's design was written to hand over: *"T-054-01-02
inherits a status channel that carries color, so when it drops the token it is
removing a redundant label rather than removing the only signal."* The mapping
is `TicketStatus::color_code()` — the same one the token uses in full mode, so
red still means blocked and green still means working across both modes. RDY
cyan, WRK green, REV bright yellow, BLK red are four hues wide apart on any
terminal palette; the pinned trio blocked/working/ready is red/green/cyan.

**The cost, stated plainly:** condensed mode drops the phase channel. The id can
carry one color and status wins it, because status is what the board is scanned
for — what is blocked, what is moving — and because the phase is legible in the
Operations and Present views at any width. Nothing that was reduced was reduced
in *full* mode, which is what AC1 protects.

Chosen: **B**.

#### The legend must follow the color

In condensed mode the id's color means status, so leaving `Phases: ○ Rdy ◐ Res …`
under the board would document a code the board is no longer using. The legend
is **swapped, not supplemented**:

```
full mode:       Phases: ○ Rdy ◐ Res ◑ Des ◒ Str ◓ Pln ● Imp ◎ Rev ✓ Don
condensed mode:  Status: RDY WRK REV BLK        (each word in its own color)
```

One legend, always the accurate one. The status words appear here — a legend
defining a color code, twenty-odd columns, once per view — and nowhere in a
node, which is what AC2 constrains ("condensed node **text**"). Tests assert
token-absence against the graph body lines, which is where the criterion points.

## Measurement

A production helper, because the ticket makes measurement a behavior:

```rust
fn visible_width(line: &str) -> usize   // characters, SGR sequences skipped
fn widest_visible_line(rendered: &str) -> usize
```

Two properties, both load-bearing:

- **Characters, not bytes.** The line vocabulary includes `→ ┌ ─ ↓ └ ┐`, all
  multi-byte and all one column. `.len()` would overcount a routed board by
  roughly a factor of three and condense boards that fit comfortably.
- **Escapes are not ink.** The fit decision reads `rendered`, which is pre-color
  by construction — coloring happens in a later loop (ui.rs:1047). Making
  `visible_width` skip SGR anyway means the invariant holds by function, not by
  call ordering, and AC3's "colored fixture measures identical to its uncolored
  twin" becomes a direct assertion on the function rather than an inference
  about where it is called.

Skipping mirrors the existing test-local `strip_ansi` (4196): on `\u{1b}`,
consume through the terminating `m`. The production helper counts rather than
allocating a stripped copy.

**Scope of the measurement: the graph body only.** The header, the hidden-done
summary and the legend are short, fixed-width chrome that condensing cannot
affect; folding them into the max would let a long legend trigger a condense
that does nothing for it. The number the indicator reports is the graph's.

## The indicator line

Appended immediately after the body, before the summary and legend — where the
eye already is when it runs off the right edge. Plain and dimmed, no jargon:

```
(28 columns off-screen — the map needs 128, the pane has 100)
```

It names the overflow in columns, as AC4 asks, and states both numbers so the
reader knows how much wider a pane would have to be. It is emitted from a single
place, guarded by the same `pane_cols > 0 && widest > pane_cols` predicate that
drives condensing, so "silent clip" is not a reachable state: the only way to
exceed the pane is to have already condensed and still exceed it, and that path
ends at this line. S-054-02 will add the pan keys to this sentence.

## Rejected outright

- **A config knob for the threshold.** The pane width is the threshold (AC1,
  and the story's honest boundary).
- **Truncating or ellipsising labels.** Loses id characters — the one thing on a
  node that is not ceremony. Condensing removes prefix and token, both
  recoverable by rule; a truncated id is gone.
- **Wrapping or re-laying-out the graph.** ascii-dag owns layout (N4). We choose
  label strings and report the result.
- **A third, more aggressive style.** Two styles, one `if`. If condensed still
  overflows, the honest answer is to say so, not to keep shaving.
- **Epic-prefix elision on single-epic boards** — recorded as an option, not
  built, per the ticket. When every node shares `054-`, that prefix is
  redundant on-screen and worth four more columns. It needs a uniqueness check
  across the active set and a way to say which epic is elided, so it is a
  ticket, not a branch here.

## Consequences

The DAG becomes the first view that answers to its pane. A board that fits is
untouched, byte for byte. A board that does not sheds six columns a node and
usually fits. A board that still does not stops lying about it. The width now
threaded to `render_dag_view` is the same argument S-054-02's panning will need,
and `visible_width` is the measurement any future clamp will want.
