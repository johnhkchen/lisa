# Research — T-054-01-02 shed-ceremony-first

## What the ticket touches

One file: `crates/lisa-plugin/src/ui.rs` (4420 lines). Three call sites inside
it, one caller outside it (`lib.rs:9452`). No other crate renders the DAG.

## The width pipeline as it stands today

Zellij hands the plugin the real pane size:

```
lib.rs:9440  fn render(&mut self, rows: usize, cols: usize)
lib.rs:9452  ui::print_dashboard(&ui_state, rows, cols, self.scroll_offset)
```

`cols` here is the true pane width. It survives exactly one hop:

```
ui.rs:2022  pub fn print_dashboard(state, rows, cols, scroll_offset)
ui.rs:2031      let lines = render_dashboard_lines(state, cols.min(100), rows);
```

`cols.min(100)` is the clamp the ticket names (it cites ~1669; it now sits at
2031 after the E-053 train). Every renderer downstream sees the clamped number
and calls it `width`. The true `cols` is discarded at this line and is not
recoverable further in.

`render_dashboard_lines` (1494) uses `width` for the title separator (1503) and
passes it to three of four presets:

```
ui.rs:1506  ViewPreset::Operations => render_operations_view(state, width, height, output)
ui.rs:1507  ViewPreset::Present    => render_present_view(state, width, output)
ui.rs:1508  ViewPreset::Dag        => render_dag_view(state, output)          // <- no width
ui.rs:1509  ViewPreset::Activity   => render_activity_view(state, height, output)
```

`render_dag_view` (1544) is a one-line passthrough to `render_dag` (959), which
takes `(state, output)` and never learns how wide anything is. Vertical space is
clamped once, at the very end, by `print_dashboard`'s `.take(rows)` (2037).
Horizontally there is no clamp at all: lines longer than the pane are emitted in
full and the terminal decides what happens past the edge. That is the silent
clip the ticket retires.

`print_dashboard` is `pub` and called from exactly one place. `render_dashboard_lines`
is private and called from `print_dashboard` plus ten tests, all of which pass
`80` as the width.

## How the DAG is built and rendered

`render_dag` (959–1095) runs six steps:

1. Header `≡≡ DAG ≡≡` plus a blank line (960–961).
2. Early returns for "no tickets" (963) and "all done" (977).
3. Filter out `TicketStatus::Done`; count the hidden ones (969–975).
4. Build labels and integer ids. The label is one `format!`:

   ```rust
   ui.rs:998   let label = format!("{} {}", t.id, t.status.token());
   ```

   `id_to_int` is a 1-based index map (988–992); `edges` keeps only edges whose
   parent is also active (1003–1013).
5. Hand both to the layout owner and render:

   ```rust
   ui.rs:1021  let dag = ascii_dag::DAG::from_edges(&node_refs, &edges);
   ui.rs:1022  let rendered = dag.render();
   ```
6. Post-process each rendered line to inject color, then push it (1047–1068).
   Summary line for hidden done tickets (1071–1080) and a `Phases:` legend
   (1082–1094) close the view.

## The ink layer T-054-01-01 left behind

`NodeInk` (944–951) carries five borrowed strs per ticket: `label`, `ticket_id`,
`token`, `phase_color`, `status_color`. The post-processing loop keys on the
**whole label**, replacing `T-054-01-01 WRK` with the id in its phase color and
the token in its status color, and falls back to id-only coloring when a line
carries an id without its label (1058–1065). Research for that ticket found no
line that takes the fallback with today's ascii-dag.

The two mappings the ticket needs already exist as named methods:

```rust
ui.rs:60   TicketStatus::token()       -> RDY WRK REV BLK DON
ui.rs:74   TicketStatus::color_code()  -> CYAN GREEN BRIGHT_YELLOW RED BRIGHT_GREEN
ui.rs:128  Phase::color_code()         -> DIM CYAN MAGENTA YELLOW BLUE GREEN BRIGHT_YELLOW BRIGHT_GREEN
```

`Done` is unreachable in the DAG (filtered at step 3), which is why its color is
asserted at the mapping rather than on output (`test_done_status_color_is_bright_green`,
2969).

Coloring happens strictly **after** `render()`. Any measurement taken on
`rendered` is therefore pre-color by construction — the ordering, not a stripping
pass, is what makes AC3 easy to hold.

## What ascii-dag actually produces

Measured directly against `ascii-dag 0.8` (the pinned dependency,
`crates/lisa-plugin/Cargo.toml:19`) in a scratch binary:

