# Progress — T-053-01-02 · choices-not-essays

## Status — complete

| Step | State | Commit |
| --- | --- | --- |
| 1 — the reason step's presentation (`ui.rs`) | done | `8fd0ad9` |
| 2 — the step in plugin state and the keys that drive it | done | `719bc1b` |
| 3 — the sealed happy path, end to end | done | `7f0f562` |

`just check` — **exit 0** (fmt, clippy, `cargo check --target wasm32-wasip1`,
`cargo test --workspace`). 512 plugin tests (was 490), 277 core tests unchanged.

---

## Criterion 1 — the key sequence, demonstrated

The four keypresses, driven only through `handle_key` in
`four_keypresses_seal_a_parked_ticket`:

```
[d]      → the board lists what it can finish:  T-AFTER, T-PARKED
[j]      → down to T-PARKED (the parked one, one row in)
[Enter]  → the reason step opens, "The work already covers this — …" preselected
[Enter]  → signed
```

Four keypresses in, the assertions that hold:

- `T-PARKED` reaches `phase: done` **and** `status: done`
- `all_dependencies_done("T-AFTER")` is true — the dependent flows on the next pass
- the ledger carries one `OperatorOverride` row: `actor: "operator"`,
  `reason_id: "evidence-satisfies"`, the frozen catalog copy, and
  `overridden_ask: "Sign into Xcode with an Apple ID, then re-run the signed build."`

Three keypresses when the parked ticket is already under the cursor; the test
deliberately uses the one-row-of-travel case because that is the budget's worst
case, not its best.

Esc-reversibility at both depths is separate:
`esc_on_the_reason_step_returns_to_the_ticket_list` (both `Esc` and `q`, ticket
cursor preserved, nothing signed) and `esc_on_the_ticket_list_still_closes_the_modal`.

## What the modal looks like

Rendered from the real fixture at 50 columns, cursor one row down:

```
┌────────────────────────────────────────────────┐
│                Sign T-015-02-02                │
├────────────────────────────────────────────────┤
│ Sign into Xcode with an Apple ID, then re-run  │
│ the signed build.                              │
├────────────────────────────────────────────────┤
│   The work already covers this — the review a… │
│ ▸ This can't be checked from this machine — a… │
│   This is past what the ticket can reach — ac… │
├────────────────────────────────────────────────┤
│              Enter=sign  Esc=back              │
└────────────────────────────────────────────────┘
```

and on a ticket with no review at all:

```
├────────────────────────────────────────────────┤
│ No review was left for this ticket.            │
├────────────────────────────────────────────────┤
│ ▸ No agent review was left for this one — acc… │
```

No criterion quote, no serde error, no `codesign` flag — the two things the
0.4.4 field screenshot put on screen are both absent by construction, because
`ask_header_lines` cannot reach either field.

---

## Deviation — step 1 could not stand alone

**Planned:** step 1 was `ui.rs` only — "self-contained, compiles and tests alone
against the core catalog."

**Why it doesn't hold:** `lib.rs` builds `ui::ModalState` field-by-field at
9456, so adding a field to that struct breaks the plugin build until lib.rs
names it. `ModalState` derives `Default`, but this construction site does not
use `..Default::default()`.

**Done instead:** commit 1 carries one extra line in lib.rs — `reason_step:
None` at the projection — and the commit includes lib.rs. The real projection
lands in commit 2. The separation the plan wanted still holds in substance: a
renderer regression in commit 1 is attributable to `ui.rs`, because the only
lib.rs change is a literal `None`.

## Deviation — a retired test the plan did not anticipate

**Found while running step 2:**
`operator_recovery_matrix::blocked_disposition_rejects_operator_recovery_with_name_and_correlation`
failed. It drove `[d]` + Enter on a blocked-disposition ticket through
`submit_from_done_key` and asserted a `DispositionBlocked` rejection reached the
modal. That is exactly the dead end criterion 3 removes — the test was pinning
the behavior this ticket retires.

