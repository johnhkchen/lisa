# T-053-02-02 — Review: one key to the desk

## What changed

Two files, four commits (`0ab7c7e`, `7d71315`, `15f40c9`, `6313089`). Nothing
created or deleted, and no public type changed shape.

**`crates/lisa-plugin/src/ui.rs`** — new private `view_key_hints`, called by
`render_status_line` in place of a hard-coded hint tail. Module doc rewritten.
Two tests added, one modified, one comment corrected.

**`crates/lisa-plugin/src/lib.rs`** — `handle_key`'s desk branch moved above the
global keys and gained `[d]` and `[s]`; `enter_view`, `open_desk_signature`, and
`send_back_for_review` are new; `desk_card_count` became `desk_cards`;
`desk_state`'s review-wait pass gained one predicate. Thirteen tests added, one
fixture added.

No completion, parking, scheduling, or journal code was touched. `[d]` supplies
a cursor to the flow T-053-01-02 built and nothing else; `[s]` writes one
frontmatter field.

## Three decisions a reviewer should look at first

**The criterion could not be satisfied without changing what a review-wait card
means.** `[s]` flips a parked ticket from `blocked` to `open` and leaves it at
`phase: review` — `lisa unblock`'s exact flip. But the desk cards *every*
Review-phase ticket, so the card would not have left: it would have changed
costume into "Review finished — this one is waiting for you," on a ticket that
was about to be picked up. The review-wait pass now skips tickets holding a
`ThreadStatus::Running` thread. That claim is false while an agent is mid-review
regardless of this ticket — the same card already appeared for tickets an agent
was actively reviewing, inherited from the old ATTENTION box (`git show
9793755~1`, filter is `t.phase == Phase::Review` and nothing else). Parked
threads, the class the card was built for, are untouched and pinned by a test
that flips the same thread from Running to Parked and watches the card return.

**Send-back is the flip only.** `lisa unblock` also runs the remedy's `check`
first and writes no ledger row. This takes the flip and the no-ledger-row, and
drops the check on purpose: `[s]` is the operator saying *I read this and I
disagree*, which is precisely the case a machine check cannot settle. Running it
would also mean a host command and a pending-state machine inside a keypress.
The CLI keeps its check for the unattended path. Stated as a boundary, not an
oversight — a reviewer who disagrees should say so, because it is a deliberate
divergence from a cited reference.

**`[d]` on a card the board cannot finish falls back rather than refusing.** A
note is a receipt from completed work, so its ticket is usually Done and
`open_mark_done_modal` does not list it. Options were "make `[d]` inert on note
cards" or "open the plain modal". Inert loses: the same key doing nothing on one
card and everything on the next, with no visible reason, is the class of
behavior this epic exists to remove. `[d]` now means one thing — *sign* — scoped
to the selected card when that ticket is finishable, plain otherwise.

## Criterion-by-criterion

| criterion | evidence |
|---|---|
| `[p]` lands on the desk from every preset, no-op on the desk | `p_lands_on_the_desk_from_every_preset_and_rests_there` — all four presets, and the no-op leg asserts an expanded card survives the press |
| `[v]` cycles in the old order | `v_cycles_the_presets_in_the_old_order` — `ViewPreset::next()` is unmodified |
| status-line hints match the estate | `the_desk_status_line_names_the_desks_own_keys`, `off_the_desk_the_status_line_never_offers_the_desks_keys` (see the caveat below) |
| Enter toggles detail | `desk_keys_select_and_expand_only_on_the_present_view` (T-053-02-01, still green) |
| `[d]` pre-scoped, seals end to end | `d_on_a_desk_card_opens_the_reason_step_already_scoped_to_it`, `two_keypresses_seal_the_selected_card` |
| `[s]` returns to review, card leaves on the next poll | `s_returns_a_parked_ticket_to_review_and_its_card_leaves_on_the_next_poll` |
| cursor/expansion/scroll reset on entry | `entering_a_view_resets_cursor_expansion_and_scroll` |
| keys inert on an empty desk | `desk_keys_are_inert_on_an_empty_desk` |
| negative fixture: `[s]` on note and review-wait | `s_does_nothing_on_a_note_card_or_a_review_wait_card` |
| `just check` green | exit code 0 |

## The status-line hint claim, stated exactly

The criterion reads "no hint without a working key, no key without a hint". The
first half is strictly satisfied and tested in both directions: every hint on
the desk names a key the desk answers, and no other view advertises `[s]`,
`[enter]`, or `[↑↓]`. `[p]` is absent from the desk line because it is a no-op
there.

