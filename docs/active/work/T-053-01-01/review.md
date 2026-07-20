# Review — T-053-01-01 · an-override-with-a-receipt

`[d]` was a dead lever on exactly the tickets it advertised itself for. This
ticket builds the missing branch: `dispatch_completion_at` now accepts an
operator-chosen catalog reason, turns it into a real `DispositionNote`, and rides
the untouched sealing path. The lever is live at the dispatcher; the modal that
lets a person pull it is T-053-01-02.

## What changed

**`crates/lisa-core/src/operator_override.rs` (new, 400 lines with tests)**
The catalog: `OverrideReason` (four entries, each with a stable `id()` and
operator-facing `summary()`), `OverriddenAsk` (the three states an override can
answer, with `applicable_reasons` / `recommended_reason`), `OperatorOverride`, and
`build_operator_override`. Plus `OperatorOverride::recover`, which rebuilds a
receipt from a durable note after a replayed completion.

**`crates/lisa-core/src/lib.rs`** — one `pub mod` line.

**`crates/lisa-core/src/provenance.rs`** — `OperatorOverrideRecord`,
`OperatorOverrideType`, a `ProvenanceLedgerRecord` arm, and
`append_operator_override_record`.

**`crates/lisa-plugin/src/lib.rs`** — the branch and its plumbing:

| Added | Role |
| --- | --- |
| `AdmittedCompletion` | Bundles note + receipt so the effect executor keeps a reviewable signature |
| `PendingCompletion.operator_override` | Carries the receipt from dispatch to seal |
| `CompletionInput::OperatorRequested.override_reason` | The chosen reason, `None` for an ordinary `[d]` |
| `admit_operator_completion` | The widened guard |
| `observed_override_state` | Reads the durable verdict; tells missing from unreadable by `exists()` |
| `inspected_paths` | Citation paths, each observed to exist |
| `emit_operator_override_receipt` | The ledger row, free of thread/lease preconditions |
| `mark_ticket_done_with_override`, `override_choices_for` | The seam T-053-01-02 consumes |

No files deleted. `passing_review_disposition`, the MarkDone modal, and every UI
string are untouched.

## Acceptance criteria

**1 — seals through the existing path.**
`override_completion_seals_and_unblocks_dependents`: a blocked ticket reaches
`phase: done` / `status: done`, `all_dependencies_done` goes true for its
dependent, the pending completion clears and the thread entry is gone. Asserted on
the *outputs* of `finish_successful_completion`, not on override-specific
machinery. N4 is checked by the build, not by prose: the pre-existing structural
test pinning exactly one `execute_completion_effect` caller is untouched and green.

