# Research — T-054-01-01 ink-the-status

## Scope

`render_dag` computes a status color per node and then throws it away. Map the
code that builds DAG lines, the coloring pass that drops the status color, the
rendering the post-processor actually operates on, and the tests that would move.

## Where the code lives

Single file: `crates/lisa-plugin/src/ui.rs` (4201 lines). Nothing outside it
renders the DAG.

| Concern | Location |
| --- | --- |
| ANSI constants (`RESET`, `CYAN`, `GREEN`, `RED`, `BRIGHT_YELLOW`, `BRIGHT_GREEN`, `DIM`) | ui.rs:21–32, module `colors` |
| `Phase::color_code` | ui.rs:101–112 |
| `TicketStatus` enum (plugin-local, 5 variants) | ui.rs:45–56 |
| `render_dag` | ui.rs:918–1042 |
| Done filter | ui.rs:928–932 |
| Node label build (status token match) | ui.rs:954–967 |
| Color map build (status color match) | ui.rs:990–1003 |
| Post-process loop that discards the status color | ui.rs:1005–1015 |
| `render_dag_view` (only caller) | ui.rs:1491–1492 |
| Existing DAG tests | ui.rs:2286–2299, 2857–3120 |
| Test helper `strip_ansi` | ui.rs:3977–3992 |

The ticket cites ~816–828 / ~833 / ~752; the file has since shifted by roughly
+170 lines (health-alert rendering grew above it). The cited structures are all
present, just lower. Line numbers below are the current ones.

## How `render_dag` works today

1. **Filter** (ui.rs:928–932). Every `TicketStatus::Done` ticket is dropped
   before nodes are built; `done_count` becomes a dim "(N done tickets hidden)"
   footer. This is the load-bearing fact behind the ticket's Done-arm note: a
   `DON` token is computed at ui.rs:962 but can never reach a rendered line.
2. **Label build** (ui.rs:954–967). Each active ticket becomes
   `format!("{} {}", t.id, status_str)` — e.g. `T-054-01-01 WRK` — paired with a
   1-based integer id for ascii-dag.
3. **Edges** (ui.rs:969–979), restricted to deps that are themselves active.
4. **Render** (ui.rs:987–988). `ascii_dag::DAG::from_edges(...).render()`
   returns one `String`; the plugin only ever sees its `.lines()`.
5. **Color map** (ui.rs:990–1003). `HashMap<&str, (&str, &str)>` keyed by ticket
   id, holding `(phase_color, status_color)`. Status colors: Ready→`CYAN`,
   InProgress→`GREEN`, WaitingReview→`BRIGHT_YELLOW`, Blocked→`RED`,
   Done→`BRIGHT_GREEN`.
6. **Post-process** (ui.rs:1005–1015). For each rendered line, for each map
   entry, `line.replace(ticket_id, "{phase_color}{ticket_id}{RESET}")`. The
   destructuring is `(phase_color, _status_color)` — the second element is bound
   and immediately discarded. This is the bug.

## What the renderer actually emits

Empirically dumped (temporary test, since reverted) for four tickets — WRK/BLK
in one chain, REV branching, an unrelated RDY root. `^` marks `\x1b`:

```
RAW[^[1m^[36m≡≡ DAG ≡≡^[0m]
RAW[]
RAW[   [^[36mT-054-01-01^[0m WRK]   [^[2mT-055^[0m RDY]]
RAW[          ┌└─────────────────┐]
RAW[          ↓                  ↓]
RAW[  [^[2mT-054-01-02^[0m BLK]   [^[93mT-054-02^[0m REV]]
RAW[]
RAW[^[2mPhases: ○ Rdy ◐ Res ◑ Des ◒ Str ◓ Pln ● Imp ◎ Rev ✓ Don^[0m]
```

Four facts that constrain any fix:

- **Labels stay contiguous.** ascii-dag renders `[{id} {status}]` inline; it does
  not wrap, truncate, or split a label across lines. `render()` takes no width
  argument, so there is no narrow-terminal path that fragments a label.
- **Multiple nodes share a line.** Sibling nodes in the same layer render
  side-by-side, so a single line can carry several ids and several status tokens.
- **The id is already wrapped** in phase color + `RESET` by the time any second
  pass would run, and the inserted SGR bytes sit *before* the id and *after* it —
  never between the id and its status token.
- **Only labels and box-drawing characters exist** in the rendered body. The
  literals `RDY`/`WRK`/`REV`/`BLK` appear nowhere else in DAG output; the header
  is `≡≡ DAG ≡≡` and the legend uses phase names (`Rdy`, `Rev`), not tokens.

## Constraints and assumptions

- **Byte-identical raw content.** AC3 forbids layout drift, so the fix may only
  *insert* SGR escapes. Any approach that reflows, pads, or re-lays-out the label
  is out.
- **Color is a pure function of the token.** `RDY` always maps to `CYAN`, `WRK`
  to `GREEN`, `REV` to `BRIGHT_YELLOW`, `BLK` to `RED` — independent of which
  ticket carries it. So a token-keyed and a ticket-keyed approach agree on
  output; they differ only in matching precision.
- **The Done arm is unreachable through the renderer.** Any test asserting
  Done→`BRIGHT_GREEN` has to reach the mapping directly, not through
  `render_dag`. Today the mapping is an anonymous `match` inside a closure inside
  `render_dag` — not addressable from a test. Making the Done arm assertable
  therefore requires giving the mapping a name.
- **`strip_ansi` already exists** in the test module (ui.rs:3977) and is the
  natural tool for the "raw content unchanged" assertion.
- **Existing DAG tests assert on `output.join("\n")` with `contains`** against
  bare ids (`full.contains("T-002")`) and box-drawing characters. Bare ids still
  appear in the colored output, so id-based assertions are unaffected by adding
  status color. No existing test asserts on a status token.

## Pre-existing hazard, noted not owned

The id replacement at ui.rs:1010 is a plain substring `replace` over an
unordered `HashMap`. If a board ever held both `T-054-01` and `T-054-01-01`, the
shorter id could match inside the longer one and corrupt it, with the outcome
depending on HashMap iteration order. Not reachable in the sample boards, not in
this ticket's acceptance criteria, and out of scope — but it is the reason to
prefer a matching key that is longer and more specific rather than shorter.

## Duplication worth knowing about

The status→token match (ui.rs:957–963) and the status→color match
(ui.rs:994–1000) are two separate `match` expressions over the same enum, forty
lines apart, that must stay in lockstep. Adding a `TicketStatus` variant today
requires remembering both. `Phase` already models the better shape: `indicator()`
and `color_code()` are named methods on the enum (ui.rs:101–125), each
independently callable and testable. `TicketStatus` has no methods at all.

## Verification surface

`just check` = `check-wasm` (`cargo check -p lisa-plugin --target wasm32-wasip1`)
→ `fmt-check` → `lint` (clippy) → `cargo test --workspace`. 537 tests currently
in the plugin crate's `ui` module alone. Tests run on the native target; the WASM
target is type-checked only.