The second half is satisfied for the keys this ticket introduces or moves, not
for every key the plugin binds. `j`/`k`, `D` (state dump), and `q` have never
been hinted and are not hinted now; `[r]` and `[p]` work on the desk without a
hint there. Adding them would run the desk line past 100 characters, and the
title bar has no width clamp. A reviewer who reads the criterion as "every bound
key, everywhere" should treat this as unmet and say so — I read it as being
about the estate the ticket rewires, and I did not want to claim more than the
tests assert.

Measured: 54 characters off the desk, 73 on it, against 44 before. With the
counts prefix, a terminal narrower than roughly 120 columns will wrap the desk
line. There was no clamp before this ticket either; adding one is a change to
the title bar, not to the hints, and is out of scope here.

## Card copy check (brand voice)

Two new operator-facing strings and one changed one, read at a kitchen table:

| string | where |
|---|---|
| `[p] desk` | status line, off the desk |
| `[s] send back` | status line, on the desk |
| `Sent T-XXX back for another review pass` | activity feed |
| `T-XXX isn't waiting, so there is nothing to send back` | activity feed, refused `[s]` |

No mechanism words: no `disposition`, `frontmatter`, `DAG`, `status`, `phase`,
or `seal`. `[d] done` was kept verbatim rather than reworded to "sign", because
the cards say "→ [d] mark it done" and one key must not carry two names.

## Test coverage

Thirteen tests added, all green; 537 plugin tests total (was 524 at the end of
T-053-02-01). Every criterion naming a keypress is driven through `handle_key`
against a temp-dir `State` with a real ticket directory, so assertions land on
files on disk and a real DAG rebuild.

Two tests were rewritten during Implement because their first drafts were weak.
The note-card negative fixture originally constructed a `DeskCard` literal and
checked its kind against the gate — testing the gate's spelling, not the key.
Both now use a new `desk_without_a_block()` fixture that seeds a real completion
journal (Requested → CommandInFlight → Confirmed with a `DispositionNote`) so
`collect_notes` yields a genuine Note card beside a genuine ReviewWait card,
presses `[s]` on each through `handle_key`, and compares both ticket files
byte-for-byte before and after.

Modified from earlier tickets: `test_status_line` (two hint assertions) and one
comment inside `cards_advertise_only_keys_that_work` that said "send-back does
not exist in the plugin yet" — its assertion is unchanged, only the reason it
gives. No test was deleted. `four_keypresses_seal_a_parked_ticket` and
`no_free_text_input_exists_in_the_flow` are green untouched; the second is the
check that the hoisted desk block did not leak into modal mode.

### Gaps

- **The one-poll window is real and untested as a window.** Between `[s]` and
  the next scheduling pass, the ticket shows as a review wait. The test asserts
  the two endpoints (block card gone immediately; desk empty after the poll) but
  does not pin what the middle looks like, because the middle is honest — a
  Review-phase ticket with nothing running — and pinning it would freeze an
  artifact of poll timing.
- **`[s]` has no receipt outside the activity feed.** If the feed scrolls past,
  nothing durable records that an operator sent a ticket back. `lisa unblock`
  has the same hole; if it matters, it should be fixed in both at once.
- **No test drives a real Zellij render or pane spawn.**
  `schedule_ready_tickets()` in the send-back test exercises the scheduler
  natively, which is as close to a poll as this crate reaches. Pre-existing
  boundary.
- **The status line has no width test.** The lengths above were measured by hand
  once. Nothing stops a future hint from pushing the line past a terminal.

## Open concerns

1. **The review-wait filter is a behavior change beyond this ticket's headline.**
   A Review-phase ticket with a running agent no longer appears on the desk at
   all. I believe that is strictly more honest, and the test says so — but it is
   the kind of change that shows up in a screenshot rather than a diff, and a
   reviewer who liked seeing in-flight reviews listed should know they moved to
   the Threads table only.
2. **`[r]` on the desk is unscoped while `[d]` is scoped.** Pressing `[r]` there
   opens the plain reset list, ignoring the selected card. That is the key's
   unchanged global behavior and nothing advertises otherwise, but the asymmetry
   is visible once you notice it. Scoping `[r]` was not in the ticket.
3. **A note can still be read but not cleared** (carried over from
   T-053-02-01) — in-plugin note acknowledgment does not exist, so a note card
   sits on the desk until `lisa notes read` settles it. `[s]` deliberately does
   not touch it.
4. **`desk_cards()` rebuilds the whole UI state on each desk keypress**, now
   including `[d]` and `[s]`. Same cost as before, at human keypress rate, and
   it is what guarantees the key acts on the desk actually on screen.

## Nothing critical for human attention

Scheduling, completion, parking, and the journal are unchanged. The only durable
write this ticket adds is one frontmatter field flip (`status: blocked` →
`status: open`) on an explicit keypress, guarded twice — once on the card's kind
and once on the live ticket's status.
</content>
</invoke>
