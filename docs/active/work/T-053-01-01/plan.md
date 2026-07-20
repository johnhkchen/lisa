# Plan — T-053-01-01 · an-override-with-a-receipt

Six commits, each independently verifiable. Every step ends with the gate named
in its **Verify** line; `just check` runs at steps 3 and 6 in full.

Commits go through `lisa commit-ticket --ticket-id T-053-01-01 --message <m>
--include <exact paths>`. No ordinary `git add`/`git commit`.

---

## Step 1 — the catalog

**Files:** `crates/lisa-core/src/operator_override.rs` (new),
`crates/lisa-core/src/lib.rs` (one `pub mod` line).

Build `OverrideReason`, `OverriddenAsk`, `OperatorOverride`, and
`build_operator_override` exactly as Structure specifies. No plugin changes.

**Tests (in-module, native):**

| Test | Asserts |
| --- | --- |
| `catalog_copy_passes_the_kitchen_table_read` | No `summary()` contains `frontmatter`, `disposition`, `dag`, or `seal` (case-insensitive, word-boundary) — **criterion 5 as code** |
| `no_catalog_entry_hedges_on_quality` | No `summary()` contains `maybe`, `unsure`, `probably`, `risky`, `questionable`, `looks wrong`, `doubt` — the S-049-06 discipline |
| `catalog_ids_are_unique_and_every_reason_is_listed` | `ALL` covers four variants, `id()`s distinct |
| `block_offers_three_reasons_and_recommends_evidence_satisfies` | applicability table, block row |
| `fail_closed_states_offer_only_the_no_review_reason` | applicability table, both fail-closed rows |
| `inapplicable_reason_is_refused` | `NoReviewOnFile` on a `Block` errs; each block reason on both fail-closed states errs |
| `block_note_quotes_the_ask_and_cites_inspected_paths` | quote == ask verbatim; citation joins supplied paths |
| `missing_review_note_states_the_absence_plainly` | quote == `no agent review on file for T-001`; citation == work dir |
| `unreadable_review_note_quotes_the_parse_failure` | quote == supplied detail verbatim |
| `every_shape_produces_three_non_empty_fields` | all three shapes, no field blank — **criterion 2's "no empty fields"** |
| `empty_inspected_paths_is_refused` | builder errs rather than emitting a blank citation |

**Verify:** `cargo test -p lisa-core operator_override`.

---

## Step 2 — the ledger record

**Files:** `crates/lisa-core/src/provenance.rs`.

`OperatorOverrideType`, `OperatorOverrideRecord`, the `ProvenanceLedgerRecord`
arm, `append_operator_override_record`.

**Tests (in-module):**

| Test | Asserts |
| --- | --- |
| `operator_override_record_round_trips_through_the_mixed_ledger` | append → read back as `ProvenanceLedgerRecord::OperatorOverride`; JSON contains `"record_type":"operator-override"`, the actor, the reason id, and the overridden ask |
| `operator_override_row_does_not_absorb_or_get_absorbed` | An override row deserializes to its own variant and **not** to any earlier one; one sample of every pre-existing variant still deserializes to its own variant after the arm is added |
| `usage_fold_ignores_operator_override_rows` | `fold_ticket_usage` over a ledger containing an override row yields the same totals as without it |

The absorption test is the one that would catch an untagged-enum ordering
mistake, which is the only way this addition could break existing readers.

**Verify:** `cargo test -p lisa-core provenance`.

---

## Step 3 — plugin plumbing (pure refactor)

**Files:** `crates/lisa-plugin/src/lib.rs`.

`AdmittedCompletion`; `PendingCompletion.operator_override`;
`execute_completion_effect`'s parameter swap; `override_reason: None` on the
`CompletionInput::OperatorRequested` variant and at all five construction sites
(8562, and tests near 14888 / 14954 / 15005 / 25582).

No behavior change. No new tests.

**Verify:** `just check` fully green with the **existing** suite unmodified except
for the mechanical `override_reason: None` additions. A failure here is signature
churn, not logic, and must be resolved before step 4 begins.

---

## Step 4 — the widened guard

**Files:** `crates/lisa-plugin/src/lib.rs`.

`admit_operator_completion`, `observed_override_state`, `inspected_paths`, and
the swap at the operator guard (2520–2528).

**Tests:**

| Test | Asserts |
| --- | --- |
| `operator_override_admits_a_blocked_ticket_with_the_blocks_own_ask` | A `Block` disposition + `EvidenceSatisfies` dispatches; the pending completion's note quotes the block's `ask` and cites the disposition file |
| `operator_override_cites_review_and_progress_only_when_they_exist` | With `review.md` present and `progress.md` absent, the citation names the first and not the second — **criterion 2's "nothing fabricated"** |
| `operator_override_admits_a_missing_review_with_the_no_review_reason` | No disposition file + `NoReviewOnFile` dispatches; quote is `no agent review on file for T-001`, citation is the work directory |
| `operator_override_admits_an_unreadable_review_quoting_the_parse_failure` | Malformed JSON + `NoReviewOnFile` dispatches; quote contains the parser's own failure text |
| `operator_override_refuses_a_reason_that_does_not_fit_the_state` | `NoReviewOnFile` on a real block is rejected, modal shows `DispositionBlocked` |
| `operator_override_on_a_passing_ticket_adds_no_note` | `Pass` + a reason → dispatches with `completion_note == None`; the agent's verdict stands |
| **`blocked_ticket_without_a_chosen_reason_still_refuses_to_seal`** | **The negative fixture.** `Block` + `override_reason: None` → `dispatch` returns `false`, no pending completion, modal outcome is `Rejected { kind: DispositionBlocked }`, ticket frontmatter still not Done |
| `missing_review_without_a_chosen_reason_still_refuses_to_seal` | Same for the fail-closed shape |

