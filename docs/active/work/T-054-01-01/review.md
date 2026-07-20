# Review — T-054-01-01 ink-the-status

## What changed

One file, one commit.

- **Commit** `7f7720d` — *feat(plugin): ink the status token so blocked reads red at a glance*
- **File** `crates/lisa-plugin/src/ui.rs`, +243 / −24
- Nothing created, nothing deleted, no other crate, no dependency change, no
  public surface outside the plugin crate.

### The fix

`render_dag` computed a status color for every node and then destructured the
pair as `(phase_color, _status_color)`, dropping it. Only the ticket id was
painted; the status token rendered in whatever color the terminal had loaded.

The post-process loop now matches the **whole node label** — `T-054-01-01 WRK` —
and re-emits it as the id in its phase color plus the token in its status color.
Two channels, side by side: phase on the id, status on the token.

### Supporting change

`TicketStatus` gained `token()` and `color_code()`, mirroring the shape `Phase`
already uses in the same file (`short_name()` / `color_code()` / `indicator()`).
This replaced two `match` expressions over the same enum sitting forty lines
apart that had to stay in lockstep. It is also what makes the Done arm
assertable: `Done` is filtered out before nodes are built, so `Done =>
BRIGHT_GREEN` was unreachable through the renderer and untestable while it lived
anonymously inside a closure.

`NodeInk` — a five-field borrow struct — replaced the map's `(&str, &str)` value.
A 4-tuple of `&str` read at a use site three lines away would have been a
positional puzzle.

## Acceptance criteria

| AC | Status | Evidence |
| --- | --- | --- |
| Status tokens wrapped in mapped colors — RDY/WRK/REV/BLK, one test per status; Done at the color-map level | met | `test_dag_status_tokens_are_colored` (four assertions), `test_done_status_color_is_bright_green` |
| Phase coloring of the id unchanged; existing tests stay green **untouched** | met | 537/537 passed with zero test-file edits after the refactor and before the paint; `test_dag_ticket_ids_keep_phase_color` pins it forward |
| No layout drift — raw line content byte-identical | met | `test_dag_raw_content_unchanged_by_coloring` |
| `just check` green | met | exit code 0, re-confirmed on the committed tree |

Judged by exit code, not by reading output.

## Test coverage

542 tests, up from 537.

| Test | Guards |
| --- | --- |
| `test_dag_status_tokens_are_colored` | each renderable status token carries its color |
| `test_done_status_color_is_bright_green` | the Done arm, at the mapping — with a comment recording why it cannot be a rendering test |
| `test_dag_ticket_ids_keep_phase_color` | the phase channel across four phases |
| `test_dag_status_color_is_independent_of_phase` | the token not inheriting the phase channel |
| `test_dag_raw_content_unchanged_by_coloring` | insertion-only; rebuilds the graph through `ascii_dag` and asserts byte equality after `strip_ansi` |

The fixture pairs each status with a phase whose color differs from it
(Ready/DIM vs CYAN, Design/MAGENTA vs GREEN, Structure/YELLOW vs BRIGHT_YELLOW,
Plan/BLUE vs RED), so a token that silently sourced the phase color cannot pass
by coincidence.

**Mutation-checked.** Swapping `ink.status_color` → `ink.phase_color` — the
plausible wrong version of this fix — failed `test_dag_status_tokens_are_colored`
and `test_dag_status_color_is_independent_of_phase` while the phase-color and
raw-content tests correctly stayed green. The tests discriminate; they do not
merely execute the code. Mutation reverted before commit.

### Gaps

- **The `else if` fallback is untested and dead** against today's ascii-dag —
  every line carrying an id carries its full label. It exists so that a renderer
  change cannot silently regress ids to uncolored. Testing it would mean
  fabricating a rendering ascii-dag does not produce.
- **No test asserts the *absence* of color on non-label text.** The raw-content
  test covers the stronger property (content byte-identity), so a stray escape
  in the box-drawing region would have to survive both — but it is not directly
  named.
- **No visual/terminal test.** Consistent with the rest of this file; every
  renderer here is tested as strings.

## Open concerns

**Pre-existing prefix-collision hazard, not introduced and not fixed.** The id
fallback still does a plain substring `replace` over an unordered `HashMap`. A
board holding both `T-054-01` and `T-054-01-01` could have the shorter id match
inside the longer one, with the corruption depending on iteration order. This
change **narrows** it — the primary path now keys on the full label, and
`T-054-01 REV` cannot match inside `T-054-01-01 WRK` — but the fallback branch
still carries it. Out of scope here; worth its own ticket if nested ids of that
shape ever land on a board. Documented in research.md and progress.md.

**One commit rather than the planned two.** The plan split the refactor from the
tests; both live in the same file and `lisa commit-ticket` selects by path, not
by hunk, so the split would have shipped test code inside a commit labeled as a
refactor. Committed once, honestly. The output-neutrality the split was meant to
demonstrate is recorded as evidence in progress.md instead: 537/537 green with no
test edited, after the refactor and before the paint.

**Refactor on a small ticket.** Adding methods to `TicketStatus` is more than
"paint one token." It earns its place: AC1's Done arm cannot be asserted without
some seam, the method pair is the shape the sibling enum in the same file already
uses, and it removes duplication rather than adding indirection. Flagged because
a reviewer expecting a three-line diff should know why it is longer.

## For the next ticket

T-054-01-02 drops the token and moves status entirely into color. It inherits a
status channel that visibly works, so it is removing a redundant label rather
than removing the only signal — and `TicketStatus::color_code()` is already the
named seam it will need.

## Handoff

No TODOs, no dead code beyond the documented fallback, no follow-up required to
land this. Working tree carries no ticket-owned changes: the source commit is in,
and the only modified files are Lisa's own bookkeeping (`.lisa.toml`,
`.lisa/*.jsonl`), which this ticket does not own.