**2 — the fail-closed shapes.**
`operator_override_admits_a_missing_review_with_the_no_review_reason` (quote:
`no agent review on file for T-001`, citation: the work directory) and
`operator_override_admits_an_unreadable_review_quoting_the_parse_failure` (quote:
the reader's own `malformed JSON` text, citation: the file). Nothing fabricated:
`operator_override_cites_review_and_progress_only_when_they_exist` puts
`review.md` on disk and leaves `progress.md` off, then asserts the citation names
the first and not the second. No empty fields is enforced by the type —
`DispositionNote::new` is the only constructor and refuses a blank field —
and covered across all three shapes by
`every_shape_produces_three_non_empty_fields`. The receipt says the absence
plainly, in the catalog's own words: *"No agent review was left for this one."*

**3 — receipts end-to-end.**
Ledger: `override_completion_writes_an_operator_receipt_without_a_thread` asserts
`actor: "operator"`, `reason_id`, the frozen reason copy, and the overridden ask —
on a fixture with no thread and no lease, which is the case that previously
produced no row at all. Journal:
`override_completion_journals_the_same_note_at_request_and_confirm` asserts the
note appears on `requested` and `confirmed` and that the two are equal.
Notes-for-you: `override_completion_surfaces_in_notes_for_you` runs the real
`collect_notes` projection and finds the override's note. No UI code was written —
that surface already handles any confirmed note.

**4 — negative fixture, demonstrated red.**
`blocked_ticket_without_a_chosen_reason_still_refuses_to_seal` and
`missing_review_without_a_chosen_reason_still_refuses_to_seal`. Both were run
against a deliberately weakened guard (a `None` reason falling through to the
recommended entry); the block fixture failed at **exit 101**, transcript captured
verbatim in `progress.md`. The weakening was reverted and both re-run to exit 0.
Structurally, `None` delegates to `passing_review_disposition` unchanged, so the
no-reason path is byte-for-byte today's behavior.

**5 — the kitchen-table read.** Recorded below.

**6 — `just check`.** Exit 0: fmt, clippy, `cargo check --target wasm32-wasip1`,
`cargo test --workspace`.

## Criterion 5 — the kitchen-table read, recorded

The four operator-facing strings, verbatim:

| `id` | Copy |
| --- | --- |
| `evidence-satisfies` | The work already covers this — the review asked for more than the ticket did. |
| `cannot-verify-here` | This can't be checked from this machine — accepted as far as it can be checked here. |
| `beyond-ticket-reach` | This is past what the ticket can reach — accepted as it stands. |
| `no-review-on-file` | No agent review was left for this one — accepted on the work as it stands. |

**The read.** Each is a sentence a person would say aloud to another person about
a piece of work. None names a mechanism: no *frontmatter*, no *disposition*, no
*DAG*, no *seal* — checked as code by `catalog_copy_passes_the_kitchen_table_read`
rather than by my eye alone. Each is verb-forward about the decision being made
(*covers*, *checked*, *reach*, *left*) and each states plainly what happens next
(*accepted*). "Review" and "ticket" survive because they are the plain words for
the things themselves, and the ticket's own criterion names *frontmatter,
disposition, DAG, seal* as the vocabulary to keep out.

**No entry hedges.** Every one accepts the work and explains why the ask doesn't
apply. None says the work might be wrong, because that decision has no signature —
it routes to send-back. `no_catalog_entry_hedges_on_quality` pins this against a
word list, and the schema backs it up independently:
`disposition.rs`'s note check rejects any complaint field with *"use a block when
the work itself needs changes."*

**Where the read does not reach.** The `UnreadableReview` quote carries a serde
parse error into Notes-for-you (`review disposition is malformed JSON: expected
value at line 1 column 1`). That is machine text describing a broken file, which
the ticket explicitly asks the quote to hold. The kitchen-table gate binds the
catalog's copy; it cannot bind a description of a file that is garbage.

## Test coverage

| Area | New tests | Suite |
| --- | --- | --- |
| `lisa-core::operator_override` | 14 | 277 core tests green |
| `lisa-core::provenance` | 3 | (same suite) |
| `lisa-plugin` | 16 | 490 plugin tests green |

The two that carry the most weight are
`operator_override_row_does_not_absorb_or_get_absorbed` — `ProvenanceLedgerRecord`
is `#[serde(untagged)]`, so a new arm is exactly the change that can silently
swallow existing rows, and it is checked in both directions — and
`override_completion_writes_an_operator_receipt_without_a_thread`, which pins the
gap that made this ticket necessary.

**Gap I did not close:** there is no test of the replay path actually re-emitting
a recovered receipt end-to-end. `OperatorOverride::recover` is unit-tested for
round-trip and for correctly declining agent-authored notes, and it is wired into
`replay_in_flight_completion`, but the composed path (lose a result → replay →
confirm → receipt) is not exercised. The existing replay fixtures do not carry
notes, so building one would have been a meaningful piece of new scaffolding. I
judged the unit coverage plus the wiring sufficient; a reviewer who disagrees
should say so.

## Deviations from plan

Two, both written into `progress.md` with rationale before the code changed:

1. **Steps 3 and 4 merged.** The planned pure-refactor commit could not stand
   alone — adding `override_reason` to the input creates an unused binding until
   the guard consumes it, and clippy fails the build. The attribution benefit was
   preserved by running the existing workspace suite to green before writing any
   new test.
2. **A replay gap the plan didn't anticipate.** `replay_in_flight_completion`
   rebuilds from durable history, which stores the note but not the receipt, so a
   replayed override would have sealed with no ledger row. Closed with
   `OperatorOverride::recover`.

## Open concerns

1. **`[d]` still cannot reach this.** The branch is live at the dispatcher and the
   entry point exists, but no keypress reaches it until T-053-01-02 wires the
   reason step. Until then the epic's "painted lever" complaint is only half
   answered — pressing `[d]` on a blocked ticket still dies in the rejection
   modal. That sequencing is the story's design, not an oversight, but it means
   this ticket alone does not make the field screenshot's promise true.
2. **`override_choices_for` and `mark_ticket_done_with_override` are dead code**
   until that ticket, and carry `#[allow(dead_code)]` with a comment naming it.
   If T-053-01-02 slips, these are unexercised surface.
3. **`overridden_ask` duplicates `criterion_quote`** in all three shapes. Kept
   deliberately — a ledger row should be readable without knowing note semantics,
   and the equality is what makes `recover` lossless rather than a guess. A
   reviewer may reasonably prefer one field.
4. **Untagged-enum discipline is now load-bearing.** The next person adding a
   `ProvenanceLedgerRecord` variant must keep required fields disjoint.
   `operator_override_row_does_not_absorb_or_get_absorbed` will catch a mistake
   against *existing* variants but cannot catch one against a future one.
5. **Catalog copy is frozen into receipts.** `reason` stores the summary verbatim
   at signing time, so rewording the catalog leaves old rows reading the old
   words. That is intended (an old receipt should say what was signed), but it
   means the copy is a durable artifact, not just UI text.
