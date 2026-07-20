# Design — T-054-01-01 ink-the-status

## The decision in one line

Replace the whole label — `{id} {token}` — in one pass, wrapping each half in its
own color, and give `TicketStatus` two named methods (`token()`, `color_code()`)
so the unreachable Done arm becomes assertable without rendering.

## What has to be true

From Research, four constraints bound the solution space:

1. Only SGR bytes may be inserted; raw content stays byte-identical (AC3).
2. A `DON` token never reaches a rendered line, so the Done arm needs a
   non-rendering test seam (AC1).
3. Labels are contiguous in the rendered output, and several share a line.
4. Phase coloring of the id must not change (AC2).

## Options

### Option A — token-keyed replace

After the existing id pass, run a second pass per line:

```rust
for (token, color) in [("RDY", CYAN), ("WRK", GREEN), ("REV", BRIGHT_YELLOW), ("BLK", RED)] {
    colored_line = colored_line.replace(token, &format!("{color}{token}{RESET}"));
}
```

**For:** smallest diff; correct output, since color is a pure function of the
token (Research). Needs no knowledge of which ticket owns which token.

**Against:** it matches the shortest, least specific thing on the line. Three
uppercase letters is a weak key — it is correct only because DAG body text
happens to contain nothing else, an invariant that lives in ascii-dag's output
and in the legend's wording, neither of which this file controls. The moment a
label gains a title, or the legend gains a token, it silently miscolors. It also
hardcodes a second copy of the status→color mapping *and* a second copy of the
status→token list, making the ui.rs:957/ui.rs:994 duplication a threesome. And
it leaves the Done arm exactly as untestable as it is now.

### Option B — label-keyed replace (chosen)

Fold the status into the existing single pass. Key on the full label:

```rust
for (ticket_id, (label, phase_color, status_color, token)) in &color_map {
    colored_line = colored_line.replace(
        label,
        &format!("{phase_color}{ticket_id}{RESET} {status_color}{token}{RESET}"),
    );
}
```

**For:**

- **Precise key.** `T-054-01 REV` cannot match inside `T-054-01-01 WRK`; the
  label is longer and more specific than either half. This strictly narrows the
  pre-existing prefix-collision hazard noted in Research rather than widening it.
- **One pass, one mapping.** The status color and the status token come from the
  same per-ticket record that already exists; no second traversal of the line, no
  second copy of the mapping.
- **Byte-identical raw content by construction.** The replacement is the original
  label with SGR escapes inserted at three points — strip the escapes and you get
  the input back, character for character.
- **The ticket owns its color.** Reads the way the bug's fix should read: the
  status color that was computed for this ticket is applied to this ticket's
  token, instead of being dropped on the floor.

**Against:** needs a fallback for a line that carries an id without its label.
Research found no such line, but the code should not become strictly less
capable than what it replaces.

### Option C — pass colors into ascii-dag

`ascii-dag` 0.8 exposes `render_scanline_colored(palette)` and
`render_to_buffer_colored`. Color the nodes at the source instead of
post-processing.

**Against:** rejected outright. It swaps the renderer entry point (`render()` →
a scanline variant), which changes glyphs and layout — a direct AC3 violation and
a much larger blast radius than "paint one token." The palette API colors whole
nodes, not label halves, so it cannot express phase-on-id + status-on-token at
all. Wrong tool for a color-only ticket that lands before T-054-01-02.

## Chosen: Option B, plus a named mapping

Option B fixes the rendering. It does not by itself fix testability of the Done
arm, which AC1 explicitly calls for. That needs the mapping to have a name.

Give `TicketStatus` the shape `Phase` already has (ui.rs:101–125 — `indicator()`
and `color_code()` as methods):

```rust
impl TicketStatus {
    pub fn token(&self) -> &'static str { ... }      // RDY WRK REV BLK DON
    pub fn color_code(&self) -> &'static str { ... } // CYAN GREEN BRIGHT_YELLOW RED BRIGHT_GREEN
}
```

Then ui.rs:957–963 and ui.rs:994–1000 both collapse into calls, the two matches
that had to stay in lockstep become one arm each, and `TicketStatus::Done
.color_code() == BRIGHT_GREEN` is a one-line test that never touches the
renderer — which is precisely the "asserted at the color-map level" that AC1
asks for. Neither method changes any emitted string, so AC3 is untouched.

This is a refactor, and refactors on a small ticket deserve suspicion. It earns
its place on three counts: AC1 cannot be satisfied without *some* seam; the
method pair is the shape the sibling enum in the same file already uses, so it
adds no new concept; and it removes duplication rather than adding indirection.

## Fallback behavior

When a line contains a ticket id but not its full label, fall back to the current
id-only replacement. This keeps the change strictly additive: every line colored
before is still colored at least as well now. Research found no line that takes
this path with today's ascii-dag, so it is insurance against a renderer change,
not a live path — and it is worth one branch rather than a future silent
regression to uncolored ids.

## Rejected refinements

- **Regex on the token.** Adds a dependency to a WASM plugin to solve a problem
  exact-substring matching already solves precisely.
- **Sorting the color map by descending id length** to fully fix the prefix
  hazard. It is a real latent bug but not this ticket's — and label-keyed
  matching already removes its reachable path. Left for whoever owns it, noted
  in Research and Review.
- **Coloring the surrounding brackets.** `[` and `]` are ascii-dag's box glyphs,
  not the ticket's; painting them would blur node boundaries into status noise
  and spend legibility budget T-054-01-02 needs.

## Consequences

Blocked-versus-working becomes a red-versus-green scan. T-054-01-02 inherits a
status channel that carries color, so when it drops the token it is removing a
redundant label rather than removing the only signal. `TicketStatus` arrives
there with the methods that ticket will need to move status entirely into color.
