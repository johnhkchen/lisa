# T-053-02-01 — Review: the desk view

## What changed

Four files, two commits (`9793755`, `b66e16f`). No files created or deleted.

**`crates/lisa-core/src/parking.rs`** — one shared ledger walk now backs both
`latest_park_attempt_leases` (unchanged) and a new `latest_park_stamps`; `ParkedRemedy`
gained `steps`, destructured from the block disposition it was already parsing.

**`crates/lisa-cli/src/status.rs`** — five test literals gained the new field. No production
change.

**`crates/lisa-plugin/src/ui.rs`** — gained the card types (`DeskCardKind`, `DeskCard`,
`DeskDetail`, `DeskState`), four copy constants, a `Present` preset, and three renderers
(`desk_card_lines`, `render_present_view`, `render_desk_pointer`). Lost
`render_waiting_on_you` and `render_notes_for_you`. `render_attention_banner` became
`render_health_alerts` — alert rows only, no review rows, no `"Press [d] to mark done"`.
`PluginState` traded `waiting_items`/`note_items` for `desk`.

**`crates/lisa-plugin/src/lib.rs`** — `desk_state` and `fail_closed_desk_cards` assemble the
four card classes; `State` gained `desk_selected`/`desk_expanded`; `handle_key` gained a
`Present`-scoped Up/Down/Enter branch.

## Two decisions a reviewer should look at first

**The ticket's age model was wrong, and the fix reads the ledger.** The ticket says parked
blocks "carry an in-memory `parked_at` that resets with the plugin". They do not: a ticket
parks by having its thread *removed* (lib.rs, the durable-park path), so nothing in memory
remembers when. Under the ticket's stated model every block card would read `—` forever and
the epic's own worked example (`waiting 2h` on a parked block) would be unreachable. The
durable `ParkingTransitionRecord.ended_at` is the real stamp and is now read through
`latest_park_stamps`. Nothing is invented: a park row that was never written (the ledger
refuses rows it cannot attribute to a current attempt) yields no stamp and the card says
`—`.

**A parked ticket was carded twice, and a test caught it.** A ticket parks by taking
`status: blocked` while keeping `phase: review`, so it satisfied both the Block source and
the Review-wait source. On a real board nearly every parked block would have appeared twice
in two framings, one of them wrong. The review-wait pass now skips any ticket already
asking for something. Pinned by `a_parked_ticket_is_one_card_not_a_block_and_a_review_wait`.

## Card copy check (acceptance criterion 5)

Every string a collapsed card can render, read at a kitchen table:

| line | copy |
|---|---|
| identity | `T-015-02-02 · signed-build · 2h ago` — id, short title, age. Ages are E-052's four buckets or `—`. |
| ask (block) | the disposition's own `ask`, verbatim; `LEGACY_BLOCK_ASK` for unstructured legacy blocks |
| ask (no review) | "No review was left for this ticket." |
| ask (unreadable review) | "No review Lisa can read was left for this ticket." |
| ask (review wait) | "Review finished — this one is waiting for you." |
| ask (note) | the note's own `summary`, verbatim |
| action | "→ [d] mark it done" · "→ [d] mark it done · Lisa checks on its own" · "→ [enter] read it" |
| chrome | "Your desk" · "Nothing needs you." · "4 waiting, 1 note — [p]" |

No line contains `disposition`, `frontmatter`, `DAG`, or `seal`, and none names a subsystem,
a file path, a phase code, or a measurement. `collapsed_lines_carry_no_mechanism_vocabulary`
asserts this by tokenized match rather than substring, following the precedent in
`operator_override.rs`.

**Verbatim, not summarized.** `asks_render_verbatim_from_their_disposition_fields` feeds the
60-word codesign jargon wall from the 0.4.4 field screenshots in as a block's ask and
asserts the rendered line is that ask's own opening characters. The desk truncates for
width; it never rewords. A jargony ask stays the disposition author's bug to fix upstream,
which is the honest boundary the story drew (N4).

**Limit of that check, stated plainly:** it verifies Lisa's own copy and one pinned
specimen. It cannot enforce the kitchen-table rule on an arbitrary agent-authored ask —
by design, since rewriting one would be exactly the summarization pass this ticket forbids.

## Test coverage

Nine new ui.rs tests over a shared five-card fixture (two blocks, one review wait, one note,
one fail-closed block), seven new lib.rs tests over real temp work dirs and ledgers, three
new lisa-core tests. Three pre-existing lib.rs tests were retargeted from `waiting_items`/
`note_items` onto `desk.cards` — a closer read of what the operator actually sees.

Six ui.rs tests were deleted with the renderers they exercised. Every assertion in them has
a named successor: the world-waiting suffix is now `cards_advertise_only_keys_that_work`,
the legacy-ask substitution is in the lib.rs projection test, and the note's
summary/criterion/evidence triple is `expanding_a_note_reveals_its_criterion_and_evidence`.
The three attention-banner tests that asserted review rows were rewritten as
`a_review_wait_is_a_desk_card_and_not_an_alert` and
`a_review_wait_and_an_alert_land_on_different_surfaces`, which assert the same facts about
their new home.

`just check` green by exit code 0 (fmt, clippy, WASM check, 524 plugin + 381 core + CLI
tests).

### Gaps

- **The pointer and the desk agree by construction, not by test.** `render_desk_pointer`
  counts the same `state.desk.cards` that `render_present_view` renders, so they cannot
  drift — but that is an argument from the code, not an assertion.
- **No integration test.** Every criterion is reachable at the renderer or assembler seam,
  and the plugin has no harness that drives a real Zellij render.
- **`[d]`'s honesty is not pinned here.** The action line is truthful because
  `open_mark_done_modal` lists blocked and Review-phase tickets and, since T-053-01-02,
  routes a ticket needing a signature to the reason step. If that ever changes, the card
  becomes an N3 violation and no test in this ticket will say so.

## Open concerns

1. **The desk's keys are undiscoverable from the status line.** It still reads
   `[p] view  [space] pause  [d] done  [r] reset` — Up/Down/Enter are not hinted. This is a
   deliberate handoff: the story assigns the key estate and its status-line hints to
   T-053-02-02. In the meantime discoverability rests on the `▸` selection marker and the
   note card's own "[enter] read it". Nothing advertises a key that does not work, so this
   is a gap in help, not an N3 violation.
2. **`[p]` reaches the desk by cycling, not directly.** Operations → Present is one press
   today; T-053-02-02 makes `[p]` go-to-desk and moves cycling to a free key.
3. **`desk_card_count()` rebuilds the whole UI state on each desk keypress**, which reads
   the provenance ledger and every blocked ticket's disposition. This is the same work the
   five-second render already does, at human keypress rate, and it is what guarantees the
   selection is bounded by the desk actually on screen. If a large board ever makes this
   felt, the fix is one shared read per poll, not a cache.
4. **A note can be read on the desk but not cleared from it.** In-plugin note
   acknowledgment does not exist; only `lisa notes read` from the CLI settles one. The card
   honestly offers `[enter]` and nothing more, per the story's "notes never grow action keys
   beyond expand/dismiss" — dismiss is simply not built yet.
5. **The Review-phase artifact filename left the default screen.** The old banner showed
   `design.md` per review row; the desk's review-wait card carries that path as its evidence
   citation, visible on expand. Relocated, not lost — but a reviewer who liked seeing it at
   a glance should know it moved.

## Nothing critical for human attention

No behavior outside the dashboard changed. Scheduling, completion, parking, and the journal
are untouched — this ticket added one view and one authority-free navigation branch, and
moved existing fields to a new place on screen.
