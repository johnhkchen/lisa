# T-053-02-02 — Design: one key to the desk

Five decisions. Each names the options that were live, what the research says
about them, and what was rejected.

---

## D1. Where the desk's keys are handled

**Options**

1. **Leave the desk branch where it is** (after global `p`/`d`/`r`, before
   `j`/`k`) and add `d`/`s` cases to it. Fails immediately: the global `d` at
   lib.rs:8703 returns before the desk branch is ever reached, so a desk `[d]`
   could never be scoped.
2. **Special-case inside the global `d` guard** — `if view_preset == Present {…}`
   nested in the existing branch. Splits desk behavior across two places in the
   function and leaves `s` needing its own guard anyway.
3. **Hoist the desk branch above the global keys**, so one block owns every key
   the desk answers.

**Chosen: 3.** The desk becomes the first thing normal mode consults, and its
`match` falls through (`_ => {}`) for everything it does not claim — so `space`,
`r`, `D`, `q`, and `p` keep working on the desk unchanged, and `j`/`k` scroll
still belongs to every other view because the block is view-gated. One block,
one place to read the desk's estate.

The block also computes the card list **once** per keypress and uses it for both
the clamp and the selected card's identity, replacing the current
`desk_card_count()` call. Research §7: a rebuild costs a ledger read plus a
disposition parse per blocked ticket, and today `[d]` scoped naively would pay
that twice.

---

## D2. `[p]` and `[v]`

**Options for "no-op on the desk itself"**

1. `[p]` on the desk re-enters Present — resetting cursor, expansion, and
   scroll. Cheap to write, but it is a *reset* dressed as a no-op: an operator
   who has scrolled to card 9 and taps `[p]` out of habit loses their place.
2. `[p]` on the desk returns `false` — nothing changes, no re-render.

