# Progress — T-054-01-01 ink-the-status

## Status: implementation complete, `just check` green (exit 0)

## Steps executed

| Step | Plan | Outcome |
| --- | --- | --- |
| 1 | `TicketStatus::token()` / `color_code()` | done — arms copied verbatim from ui.rs:957–963 and ui.rs:994–1000 |
| 2 | `render_dag` calls the methods; `NodeInk` map | done — map built by zipping `active` with `nodes` so `label` borrows the exact string given to ascii-dag |
| 3 | Paint the token in the post-process loop | done — label branch first, id fallback in `else if` |
| 4 | Five tests | done |
| 5 | `just check` | green, exit 0 |

## Deviations from the plan

**One commit, not two.** The plan split the refactor (Steps 1–3) from the tests
(Steps 4–5). Both live in `crates/lisa-plugin/src/ui.rs`, and `lisa
commit-ticket` selects by path, not by hunk — so a two-commit split would have
put the test code in the first commit anyway, labeled as something it wasn't.
Committed once with an honest message instead. The output-neutrality the split
was meant to demonstrate is recorded below and holds regardless.

**`Phase::name()` → `Phase::short_name()`.** The plan's test sketch called a
method that does not exist; the real accessor is `short_name()` (ui.rs:99).
Caught at compile time, one-line correction in a test assertion message.

**`else if` fallback kept.** Confirmed dead against today's ascii-dag — every
line carrying an id carries its full label. Kept per design.md as insurance
against a renderer change, so no line can silently regress to uncolored.

## Evidence

**Steps 1–2 are output-neutral.** After the refactor and before the token was
painted, `cargo test --workspace` passed **537/537 with no test file edited**.
That is the AC2 guarantee: phase coloring of the id is untouched, and existing
rendering tests stayed green *untouched* rather than being updated.

**The token is actually painted.** Rendered output before the change, `^` = `\x1b`:

```
   [^[36mT-054-01-01^[0m WRK]   [^[2mT-055^[0m RDY]
```

The tokens `WRK` and `RDY` carry no SGR bytes. After: each is wrapped in its
mapped color. Asserted by `test_dag_status_tokens_are_colored`, one assertion per
renderable status.

**Mutation check.** Swapped `ink.status_color` → `ink.phase_color` in the format
— i.e. the token inherits the phase channel, the plausible wrong version of this
fix. Result: `test_dag_status_tokens_are_colored` and
`test_dag_status_color_is_independent_of_phase` FAILED;
`test_dag_ticket_ids_keep_phase_color` and
`test_dag_raw_content_unchanged_by_coloring` passed. Exactly the tests that
should discriminate do, and the ones that shouldn't don't. Mutation reverted.

**AC3, no layout drift.** `test_dag_raw_content_unchanged_by_coloring` rebuilds
the graph through `ascii_dag` directly and asserts `strip_ansi` of the rendered
rows equals those raw lines byte for byte — not a width heuristic, an equality.

## Tests added (542 total, up from 537)

- `test_dag_status_tokens_are_colored` — RDY/WRK/REV/BLK each inked (AC1)
- `test_done_status_color_is_bright_green` — Done arm at the mapping, no
  renderer, with a comment recording why it cannot be a rendering test (AC1)
- `test_dag_ticket_ids_keep_phase_color` — four phases, ids unchanged (AC2)
- `test_dag_status_color_is_independent_of_phase` — two Blocked tickets in
  different phases both red; ids differ
- `test_dag_raw_content_unchanged_by_coloring` — byte-identical raw rows (AC3)

Fixture `four_status_state()` picks phases whose colors differ from the status
color beside them (Ready/DIM vs CYAN, Design/MAGENTA vs GREEN, Structure/YELLOW
vs BRIGHT_YELLOW, Plan/BLUE vs RED), so a token inheriting the phase channel
cannot pass by coincidence. Ids `T-901`–`T-904` share no prefix, so the sample
cannot accidentally exercise the prefix-collision hazard and mask a result.

## Not done, deliberately

The `HashMap` prefix-collision hazard (research.md): a board holding both
`T-054-01` and `T-054-01-01` could corrupt the longer id, order-dependent.
Label-keyed matching removed its reachable path — `T-054-01 REV` cannot match
inside `T-054-01-01 WRK` — but the id fallback still carries it. Out of scope,
carried into review.md.

## Files

`crates/lisa-plugin/src/ui.rs` — only file touched. +243 / −24.
