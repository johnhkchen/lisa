# T-053-02-02 — Research: one key to the desk

Descriptive map of the key estate, the desk the keys must drive, and the two
state transitions ([d] sign, [s] send back) they have to reach. No solutions.

## 1. The key estate as it stands

All normal-mode keys live in one function, `State::handle_key`
(crates/lisa-plugin/src/lib.rs:8575). Its shape is a modal early-return block
followed by a flat sequence of `if key.bare_key == …` guards, each returning
`true` (re-render) when it consumes the key. The tail returns `false`.

| key | line | effect |
|---|---|---|
| `p` | 8683 | `view_preset = view_preset.next()`, `scroll_offset = 0` |
| `space` | 8690 | toggle `paused`, log an Info line |
| `d` | 8703 | `open_mark_done_modal()` |
| `r` | 8709 | `open_reset_modal()` |
| Up/Down/Enter | 8720–8748 | **desk-scoped**: move `desk_selected`, toggle `desk_expanded` |
| `j`/Down, `k`/Up | 8751–8758 | `scroll_offset` ±1 (every non-desk view) |
| `D` | 8761 | write `/host/.lisa-state-dump.txt` |
| `q` | 8776 | `try_quit()` |

Modal mode (8576–8680) binds Esc/q (close or back), Up/k, Down/j, Enter, and
inside `ModalMode::QuitConfirm` also `q`. A test at lib.rs:16596 pins the full
bound set as `['j','k','q','d','r','p','D',' ']`; `s` and `v` appear nowhere,
so both are free.

Ordering matters and is already load-bearing: the desk branch sits *after* the
global `p`/`d`/`r` guards and *before* the `j`/`k` scroll guards, which is what
lets the desk own j/k without disturbing any other view (comment at 8714–8719).
Anything that must beat a global key has to be hoisted above it.

`ViewPreset` (ui.rs:470) is a four-value enum — Operations, Present, Dag,
Activity — with `next()` (ui.rs:484) cycling in that order and `label()`
(ui.rs:494) naming it for the status bar. `Present` is the desk, added by
T-053-02-01. The module doc (ui.rs:3) still says "four preset views cycled with
`[p]`".

`scroll_offset` (lib.rs:1055) is reset to 0 in exactly one place: the `p`
branch. `desk_selected` (lib.rs:991) and `desk_expanded` (lib.rs:995) are reset
nowhere — they are only clamped, at the top of the desk branch (8724) and again
in `desk_state` (9428).

## 2. The status line

`render_status_line` (ui.rs:1362) builds one string, view-independent except
for `state.active_view.label()` in the leading `[…]` chip. The hint tail is a
single hard-coded literal (ui.rs:1397):

```
[p] view  [space] {pause|resume}  [d] done  [r] reset
```

It is embedded in the title bar (ui.rs:1425) with no width clamp, so length is
a real constraint on any addition. Ten tests read it (`test_status_line`,
`test_status_line_has_reset_hint`, `test_status_line_paused`, …, ui.rs:3113
onward); `test_status_line` asserts `[p] view` verbatim at ui.rs:3120.

Nothing else in the shipped source documents keys. A repo-wide grep for
`[p] view` / `cycles preset` / `cycled with` hits only lib.rs:8682 (a comment),
ui.rs:3, ui.rs:1397, and ui.rs:3120. No README, template, or generated
CLAUDE.md carries a key table.

## 3. The desk the keys drive

`ui::DeskCard` (ui.rs:255) carries `ticket_id`, `title`, `age_stamp`, `kind`,
`ask`, and a separate `DeskDetail` (ui.rs:238) that the collapsed renderer never
reads. `DeskCardKind` (ui.rs:219) has four values:

- `Block` — a readable Block disposition, from `collect_parked_remedies`
  (lisa-core/src/parking.rs:140), which filters `status == Blocked` and keeps
  only Operator- and World-owned remedies (lib.rs:9577–9604).
- `NoReviewOnFile` — `fail_closed_desk_cards` (lib.rs:9452): tickets with
  `status == Blocked` that the remedy collector cannot see, classified by
  `observed_override_state` (lib.rs:2181).
- `ReviewWait` — every DAG ticket at `phase == Review` not already carded
  (lib.rs:9390–9410). No thread condition of any kind.
- `Note` — `lisa_core::notes::collect_notes`, receipts from completed work.

So **both** Block classes imply `status == Blocked`, and neither `ReviewWait`
nor `Note` can. That is the exact distinction the ticket's negative fixture
asks about.

`desk_state` (lib.rs:9330) assembles the cards; `desk_card_count`
(lib.rs:9440) is `self.to_ui_state().desk.cards.len()` — a full rebuild per
call, deliberately (comment at 9434–9439). `card_action_line` (ui.rs:634)
renders the one recommended key per card: `[enter]` for notes, `[d]` otherwise,
with a `· Lisa checks on its own` suffix for world-owned remedies. A ui test
(`cards_advertise_only_keys_that_work`, ui.rs:2546) asserts no card line
contains `[s]`, with the comment "Send-back does not exist in the plugin yet".

