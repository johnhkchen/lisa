# Progress — T-053-01-01 · an-override-with-a-receipt

## Status — complete

| Step | State | Commit |
| --- | --- | --- |
| 1 — the catalog | done | `0c959c8` |
| 2 — the ledger record | done | `8f4d6f3` |
| 3+4 — plumbing + widened guard | done (merged) | `f875861` |
| 5 — the receipt | done | `f875861` |
| 6 — entry points and close-out | done | `f875861` |

`just check` — **exit 0** (fmt, clippy, WASM check, workspace tests). 490 plugin
tests, 277 core tests, all suites green.

---

## Step 1 — the catalog (`0c959c8`)

`crates/lisa-core/src/operator_override.rs` (new) + one `pub mod` line in
`crates/lisa-core/src/lib.rs`.

12 tests green (`cargo test -p lisa-core operator_override`, exit 0). Criterion 5
landed as an executable check (`catalog_copy_passes_the_kitchen_table_read`)
rather than a promise, alongside `no_catalog_entry_hedges_on_quality` for the
S-049-06 discipline.

## Step 2 — the ledger record (`8f4d6f3`)

`OperatorOverrideRecord`, `OperatorOverrideType`, the `ProvenanceLedgerRecord`
arm, and `append_operator_override_record` in
`crates/lisa-core/src/provenance.rs`.

Three tests. The one that earns its place is
`operator_override_row_does_not_absorb_or_get_absorbed`: `ProvenanceLedgerRecord`
is `#[serde(untagged)]`, so a new arm is exactly the change that can silently
swallow existing rows. Checked in both directions.

`cargo test -p lisa-core` — exit 0.

**Small deviation:** the usage fold is named `correct_usage`, not
`fold_ticket_usage` as the plan wrote it. Test calls the real function; no design
change.

---

## Deviation — steps 3 and 4 merged

**Planned:** step 3 was a pure-refactor commit (the `AdmittedCompletion` bundle,
the `PendingCompletion` field, the executor's parameter swap, and
`override_reason: None` at every call site), with step 4 adding the guard branch
on top, so a regression in the existing suite would be attributable to signature
churn rather than logic.

**Why it doesn't hold:** the refactor includes adding `override_reason` to
`CompletionInput::OperatorRequested`. Once that field exists and is destructured
in `dispatch_completion_at`, it is an unused binding until the guard consumes it —
and `just check`'s clippy gate fails an unused variable. The two are not separable
without a throwaway `_override_reason` binding.

**Done instead:** one commit carrying both, with the attribution benefit preserved
in a weaker but sufficient form — the **existing** workspace suite was run to
green *before* any new test was written (`cargo test --workspace`, exit 0, 474
plugin tests unchanged). A regression in it would still have been isolated from
the new behavior.

---

## Deviation — a replay gap the plan did not anticipate

Found while wiring the `PendingCompletion` field:
`replay_in_flight_completion` rebuilds a pending completion from durable journal
history after a lost result. The journal stores the admitted **note** but not the
receipt around it, so a replayed override would have sealed with no ledger row —
a hole in criterion 3 on a path that really happens.

**Fix:** `OperatorOverride::recover(&DispositionNote) -> Option<Self>` in
`lisa-core`. Nothing is invented to rebuild the receipt: the summary is a catalog
entry verbatim, and in every shape the builder produces, the overridden ask *is*
the criterion quote. A note whose summary matches no catalog entry was authored by
an agent, not signed by a person, and recovers as `None` — pinned by
`an_agent_authored_note_is_not_recovered_as_an_override`.

This surfaced a redundancy worth naming: `overridden_ask` and `criterion_quote`
carry the same string in all three shapes. Kept as two fields deliberately — the
ledger row should be readable without a reader knowing note semantics — and the
equality is what makes recovery lossless rather than a guess.

---

## Step 3+4+5+6 — the branch, the receipt, the entry points (`f875861`)

`crates/lisa-plugin/src/lib.rs`, plus `recover` in the core module.

Landed as Structure specified: `AdmittedCompletion`,
`PendingCompletion.operator_override`, `admit_operator_completion`,
`observed_override_state`, `inspected_paths`, `review_disposition_path`,
`emit_operator_override_receipt`, `request_operator_completion`,
`mark_ticket_done_with_override`, `override_choices_for`.

16 new plugin tests, all green.

### Receipts end-to-end (criterion 3)

Demonstrated by `override_completion_writes_an_operator_receipt_without_a_thread`,
`override_completion_journals_the_same_note_at_request_and_confirm`, and
`override_completion_surfaces_in_notes_for_you`, on a fixture with **no thread and
no lease** — the shape the override actually meets in the field.

- **Ledger line:** one `OperatorOverride` row with `actor: "operator"`,
  `reason_id: "cannot-verify-here"`, the frozen reason copy, and
  `overridden_ask: "Sign into Xcode with an Apple ID, then re-run the signed
  build."` The same test asserts **no** `Execution` row was written — the receipt
  does not fabricate an attempt that never ran.
- **Journal entry:** the note appears on exactly two rows, `requested` then
  `confirmed`, byte-identical (the completion_journal.rs:1022 stability rule).
- **Notes-for-you:** `collect_notes` returns the override's note for the ticket,
  through the same projection an agent note uses. No UI code was written.

### The red demonstration (criterion 4)

The negative fixture is
`blocked_ticket_without_a_chosen_reason_still_refuses_to_seal`. To show it guards
something real, `admit_operator_completion` was temporarily weakened so a `None`
reason fell through to the recommended catalog entry, and the fixture was run.
Verbatim, judged by exit code:

```
running 1 test
test tests::blocked_ticket_without_a_chosen_reason_still_refuses_to_seal ... FAILED

failures:

---- tests::blocked_ticket_without_a_chosen_reason_still_refuses_to_seal stdout ----

thread 'tests::blocked_ticket_without_a_chosen_reason_still_refuses_to_seal' (11895006) panicked at crates/lisa-plugin/src/lib.rs:15406:9:
assertion failed: !state.dispatch_completion(CompletionInput::OperatorRequested {
            ticket_id: "T-001".to_string(),
            source: OperatorRequestSource::MarkDoneKey,
            override_reason: None,
        })

failures:
    tests::blocked_ticket_without_a_chosen_reason_still_refuses_to_seal

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 482 filtered out
```

**Exit code 101.** The weakening was then reverted and the fixture re-run: exit 0.
The fixture is kept, along with `missing_review_without_a_chosen_reason_still_refuses_to_seal`
for the fail-closed shape.

---

## What was deliberately not touched

- `passing_review_disposition` — unchanged, so the negative fixture does not
  depend on the code it guards.
- The MarkDone modal — T-053-01-02 owns it; `override_choices_for` is the seam,
  carrying `#[allow(dead_code)]` and a comment naming that ticket.
- `render_notes_for_you` and every other UI string — the projection already
  handles operator notes.
- The single completion-launch boundary (lib.rs structural test) — untouched and
  still green, which is N4 checked by the build rather than asserted in prose.
