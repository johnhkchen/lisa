# T-053-02-01 — Design: the desk view

Six decisions, each grounded in research.md.

---

## D1. Where cards are assembled

**Options**

- **A. Derive in ui.rs** from the `PluginState` fields that already exist
  (`waiting_items`, `note_items`, `tickets`). The fifth class is derivable as "a ticket
  whose status is `Blocked` that appears in no `waiting_item`".
- **B. Assemble in lib.rs** into a new `desk` field on `PluginState`; ui.rs renders it.

**Chosen: B.**

A is tempting because ui.rs tests build `PluginState` literals cheaply. It fails on three
counts research found. (1) The fail-closed card needs to distinguish *no review* from
*unreadable review* — the parser reports both as `Invalid` and only a filesystem existence
probe separates them (lib.rs:2180-2184). ui.rs has no filesystem access and, in the WASM
target, should not grow one. (2) The expanded card needs `check` and `steps`, which are
dropped before the UI today (research §2a). (3) The evidence citation for a fail-closed
card is `inspected_paths` (lib.rs:2192), which reads the work directory.

B also preserves the existing architecture: ui.rs is a pure function of `PluginState`, and
every filesystem read already lives in `to_ui_state`. Testability is not lost — ui.rs tests
build `Vec<DeskCard>` fixtures directly, which is a *more* direct fixture for AC 1 than a
`PluginState` whose cards are computed behind the assertion.

---

## D2. The card shape

**Chosen shape** (ui.rs types, mirroring how `WaitingItem`/`NoteItem` already live there):

```rust
pub enum DeskCardKind { Block, NoReviewOnFile, ReviewWait, Note }

pub struct DeskCard {
    pub ticket_id: String,
    pub title: String,
    /// Unix-epoch stamp for the age line. `None` renders "—".
    pub age_stamp: Option<Duration>,
    pub kind: DeskCardKind,
    /// The one sentence, verbatim from the disposition field that carries it.
    pub ask: String,
    pub detail: DeskDetail,
}

pub struct DeskDetail {
    pub reason: Option<String>,
    pub criterion_quote: Option<String>,
    pub evidence_citation: Option<String>,
    pub steps: Vec<String>,
    pub check: Option<String>,
    pub proposal: Option<TriageProposal>,
    /// World-owned remedies: Lisa re-probes reality on its own.
    pub checks_on_own: bool,
}
```

Rejected: one flat struct with every field `Option`. The kind tag is needed anyway to pick
the key hint and the fail-closed framing, and splitting collapsed fields from expanded
fields makes "no criterion quote while collapsed" a property of the *types*, not of a
renderer's discipline — the collapsed renderer never touches `DeskDetail`.

Rejected: reusing `WaitingItem`/`NoteItem` directly as card payloads. They are two shapes
for four classes, neither carries a stamp, and `WaitingItem` has already lost `check`.

**Ask provenance per kind — no new prose, per the honest boundary:**

| kind | `ask` source |
|---|---|
| `Block` | `ParkedRemedy.ask` (already `LEGACY_BLOCK_ASK`-substituted for unstructured blocks) |
| `NoReviewOnFile` | the two sentences already pinned in `ask_header_lines` (ui.rs:1420-1430) |
| `ReviewWait` | `"Review finished — this one is waiting for you."` |
| `Note` | `DispositionNote::summary()` |

Only `ReviewWait` needs a sentence that does not exist yet, because a Review-phase ticket
has no disposition to quote — there is no authored field to render verbatim. It is a fixed
constant, not generated or summarized per ticket, so N4 holds.

---

## D3. Age

**Options**

- **A. `"—"` for every card except review waits.** The ticket's stated model — it assumes
  blocks keep an in-memory `parked_at`.
- **B. Read the durable Park stamp from the provenance ledger for blocked tickets.**

**Chosen: B.**

Research corrected the ticket's premise: a durably parked ticket has its thread *removed*
(lib.rs:6067-6068), so no in-memory stamp survives for it at all. Under A, the cards the
epic cares about most — "waiting 2h" on a parked block — would *always* read "—", and the
worked example in E-053's "Done looks like" would be unreachable.

The stamp B uses is real and already written: `ParkingTransitionRecord.ended_at`
(lib.rs:6916, 6932) is the epoch second the park was recorded. `latest_park_attempt_leases`
(parking.rs:86) already walks exactly these records — keeping the latest Park per ticket and
dropping it on Unpark — and throws the stamp away. B keeps it.

This is not invented data: where `emit_review_block_transition` refused to write a record
(missing or inconsistent attempt lease, lib.rs:6877-6884) there is no stamp and the card
renders "—". The "never an invented number" rule is preserved exactly.

Per-kind age sources:

| kind | stamp |
|---|---|
| `Block`, `NoReviewOnFile` | ledger Park `ended_at`, else `None` |
| `ReviewWait` | `ParkedThread.parked_at` where a parked thread exists, else `None` |
| `Note` | always `None` — notes carry no stamp (research §2c) |

Rendering goes through `format_age_bucket` (ui.rs:486), which already emits `"—"` for a
zero/absent stamp and already clamps future timestamps. No new formatter.

---

## D4. What happens to the three Operations sections

**Options**

- **A. Keep the renderers, gate them behind a flag.** Two code paths for the same content,
  and AC 3's "the paragraph renderers no longer run" becomes a claim about a boolean.