`render_desk_pointer` (ui.rs:742) prints `"{n} waiting, {m} notes — [p]"` on
the Operations view — a live claim about what `[p]` does.

## 4. What [d] already reaches

The signature flow landed in T-053-01-02 and is whole:

- `open_mark_done_modal` (lib.rs:8785) lists non-Done tickets without a running
  thread, plus Review-phase tickets and Implement-phase tickets that have a
  `review.md`. Sorted by id. Logs and returns without opening if the list is
  empty.
- Enter on the list (lib.rs:8652–8666) branches on
  `override_choices_for(&ticket_id)` (lib.rs:8864 → `observed_override_state`):
  `Some(ask)` → `open_reason_step` (lib.rs:8872), `None` → `mark_ticket_done`.
- `open_reason_step` preselects `ask.recommended_reason()` — the reason the
  epic's four-keypress budget depends on.
- `confirm_reason_step` (lib.rs:8897) → `mark_ticket_done_with_override` →
  `request_operator_completion` → the ordinary completion path.

`four_keypresses_seal_a_parked_ticket` (lib.rs:16494) drives this end to end
through `handle_key` against `sealing_fixture` (lib.rs:16037), which is a temp
dir with `T-PARKED` (blocked, phase review) and `T-AFTER` (depends on it),
`completion_seal: Journal`, and `lisa_bin: None` — a state that actually seals
natively. `press` (lib.rs:16262), `choose_ticket` (16270), and `open_step`
(16281) are the existing helpers.

So a desk `[d]` does not need new completion machinery. It needs a way to enter
that flow with `modal.cursor` and the reason step already pointed at one ticket.

## 5. What [s] has to mirror

`lisa unblock`'s flip is small (crates/lisa-cli/src/unblock.rs:41–82):

1. refuse unless `ticket.status == TicketStatus::Blocked`;
2. find the parked remedy; run its `check` if it has one;
3. `ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)`.

It writes **no** provenance record — the ledger append at unblock.rs's
world-recheck sibling (lib.rs:10705 asserts one for the *world* path) has no
counterpart here.

Phase is untouched, so the ticket stays at `phase: review` and becomes
schedulable: `Dag::can_start` (dag.rs:185) requires only a startable phase
(`is_startable` = "not Done", types.rs:201), a non-Blocked status, and done
ancestors. `schedule_ready_tickets` (lib.rs:5241) then spawns unless a thread
already exists for the id.

The plugin already writes ticket status in five places (lib.rs:3169, 6056,
6185, 8994, and the CLI-mirroring test paths), so `update_ticket_status` from
`handle_key` is an established move. `reset_ticket` (lib.rs:8966) is the
closest existing template: resolve `file_path` from the DAG, write, log an
Info line, `rebuild_dag()`. Note it takes `file_path` from `dag.get_ticket`,
not from `config.ticket_dir` — but `rebuild_dag` rescans `config.ticket_dir`,
so any fixture that flips status must have a real `ticket_dir`.

## 6. The tension the acceptance criteria expose

The ticket says the card "leaves the desk on the next poll" after `[s]`.
Tracing it: `[s]` sets status Open, so the ticket leaves both Block classes.
But it is still at `phase: review`, and the `ReviewWait` pass (§3) cards
*every* Review-phase ticket with no thread condition. The card would not leave
— it would change costume, from a Block card to one reading "Review finished —
this one is waiting for you."

The same gap already exists without `[s]`: a ticket an agent is actively
reviewing right now is at `phase: review` and gets a card claiming review
finished. `parked_threads` — the only thread data the review-wait pass reads
(lib.rs:9386, for `age_stamp` and `artifact_path`) — comes from
`ThreadStatus::Parked` threads (lib.rs:9546), i.e. agents that finished and are
awaiting review. Running threads (lib.rs:9520) are never consulted.

The old ATTENTION box behaved identically (`git show 9793755~1` — filter is
`t.phase == Phase::Review`, nothing else), so this is inherited, not new.

## 7. Constraints and assumptions carried into Design

- **`s` and `v` are both free**, confirmed against the bound-key list at
  lib.rs:16596. `v` is the ticket's suggested cycle key.
- **Empty-desk safety already holds** for Up/Down/Enter
  (`an_empty_desk_swallows_its_keys_without_selecting_anything`, lib.rs:16934…
  actually 10934). Any new desk key must keep it.
- **The desk rebuilds per keypress.** `desk_card_count` costs a ledger read and
  a disposition parse per blocked ticket. A key needing the selected card's
  identity should reuse one build, not add a second.
- **The status line has no width clamp.** Hints must be counted, not assumed.
- **No test drives a real Zellij render.** Everything is asserted at the
  renderer seam (`render_status_line`, `desk_card_lines`) or through
  `handle_key` against a temp-dir `State`.
- **`schedule_ready_tickets` runs natively in tests** and does create threads
  (lib.rs:11640 asserts one appears), given `permissions_granted`,
  `slots_discovered`, a pushed `agent_slots` entry, and `!paused`.
- Notes are receipts. The story forbids growing action keys on them beyond
  expand/dismiss, and dismiss does not exist in the plugin at all.
</content>
</invoke>
