# T-070-01-02 — the count of schedulers agrees with the list

## What the output looks like now

Same four records the `renderer` board had on 2026-08-14, replayed against a
built `lisa` binary in a scratch project:

```
$ lisa schedulers | head -8
1 of 4 schedulers on this board is running.

  renderer-3 (renderer-3-49ded6ab)  — running
    started 46m ago, last seen 88s ago, zellij server pid 15340
    stop it with: zellij kill-session renderer-3

  renderer-2 (renderer-2-93ea8bf4)
    started 5h ago, stopped stamping 53m ago, zellij server pid 71809
```

Three changes, all in the listing. Nothing about liveness, stopping, or the
per-line detail moved.

1. **The running one carries the word.** `— running` goes on the live line; the
   runs that ended lose the `— not running` label and are described by what they
   did, `stopped stamping 53m ago`. No line means anything by its absence any
   more, and the marker is on the one line that matters.
2. **The first line names both numbers.** `1 of 4 schedulers on this board is
   running` is true read alone. Every shape does the same: `All 3 schedulers on
   this board are running`, `No scheduler is running on this board. All 3 records
   below are runs that ended`. The one that stayed as it was is the ordinary
   single-run board — `1 scheduler is running on this board.`
3. **The live one is first.** Order is live before ended, then most recently
   stamped first, so what `head` cuts is the oldest history rather than the
   answer.

## Files

- `crates/lisa-cli/src/schedulers.rs` — `describe` marks live instead of dead;
  new `headline(live, total)` and `reading_order(...)`; `list` uses both. Six
  tests added or rewritten.
- `README.md` — three sentences in the `lisa schedulers` section describing the
  order, the marker, and the count. The paragraph already said `--stop` runs the
  stop command; it now also says it takes a run that has ended.

## Tests

`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
warnings`, and `cargo fmt --all -- --check` all exit 0.

New or changed, all in `crates/lisa-cli/src/schedulers.rs`:

- `the_first_eight_lines_answer_which_run_is_holding_the_board` — the ticket's
  own board, asserting on `head -8`: the count names both numbers, the live run
  and its detail and its stop command are inside the truncation, the four-hour
  history is not, and the ended runs that follow are ordered most recent first.
- `the_count_says_what_it_is_counting` — every shape of the first line, read with
  nothing under it.
- `a_dead_record_can_be_stopped_while_a_live_scheduler_holds_the_board` —
  `--stop` on an ended run while a live one is on the board: the ended record is
  forgotten, the live one is untouched.
- `three_schedulers_are_counted_named_and_each_given_its_stop_command`,
  `one_scheduler_is_listed_without_a_warning`, and
  `a_record_left_by_a_run_that_ended_is_shown_as_history` (renamed from
  `..._is_shown_as_not_running`) updated to the new wording, the last one now
  asserting that `— running` appears nowhere when nothing is running.

Reproduced by hand as well, against a temporary project seeded with the four
records above: `head -8` as printed at the top, and `lisa schedulers --stop
renderer-2-93ea8bf4` on an ended run answered `was not running any more … Lisa
forgot its record` with exit 0, after which the count read `1 of 3`.

## Decisions worth a reader's attention

- **The Notes ask whether `stopped stamping 4h ago` and `last seen 87s ago`
  should be one phrasing. Kept as two.** They now carry the distinction the label
  used to: `last seen` reads as an ongoing run, `stopped stamping` as one that
  ended, and each appears only on the kind of line it fits. Collapsing them would
  put the whole live/ended difference back on the `— running` marker alone.
- **Ended runs still print `stop it with: zellij kill-session <name>`.** For a
  session that is already gone that is a command that cannot succeed — which is
  `T-070-01-01`'s fourth criterion ("the remedy printed is a remedy that can
  work"), not this ticket's. I left the line untouched rather than edit the same
  function from two tickets at once.

## Concerns

- **`T-070-01-01` is in `implement` at the same time, on the same file, with no
  dependency edge between the two tickets.** Both change
  `crates/lisa-cli/src/schedulers.rs`; that ticket also changes liveness itself,
  which this listing reads through `is_live`. My commit landed with the file
  holding only my changes (`git status` was clean for it immediately before and
  after), and the two changes do not overlap in function — but whichever ticket
  commits second will be committing the file as it stands, including the other's
  work. Worth checking the resulting diff once both are done.
- **My change composes with whatever `is_live` becomes.** If `T-070-01-01` makes
  `is_live` also ask about `zellij_pid`, the count, the `— running` marker, and
  the ordering all follow from it without further edits. The one thing to look at
  is `reading_order`'s tiebreak: it sorts ended runs by `stamped_at`, and a run
  found dead by its pid while its stamp is fresh would sort to the top of the
  history. That is the right place for it — most recently stopped first — but it
  is a coupling neither ticket states.
