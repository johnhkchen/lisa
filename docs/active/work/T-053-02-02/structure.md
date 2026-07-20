# T-053-02-02 — Structure: file-level shape

Two source files change. Nothing is created or deleted.

```
crates/lisa-plugin/src/ui.rs      module doc, ViewPreset doc, render_status_line, tests
crates/lisa-plugin/src/lib.rs     handle_key, enter_view, desk_cards, [d] scoping,
                                  send_back_for_review, desk_state review-wait filter, tests
```

No change to `lisa-core`, `lisa-cli`, or any public plugin type. `DeskCard`,
`DeskCardKind`, `DeskState`, and `DeskDetail` keep their shapes: this ticket
wires keys into the view T-053-02-01 built, it does not reshape the view.

---

## `crates/lisa-plugin/src/ui.rs`

### Modified: module doc (line 3)

`"Provides four preset views cycled with [p]"` → cycled with `[v]`, with `[p]`
named as the jump to the desk. Documentation that names a key must be as true as
the status line.

### Modified: `ViewPreset::next` doc (line 483)

The doc says "cycle to the next view preset" — still true. `ViewPreset` gains no
variants and `next()`'s order is untouched (criterion 1: old order).

### Modified: `render_status_line` (line 1362)

The single hard-coded hint tail becomes a `match state.active_view` producing
one of two strings, built before the final `format!`. Signature unchanged; it
still reads only `&PluginState`, so `active_view` — already a field (ui.rs:525)
— is the only new input.

```
ViewPreset::Present => "[↑↓] pick  [enter] open  [d] done  [s] send back  [v] view  [space] {hint}"
_                   => "[p] desk  [v] view  [space] {hint}  [d] done  [r] reset"
```

The `{hint}` slot is the existing `pause_hint` (`pause`/`resume`), so both
strings stay pause-aware. The rest of the line — chip, slots, active/done
counts, alerts — is untouched.

Interface note: the hint tail is the *only* view-dependent part. Everything a
test asserts about counts, slots, alerts, or the `PAUSED` marker keeps working
against either variant.

### Modified: `card_action_line` doc + `cards_advertise_only_keys_that_work`

No behavior change. The test's assertion (`no card line contains "[s]"`) stands;
its comment is rewritten from "send-back does not exist in the plugin yet" to
the reason that now holds — a card names one recommended key, and the desk's
full estate is on the status line.

### Tests added/modified (ui.rs)

| test | change |
|---|---|
| `test_status_line` (3113) | modified — `[p] view` → `[p] desk` and `[v] view` |
| `test_status_line_has_reset_hint` (3754) | unchanged (non-desk default) |
| `test_status_line_paused` / `_not_pause` (3732/3743) | unchanged |
| `the_desk_status_line_names_the_desks_own_keys` | **new** — Present view: contains `[s] send back`, `[enter] open`, `[↑↓] pick`; does **not** contain `[p]` |
| `off_the_desk_the_status_line_never_offers_the_desks_keys` | **new** — Operations/Dag/Activity: contains `[p] desk`, `[v] view`; does **not** contain `[s]` |

---

## `crates/lisa-plugin/src/lib.rs`

### New: `State::enter_view(&mut self, preset: ui::ViewPreset)`

Placed immediately above `handle_key`. Sets `view_preset` and clears
`scroll_offset`, `desk_selected`, `desk_expanded`. The single seam for
criterion 3's "cursor, expansion, and scroll state reset on view entry" — one
function, so no future view key can forget half of it.

### New: `State::desk_cards(&self) -> Vec<ui::DeskCard>`

`self.to_ui_state().desk.cards`. `desk_card_count` (9440) is re-expressed as
`self.desk_cards().len()` and keeps its doc comment about deliberate rebuilding.
The key handler calls `desk_cards()` once and reads both the length and the
selected element from it (design D1), so scoping `[d]` costs no extra rebuild.

### New: `State::open_desk_signature(&mut self, ticket_id: &str)`

- `open_mark_done_modal()`
- if the modal did not open (empty list) → return; the existing Info line
  already said why
