# Structure — T-054-01-01 ink-the-status

## Files touched

| File | Change |
| --- | --- |
| `crates/lisa-plugin/src/ui.rs` | modified — one `impl` block added, two inline matches replaced by calls, one post-process loop rewritten, five tests added |

Nothing created, nothing deleted. No other crate, no `Cargo.toml`, no public API
outside the plugin crate.

## Change 1 — `impl TicketStatus` (new, after the enum at ui.rs:45–56)

```rust
impl TicketStatus {
    /// The three-letter token shown on a DAG node.
    pub fn token(&self) -> &'static str {
        Ready => "RDY", InProgress => "WRK", WaitingReview => "REV",
        Blocked => "BLK", Done => "DON"
    }

    /// ANSI color for the status token, matching `Phase::color_code`'s shape.
    pub fn color_code(&self) -> &'static str {
        Ready => CYAN, InProgress => GREEN, WaitingReview => BRIGHT_YELLOW,
        Blocked => RED, Done => BRIGHT_GREEN
    }
}
```

Boundary: both are pure `&self -> &'static str`, no allocation, no state. They
carry the *same* mappings that exist today at ui.rs:957–963 and ui.rs:994–1000 —
byte-for-byte the same strings, so no rendered output can move. Placed
immediately below the enum, mirroring how `Phase`'s methods sit below `Phase`
(ui.rs:101–125). `pub` to match `Phase::color_code`, which is `pub` in the same
private-to-crate module.

The Done arms exist on both methods even though the renderer filters Done out.
They are the enum's business, not the renderer's, and `Done => BRIGHT_GREEN` is
the arm AC1 asks to assert directly.

## Change 2 — node label build (ui.rs:954–967)

The inline `let status_str = match &t.status { ... }` collapses:

```rust
let label = format!("{} {}", t.id, t.status.token());
```

The formatted string is unchanged, so ascii-dag receives identical input and
returns an identical layout. This is what makes AC3 checkable rather than merely
asserted: nothing upstream of `render()` changed.

## Change 3 — color map (ui.rs:990–1003)

The map gains the two pieces the post-processor needs and loses its anonymous
match. Value shape moves from `(&str, &str)` to a named struct — a 4-tuple of
`&str`s at a use site three lines away would be a positional puzzle:

```rust
/// What the post-processor needs to ink one node's label.
struct NodeInk<'a> {
    label: &'a str,        // "T-054-01-01 WRK", the exact rendered substring
    ticket_id: &'a str,
    token: &'static str,
    phase_color: &'static str,
    status_color: &'static str,
}
```

Declared file-locally next to `render_dag`. `label` borrows from the `nodes` vec
built at ui.rs:954–967, which outlives the post-process loop — so the map is
built from `nodes` (which owns the label `String`s) zipped with `active`, not
from `active` alone. This is the one ownership subtlety in the change: the label
must be the *same* string that was handed to ascii-dag, not a re-`format!` of it,
so the two cannot drift.

Keying stays `HashMap<&str, NodeInk>` by ticket id — unchanged, and the id key
is still what the fallback path needs.

## Change 4 — post-process loop (ui.rs:1005–1015)

```rust
for line in rendered.lines() {
    let mut colored_line = line.to_string();
    for ink in color_map.values() {
        if colored_line.contains(ink.label) {
            // Whole label: id in phase color, token in status color.
            colored_line = colored_line.replace(ink.label, &format!(
                "{}{}{} {}{}{}",
                ink.phase_color, ink.ticket_id, RESET,
                ink.status_color, ink.token, RESET,
            ));
        } else if colored_line.contains(ink.ticket_id) {
            // Fallback: a line carrying the id without its label.
            colored_line = colored_line.replace(ink.ticket_id, &format!(
                "{}{}{}", ink.phase_color, ink.ticket_id, RESET));
        }
    }
    output.push(colored_line);
}
```

The `else if` ordering is load-bearing: label first, id second. Reversed, the
id pass would insert SGR bytes *inside* the label and destroy the label match.
The `_status_color` binding that named the bug disappears.

Iteration moves from `for (id, (a, b)) in &map` to `for ink in map.values()`
since the id now lives in the value. Order remains HashMap-arbitrary, which is
fine: after a label is replaced, every other ticket's label is still intact and
contiguous elsewhere on the line (Research: replacements only insert around a
match, never inside a sibling).

## Change 5 — tests (append to the existing `tests` module)

Five tests, placed with the other `test_render_dag_*` tests at ui.rs:2857+:

| Test | Asserts |
| --- | --- |
| `test_dag_status_tokens_are_colored` | one board carrying RDY/WRK/REV/BLK; each token appears as `{color}{token}{RESET}` — four assertions, one per status (AC1) |
| `test_done_status_color_is_bright_green` | `TicketStatus::Done.color_code() == BRIGHT_GREEN` and `.token() == "DON"`, no renderer (AC1, Done arm) |
| `test_dag_ticket_ids_keep_phase_color` | id still wrapped in `Phase::color_code()` across differing phases (AC2) |
| `test_dag_raw_content_unchanged_by_coloring` | `strip_ansi` of every rendered line equals the uncolored line content (AC3) |
| `test_dag_status_color_independent_of_phase` | two tickets, same status, different phases → same status color; guards the token against inheriting phase color |

AC3's test needs the pre-color baseline. Rather than freezing a golden string
(brittle against an ascii-dag bump, and it would fail for layout reasons while
claiming a color regression), assert the invariant directly: for each output
line, `strip_ansi(line)` must contain no escape bytes and must equal the line
with escapes removed by construction — i.e. compare `strip_ansi` of the colored
render against a render whose label substitution is the identity. Concretely:
build the same DAG, and assert `strip_ansi(colored)` reproduces `{id} {token}`
labels with single spaces and no stray characters, plus total visible width per
line unchanged from the ascii-dag `render()` this test recomputes locally.

## Ordering

1. Change 1 (methods) — compiles standalone, changes no behavior.
2. Changes 2–3 (call the methods, widen the map) — still no output change;
   `_status_color` becomes an unused field, which clippy will flag, so 4 lands
   with it.
3. Change 4 (paint the token) — the behavior change.
4. Change 5 (tests).

Steps 1–3 are one commit (a no-op refactor is not independently meaningful) and
4–5 are the second. Both must leave `just check` green.

## Not in scope

- The HashMap prefix-collision hazard (Research). Label-keyed matching removes
  its reachable path; the id fallback still carries it. Documented, not fixed.
- Dropping the token entirely / condensed mode — that is T-054-01-02.
- `Phase`, the legend, the status line, health alerts, the desk.
