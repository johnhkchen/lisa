# Review — T-053-01-02 · choices-not-essays

T-053-01-01 built the branch that lets an operator sign a parked completion, but
nothing reached it — pressing `[d]` on a parked ticket still died in a rejection
modal. This ticket wires the keypress: the MarkDone modal gains a second step
that shows what the block asked for, offers the reasons that honestly fit it,
and preselects the recommended one. Four keypresses from the board to a sealed
ticket with a receipt.

## What changed

**`crates/lisa-plugin/src/ui.rs`**

| Added | Role |
| --- | --- |
| `ReasonStepState` | the step's UI-side shape: ticket, `OverriddenAsk`, choices, cursor |
| `ModalState.reason_step` | `Option<ReasonStepState>` — `None` means the ticket list is showing |
| `ask_header_lines` | **the one place criterion 2's copy rule lives** |
| `fit_modal_line` | character-measured truncation for the choice rows |
| `render_reason_step_modal` | the step's box |
| `render_modal` dispatch arm | outcome first, then step |

**`crates/lisa-plugin/src/lib.rs`**

| Added / changed | Role |
| --- | --- |
| `ReasonStep` | the step's plugin-side state, with **its own cursor** |
| `MarkDoneModal.reason_step` | the sub-state; three construction sites take `None` |
| `handle_key` reason layer | Esc/q = back, j/k/arrows = move, Enter = sign |
| `handle_key` MarkDone Enter arm | forks on `override_choices_for` |
| `open_reason_step`, `confirm_reason_step` | open with the recommendation preselected; sign with the cursor's choice |
| `#[allow(dead_code)]` removed | from `override_choices_for` and `mark_ticket_done_with_override` — they have a caller now |
| the `ui::ModalState` projection | one more field-copy |

**`crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`** — the retired
dead end, handled explicitly rather than deleted (see *Deviations*).

No files created or deleted. **Nothing in `lisa-core` changed**: the catalog,
the disposition reader, the ledger, and the completion machinery are read, not
edited.

## Acceptance criteria

**1 — ≤4 keypresses, each step Esc-reversible, sequence in progress notes.**
`four_keypresses_seal_a_parked_ticket` drives `d`, `j`, `Enter`, `Enter` through
`handle_key` alone and asserts the ticket reaches `phase: done` / `status: done`,
its dependent's `all_dependencies_done` goes true, and the ledger carries the
operator receipt with the right actor, reason id, frozen copy, and overridden
ask. The sequence is transcribed in `progress.md`. Reversibility is separate:
`esc_on_the_reason_step_returns_to_the_ticket_list` (both `Esc` and `q`; ticket
cursor preserved; nothing signed) and `esc_on_the_ticket_list_still_closes_the_modal`.

**2 — the ask verbatim; fail-closed says so plainly.**
`reason_step_shows_the_blocks_ask_verbatim` walks the ask word-for-word in order
through the rendered lines (it wraps, so a substring check would be wrong).
`reason_step_never_prints_the_parse_error` asserts no `malformed`, `JSON`,
`column`, or `expected value` survives to the screen for an `UnreadableReview`,
and that the plain sentence does; `reason_step_says_plainly_when_no_review_is_on_file`
covers the missing case. `reason_step_never_prints_the_blocks_technical_reason`
keeps `codesign` off the screen — the old dump does not return one line lower.

The enforcement is structural, not incidental: `ask_header_lines` destructures
the two fail-closed variants with `{ .. }`, so the parse detail and the block's
technical reason are *unreachable* from the function, not merely unused.

**3 — no dead ends; the modal state machine's arms unit-tested.**
Four arm tests — `enter_on_a_blocked_ticket_opens_the_reason_step`,
`enter_on_a_ticket_with_no_review_...`, `enter_on_an_unreadable_review_...`,
`enter_on_a_passing_ticket_still_dispatches_without_a_reason_step` — plus
`every_listed_ticket_leads_somewhere`, which sweeps all four verdicts and
asserts each either seals or offers a signature, and that **no**
`DispositionBlocked` rejection is produced or displayed on the `[d]` path.

The claim is structural: the three verdicts that produced `DispositionBlocked`
now route to the step, and the step lists only `applicable_reasons()`, which
`build_operator_override` cannot refuse. The rejection modal survives only for
the four rejection kinds that have an honest one-sentence explanation
(already-pending, stale lease, dependency, launch failure).

Demonstrated red: with the fork reverted, `every_listed_ticket_leads_somewhere`
fails at **exit 101** with *"a blocked review dead-ends: neither sealed nor
offering a signature"*; transcript verbatim in `progress.md`. Restored, exit 0.