**Why this was not a licence to just delete it:** the test carries a second
claim that is still true and still worth pinning — the guard names its refusal
and correlates it to the operator's request. Only the *route* changed, not the
guard.

**Done instead**, three edits rather than a deletion:

1. `assert_named_rejection` split into `assert_named_rejection_event` (the
   dispatcher's log line) and `assert_named_rejection` (that plus the modal
   display). One rejection kind no longer reaches the modal, so the two claims
   are no longer one claim.
2. The blocked test now drives `mark_ticket_done` directly — the surface its
   claim is actually about — and uses the event-only assertion.
3. A new `blocked_disposition_no_longer_dead_ends_on_the_done_key` takes the old
   test's place at the old test's site, asserting the `[d]` path now opens the
   reason step and logs **no** `DispositionBlocked` rejection at all.

The retirement is recorded where someone looking for the old expectation will
find it, rather than vanishing from the diff.

## Deviation — two tests asserted on the wrong observable

`enter_on_a_passing_ticket_still_dispatches_without_a_reason_step` and
`enter_on_the_reason_step_signs_the_chosen_reason` were written against
`pending_completions`, following the T-053-01-01 tests that use `parked_fixture`.
They failed: `sealing_fixture` is journal-sealed with `lisa_bin: None`, so the
completion resolves inline and `pending_completions` is empty by the time the
key handler returns. Rewritten to assert the sealed outcome — `phase: done` plus
the ledger row — which is the better assertion anyway: it checks what the
operator got, not which intermediate map was populated.

---

## The red demonstration (criterion 3)

`every_listed_ticket_leads_somewhere` and `four_keypresses_seal_a_parked_ticket`
were run against the fork reverted — `override_choices_for` called and its
result discarded, so Enter always went to `mark_ticket_done`, i.e. today's
behavior before this ticket. Verbatim:

```
running 1 test
test tests::every_listed_ticket_leads_somewhere ... FAILED

failures:

---- tests::every_listed_ticket_leads_somewhere stdout ----

thread 'tests::every_listed_ticket_leads_somewhere' (12028409) panicked at crates/lisa-plugin/src/lib.rs:16116:13:
a blocked review dead-ends: neither sealed nor offering a signature

failures:
    tests::every_listed_ticket_leads_somewhere

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 511 filtered out
```

**Exit code 101**, judged by `$?` on a redirected run, not by reading output.
`four_keypresses_seal_a_parked_ticket` failed in the same state. The fork was
restored and the full suite re-run: **exit 0**.

## Criterion 4 — no free text, checked twice

`no_free_text_input_exists_in_the_flow` feeds every printable ASCII character
that is not a bound key (`j k q d r p D` and space) into **both** steps and
asserts the handler returns `false`, the ticket list is unchanged, the reason
cursor has not moved, the catalog is unaltered, and nothing sealed.

`the_modal_holds_no_typed_text` is the structural companion, in the shape of the
existing `completion_has_one_typed_request_gateway`: it slices the production
half of lib.rs and asserts neither `MarkDoneModal` nor `ReasonStep` carries a
`: String` field, an `input`, or a `buffer`. Honest limit: `TicketId` is an alias
for `String`, so an identifier field passes — the check targets prose fields,
which is the shape free text would actually arrive in.

## What was deliberately not touched

- **`mark_ticket_done`, `dispatch_completion`, `admit_operator_completion`,
  `passing_review_disposition`** — the guard is unchanged. The fork is above it,
  in the key handler, so `mark_ticket_done_keeps_its_fail_closed_behavior` and
  both T-053-01-01 negative fixtures are untouched and green. A completion
  carrying no operator-chosen reason still refuses to seal.
- **The `"Press [d] to mark done"` hint (ui.rs:750)** and the footer key legend —
  Design §7: after this ticket the sentence is true, which is the correction N3
  asks for. Rewording is S-053-02's surface.
- **`ModalMode`** — no fourth variant, so `ResetTicket` and `QuitConfirm` carry
  no arms for a state they cannot be in.
- **The single completion-launch boundary** — `completion_has_one_typed_request_gateway`
  untouched and green. N4 checked by the build.
