# T-053-02-02 — Progress

Four commits, all five plan steps landed. `just check` exits 0.

| step | plan | commit | state |
|---|---|---|---|
| 1 | status line + module doc | `0ab7c7e` | done |
| 2 | `[p]` → desk, `[v]` → cycle, `enter_view` | `7d71315` | done |
| 3 | a running review is not a review wait | `15f40c9` | done |
| 4+5 | desk `[d]` scoping and `[s]` send-back | `6313089` | done — **merged**, see below |

---

## Step 1 — `0ab7c7e` · `crates/lisa-plugin/src/ui.rs`

New private `view_key_hints(view, pause_hint)`; `render_status_line` calls it
instead of carrying a hard-coded tail. Module doc rewritten: `[p]` reaches the
desk, `[v]` cycles.

```
off the desk:  [p] desk  [v] view  [space] {pause|resume}  [d] done  [r] reset
on the desk:   [↑↓] pick  [enter] open  [d] done  [s] send back  [v] view  [space] {pause|resume}
```

`test_status_line` modified (`[p] view` → `[p] desk` + `[v] view`). Two tests
added: `the_desk_status_line_names_the_desks_own_keys` (every desk hint present,
`[p]` absent) and `off_the_desk_the_status_line_never_offers_the_desks_keys`
(loops the other three presets; `[s]`, `[enter]`, `[↑↓]` absent from each).

The comment inside `cards_advertise_only_keys_that_work` was rewritten — it said
"send-back does not exist in the plugin yet", which stopped being true four
commits later. The assertion is unchanged; only its stated reason moved.

## Step 2 — `7d71315` · `crates/lisa-plugin/src/lib.rs`

`enter_view(preset)` added above `handle_key`: sets the preset and clears
`scroll_offset`, `desk_selected`, `desk_expanded`. `[p]` returns `false` when
already on the desk and otherwise enters Present; `[v]` enters `next()`.

Three tests: `p_lands_on_the_desk_from_every_preset_and_rests_there` (all four
presets, plus the no-op keeping an expanded card open),
`v_cycles_the_presets_in_the_old_order`,
`entering_a_view_resets_cursor_expansion_and_scroll`.

## Step 3 — `15f40c9` · `crates/lisa-plugin/src/lib.rs`

One predicate in `desk_state`'s review-wait pass: skip tickets holding a
`ThreadStatus::Running` thread. `a_running_review_is_not_a_review_wait` asserts
the card disappears when a running thread attaches and returns when the same
thread parks. `desk_cards_are_grouped_and_ordered_by_ticket_id` stayed green
unmodified, which is the regression the plan asked to watch.

## Steps 4 and 5 — `6313089` · `crates/lisa-plugin/src/lib.rs`

**Deviation: merged into one commit.** The plan sequenced `[d]` scoping before
`[s]`, but both keys are cases in the same hoisted `match`, and hoisting the
desk block above the global keys is what makes either possible. Splitting them
would have meant moving the block once, committing a half-populated match, then
touching the same twenty lines again. Landing the hoist with both cases is one
reviewable unit; the tests are still split per criterion.

- `desk_card_count()` became `desk_cards() -> Vec<ui::DeskCard>` — it had no
  remaining callers under its old name once the handler needed the selected
  card's identity, so it was renamed rather than left beside a near-duplicate.
- The desk block moved above `[p]`/`[v]`/`[space]`/`[d]`/`[r]`, builds the card
  list once, and gained `Char('d')` and `Char('s')`.
- `open_desk_signature(ticket_id)`: `open_mark_done_modal`, move `modal.cursor`
  onto the ticket, open the reason step when the verdict needs a signature.
  Returns silently when the ticket is not listed, leaving the plain modal.
- `send_back_for_review(ticket_id)`: resolve the file path, refuse anything not
  `Blocked` with a Warning, `update_ticket_status(Open)`, log, `rebuild_dag()`.

Seven tests: `d_on_a_desk_card_opens_the_reason_step_already_scoped_to_it`,
`two_keypresses_seal_the_selected_card`,
`d_falls_back_to_the_plain_modal_for_a_ticket_the_board_cannot_finish`,
`s_returns_a_parked_ticket_to_review_and_its_card_leaves_on_the_next_poll`,
`s_does_nothing_on_a_note_card_or_a_review_wait_card`,
`desk_keys_are_inert_on_an_empty_desk`,
`send_back_refuses_a_ticket_that_is_no_longer_blocked`.

**Deviation: two negative fixtures got real cards instead of constructed ones.**
The first drafts of the note-card assertions built a `DeskCard` literal and
checked its kind against the gate — which tests the gate's spelling, not the
key. Both were rewritten onto a new `desk_without_a_block()` fixture that seeds
a real completion journal (Requested → CommandInFlight → Confirmed with a
`DispositionNote`) so `collect_notes` produces a genuine Note card next to a
genuine ReviewWait card. `[s]` is then pressed on each through `handle_key`, and
both ticket files are compared byte-for-byte before and after.

---

## Receipts, end to end

`s_returns_a_parked_ticket_to_review_and_its_card_leaves_on_the_next_poll` runs
the whole chain natively against a temp repo: one Block card on the desk → `[s]`
→ `T-PARKED.md` on disk reads `status: open` **and** still `phase: review` → no
Block card → `schedule_ready_tickets()` → a thread exists for `T-PARKED` and the
desk is empty.

`two_keypresses_seal_the_selected_card` runs the signature chain: `[d]`, Enter,
and the ticket reaches `Phase::Done`, `T-AFTER`'s dependencies read done, and
the provenance ledger carries an `OperatorOverride` receipt naming the actor and
the overridden ask. Nothing about the completion path was touched to get there.

## Not done, on purpose

- **No ledger row for send-back.** `lisa unblock` writes none and is the ticket's
  named reference; adding one here would make the CLI and the plugin disagree
  about what a send-back is. The receipt is the activity line.
- **No remedy `check` execution.** Send-back is the operator disagreeing, which
  is the case a machine check cannot settle.
- **Cards still name one key.** `[s]` lives on the status line; the reasoning is
  in design.md D6.
</content>
</invoke>