- find `ticket_id` in `modal.ticket_ids`; if absent → return, leaving the
  ordinary unscoped modal (design D3's fallback)
- set `modal.cursor` to that index
- `override_choices_for(ticket_id)` → `Some(ask)` opens `open_reason_step`,
  `None` leaves the list with the cursor parked on the ticket so `Enter` seals

Reuses `open_mark_done_modal`, `override_choices_for`, and `open_reason_step`
untouched. Adds no completion code (N4).

### New: `State::send_back_for_review(&mut self, ticket_id: &str)`

Modeled on `reset_ticket` (8966), narrower:

1. resolve `file_path` from `dag.get_ticket`; log an Error and return if absent
   or empty
2. refuse unless `status == TicketStatus::Blocked`, logging a Warning — the
   second guard of design D4
3. `ticket::update_ticket_status(&file_path, TicketStatus::Open)`; log an Error
   and return on failure
4. log an Info line naming the ticket
5. `rebuild_dag()`

No thread kill, no slot release: a parked block has neither (parking removes the
thread, lib.rs:3189). No ledger write, matching `lisa unblock` (design D4).

### Modified: `State::handle_key` (8575)

The desk branch (currently 8714–8748) moves **above** the global `p` guard and
gains two cases. Shape after the move:

```
modal handling (unchanged, early-returns)
│
├─ if view_preset == Present:                  ← hoisted, one card build
│     cards = self.desk_cards()
│     clamp desk_selected
│     Up|k / Down|j / Enter   (unchanged bodies)
│     Char('d') → selected card ⇒ open_desk_signature; else fall through
│     Char('s') → selected card of kind Block|NoReviewOnFile ⇒ send_back_for_review
│                 otherwise return false
│     _ → fall through
│
├─ 'p'  → Present already? return false : enter_view(Present)
├─ 'v'  → enter_view(view_preset.next())
├─ ' '  → pause toggle          (unchanged)
├─ 'd'  → open_mark_done_modal  (unchanged; the desk's fallback lands here)
├─ 'r'  → open_reset_modal      (unchanged)
├─ j/k  → scroll                (unchanged)
├─ 'D'  → snapshot              (unchanged)
└─ 'q'  → try_quit              (unchanged)
```

`Char('d')` inside the desk block falls through when the desk is empty or the
selected card's ticket is not finishable, so the ordinary `[d]` answers with no
duplicated code. `Char('s')` never falls through — `s` is bound nowhere else.

The comment block at 8714–8719 is rewritten: it currently says which key reaches
the desk "is the key estate's business, not this view's," which was a handoff
note to this ticket and is now stale.

### Modified: `State::desk_state` (9330), review-wait pass (9390–9410)

One added predicate: skip tickets whose thread is `ThreadStatus::Running`
(design D5). Read from `self.threads` directly — `desk_state` already takes
`&self` and already reads `self.dag` and `self.ledger_path`, so no signature
change. The doc comment gains a sentence on why a running review is not a wait.

`age_stamp`/`evidence_citation` still come from `parked` (`ThreadStatus::Parked`
threads), untouched.

---

## Test surface (lib.rs)

Existing fixtures are reused rather than replaced:

- `desk_state_from` (10738) — no `ticket_dir`, so usable only for read-only
  desk assertions. Keeps its current tests.
- `sealing_fixture` (16037) — real `ticket_dir`, journal seal, `T-PARKED`
  blocked at Review with a readable Block. The fixture for `[d]` scoping and
  `[s]`.
- A new `armed_sealing_fixture` wrapper sets `permissions_granted`,
  `slots_discovered`, and one `fresh_slot`, so `schedule_ready_tickets()` can
  spawn — needed only by the "leaves on the next poll" test.

| test | criterion |
|---|---|
| `p_lands_on_the_desk_from_every_preset_and_rests_there` | 1 |
| `v_cycles_the_presets_in_the_old_order` | 1 |
| `entering_a_view_resets_cursor_expansion_and_scroll` | 3 |
| `d_on_a_desk_card_opens_the_reason_step_already_scoped_to_it` | 2 |
| `two_keypresses_seal_the_selected_card` (`[d]`, `Enter`) | 2 |
| `s_returns_a_parked_ticket_to_review_and_its_card_leaves_on_the_next_poll` | 2 |
| `s_does_nothing_on_a_note_card_or_a_review_wait_card` | 4 |
| `desk_keys_are_inert_on_an_empty_desk` | 3 |
| `a_running_review_is_not_a_review_wait` | 2 (D5) |

Modified existing tests: `desk_keys_select_and_expand_only_on_the_present_view`
(10888) sets `view_preset` directly rather than through `[p]` — still valid, but
its Operations leg now needs `enter_view` semantics checked. `no_free_text_input_exists_in_the_flow`
(16595) lists bound characters; `s` and `v` are unbound *inside the modal*, so
its `BOUND` set is unchanged and the test must keep passing untouched — a
regression check that the desk block does not leak into modal mode.

## Ordering

1. ui.rs status line + its tests (independent, provable alone)
2. lib.rs `enter_view` / `[p]` / `[v]` + tests
3. lib.rs review-wait filter + test (must precede `[s]`, which asserts against it)
4. lib.rs desk `[d]` scoping + tests
5. lib.rs `[s]` + tests

Each step compiles and passes `just check` on its own, so each is one
`lisa commit-ticket` unit.
</content>
</invoke>