- **B. Delete `render_waiting_on_you` and `render_notes_for_you`; reduce the attention
  banner to health alerts only.**

**Chosen: B.**

Their duty moves wholesale to the desk, and a deleted function is the strongest possible
form of "no longer runs on the default screen". Nothing is lost in the move:

- the triage-proposal block (`First responder` / `Suggested` / `Prepared`) becomes expanded-card
  content — it is staff work, which is what the expanded card is for;
- `checks_on_own` becomes a suffix on the collapsed card's action line, so a world-owned
  remedy still says Lisa re-checks it;
- the criterion quote and evidence citation move to the expanded card, which is the whole
  point of the ticket.

The attention banner is the one section that does **not** collapse cleanly, because it
carries a second, unrelated duty: health alerts for stuck/failed/idle/timed-out sessions
(ui.rs:691-750). Those are not pending decisions and appear in none of the four card
sources, so folding them into the desk would be a category error and dropping them would be
data loss. So the banner keeps its box and its alert rows, loses its Review-phase rows to
the desk, and is renamed `render_health_alerts` to match what it now is.

Its hardcoded `"Press [d] to mark done"` hint (ui.rs:771) goes with the review rows. In an
alerts-only box the hint has no referent — a stuck session is not a thing you mark done —
so keeping it would be a fresh instance of the exact N3 sin E-053 exists to correct. The
*status line's* key hints are T-053-02-02's scope and are not touched here.

Operations gains one pointer line in place of the two deleted sections, e.g.
`5 waiting, 2 notes — [p]`, with counts computed from the same card list the desk renders,
so the pointer and the desk cannot disagree. It is suppressed entirely when there are no
cards.

---

## D5. Reaching the desk, and its selection state

**Options**

- **A. Render-only.** Add the preset and the selection/expansion fields; wire no keys, and
  leave every key to T-053-02-02.
- **B. Wire the desk's own navigation** (Up/Down select, Enter expand/collapse) here; leave
  `[p]` go-to-desk, `[v]` cycle, scoped `[d]`, and `[s]` send-back to T-053-02-02.

**Chosen: B.**

Under A the collapsed card cannot honestly advertise a key. A note card's only affordance
is expand — in-plugin dismissal does not exist (research §2c) — so with Enter unwired, a
note card would either carry no action line at all or advertise a key that does nothing.
The second is N3. The first ships a view whose expanded state, which AC 2 is entirely
about, is unreachable by any keypress.

B is small and self-contained: selection and expansion are the desk's *internal* state, not
the shared key estate the next ticket redistributes. `Up`/`Down` (and `j`/`k`, matching how
the modals already bind both, lib.rs:8615-8622) move the selection **only while the Present
preset is active**; every other view keeps today's scroll behavior untouched. `Enter`
toggles expansion. T-053-02-02 still owns every key that changes what `[p]`, `[v]`, `[d]`,
and `[s]` mean.

The preset itself: `ViewPreset` gains a `Present` variant, and `next()` cycles
Operations → Present → Dag → Activity → Operations, so the desk is reachable through the
`[p]` cycling that exists today. T-053-02-02 replaces that reachability with a direct key.

**Action line per kind** — every one names a key that works today:

| kind | action line |
|---|---|
| `Block` (operator) | `→ [d] mark it done` |
| `Block` (world) | `→ [d] mark it done · Lisa checks on its own` |
| `NoReviewOnFile` | `→ [d] mark it done` |
| `ReviewWait` | `→ [d] mark it done` |
| `Note` | `→ [enter] read it` |

`[d]` opens the mark-done modal, which lists blocked and Review-phase tickets alike
(lib.rs:8753-8775) and, since T-053-01-02, routes a ticket needing a signature to the reason
step rather than a rejection (lib.rs:8654-8656). Every one of these is a lever that moves.
`[s]` is not advertised, because send-back does not exist in the plugin yet.

---

## D6. Getting `check` and `steps` to the expanded card

**Options**

- **A. Re-parse the disposition in lib.rs** when building cards.
- **B. Add `steps` to `ParkedRemedy`** next to the `check` it already carries, and stop
  dropping `check` in the `WaitingItem` projection.

**Chosen: B.**

`ParkedRemedy` is documented as "the small remedy projection needed by status, dashboard,
and unblock UX" (parking.rs:71) — the expanded card is that dashboard need, so the field
belongs there. A would parse each disposition twice per five-second render pass and would
put disposition-schema knowledge in a second place.

Cost of B is bounded and known: five `ParkedRemedy` struct literals in
crates/lisa-cli/src/status.rs tests (research §2a) gain one field. The CLI's rendering is
unchanged.

---

## Consequences, stated plainly

- One new lisa-core function (`latest_park_stamps`, sharing the existing ledger scan), one
  new `ParkedRemedy` field, and one new `PluginState` field.
- Two ui.rs renderers deleted, one renamed and narrowed, one added.
- The tests attached to the deleted renderers are deleted with them; their subject moves to
  the desk tests, which assert the same content in its new place.
- The desk performs one extra full read of the provenance ledger per render pass. The
  render path already performs two (`collect_parked_remedies` and `collect_notes`), so this
  is the same class of cost, not a new one. If it ever matters, the fix is one shared scan,
  not a cache.
