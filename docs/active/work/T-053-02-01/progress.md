# T-053-02-01 — Progress

## Status: complete

Two commits through `lisa commit-ticket`. `just check` green by exit code (0).

| commit | subject |
|---|---|
| `9793755` | feat(core): carry a block's steps and its park stamp to the dashboard |
| `b66e16f` | feat(plugin): build the desk and collapse Operations to a pointer |

---

## What landed

### `crates/lisa-core/src/parking.rs`
- One shared ledger walk (`latest_park_records`) now backs both
  `latest_park_attempt_leases` (unchanged signature, unchanged behavior) and the new
  `latest_park_stamps`, which returns each current park's `ended_at`.
- `ParkedRemedy` gained `steps: Vec<String>`, destructured out of the block disposition it
  was already parsing and previously discarding.
- Two tests added: the stamp tracks Park and clears on Unpark; a block's prepared steps and
  check command reach the projection.

### `crates/lisa-cli/src/status.rs`
- Five `ParkedRemedy` test literals gained `steps`. No production change.

### `crates/lisa-plugin/src/ui.rs`
- New types: `DeskCardKind`, `DeskCard`, `DeskDetail`, `DeskState`.
- New copy constants: `NO_REVIEW_ASK`, `UNREADABLE_REVIEW_ASK` (lifted out of
  `ask_header_lines` so the modal and the card say the same thing), `REVIEW_WAIT_ASK`,
  `EMPTY_DESK`.
- `ViewPreset::Present` added; `next()` cycles Operations → Present → DAG → Activity.
- New renderers: `desk_card_lines`, `render_present_view`, `render_desk_pointer`.
- Deleted: `render_waiting_on_you`, `render_notes_for_you`.
- `render_attention_banner` → `render_health_alerts`: review rows, the parked-thread
  lookup, and the `"Press [d] to mark done"` hint removed; alert rows unchanged.
- `PluginState` lost `waiting_items`/`note_items` and gained `desk`.

### `crates/lisa-plugin/src/lib.rs`
- `State` gained `desk_selected` / `desk_expanded`.
- New `desk_state` and `fail_closed_desk_cards` assemble the four card classes.
- `to_ui_state`'s waiting projection stopped dropping `steps` and `check`.
- `handle_key` gained a `Present`-scoped branch: Up/`k`, Down/`j`, Enter.

---

## Deviations from the plan

**Plan steps 2, 3, and 4 landed as one commit rather than three.** Step 2 (types) could not
compile without a `ViewPreset::Present` match arm, and the only honest arm is the real
renderer — a placeholder would have committed a preset that renders a blank screen. Step 4
(assembly) then had to ride along because removing `waiting_items`/`note_items` from
`PluginState` leaves the desk as their only path to the screen. Steps 5's keys landed in
the same commit for the same reason: a note card advertises `[enter]`, and committing that
string before Enter worked would have been the N3 sin this epic exists to correct.

**`PluginState.waiting_items` and `note_items` were deleted, not kept.** The plan assumed
they would survive as the desk's inputs. Once the desk became their only renderer, the
compiler flagged both as never read. They remain as local projections inside `to_ui_state`;
they just no longer travel to the UI. Three lib.rs tests that asserted on those fields now
assert on `desk.cards` — a strictly closer read of what the operator sees.

**Two design decisions were confirmed by test failures rather than by reasoning.** Both are
recorded below because they are the kind of thing a reviewer should not have to rediscover.

---

## What the tests caught that the design missed

**A parked ticket was carded twice.** `dashboard_projection_reads_the_canonical_operator_ask_for_a_durable_park`
failed with two cards where one was expected. Cause: a ticket parks by taking
`status: blocked` while keeping `phase: review`, so it satisfied both the Block source and
the Review-wait source. Neither research nor design caught this; the fixture did. Fixed by
having the review-wait pass skip any ticket already asking for something, on the ground
that what a ticket *needs* is the real decision and the phase it never left is not. Pinned
by `a_parked_ticket_is_one_card_not_a_block_and_a_review_wait`.

This matters beyond the test: without it, nearly every parked block on a real board would
have appeared twice on the desk, in two different framings, one of which was wrong.

**The fixture's ages overflowed.** `NOW` was 100_000s and one card was aged two days.
`Duration` subtraction panics rather than saturating. Fixed by moving `NOW` out to
1_000_000s. Worth noting the production path was never at risk — `format_age_bucket`
saturates internally — but a fixture that cannot express a two-day-old card is a fixture
that cannot test the card the epic cares most about.

---

## Acceptance criteria

| # | criterion | where |
|---|---|---|
| 1 | five collapsed cards, ≤3 lines, no staff work visible; `—` without a stamp | `desk_renders_five_collapsed_cards_with_no_staff_work_visible`, `a_card_with_no_age_source_shows_a_dash` |
| 2 | expanding reveals staff work for that card only; collapsing restores the shape | `expanding_reveals_staff_work_for_the_selected_card_only`, `expanding_a_note_reveals_its_criterion_and_evidence`, `collapsing_restores_the_three_line_shape` |
| 3 | Operations shows pointers with true counts; paragraph renderers gone | `operations_shows_a_pointer_line_not_paragraphs` — plus the renderers no longer exist to run |
| 4 | empty state is exactly the calm sentence, no chrome | `empty_desk_is_one_calm_sentence` |
| 5 | card copy check recorded; asks verbatim | `collapsed_lines_carry_no_mechanism_vocabulary`, `asks_render_verbatim_from_their_disposition_fields`, recorded in review.md |
| 6 | `just check` green | exit code 0 |

## Receipts

The fifth class is proven end to end by `desk_gives_a_blocked_ticket_with_no_readable_review_its_own_card`:
a real temp work directory with one missing disposition and one malformed one, both
surfacing as cards, each wearing the sentence the reason step already shows for that state,
with the parse failure kept off the card's face and in its staff work.
