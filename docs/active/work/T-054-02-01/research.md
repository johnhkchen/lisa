# Research — T-054-02-01 pan-without-garbage

What exists, where, and how it connects. No proposals; the tradeoffs land in
design.md.

## 1. The vertical pattern this ticket is asked to twin

Four parts, spread across two files. The ticket names all four by line number and
they are all still there:

| Part | Location | Code |
|---|---|---|
| State | `lib.rs:1055` | `scroll_offset: usize` on `State` |
| Reset on view switch | `lib.rs:8583` | `self.scroll_offset = 0` inside `enter_view` |
| Keys | `lib.rs:8807–8815` | `j`/`Down` → `+= 1`; `k`/`Up` → `saturating_sub(1)` |
| Application | `ui.rs:2189–2197` | `skip(offset).take(rows)` in `print_dashboard` |

The shape worth naming: **the handler mutates freely and unclamped; the renderer
clamps.**

```rust
// lib.rs — no upper bound known here
self.scroll_offset += 1;

// ui.rs print_dashboard — the bound is discovered where the content is
let max_scroll = lines.len().saturating_sub(rows);
let offset = scroll_offset.min(max_scroll);
```

`scroll_offset` reaches the renderer as a **parameter**, not a field of
`PluginState`: `print_dashboard(state, rows, cols, scroll_offset)` (`ui.rs:2180`,
called at `lib.rs:9452`). `PluginState` is the converted view-model; the offset
is viewport state and travels beside it.

`enter_view` (`lib.rs:8581`) is documented as "the one seam for entering a view
starts clean" and resets three things — `scroll_offset`, `desk_selected`,
`desk_expanded`. Both view-switch keys route through it: `p` (`8768`) and `v`
(`8778`). There is no third path that changes `view_preset`. A test pins the
seam: `entering_a_view_resets_cursor_expansion_and_scroll` (`lib.rs:11139`)
loops over both keys asserting all three fields.

Note the asymmetry the ticket implies: vertical scrolling is **global** (every
view scrolls), horizontal panning is to be **DAG-only**.

## 2. The DAG render path, end to end

```
State::render(&mut self, rows, cols)                      lib.rs:9440
  └─ ui::print_dashboard(&ui_state, rows, cols, scroll_offset)   ui.rs:2180
       └─ render_dashboard_lines(state, cols, rows)              ui.rs:1647
            │  let width = pane_cols.min(100);   ← legacy clamp, text presets only
            └─ match state.active_view
                 ViewPreset::Dag => render_dag_view(state, pane_cols, out)  ui.rs:1702
                      └─ render_dag(state, pane_cols, out)                  ui.rs:1064
```

`render(&mut self, ...)` is **mutable** (`lib.rs:9440`). That matters for any
scheme where the renderer reports something back to the plugin.

The DAG is the only view handed unclamped `pane_cols`; T-054-01-02 moved the
`.min(100)` one scope inward precisely so it could be.

## 3. Inside `render_dag` — where a slice could go

`render_dag` (`ui.rs:1064–1206`) emits, in order:

1. `≡≡ DAG ≡≡` header (colored) and a blank line — always lines `[0]` and `[1]`.
2. Early returns for `(no tickets)` and `All N tickets complete!`.
3. Node/edge construction; `active` filters out `TicketStatus::Done` (`1077`).
4. **The fit decision** (`1115–1121`): render full, and if
   `pane_cols > 0 && widest_visible_line(&rendered) > pane_cols`, re-render
   condensed. One `if`, two styles, no loop.
5. **The ink map** — `HashMap<&str, NodeInk>` keyed by ticket id (`1126–1145`).
6. **The colorization loop** (`1150–1165`) — the crux for this ticket:

```rust
for line in rendered.lines() {          // rendered = raw, uncolored
    let mut colored_line = line.to_string();
    for ink in color_map.values() {
        if colored_line.contains(ink.label) {
            colored_line = colored_line.replace(ink.label, &ink.inked());
        } else if colored_line.contains(ink.ticket_id) {
            colored_line = colored_line.replace(ink.ticket_id, &format!(...));
        }
    }
    output.push(colored_line);
}
```