**Red demonstration (criterion 4).** After the tests are green, temporarily edit
`admit_operator_completion` so the override branch runs regardless of
`override_reason` (i.e. treat `None` as the recommended reason), run
`cargo test -p lisa-plugin blocked_ticket_without_a_chosen_reason_still_refuses_to_seal`,
and capture the failing output into `progress.md` **verbatim with its exit code**.
Then revert the edit and re-run to green. The fixture is kept; the transcript of
its red state is the evidence that it guards something real. Judge red/green by
exit code, not by reading output.

**Verify:** `cargo test -p lisa-plugin`.

---

## Step 5 — the receipt

**Files:** `crates/lisa-plugin/src/lib.rs`.

`emit_operator_override_receipt` + the call in `finish_successful_completion`.

**Tests:**

| Test | Asserts |
| --- | --- |
| `override_completion_writes_an_operator_receipt_without_a_thread` | A blocked ticket with **no thread and no lease** seals through the override; the ledger gains an `OperatorOverride` row carrying `actor`, `reason_id`, `reason`, `overridden_ask`, and the note — **criterion 3's ledger half, and the gap Research §4 found** |
| `override_completion_journals_the_same_note_at_request_and_confirm` | Journal `requested` and `confirmed` rows carry byte-identical notes (the completion_journal.rs:1022 invariant) — **criterion 3's journal half** |
| `override_completion_surfaces_in_notes_for_you` | `collect_notes(journal, ledger)` returns the override's note for the ticket, and it renders through the same `NoteItem` path an agent note does |
| `override_completion_seals_and_unblocks_dependents` | Ticket reaches Done, a dependent that was blocked on it becomes startable after `rebuild_dag`, the seat is released — **criterion 1**, asserted on the existing path's outputs |
| `ordinary_completion_writes_no_operator_receipt` | A `Pass` completion leaves no `OperatorOverride` row |

`override_completion_seals_and_unblocks_dependents` is the one that proves "no
parallel completion code": it asserts the *outputs* of
`finish_successful_completion` (Done frontmatter, DAG readiness, seat release)
rather than any override-specific machinery.

**Verify:** `cargo test -p lisa-plugin`.

---

## Step 6 — entry points and close-out

**Files:** `crates/lisa-plugin/src/lib.rs`.

`request_operator_completion` (private), `mark_ticket_done` delegating to it,
`mark_ticket_done_with_override`, `override_choices_for`.

**Tests:**

| Test | Asserts |
| --- | --- |
| `mark_ticket_done_with_override_seals_a_parked_ticket` | End-to-end through the plugin entry point the modal will call |
| `mark_ticket_done_keeps_its_fail_closed_behavior` | The no-reason entry point is unchanged |
| `override_choices_for_lists_and_recommends_per_state` | Block → three reasons, recommends `EvidenceSatisfies`; missing/unreadable → one reason. The seam T-053-01-02 consumes |
| `override_choices_for_returns_none_on_a_passing_ticket` | Nothing to override |

**Verify:** `just check` — fmt, clippy, WASM check, workspace tests — **criterion 6**.

---

## Verification criteria per acceptance criterion

| Criterion | Proven by |
| --- | --- |
| 1 — seals through `finish_successful_completion`, dependents flow, seat released, no parallel code | Step 5 `override_completion_seals_and_unblocks_dependents`; the untouched single-launch-boundary test at lib.rs:9508 |
| 2 — fail-closed shapes, defined field semantics, nothing fabricated, no empty fields | Step 1 `every_shape_produces_three_non_empty_fields` + the per-shape quote/citation tests; Step 4 missing/unreadable admission tests and the exists-only citation test |
| 3 — ledger + journal receipts, note in Notes-for-you | Step 5's first three tests |
| 4 — negative fixture, demonstrated red, kept | Step 4's two refusal fixtures + the captured red transcript in `progress.md` |
| 5 — catalog copy passes the kitchen-table read, recorded in review.md | Step 1 `catalog_copy_passes_the_kitchen_table_read` (executable) + the copy table transcribed into `review.md` with the read recorded |
| 6 — `just check` green | Step 6 |

## Deviation policy

Any departure from this plan gets written into `progress.md` with its rationale
**before** the code changes. The most likely deviations: the `AdmittedCompletion`
bundle proving unnecessary if the parameter count works out differently, or
`observed_override_state` needing to be `&mut self` for logging. Both are local
and neither changes the design.

## Out of scope, restated

No modal changes, no `passing_review_disposition` edits, no UI strings, no
send-back. Each has a named owner elsewhere in E-053.
