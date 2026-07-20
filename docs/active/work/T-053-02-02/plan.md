# T-053-02-02 — Plan

Five commits, each compiling green on its own. Verification is `just check`
(fmt, clippy, WASM check, workspace tests) judged **by exit code**, never by
reading grepped output.

---

## Step 1 — The status line tells the truth about the new estate

**Files:** `crates/lisa-plugin/src/ui.rs`

- Module doc (line 3): presets are cycled with `[v]`; `[p]` jumps to the desk.
- `render_status_line` (1362): the hard-coded hint tail becomes a
  `match state.active_view` over two strings (structure §ui.rs).
- `card_action_line`'s neighbouring test comment in
  `cards_advertise_only_keys_that_work` (2546) is rewritten — the assertion is
  unchanged, its stated reason is not.

**Tests**

- modify `test_status_line` (3113): `[p] desk`, `[v] view`.
- new `the_desk_status_line_names_the_desks_own_keys` — `active_view: Present`;
  asserts `[↑↓] pick`, `[enter] open`, `[d] done`, `[s] send back`, `[v] view`;
  asserts the line does **not** contain `[p]`.
- new `off_the_desk_the_status_line_never_offers_the_desks_keys` — loops
  Operations/Dag/Activity; asserts `[p] desk` present and `[s]`, `[enter]`
  absent.

**Verification.** `cargo test -p lisa-plugin` green. This step is provable
alone: nothing yet answers `[v]` or `[s]`, so the desk hint line is briefly a
hint without a key — which is why steps 2 and 5 land in the same ticket and this
step is never a stopping point. Noted rather than hidden.

**Commit:** `crates/lisa-plugin/src/ui.rs`

---

## Step 2 — `[p]` goes to the desk, `[v]` cycles, entering a view resets

**Files:** `crates/lisa-plugin/src/lib.rs`

- new `enter_view(&mut self, preset)` above `handle_key`: sets `view_preset`,
  zeroes `scroll_offset`, `desk_selected`, `desk_expanded`.
- `'p'` guard (8683): no-op returning `false` when already `Present`; otherwise
  `enter_view(Present)`.
- new `'v'` guard: `enter_view(self.view_preset.next())`.
- stale comment at 8682 rewritten.

**Tests**

- `p_lands_on_the_desk_from_every_preset_and_rests_there`: for each of the four
  presets, set it, press `p`, assert `view_preset == Present`; then press `p`
  again and assert the return value is `false` and nothing moved.
- `v_cycles_the_presets_in_the_old_order`: four presses from Operations walk
  Present → Dag → Activity → Operations.
- `entering_a_view_resets_cursor_expansion_and_scroll`: seed
  `desk_selected = 2`, `desk_expanded = true`, `scroll_offset = 7`; press `v`;
  assert all three are zero/false. Repeat entering the desk with `p` from
  Operations.

**Verification.** `cargo test -p lisa-plugin` green.
**Commit:** `crates/lisa-plugin/src/lib.rs`

---

## Step 3 — A running review is not a review wait

**Files:** `crates/lisa-plugin/src/lib.rs`

- `desk_state`'s review-wait pass (9390): skip tickets holding a
  `ThreadStatus::Running` thread. Doc sentence explaining why.

Lands **before** `[s]` because step 5's headline assertion depends on it.

**Tests**

- `a_running_review_is_not_a_review_wait`: a Review-phase ticket cards; attach a
  Running thread for it; assert the desk is empty. Then flip the thread to
  `Parked` and assert the card returns — the class the card was built for is
  untouched.

**Regression watch.** `desk_cards_are_grouped_and_ordered_by_ticket_id` (10857)
includes `T-REVIEW` with no thread; it must stay green unmodified. If it does
not, the predicate is wrong, not the test.

**Verification.** `cargo test -p lisa-plugin` green.
**Commit:** `crates/lisa-plugin/src/lib.rs`

---

## Step 4 — `[d]` on a card enters the reason flow already scoped

**Files:** `crates/lisa-plugin/src/lib.rs`

- new `desk_cards()`; `desk_card_count()` re-expressed over it.
- new `open_desk_signature(&mut self, ticket_id)` (structure §lib.rs).
- the desk branch is hoisted above the global `p`/`v`/`d`/`r` guards, computes
  `cards` once, and gains `Char('d')`. Up/Down/Enter bodies unchanged except
  that they now read `cards.len()` instead of calling `desk_card_count()`.

**Tests**

