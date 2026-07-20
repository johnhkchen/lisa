# Research — T-053-01-01 · an-override-with-a-receipt

What exists today, where it lives, and how it connects. Line numbers are from the
working tree at `e087fe6` and drift a little from the ticket's ~estimates.

## 1. The `[d]` chain, end to end

| Step | Location | Behavior |
| --- | --- | --- |
| Key handler opens the modal | `open_mark_done_modal` — lib.rs:8506 | Lists non-Done tickets with no Running thread, plus Review-phase tickets, plus Implement-phase tickets that have a `review.md` on disk. Sorted, no filter on disposition. |
| Enter selects | `mark_ticket_done` — lib.rs:8561 | Builds `CompletionInput::OperatorRequested { ticket_id, source: OperatorRequestSource::MarkDoneKey }` and calls `dispatch_completion`. On success stashes `OperatorModalOutcome::Pending`. |
| Dispatcher | `dispatch_completion_at` — lib.rs:2382 | Non-`Reconcile` arm at 2449. Operator guard at **2520–2528**. |
| Operator guard | lib.rs:2520 | `if matches!(source, CompletionSource::OperatorRequested(_)) { match self.passing_review_disposition(&ticket_id) { Ok(note) => completion_note = note, Err(rejection) => { log_completion_rejection(...); return false } } }` |
| Verdict reader | `passing_review_disposition` — lib.rs:2042 | Reads `<work_dir>/<id>/review-disposition.json` through `parse_review_disposition`. |
| Rejection surface | `log_completion_rejection` — lib.rs:2119 | Maps `DispositionBlocked` to `CompletionRejectionKind::DispositionBlocked`, pushes `OperatorModalOutcome::Rejected` into the open modal (via `show_operator_modal_rejection`, 2093) and logs `ActivityEvent::CompletionRejected`. |

The dead lever, precisely: `open_mark_done_modal` offers every non-Done ticket,
including Blocked ones, and the guard at 2520 refuses exactly the Blocked and
unreviewed subset. The modal advertises a transition the dispatcher declines.

## 2. Why Block *and* the fail-closed shapes both die

`passing_review_disposition` (lib.rs:2052) maps four parse outcomes:

