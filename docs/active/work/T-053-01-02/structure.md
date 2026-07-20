# Structure — T-053-01-02 · choices-not-essays

The blueprint. Two files modified, none created or deleted. No new module: the
step is a sub-state of an existing modal, and the catalog it consumes already
exists in `lisa-core`.

| File | Change |
| --- | --- |
| `crates/lisa-plugin/src/lib.rs` | one struct field, one struct, one key-handler branch, one opener, one projection arm, tests |
| `crates/lisa-plugin/src/ui.rs` | one struct, one field, one renderer + one copy function, tests |

Nothing in `lisa-core` changes. `operator_override.rs`, `disposition.rs`,
`provenance.rs`, `dispatch_completion`, `admit_operator_completion`, and
`passing_review_disposition` are read but not edited.

---

## `crates/lisa-plugin/src/lib.rs`

### 1. `ReasonStep` — the step's own state (new, near `MarkDoneModal` ~860)

```
/// The MarkDone modal's second step: which canned reason signs this ticket.
struct ReasonStep {
    ticket_id: TicketId,     // the ticket being signed
    ask: OverriddenAsk,      // what the signature answers (from override_choices_for)
    choices: Vec<OverrideReason>,  // ask.applicable_reasons(), owned for the projection
    cursor: usize,           // index into `choices` — NOT into modal.ticket_ids
}
```

`choices` is a `Vec` rather than the catalog's `&'static [OverrideReason]`
because `ui::ReasonStepState` needs an owned copy at projection time and the
slice is at most three entries.

The doc comment carries the §5 hazard: this cursor is separate from
`MarkDoneModal::cursor` on purpose, because `operator_modal_targets` resolves
the acting ticket through the latter.

### 2. `MarkDoneModal` gains one field (~875)

```
    /// (MarkDone only) The reason step, when the operator has picked a ticket
    /// that needs a signature. `None` = the ticket list is showing.
    reason_step: Option<ReasonStep>,
```

Three existing construction sites take `reason_step: None`:
`open_mark_done_modal` (8730), `open_reset_modal` (8813), the quit path (8926).
`MarkDoneModal` derives `Default`, so `ReasonStep` needs no `Default`.

### 3. `handle_key` — the reason-step layer (inserted at 8582, after the
terminal-feedback layer, before the shared list layer)

```
if self.modal.mode == ModalMode::MarkDone && self.modal.reason_step.is_some() {
    match key.bare_key {
        Esc | Char('q')        => self.modal.reason_step = None,      // back
        Up | Char('k')         => step.cursor = step.cursor.saturating_sub(1),
        Down | Char('j')       => if step.cursor + 1 < step.choices.len() { step.cursor += 1 }
        Enter                  => self.confirm_reason_step(),
        _                      => return false,
    }
    return true;
}
```

Placement is load-bearing (design §5): after the `operator_outcome` layer so a
`Pending` request still swallows keys, and before the shared list layer so `Esc`
means *back* and `j`/`k` do not move the ticket cursor underneath.

Borrow shape: read `cursor`/`choices.len()` from `self.modal.reason_step` and
write back, or take a `&mut` scoped to the match — `confirm_reason_step` needs
`&mut self`, so it is called after the borrow ends.

### 4. `handle_key`'s Enter arm on the ticket list — the fork (8600)

`ModalMode::MarkDone` arm becomes:

```
if let Some(ticket_id) = ticket_id {
    match self.override_choices_for(&ticket_id) {
        Some(ask) => self.open_reason_step(ticket_id, ask),
        None      => self.mark_ticket_done(&ticket_id),   // unchanged path
    }
}
```

The `None` arm is today's behavior byte-for-byte.

### 5. `open_reason_step(&mut self, ticket_id: TicketId, ask: OverriddenAsk)` (new, near 8744)

```
let choices = ask.applicable_reasons().to_vec();
let cursor = choices.iter().position(|r| *r == ask.recommended_reason()).unwrap_or(0);
self.modal.reason_step = Some(ReasonStep { ticket_id, ask, choices, cursor });
```

`applicable_reasons()` always contains `recommended_reason()` (pinned by
`block_offers_three_reasons_and_recommends_evidence_satisfies` and
`fail_closed_states_offer_only_the_no_review_reason` in core), so `unwrap_or(0)`
is a floor, not a fallback in practice. It never returns an empty `choices`; a
defensive early return keeps a hypothetical empty list from rendering a step
with nothing to sign.

### 6. `confirm_reason_step(&mut self)` (new, adjacent)

```
let Some(step) = self.modal.reason_step.take() else { return };
let Some(reason) = step.choices.get(step.cursor).copied() else { return };
self.mark_ticket_done_with_override(&step.ticket_id, reason);
```

`take()` first: clearing the step before dispatch means the outcome layer paints
over the ticket list, matching what `[d]`-on-a-passing-ticket already does, and
`operator_modal_targets` resolves through `modal.cursor` — still the chosen
ticket — exactly as it does today.

`mark_ticket_done_with_override` and `override_choices_for` **lose their
`#[allow(dead_code)]`** and their "T-053-01-02 will call this" comments; the
comments become statements of who calls them. Removing the attributes is
required, not cosmetic: clippy would otherwise pass an allow that is now a lie,
and leaving it hides a future regression where the caller disappears.

### 7. The UI projection (9456)

