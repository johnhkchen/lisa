# Plan — T-054-01-01 ink-the-status

Two commits. The first moves mappings onto the enum and changes no output; the
second paints the token and proves it. Every step ends `just check` green.

## Step 1 — `TicketStatus::token()` and `TicketStatus::color_code()`

**Edit:** `crates/lisa-plugin/src/ui.rs`, new `impl TicketStatus` block below the
enum (ui.rs:45–56).

Copy the arms verbatim from ui.rs:957–963 (tokens) and ui.rs:994–1000 (colors) —
verbatim matters; a "tidied" string here is a silent layout change.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`. Dead-code
warnings expected until Step 2 uses them.

## Step 2 — call the methods from `render_dag`

**Edit:** ui.rs:954–967, label build → `t.status.token()`.
**Edit:** ui.rs:990–1003, color map value → `NodeInk` carrying `label`,
`ticket_id`, `token`, `phase_color`, `status_color`; built by zipping `nodes`
(which owns the label `String`s) with `active` so `label` borrows the exact
string handed to ascii-dag.

The post-process loop still reads only `phase_color` + `ticket_id` at this point,
so `token`/`status_color` are momentarily unread — expect clippy `dead_code`.
Step 3 consumes them. If clippy's noise is disruptive, fold Steps 2 and 3; they
are one commit regardless.

**Verify:** `cargo test --workspace`. Every existing test must pass *untouched* —
this is the proof that Steps 1–2 are output-neutral. If any existing test moves
here, the refactor changed a string and must be corrected, not the test.

## Step 3 — paint the token

**Edit:** ui.rs:1005–1015, post-process loop per structure.md Change 4. Label
branch first, id fallback in `else if`.

**Verify:** `cargo test --workspace`. Existing DAG tests assert `contains("T-002")`
against bare ids, which survive coloring, so they should stay green. Any failure
here is real signal, not test brittleness — investigate before touching a test.

**Commit A** (Steps 1–3):
`lisa commit-ticket --ticket-id T-054-01-01 --message "..." --include crates/lisa-plugin/src/ui.rs`

## Step 4 — tests

**Edit:** append to the `tests` module beside `test_render_dag_*` (ui.rs:2857+).

Board fixture: four tickets, one per renderable status, phases chosen to differ
from the status colors so the two channels can't be confused —

| id | status | token | status color | phase | phase color |
| --- | --- | --- | --- | --- | --- |
| T-901 | Ready | RDY | CYAN | Ready | DIM |
| T-902 | InProgress | WRK | GREEN | Design | MAGENTA |
| T-903 | WaitingReview | REV | BRIGHT_YELLOW | Structure | YELLOW |
| T-904 | Blocked | BLK | RED | Plan | BLUE |

Ids are 5 chars and share no prefix, so the sample cannot accidentally exercise
the prefix-collision hazard and mask a real result.

1. **`test_dag_status_tokens_are_colored`** — render, join, assert
   `contains(&format!("{CYAN}RDY{RESET}"))` and the three siblings. (AC1)
2. **`test_done_status_color_is_bright_green`** — pure mapping assertion:
   `TicketStatus::Done.color_code() == BRIGHT_GREEN`, `.token() == "DON"`, with a
   comment recording *why* it can't be a rendering test (Done is filtered at
   ui.rs:928–932). (AC1, Done arm)
3. **`test_dag_ticket_ids_keep_phase_color`** — assert
   `contains(&format!("{MAGENTA}T-902{RESET}"))` etc., one per phase in the
   fixture. (AC2)
4. **`test_dag_raw_content_unchanged_by_coloring`** — for every output line,
   `strip_ansi(line)` contains no `\x1b`, and the stripped body reproduces
   `T-902 WRK` with exactly one space. Then assert the per-line *visible* width
   equals the width of the corresponding `ascii_dag` line the test rebuilds from
   the same nodes/edges — proving insertion-only. (AC3)
5. **`test_dag_status_color_independent_of_phase`** — two Blocked tickets in
   different phases both yield `{RED}BLK{RESET}`; their ids differ in color.

**Verify:** `cargo test --workspace`.

## Step 5 — full gate

`just check` — `check-wasm`, `fmt-check`, clippy, `cargo test --workspace`.
Judge by **exit code**, not by grepping output.

**Commit B** (Steps 4–5): same `commit-ticket` form, `--include
crates/lisa-plugin/src/ui.rs`.

## Testing strategy

Unit tests only, in the existing `#[cfg(test)] mod tests` in ui.rs — matching how
every other renderer in this file is tested. No integration test: `render_dag` is
private, writes into a `Vec<String>`, and has no I/O. No snapshot/golden test:
it would couple this ticket to ascii-dag's exact glyph layout and fail loudly on
a dependency bump for reasons unrelated to color.

Coverage maps 1:1 to acceptance criteria — AC1 by tests 1+2, AC2 by test 3, AC3
by test 4, AC4 by Step 5's exit code. Test 5 is not required by any criterion; it
guards the specific way this change could be wrong-but-passing (token inheriting
phase color) and is cheap.

## Risks

| Risk | Signal | Response |
| --- | --- | --- |
| Refactor perturbs a rendered string | an existing test fails at Step 2 | fix the code; the arms must be verbatim copies |
| Fallback branch is dead in practice | no test covers `else if` | accept — it is documented insurance (design.md), not a live path |
| `NodeInk` borrow of `nodes` fights the borrow checker | Step 2 won't compile | `nodes` outlives the loop; if it resists, hold `label: String` — costs one clone per node, at DAG scale irrelevant |
| Test 4's width comparison duplicates render logic | test is long or fragile | if rebuilding the DAG in-test proves awkward, drop to the escape-free + single-space assertion, which still covers AC3's substance |

## Scope discipline

Touch one file. Do not fix the prefix-collision hazard. Do not drop the token —
that is T-054-01-02, and this ticket exists to land alone so that one has a
working status channel to spend.