- `Pass` → `Ok(None)` — completion proceeds with no note.
- `Note(note)` → `Ok(Some(note))` — **completion proceeds carrying the note.**
- `Block { reason, ask, .. }` → `Err(DispositionBlocked)`. The message leads with
  the block's `ask` and appends `[reason]` unless `ask` is empty or equal to
  `reason` (a 2026-07-18 field fix so a deadline park stopped blaming "the
  reviewer").
- `Invalid { reason }` → `Err(DispositionBlocked { "invalid review disposition: …" })`.

`parse_review_disposition` (disposition.rs:108) is fail-closed by construction:
a read error (**missing file**), a JSON syntax error (**malformed**), a non-object
document, an unknown `disposition` string, or a schema contradiction all return
`Invalid` (disposition.rs:113/126, 295–322). So the three distinct field states —
*agent blocked*, *no review on file*, *file is garbage* — collapse into one
refusal at the dispatcher, and the operator sees a rejection modal either way.

One important asymmetry for the catalog work: a malformed/missing file carries an
`Invalid.reason` string that names the parse failure or the unreadable path
(`could not read review disposition <path>: <io error>`, `review disposition is
malformed JSON: <serde error>`). That string is real, machine-produced text about
the literal state — not a fabrication — and is the natural source for the
`criterion_quote` the ticket asks for in the fail-closed shapes.

## 3. The note machinery that already authorizes completion

`DispositionNote` — disposition.rs:26 — is a three-field struct with private
fields and accessors:

```rust
pub struct DispositionNote { criterion_quote: String, evidence_citation: String, summary: String }
```

`DispositionNote::new` (disposition.rs:34) is the only constructor and returns
`Err(String)` when any field trims to empty. There is no other way to build one,
so "no empty fields" is enforced at the type boundary — an operator catalog
entry that produced an empty field would fail construction, not slip through.

`ReviewDisposition::authorizes_completion` (disposition.rs:98) is
`matches!(self, Self::Pass | Self::Note(_))` — a note is already a completing
verdict.

The note then flows through machinery that is entirely indifferent to who
authored it:

1. `dispatch_completion_at` threads `completion_note: Option<DispositionNote>`
   out of every arm (lib.rs:2388, 2552) into `execute_completion_effect`
   (lib.rs:2571, parameter at 2577).
2. `execute_completion_effect` journals `CompletionJournalTransition::Requested
   { key, prior_phase, prior_status, note }` (lib.rs:2720) and stores it on
   `PendingCompletion.completion_note` (struct at lib.rs:712).
3. `finish_successful_completion` (lib.rs:3141) journals
   `Confirmed { … note: pending.completion_note.clone() }` (lib.rs:3191), then
   `rebuild_dag()` (3211), `log_phase_transition` (3213),
   `emit_provenance_with_note(ticket_id, RunOutcome::Done, false, pending.completion_note)`
   (3220), `release_completed_slot_for_ticket` (3221), `schedule_ready_tickets`
   (3223).
4. `completion_journal` enforces note stability: a `Confirmed` transition whose
   note differs from the one admitted at `Requested` is rejected
   (completion_journal.rs:1022 — *"confirmed transition for {ticket_id} changed
   its admitted completion note"*). The note must be decided **before** dispatch
   and must not be re-derived later.
5. `notes::collect_notes` (notes.rs:163) projects `state == "confirmed"` journal
   rows that carry a `note` field, minus provenance `note-acknowledged` rows, into
   `QueuedNote`s. lib.rs:9067 turns those into `ui::NoteItem`s;
   `render_notes_for_you` (ui.rs:540) prints `summary`, then `Criterion: "…"`,
   then `Evidence: …`.

**Consequence for this ticket:** Notes-for-you needs no change at all. Any note
that reaches a confirmed journal row surfaces there. The journal receipt likewise
comes free — the `note` field is already persisted at Requested and Confirmed.

## 4. The provenance ledger is *not* free — the one real gap

`emit_provenance_with_note` (lib.rs:6869) is the only writer of the terminal
`ProvenanceRecord`, and it opens with two hard preconditions:

```rust
let Some(thread) = self.threads.get(ticket_id) else { return false };            // 6879
let Some(attempt_lease) = thread.attempt_lease.clone() else { …warn…; return false };  // 6882
```

It then reads `thread.client`, `thread.started_at`, `thread.pane_id`, and
`thread.concurrency_at_spawn` to populate the record (6901–6924), and for
`RunOutcome::Done` additionally requires the lease to still be current (6892).

`ProvenanceRecord` (provenance.rs:159) has a **non-optional** `attempt_lease:
AttemptLease` field. It is an *execution* row — it describes an attempt that ran.

The override's target population is precisely the tickets whose agent is gone:
`open_mark_done_modal` selects non-Done tickets **without** a Running thread
(lib.rs:8511–8524), and the E-053 field scenario is a ticket sitting parked in
the attention box long after its session exited. For those tickets
`self.threads.get(ticket_id)` is `None` and `emit_provenance_with_note` returns
`false` silently — no ledger row, no warning that reaches the operator.

There is precedent for a thread-free, operator-attributed ledger row. The mixed
ledger is a tagged union, `ProvenanceLedgerRecord` (provenance.rs:351):

```rust
NoteAcknowledgment | AssignmentTransition | ParkingTransition | TriageTransition
| ProposalAction | UsageCorrection | Execution
```

`ProposalActionRecord` (provenance.rs:274) is the closest shape: it records
*"creation or explicit operator disposition of one triage proposal"*, carries
`actor: String` and `occurred_at: u64`, and needs no live thread. Each variant
has its own `append_*_record` helper (e.g. `append_parking_transition_record`,
provenance.rs:538) built on a shared `append_serialized`.

So acceptance criterion 3's ledger half is the part that cannot be satisfied by
reuse alone — it is the one place the existing path genuinely has no branch.

## 5. Where an operator-chosen note would have to enter

`CompletionInput::OperatorRequested { ticket_id, source }` (lib.rs:699) carries no
note field. `OperatorRequestSource` (lib.rs:657) is a single-variant enum
(`MarkDoneKey`). `CompletionAuthority` (lib.rs:706) is `Attempt(AttemptLease) |
Operator`, and the operator branch is already honored downstream:

- `execute_completion_effect` accepts `CompletionAuthority::Operator` when the
  source is `OperatorRequested` (lib.rs:2632) rather than demanding a current lease.
- `handle_completion_result` treats operator authority as current on the same
  test (lib.rs:3244).
- `finish_successful_completion` uses the operator source to decide whether to
  push `OperatorModalOutcome::Accepted` into the modal (lib.rs:3187).

There is also an existing operator-only relaxation: a `Rejected` completion with
`Retryability::ActionRequired` is reset to `CompletionState::Eligible` when the
source is `OperatorRequested` (lib.rs:2500–2508). That is the closest existing
analogue of "the operator's presence widens the guard" — and it is scoped by
source, not a global weakening.

The dispatcher's shape means an operator-authored note must be present **at
input construction**, i.e. carried on the `CompletionInput` variant (or on the
`OperatorRequestSource`), because of the journal's note-stability rule (§3.4).

## 6. Field cases for the catalog

Real 0.4.4 material available in-repo:

**Case A — evidence satisfies the criterion; the criterion text was stale.**
`docs/active/work/T-046-06-03/operator-note-2026-07-17.md` is a hand-written
record of exactly this override, executed manually before any lever existed. The
reviewer blocked on a 225 MiB measurement against an "approximately 200 MiB" gate
that predated calibration. The operator's finding: *"The reviewer was right that
the documents disagreed"* — the evidence stood, the written criterion was the
stale artifact. The same case is pinned as `FIELD_ASK` in parking.rs tests, and
its note form is the fixture in disposition.rs:447 and notes.rs:252.

**Case B — cannot be verified from this machine.** E-053's "Done looks like":
*"A ticket parks because no Apple ID is signed into Xcode on this machine."*
The 0.4.4 screenshot that named codesign flags and `.appex` paths is the same
family. The work is as verified as this host can verify it.

**Case C — beyond the ticket's reach.** The T-046-06-03 block's second half:
the seeded Zellij 0.40.1 variant was unreachable because the platform-aware
managed default designed the hazard out. The criterion asked for something
outside what the ticket could touch.

**Case D — no agent review on file.** Not a block at all: the `Invalid` shapes.
T-053-02-01's context names this class explicitly — *"a Blocked ticket whose
disposition is missing or unparseable is invisible to the remedy collector
(parking.rs ~118) yet is exactly the ticket the no-review override serves."*
Confirmed: `collect_parked_remedies` (parking.rs:110) filters to
`status == Blocked` and then `filter_map`s away anything that is not
`ReviewDisposition::Block` — missing and malformed dispositions produce no
`ParkedRemedy` and therefore no Waiting-on-you row.

Note the catalog's floor: no entry may read as a quality hedge. `check_note_document`
(disposition.rs:190–205) already rejects a note carrying any of
`work_complaint`, `complaint`, `quality_complaint`, `quality_concern`,
`work_concern` with *"use a block when the work itself needs changes"*. The
S-049-06 discipline is enforced in the schema, not only in prose.

## 7. Citation semantics available for each shape

What the operator can honestly cite, per shape:

- **Block**: the block's own disposition file
  (`docs/active/work/<id>/review-disposition.json`), and where they exist,
  `review.md` / `progress.md` in the same directory. The block's `ask` and
  `reason` strings are in hand at the guard — `passing_review_disposition`
  currently discards them into a formatted rejection string (lib.rs:2062–2068),
  but `parse_review_disposition` returns them structured.
- **Missing file**: nothing to quote from a file that isn't there. The inspected
  path that *does* exist is the ticket's work directory. `Invalid.reason` names
  the unreadable path.
- **Malformed file**: the parse error is the literal state; the file itself is the
  inspected path.

## 8. Test surface and gates

- `just check` = `check-wasm` (`cargo check -p lisa-plugin --target wasm32-wasip1`)
  → `fmt-check` → `lint` → `cargo test --workspace` (justfile:51). Green
  `just check` == green CI.
- lib.rs carries its tests inline (`mod tests`, ~9000 onward). Helpers in scope:
  `write_canonical_review_disposition(&state, id, json)`,
  `install_current_attempt(&mut state, id)`, `State::default()` with a
  `PluginConfig` pointing at tempdirs.
- Existing operator-completion tests to sit beside:
  `test_mark_done_without_active_attempt_uses_operator_authority` (lib.rs:14954)
  — a ticket with **no thread at all**, marked done via operator authority, which
  confirms the no-thread path already reaches the executor;
  the `[d]`+Enter modal test at lib.rs:14888; and
  `test_mark_done_already_pending_keeps_named_correlated_rejection_visible`
  (lib.rs:15005).
- lib.rs:9508 has a structural test asserting `execute_completion_effect` is
  called from exactly one place and that `dispatch_completion` precedes it. Any
  new branch must keep that single launch boundary — this is N4 enforced in code.
- disposition.rs, notes.rs, completion_journal.rs, and parking.rs each carry their
  own `mod tests` with tempdir fixtures.

## 9. Constraints and assumptions carried forward

1. **The note must be chosen before dispatch.** Journal note-stability
   (completion_journal.rs:1022) forbids deriving it later.
2. **`DispositionNote` cannot grow fields.** The ticket says the three fields
   hold in every shape; the schema check (disposition.rs:206) rejects extra note
   fields outright, and `NoteItem` rendering (ui.rs:549) assumes exactly three.
3. **One launch boundary.** lib.rs:9508 pins it; the override is a new authority
   branch feeding `execute_completion_effect`, never a second executor.
4. **Provenance needs a new record variant or an operator-shaped emitter** — this
   is the only genuinely missing durable surface (§4).
5. **The modal is out of scope here.** S-053-01 assigns the two-step reason
   picker to T-053-01-02; this ticket builds the branch the modal will drive.
   Whatever entry point this ticket exposes must be callable with a chosen
   catalog reason and testable without a modal.
6. **The negative fixture must be genuinely red first.** Acceptance criterion 4
   requires demonstrating that a Blocked ticket with no reason selected still
   refuses to seal — the guard widens only for a recorded choice.