```
reason_step: self.modal.reason_step.as_ref().map(|step| ui::ReasonStepState {
    ticket_id: step.ticket_id.clone(),
    ask: step.ask.clone(),
    choices: step.choices.clone(),
    cursor: step.cursor,
}),
```

A plain field-copy, the same shape as the existing `operator_outcome` map.
`OverriddenAsk` derives `Clone`; `OverrideReason` is `Copy`.

---

## `crates/lisa-plugin/src/ui.rs`

### 8. Import (line 14 neighborhood)

```
use lisa_core::operator_override::{OverriddenAsk, OverrideReason};
```

`ui.rs` already imports `lisa_core::types::CompletionRejectionKind` and
`lisa_core::triage::TriageProposal`; this is the same direction.

### 9. `ReasonStepState` (new, near `ModalState` ~337)

```
/// (MarkDone only) The reason step: what the signature answers and what it may say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonStepState {
    pub ticket_id: String,
    pub ask: OverriddenAsk,
    pub choices: Vec<OverrideReason>,
    pub cursor: usize,
}
```

### 10. `ModalState` gains `pub reason_step: Option<ReasonStepState>` (~352)

`ModalState` derives `Default`; `Option` defaults to `None`, so existing test
constructions using `..Default::default()` are unaffected. Constructions that
list every field explicitly (ui.rs:3413, 3431, 3468) need the field added.

### 11. `ask_header_lines(ask: &OverriddenAsk, width: usize) -> Vec<String>` (new)

**The single place criterion 2's copy rule is expressed.**

```
match ask {
    Block { ask, .. }      => wrap_modal_text(ask, width),   // verbatim; `reason` never read
    NoReviewOnFile         => wrap_modal_text("No review was left for this ticket.", width),
    UnreadableReview { .. } => wrap_modal_text("No review Lisa can read was left for this ticket.", width),
}
```

The two fail-closed arms destructure with `{ .. }` deliberately — the parse
`detail` and the block's technical `reason` are not merely unused, they are
unreachable from this function. That is what makes "never a raw parse error"
checkable by reading one function rather than auditing a renderer.

### 12. `render_reason_step_modal(step, width, height) -> Vec<String>` (new, near
`render_operator_outcome_modal` ~1380)

Layout, following the outcome renderer's conventions (`box_w = width.min(50)`,
`inner_w = box_w - 2`, centered bold title, `├─┤` separators, centered dim
footer):

```
┌──────────────────────────────────────┐
│           Sign T-015-02-02           │      title
├──────────────────────────────────────┤
│ Signed build needs an Apple ID in    │      ask_header_lines, wrapped
│ Xcode; the work itself checks out.   │
├──────────────────────────────────────┤
│ ▸ The work already covers this — …   │      choices, `▸ `/`  `, cursor bold+cyan
│   This can't be checked from this …  │
│   This is past what the ticket can … │
├──────────────────────────────────────┤
│      Enter=sign  Esc=back            │      footer
└──────────────────────────────────────┘
```

Two rules the list renderer at 1560–1573 gets wrong for this content and this
renderer must not inherit:

- **Width by `chars().count()`, not `len()`.** Every `summary()` contains an em
  dash; byte length would over-pad and break the box. The outcome renderer at
  1477 already measures correctly — follow it, not 1571.
- **Choice rows wrap or truncate to `inner_w - 2`.** A `summary()` is 60–90
  characters against a 48-column inner width. Truncation with a trailing `…`
  keeps one choice on one row, which is what makes the list scannable; wrapping
  a three-item list into nine rows defeats the glance. The full sentence is
  never lost — it is the note's `summary`, and the receipt carries it verbatim.

Footer wording `" Enter=sign  Esc=back "` — "sign" is the epic's own verb and
names the consequence; "back" is honest about Esc's meaning at this depth,
where the outer list says "cancel".

### 13. `render_modal` dispatch (1515)

```
if modal.kind == ModalKind::MarkDone {
    if let Some(outcome) = modal.operator_outcome.as_ref() {
        return render_operator_outcome_modal(outcome, width, height);
    }
    if let Some(step) = modal.reason_step.as_ref() {
        return render_reason_step_modal(step, width, height);
    }
}
```

Outcome first: once a request is submitted the step is already `take()`n, so the
two cannot both be `Some` — but the ordering states the precedence explicitly
rather than relying on that.

---

## Ordering of changes

1. `ui.rs` types + `ask_header_lines` + renderer + dispatch — self-contained,
   compiles and tests alone against the core catalog.
2. `lib.rs` `ReasonStep` + `MarkDoneModal` field + three construction sites +
   projection — makes the renderer reachable.
3. `lib.rs` key handler branch + `open_reason_step` + `confirm_reason_step` +
   attribute removal — makes it operable.

2 and 3 cannot split cleanly: a `reason_step` field nothing writes is dead
weight clippy will not complain about but a reviewer will, and removing
`#[allow(dead_code)]` before a caller exists fails the build. Step 1 stands
alone; steps 2 and 3 land together. Plan sequences accordingly.

## Interfaces that do not change

`dispatch_completion`, `admit_operator_completion`, `passing_review_disposition`,
`observed_override_state`, `inspected_paths`, `mark_ticket_done`, `ModalMode`,
`ui::ModalKind`, `operator_modal_targets`, the ledger, the journal, and every
`lisa-core` signature. The single completion-launch boundary is untouched — N4
checked by the build.