7. The overflow indicator (`1170–1173`), guarded by the same predicate as the
   condense decision.
8. Optional `(N done tickets hidden)` summary, a blank, and the legend
   (`1190–1205`) — `Phases:` in full mode, `dag_status_legend()` in condensed.

**The structural fact this ticket turns on:** at line 1150 the raw, uncolored
line is in hand, and color is injected on the very next lines. Raw text and
colored text are separated by three lines of code, not by a module boundary.
Any slicing decision gets to choose which side of that seam it sits on.

## 4. What ANSI is actually injected

`NodeInk::inked()` (`ui.rs:1018`) produces:

- Full: `{phase_color}{id}{RESET} {status_color}{token}{RESET}` — four escape
  sequences per node, two of them interior to the label.
- Condensed: `{status_color}{label}{RESET}` — two per node.

Plus the fallback branch: a bare id gets `{phase_color}{id}{RESET}`.

Colors are `\u{1b}[…m` SGR sequences; `RESET` is `\u{1b}[0m`. So a single body
line for a 7-node board can carry a dozen escape sequences, some in the middle
of a bracketed node, some adjacent to the `─` and `→` routing glyphs ascii-dag
draws between nodes. A byte or `char` slice at an arbitrary column has a real
chance of landing between `\u{1b}[` and the terminating `m` — that is the
garbage the ticket is named for. A slice can also strip a `RESET` while keeping
its opening color, which leaks ink onto the rest of the line even though every
sequence in the output is individually intact.

The header, indicator, summary and legend lines carry escapes too (`BOLD`,
`CYAN`, `DIM`, `RESET`), so "which lines are eligible to be sliced" is a real
question, not an academic one.

## 5. Measurement helpers already in production

T-054-01-02 left exactly the tools this ticket needs (`ui.rs:946–970`):

```rust
fn visible_width(line: &str) -> usize      // chars, SGR consumed and not counted
fn widest_visible_line(rendered: &str) -> usize   // max over lines
```

`visible_width` walks chars, and on `\u{1b}` consumes through the terminating
`m`. It counts **characters**, not bytes — deliberate, because the edge glyphs
`→ ┌ ─ ↓ └ ┐` are multi-byte and one column wide. It is the same walk a
column-counting slicer would have to perform; the difference is that the slicer
must also decide where to cut and what to carry across the cut.

`widest_visible_line` is what the ticket's `widest_line − cols` clamp is written
against. It is currently computed twice in `render_dag` — once for the condense
decision (`1118`) and once for the indicator (`1170`), both over `rendered`
(raw).

Note: `visible_width` counts one column per `char`. Wide CJK and combining
characters would be miscounted, but ticket ids are ASCII and the routing glyphs
are single-width, so the assumption holds for everything this view draws.

## 6. The indicator line, which AC4 asks to amend

```rust
fn dag_overflow_line(widest: usize, pane_cols: usize) -> String   // ui.rs:1214
```

Current output: `(23 columns off-screen — the map needs 83, the pane has 60)`.
Its doc comment ends with `S-054-02 will add the keys for panning to this
sentence.` — the handoff is explicit.

Pinned by `overflow_beyond_condensed_carries_the_indicator` (`ui.rs:3667`) with
an exact `assert_eq!` on the full string, so amending the sentence **will**
require editing that assertion. Two other tests match loosely on
`contains("off-screen")` (`a_board_that_fits_says_nothing` `3688`,
`assert_no_silent_clip` ~`3640`) and will not need edits.

## 7. Key bindings — what is free, what is taken

Grep for `Char('h')`, `Char('l')`, `BareKey::Left`, `BareKey::Right` across
`lib.rs`: **zero hits**. All four are unbound today.

Currently bound in normal mode (`lib.rs:8763–8838`): `p`, `v`, space, `d`, `r`,
`j`/`Down`, `k`/`Up`, `D`, `q`.

Arrow keys are claimed in three earlier, higher-priority blocks — the modal
handlers (`8637`, `8656`) and the desk block (`8714`, `8721`) — all `Up`/`Down`
only, never `Left`/`Right`. The desk block is gated on
`view_preset == ui::ViewPreset::Present` and is documented as ordered ahead of
the global scroll branch on purpose. So handler **ordering matters**: a new pan
branch must sit where the desk still wins the keys it claims. Since the desk
claims no horizontal key, there is no live collision — but the ordering
convention is established and a new branch should respect it.