**4 — no free-text input anywhere.** `no_free_text_input_exists_in_the_flow`
feeds every printable ASCII character that is not a bound key into both steps
and asserts nothing accumulates, moves, or seals. `the_modal_holds_no_typed_text`
is the structural companion, asserting neither modal struct grows a `: String`,
`input`, or `buffer` field.

**5 — `just check`.** Exit 0: fmt, clippy, `cargo check --target wasm32-wasip1`,
`cargo test --workspace`. Judged by `$?`.

## Test coverage

| Area | New tests | Suite |
| --- | --- | --- |
| `ui::tests` (rendering and copy) | 7 | — |
| `lib.rs::tests` (state machine, keys, seal) | 14 | 512 plugin tests green (was 490) |
| `operator_recovery_matrix` | 1 added, 1 rerouted | (same suite) |
| `lisa-core` | 0 | 277 core tests unchanged |

The two carrying the most weight are `every_listed_ticket_leads_somewhere` — the
only test that states criterion 3 as a property over all verdicts rather than
one arm at a time, and the one demonstrated red — and
`reason_step_navigation_moves_only_its_own_cursor`, which pins the sharpest
integration hazard: `operator_modal_targets` resolves the acting ticket through
`modal.cursor`, so a shared cursor would have retargeted completion feedback at
whatever ticket sat at the chosen reason's index. That test asserts the ticket
cursor holds still *and* that `operator_modal_targets("T-PARKED")` still returns
true after navigating the reason list.

**Gap I did not close:** there is no test that a *rendered* dashboard shows the
reason step through the full `render_dashboard_lines` path. The renderer is
tested through `render_modal` directly, and the projection is a plain field-copy
tested by the state-machine tests, but the seam between them — `plugin_state()`
building `ui::ModalState` — is exercised only by the type checker. The existing
modal tests have the same shape, so this is consistent with the file rather than
a new gap, but a reviewer who wants the composed path covered should say so.

## Deviations from plan

Three, all written into `progress.md` with rationale:

1. **Step 1 could not stand alone.** `lib.rs` builds `ui::ModalState`
   field-by-field, so the field had to be named there for commit 1 to compile.
   Commit 1 carries one literal `None`; the real projection is commit 2.
2. **A retired test the plan did not anticipate.**
   `blocked_disposition_rejects_operator_recovery_with_name_and_correlation`
   pinned the exact dead end criterion 3 removes. Rather than delete it, its two
   claims were separated: the guard's naming still holds and is now tested
   against the dispatcher directly, and a new
   `blocked_disposition_no_longer_dead_ends_on_the_done_key` records the
   retirement at the old test's site.
3. **Two tests asserted on `pending_completions`** when `sealing_fixture` seals
   inline. Rewritten to assert the sealed outcome and the ledger row — what the
   operator actually got.

## Open concerns

1. **Choice rows are truncated, not wrapped.** A catalog summary is 60–90
   characters against a 48-column inner width, so each row shows roughly its
   first 46 and an ellipsis. The distinguishing words are at the front of every
   entry ("The work already covers this", "This can't be checked from this
   machine", "This is past what the ticket can reach"), so the list is still
   scannable — but an operator on a narrow terminal reads less of the sentence
   they are signing than the receipt will record. Design chose the glance over
   completeness; a reviewer may reasonably prefer wrapping.

2. **The one-item list on fail-closed tickets is deliberate ceremony.** A ticket
   with no readable review offers exactly one reason, so the step renders a
   single-choice list. Auto-confirming would have saved a keypress and collapsed
   "decide with context" into an accident, so it renders anyway. It looks like a
   formality; it is meant to.

3. **`UnreadableReview`'s parse error still reaches the receipt.** Criterion 2
   binds the *screen*, and the screen is clean. But the note's `criterion_quote`
   still carries `review disposition is malformed JSON: …` into Notes-for-you,
   because T-053-01-01's criterion asked the quote to record the literal state
   overridden. So a person who signs a corrupt-review ticket sees plain language
   at signing time and serde text in the receipt afterward. Both tickets are
   individually correct; the seam between them is where a reader might be
   surprised. Worth a look when S-053-02 reworks the notes surface.

4. **The `"Press [d] to mark done"` hint is unchanged.** After this ticket it is
   true, which is the N3 correction — but E-053 called it the type specimen and
   a reviewer expecting a string change in this diff will not find one. Design
   §7 records the reasoning; the surface belongs to S-053-02.

5. **The reason step captures `OverriddenAsk` at open time.** If the disposition
   file changes on disk between choosing the ticket and pressing Enter, the step
   signs against what it showed, not what is now on disk. That is the correct
   behavior — an operator signs what they read — but it means the receipt can
   quote a superseded ask in a race no test covers. The window is a few hundred
   milliseconds of human reaction time.