**Chosen: 2**, which is what the criterion literally asks for ("is a no-op on
the desk itself"). Returning `false` rather than `true` matters: `handle_key`'s
contract is "should the UI re-render", and a keypress that changed nothing has
no frame to draw.

`[v]` takes over `ViewPreset::next()` unchanged, so the cycle order —
Operations → Present → Dag → Activity → Operations — is exactly the old one, as
criterion 1 requires.

**View entry resets.** Both `[p]` and `[v]` route through one private
`enter_view(preset)` that sets the preset and clears `scroll_offset`,
`desk_selected`, and `desk_expanded`. Today only `scroll_offset` is reset
(research §1) and the two desk fields survive view switches, which is criterion
3's gap. Rejected alternative: reset the desk fields inside `desk_state` on the
first Present render — that hides the reset in a renderer and makes "did the
cursor move?" unanswerable from the key handler.

Because `[p]` on the desk is a no-op, it does not reset — the two rules compose
without a special case.

---

## D3. What `[d]` does on a selected card

**Options**

1. **Seal immediately** — skip the modal entirely, dispatch completion for the
   selected ticket. Fastest, and wrong: a single unconfirmed keypress that ends
   a ticket. The epic's own worked example has the operator press `[d]`, see the
   preselected reason, then press Enter. Rejected.
2. **Open the modal scoped**: `open_mark_done_modal()`, move `modal.cursor` to
   the card's ticket, and if `override_choices_for` returns `Some`, open the
   reason step right there with the recommendation preselected.
3. **New parallel modal for desk signatures.** Rejected on N4 — the epic adds
   one authority branch and one view, not a second completion system.

**Chosen: 2.** Every step already exists (research §4); this only supplies the
cursor the operator would otherwise navigate to. `[d]` then `Enter` seals — the
same two presses `four_keypresses_seal_a_parked_ticket` already spends four on
from the board.

**When the card's ticket is not listed.** `open_mark_done_modal` excludes Done
tickets, and a Note card is a receipt from *completed* work — so a note's ticket
is usually absent from the list. Options: refuse the key, or fall back. Chosen:
**fall back to the ordinary unscoped modal** — the plain global `[d]`, which is
what an operator pressing `[d]` anywhere else gets. One rule ("`[d]` scopes to
the selected card when that ticket is finishable"), no per-kind branching, and
no dead key. The same fallback covers the empty desk with no code of its own.

Rejected: making `[d]` inert on Note cards. It would mean the same key does
nothing on one card and everything on the next, with no visible reason.

---

## D4. What `[s]` does, and where it exists

**Scope of the flip.** `lisa unblock` (research §5) refuses non-Blocked
tickets, optionally runs the remedy's `check`, then writes `status: open`. The
in-plugin equivalent takes the **flip only**:

- **No `check` execution.** The check is a shell command; running it from the
  plugin means a host command, an async result, and a pending-state machine.
  `[s]` is not "verify then unblock" — it is the operator saying *I read this
  and I disagree*, which is precisely the case where a machine check is beside
  the point. `lisa unblock` keeps its check for the unattended path.
- **No provenance record.** `lisa unblock` writes none, and the ticket names it
  as the reference implementation. Adding a ledger row here would make the
  plugin and the CLI disagree about what a send-back is. Noted as a boundary,
  not an oversight — send-back leaves its receipt in the activity feed today.
- **Phase untouched**, so the ticket re-enters at `phase: review`: another
  review pass, not a rerun from Ready. `[r]` already owns rerun-from-Ready.

**Where it exists.** Guarded twice, deliberately:

1. In the key handler, on the **card's kind**: `Block` and `NoReviewOnFile`
   only. Both imply `status == Blocked` by construction (research §3);
   `ReviewWait` and `Note` cannot. This is the criterion-4 negative fixture's
   exact seam.
2. In `send_back_for_review`, on the **ticket's live status**, refusing with a
   logged line if it is not Blocked. The card list can be one poll stale; the
   ticket file is the truth.

Rejected: gating on status alone. The card's kind is what the operator is
looking at, and "send-back exists only where a block exists" is a claim about
the desk, so the desk's own type should carry it.

---

## D5. Making the card actually leave

This is the one place the criteria and the existing code disagree, and it needs
a decision rather than a patch (research §6).

After `[s]`, the ticket is Open at `phase: review`. The `ReviewWait` pass cards
*every* Review-phase ticket, so the card would not leave — it would change
costume mid-sentence into "Review finished — this one is waiting for you," on a
ticket that is about to be picked up by an agent.

**Options**

1. **Suppress the sent-back ticket in a set** until a thread appears. Rejected
   outright by criterion 3: "no hidden state."
2. **Reset the phase** so the ticket is no longer a review wait. Rejected — it
   changes what send-back means (another *review* pass) and duplicates `[r]`.
3. **Exclude Review-phase tickets that are not schedulable-and-unstarted** — a
   card only when the scheduler will not act. Rejected as over-fitted: it also
   drops the plain `T-REVIEW` fixture at lib.rs:10863 (open, Review, no thread,
   deps met), which is a legitimate review wait that nothing is running, and it
   makes the card's meaning depend on slot availability.
4. **Exclude Review-phase tickets with a `Running` thread.**

**Chosen: 4.** It is a one-predicate change and it is true independent of this
ticket: `REVIEW_WAIT_ASK` says "Review finished — this one is waiting for you,"
and while an agent holds a running thread on that ticket, review has not
finished and nobody is waiting on a person. The desk stops making that claim.

It also makes the criterion literally true on its own words — *the card leaves
the desk on the next poll*: `[s]` flips the status (the Block card goes at
once), and the next scheduling pass spawns the review thread, at which point the
review-wait card cannot appear either. Parked threads are untouched, so the
class the card was built for — an agent that finished and is awaiting a human —
still cards exactly as before.

Cost, stated plainly: between the flip and the next scheduling pass, the ticket
shows as a review wait for up to one poll. That is honest — during that window
it genuinely is a Review-phase ticket with nothing running — and it is bounded
by the poll interval.

---

## D6. Where the hints live

Criterion 1 wants the status line to match the new estate exactly. The estate is
now view-dependent: `[p]` means "go to the desk" everywhere except the desk, and
`[s]`/`[enter]`/`↑↓` mean something only *on* the desk.

**Options**

1. **One static line naming everything.** It would advertise `[s]` on the DAG
   view, where `s` does nothing — a hint without a working key, the exact N3 sin
   the epic is named after. Rejected.
2. **View-aware hint tail**: one string off the desk, another on it.

**Chosen: 2.**

```
off the desk:  [p] desk  [v] view  [space] {pause|resume}  [d] done  [r] reset
on the desk:   [↑↓] pick  [enter] open  [d] done  [s] send back  [v] view  [space] {pause|resume}
```

Every hint on the left names a key that works in the view showing it. `[p]` is
absent from the desk line because it is a no-op there (D2), and `[r]` is absent
because the desk's own two moves are what the line is for; both keys still work
and neither is advertised falsely. The desk line is 73 characters against the
old 44 — checked, because the title bar has no width clamp (research §2).

Word choice: `[d] done` is kept verbatim from today rather than reworded to
"sign", because the cards themselves say "→ [d] mark it done" and one key must
not have two names. `send back` is the plainest thing the action is.

**Cards do not gain `[s]`.** The story gives a collapsed card "the recommended
key" — singular — and `card_action_line` renders exactly one. Adding a second
key would push the `· Lisa checks on its own` suffix under truncation on narrow
terminals for world-owned remedies. The full desk estate is one glance up, on
the status line. `cards_advertise_only_keys_that_work` (ui.rs:2546) keeps its
assertion; its comment ("send-back does not exist in the plugin yet") becomes
false and gets rewritten to the reason that now holds.
</content>
</invoke>