- `d_on_a_desk_card_opens_the_reason_step_already_scoped_to_it`: `sealing_fixture`
  with the Xcode block, `view_preset = Present`; press `d`; assert
  `modal.open`, `modal.ticket_ids[modal.cursor] == "T-PARKED"`, and the reason
  step is open on `T-PARKED` with the recommendation preselected.
- `two_keypresses_seal_the_selected_card`: same fixture; `d` then `Enter`;
  assert `T-PARKED` reaches `Phase::Done`, `T-AFTER`'s dependencies are done,
  and the ledger carries an `OperatorOverride` receipt. This is criterion 2's
  "seals it end-to-end", driven only through `handle_key`.
- `d_on_a_desk_card_for_an_unfinishable_ticket_falls_back_to_the_plain_modal`:
  a desk whose selected card names a ticket the modal does not list; assert the
  modal opened unscoped rather than nothing happening.

**Verification.** `cargo test -p lisa-plugin` green; `four_keypresses_seal_a_parked_ticket`
and `no_free_text_input_exists_in_the_flow` must stay green untouched — the
first proves the board path still works, the second that the desk block did not
leak into modal mode.

**Commit:** `crates/lisa-plugin/src/lib.rs`

---

## Step 5 — `[s]` sends a block back for another review pass

**Files:** `crates/lisa-plugin/src/lib.rs`

- new `send_back_for_review(&mut self, ticket_id)` (structure §lib.rs): resolve
  file path, refuse unless `Blocked`, `update_ticket_status(Open)`, log, rebuild.
- desk branch gains `Char('s')`, gated on `DeskCardKind::Block |
  DeskCardKind::NoReviewOnFile`; every other card returns `false`.

**Tests**

- `s_returns_a_parked_ticket_to_review_and_its_card_leaves_on_the_next_poll`:
  armed `sealing_fixture`; assert one Block card; press `s`; assert the ticket
  file on disk reads `status: open`, the DAG agrees, and the Block card is gone;
  then run `schedule_ready_tickets()` (the poll) and assert the desk is empty
  and a thread exists for the ticket.
- `s_does_nothing_on_a_note_card_or_a_review_wait_card`: build a desk with a
  review wait and a note; for each, select it, press `s`, assert the return
  value is `false`, the ticket file is byte-identical, and the card list is
  unchanged. Byte comparison rather than a status re-read, so an unrelated
  frontmatter rewrite would also fail.
- `desk_keys_are_inert_on_an_empty_desk`: empty desk; press `s`; assert `false`
  and no state moved. (`d` on an empty desk opens the ordinary modal, which is
  the global key, and is asserted as such.)
- `send_back_refuses_a_ticket_that_is_no_longer_blocked`: call
  `send_back_for_review` directly on an open ticket; assert the file is
  unchanged and a Warning was logged — the stale-card guard.

**Verification.** `just check` green by exit code — the full gate, including
fmt, clippy, and the WASM target build.

**Commit:** `crates/lisa-plugin/src/lib.rs`

---

## Testing strategy

**Unit, at the renderer seam (ui.rs).** The status line is a pure function of
`PluginState`; every hint claim is asserted there, both what is present and what
is absent. Absence assertions matter more than presence here — an N3 violation
is a hint that survived a key's removal.

**Integration-style, through `handle_key` (lib.rs).** Every criterion that names
a keypress is driven by `press(&mut state, …)` against a temp-dir `State` with
a real ticket directory, so the assertions run against files on disk and a real
DAG rebuild — not against a mocked seam. Criterion 2's "seals it end-to-end" and
criterion 4's negative fixture both work this way.

**What is deliberately not tested.** No test drives a real Zellij render or a
real pane spawn; `schedule_ready_tickets` in step 5 exercises the scheduler
natively, which is as close to a poll as the harness reaches. This is the
existing boundary in this crate, not a new one.

## Verification criteria

1. `just check` exits 0.
2. Every criterion in the ticket maps to a named test in the table above.
3. No test from T-053-02-01 or T-053-01-02 is deleted; the two modified ones
   (`test_status_line`, and the comment in `cards_advertise_only_keys_that_work`)
   are named in review.md with what changed and why.

## Risks

- **Status-line width.** The desk hint tail is 73 characters against 44; with
  the counts prefix a narrow terminal may wrap. There is no clamp today
  (research §2) and adding one is out of scope; if the wrap is unacceptable in
  the field, the fix is a clamp on the title bar, not shorter hints.
- **Native spawn in step 5.** If `schedule_ready_tickets` cannot spawn under the
  journal-seal fixture, the fallback is to assert the two facts separately — the
  ticket becomes runnable (`dag.can_start`), and a ticket with a running thread
  has no card — and say so plainly in review.md rather than dropping the claim.
</content>
</invoke>