`ViewPreset` has four variants — `Operations`, `Present`, `Dag`, `Activity` —
and `next()` cycles them (pinned by `v_cycles_the_presets_in_the_old_order`,
`lib.rs:11123`).

## 8. The constraint AC3 imposes on where clamping happens

AC3: "Pan keys are inert in Operations/Activity/Present views and on any map
that fits (**no state changes**, verified)."

That is stricter than the vertical pattern. `j` increments `scroll_offset`
unconditionally and lets `print_dashboard` clamp — the state changes even when
the display cannot move. Read literally, AC3 forbids the same arrangement
horizontally: pressing `l` on a fitting map must leave the offset untouched, not
merely render identically.

The view half is easy — the handler can read `self.view_preset`. The "map that
fits" half is not, because whether the map fits depends on two things the key
handler does not have:

- **The pane width.** `cols` arrives only as an argument to
  `render(&mut self, rows, cols)` (`lib.rs:9440`); nothing stores it. Grep
  confirms no `last_cols`/`pane_cols` field on `State`.
- **The rendered width of the graph**, which today is computed inside
  `render_dag` and discarded — nothing escapes that function except the pushed
  lines.

So satisfying AC3 as written requires some value to survive from render time to
key time, or a measurement to be recomputed in the handler. `render` being
`&mut self` makes the former possible; `to_ui_state()` + `render_dag` being
pure-ish functions of state makes the latter possible. Which one, and what it
costs, is a design question.

## 9. Test surface and conventions

`ui.rs` carries ~101 `#[test]`; workspace total ~560. Relevant fixtures:

- `fan_board(n)` (`ui.rs:3242`) — n tickets `T-054-01-01..`, node 1 the root and
  every other depending on it. All `TicketStatus::Ready`, `Phase::Research`.
  Known widths: **119 columns full / 83 condensed at n=7**.
- `dag_body_lines(output)` (`ui.rs:3265`) — the graph rows only: `skip(2)` past
  header and blank, `take_while` not-legend and not-`(`-prefixed, strip ANSI,
  drop blanks. Every AC that says "node text" is asserted through this.
- `strip_ansi` (`ui.rs:4838`) — test-local, same escape walk as `visible_width`.
- `DAG_WIDE: usize = 1000` (`ui.rs:2211`) — the "no constraint" width used by
  the twelve pre-existing `render_dag` call sites.
- `assert_no_silent_clip(output, pane_cols)` (~`3640`) — the AC4 invariant
  helper: any body line wider than the pane implies the indicator is present.

`fan_board` is all-`Ready`, so **every node inks the same color**. AC2's
"fully colored board" walk wants status variety — no existing fixture provides
it. `sample_state()` has mixed statuses but only three tickets.

Key-handling tests live in `lib.rs` behind a `press(&mut state, key)` helper and
`desk_state_from(&[...])`, which build a real `State` over a temp dir.

## 10. Constraints and assumptions carried into design

- **ascii-dag owns layout** (N4). Panning is a viewport over the rendered
  string; no re-layout, no follow-the-cursor, no minimap (story's honest
  boundary).
- **`pane_cols == 0` means "caller does not know the pane"** — established by
  T-054-01-02, which never condenses and never indicates in that case. Panning
  inherits the question of what zero means.
- The condense decision and the indicator share one predicate
  (`pane_cols > 0 && widest > pane_cols`); T-054-01-02's review calls that
  single predicate "the whole safety argument." Panning adds a third reader of
  the same fact.
- **No new config** (story boundary). Bindings mirror the vertical ones.
- Full-mode boards that fit must stay byte-identical — pinned by
  `dag_wide_pane_keeps_full_labels_byte_for_byte`.
- `just check` = `check-wasm` (cargo check on `wasm32-wasip1`) + `fmt-check` +
  `lint` + `cargo test --workspace`. Judge by **exit code**, never grepped
  output.