```
chain-3-full        [T-002 WRK] → [T-003 BLK] → [T-004 RDY]        width  39
chain-3-condensed   [002] → [003] → [004]                          width  21
fan-6-full          root + 5 children, ids T-054-01-NN RDY         width  99
fan-6-condensed     same graph, labels 054-01-NN                   width  69
field-12-full       12 roots, ids T-0NN-01-01 WRK                  width 239
field-12-condensed  same graph, labels 0NN-01-01                   width 167
layer2-9-full       root + 8 children, T-100-02-NN RDY             width 159
```

Facts this establishes:

- **Layout scales with label width.** ascii-dag lays nodes out at
  `[label]` plus a fixed gap, so shortening labels genuinely narrows the board;
  the saving is not absorbed by padding.
- **The saving is exactly six columns per node.** 99→69 across 5 nodes and
  239→167 across 12 nodes are both `6 × nodes` to the column. `T-` is 2 and
  ` RDY` is 4. The ticket's "six columns per node" is arithmetic, not a guess.
- **Brackets are ascii-dag's.** `[` and `]` are emitted by the renderer around
  whatever label string it was given. We own the label, not the chrome.
- **Lines contain non-ASCII single-column glyphs** — `→ ┌ ─ ↓ └ ┐`. Byte length
  would badly overcount; a character count is correct, and no double-width
  glyph appears in the vocabulary.
- **`render()` emits trailing blank lines** (the 6-node fan reports 6 rows, two
  of them empty). Blank lines measure zero and are harmless to a max.

## Existing tests that constrain the change

`render_dag(&state, &mut output)` is called from twelve tests: 2342, 2352, 2952,
2982, 3024, 3043, 3079, 3105, 3152, 3218, 3281, 3331. Any signature change
touches all twelve.

Three matter to the design:

- **`test_dag_raw_content_unchanged_by_coloring` (3037).** Rebuilds the same
  graph through ascii-dag directly with `format!("{} {}", id, token)` labels and
  asserts the ANSI-stripped view reproduces it byte for byte. This is a live
  pin on full-label output: it fails the moment full mode renders anything but
  today's labels. AC1's "byte-identical to today" already has a guard here.
- **`test_dag_status_tokens_are_colored` (2948)** and
  **`test_dag_ticket_ids_keep_phase_color` (2978)** pin both ink channels on the
  full view.
- **The four-preset test at 3566** calls `render_dashboard_lines(&dag_state, 80, 50)`
  and asserts the DAG output `contains("T-002")`. A 3-node chain measures 39
  columns, so it stays in full labels at 80 — but this test is the tripwire that
  catches an over-eager condense threshold.

`strip_ansi` (4196) exists **inside `mod tests`** only. There is no production
helper that measures or strips escapes anywhere in the crate; `visible_width` in
the sense the ticket needs does not exist yet.

## Constraints and assumptions

1. **The clamp must keep governing the other presets.** Operations, Present and
   Activity lay out against `min(cols, 100)` today and must continue to. Only
   the DAG's fit decision may see past it (AC5).
2. **Full mode is frozen.** Condensing triggers on overflow only; a board that
   fits renders exactly as it does today, bytes included (AC1).
3. **ascii-dag stays the layout owner.** The invocation may run twice, but its
   shape may not change and no glyph may be hand-placed (AC6, N4).
4. **Status must survive the token's removal.** Once ` RDY` is gone, color is
   the only channel left carrying status — which is precisely why T-054-01-01
   landed first. Its design note says so explicitly: the token becomes "a
   redundant label rather than the only signal."
5. **Phase and status share one glyph in condensed mode.** The id is the only
   thing left to paint, and it currently carries phase. Something must give;
   which one is Design's call, and the `Phases:` legend at 1082 is downstream of
   that choice.
6. **`cols == 0` is reachable in tests and conceivable from a degenerate pane.**
   A zero width must mean "no constraint", not "condense everything".
7. **Id shortening must stay injective.** Every id on a Lisa board starts `T-`;
   dropping that prefix cannot make two ids collide. A missing prefix must
   leave the id untouched rather than eat a character.
8. **The ticket's worked example and its stated rule disagree by one character.**
   The rule is "`T-` prefix and status token shed" (six columns, twice stated);
   the arrow shows `[T-015-04-02 RDY]` → `[15-04-02]`, which also drops a
   leading zero (seven columns). Design must pick one and say which.

## Out of scope, noted

- Horizontal panning and the keys the indicator line will eventually name
  (S-054-02).
- Epic-prefix elision on single-epic boards — a design option to record, not
  build.
- The latent prefix-collision hazard in the ink loop (a shorter id matching
  inside a longer one), narrowed but not closed by T-054-01-01 and left alone
  here.
- The `Phases:` legend's glyph vocabulary, which appears in no DAG node today.
