# Plan — T-053-01-02 · choices-not-essays

Three commits. Each is independently buildable and testable; the second and
third are the pair Structure said cannot split.

Baseline to preserve: 490 plugin tests, 277 core tests, `just check` exit 0.

---

## Step 1 — the reason step's presentation (`ui.rs`)

**Commit:** `feat(plugin): render the reason step's ask and canned choices`

**Changes**
- `use lisa_core::operator_override::{OverriddenAsk, OverrideReason};`
- `pub struct ReasonStepState { ticket_id, ask, choices, cursor }`
- `ModalState { .., pub reason_step: Option<ReasonStepState> }`; add the field at
  the three explicit constructions (ui.rs:3413, 3431, 3468)
- `fn ask_header_lines(&OverriddenAsk, width) -> Vec<String>`
- `fn render_reason_step_modal(&ReasonStepState, width, height) -> Vec<String>`
- `render_modal` dispatch arm (after the outcome arm)

**Include:** `crates/lisa-plugin/src/ui.rs`

**Tests (ui.rs `mod tests`)**

| Test | Pins |
| --- | --- |
| `reason_step_shows_the_blocks_ask_verbatim` | the `XCODE_ASK` sentence appears in the rendered lines, unwrapped-and-rejoined equality — criterion 2 first half |
| `reason_step_never_prints_the_parse_error` | `UnreadableReview { detail: "…malformed JSON: expected value at line 1 column 1" }` renders no `"malformed"`, no `"JSON"`, no `"column"`; and *does* render the plain sentence — criterion 2 second half |
| `reason_step_never_prints_the_blocks_technical_reason` | `Block { ask, reason: "codesign refused: no signing identity found" }` renders no `"codesign"` — the dump stays gone |
| `reason_step_lists_every_applicable_choice_and_marks_the_cursor` | three rows for a Block, one `▸`, at the cursor index |
| `reason_step_rows_fit_the_box` | every line's `chars().count()` equal across the box — catches the `len()`-vs-`chars()` trap on em dashes |
| `reason_step_footer_offers_only_sign_and_back` | footer contains `Enter`, `Esc`, and no other key name — the no-free-text surface, checked where it renders |

**Verify:** `cargo test -p lisa-plugin ui::tests` exit 0. Renderer is unreachable
from the plugin at this point — that is expected and the commit says so.

---

## Step 2 — the step in plugin state, and the keys that drive it (`lib.rs`)

**Commit:** `feat(plugin): give the mark-done modal a reason step before it signs`

Structure §"Ordering" — the field and its driver land together: a written-nowhere
field is dead weight, and dropping `#[allow(dead_code)]` before a caller exists
fails clippy.

**Changes**
- `struct ReasonStep { ticket_id, ask, choices, cursor }`
- `MarkDoneModal { .., reason_step: Option<ReasonStep> }` + three construction
  sites (8730, 8813, 8926) take `None`
- `handle_key`: the reason-step layer inserted **after** the `operator_outcome`
  layer (8581) and **before** the shared list layer (8583)
- `handle_key`: the `ModalMode::MarkDone` Enter arm forks on
  `override_choices_for`
- `fn open_reason_step`, `fn confirm_reason_step`
- drop `#[allow(dead_code)]` from `mark_ticket_done_with_override` (8753) and
  `override_choices_for` (8765); reword their "T-053-01-02 will call this"
  comments into statements of the caller
- the `ui::ModalState` projection arm (9456)

**Include:** `crates/lisa-plugin/src/lib.rs`

**Tests (lib.rs `mod tests`, alongside the T-053-01-01 block at ~15220, reusing
`parked_fixture` / `XCODE_BLOCK` / `XCODE_ASK`)**

Criterion 3's "the modal state machine's arms are unit-tested" — one test per
arm of the two forks:

| Test | Arm |
| --- | --- |
| `enter_on_a_blocked_ticket_opens_the_reason_step` | Block → step, not dispatch: `reason_step.is_some()`, `pending_completions.is_empty()`, `operator_outcome.is_none()` |
| `enter_on_a_ticket_with_no_review_opens_the_reason_step` | missing file → step, `ask == NoReviewOnFile` |
| `enter_on_an_unreadable_review_opens_the_reason_step` | malformed → step, `ask` is `UnreadableReview` |
| `enter_on_a_passing_ticket_still_dispatches_without_a_reason_step` | Pass → today's path unchanged: pending completion exists, `reason_step.is_none()` |
| `the_reason_step_preselects_the_recommendation` | cursor lands on `EvidenceSatisfies` for a Block, `NoReviewOnFile` for the two fail-closed shapes |
| `reason_step_navigation_moves_only_its_own_cursor` | `j`/`k` on the step change `reason_step.cursor` and leave `modal.cursor` fixed — research §5's hazard, pinned |
| `esc_on_the_reason_step_returns_to_the_ticket_list` | `reason_step` cleared, `modal.open` still true, `modal.cursor` preserved |
| `esc_on_the_ticket_list_still_closes_the_modal` | outer Esc unchanged |
| `unknown_keys_on_the_reason_step_are_ignored` | a char key returns false, mutates nothing — the no-free-text surface at the handler |

**Verify:** `cargo test -p lisa-plugin` exit 0, count ≥ 490 + new.

---

## Step 3 — the sealed happy path, end to end

**Commit:** `test(plugin): pin four keypresses from the board to a sealed ticket`

**Changes:** tests only.

**Include:** `crates/lisa-plugin/src/lib.rs`

| Test | Pins |
| --- | --- |
| `four_keypresses_seal_a_parked_ticket` | criterion 1. Drive `handle_key` only: `d`, `j`, `Enter`, `Enter` on a two-ticket fixture where the parked one is second. Assert the sealed outcome: `pending_completions["T-PARKED"]` carries an `operator_override` whose `reason()` is the recommendation and whose `overridden_ask()` is `XCODE_ASK`, and the admitted note's `summary()` is that reason's `summary()`. The exact sequence goes verbatim into `progress.md` — criterion 1's "demonstrated in progress notes". |
| `every_listed_ticket_leads_somewhere` | criterion 3, as a sweep rather than a claim: a fixture with one ticket per verdict (pass, block, missing, malformed), each opened and Entered; assert each ends in **either** a pending completion **or** a reason step, and that **no** `OperatorModalOutcome::Rejected { kind: DispositionBlocked, .. }` is ever produced from the `[d]` path. |
| `no_free_text_input_exists_in_the_flow` | criterion 4. Feed every printable-character key that is not a bound navigation key into both steps; assert no state field changes and no ticket seals. A structural companion asserts the plugin has no `String`-accumulating modal field — `grep`-shaped, kept as a source-level assertion the way lib.rs's existing single-launch-boundary test works. |

**Verify:** `cargo test -p lisa-plugin` exit 0.

---

## Closing

**`just check`** — fmt, clippy, `cargo check --target wasm32-wasip1`, `cargo
test --workspace`. Judged by **exit code**, not by reading output. A non-zero
exit is a Review block, not a footnote.

Then `progress.md` (with the verbatim keypress sequence), `review.md`, and
`review-disposition.json`; then `lisa check-disposition T-053-01-02`.

---

## Testing strategy

**Unit, no integration.** Every criterion is reachable from `handle_key` and
`render_modal`, both pure functions over state. The completion machinery
underneath was integration-tested by T-053-01-01 — re-testing it here would be
testing the previous ticket.

**Assert on outputs, not internals**, where the criterion is about outcome:
step 3 asserts the *pending completion's receipt*, not that
`confirm_reason_step` was called.

**The negative criteria get positive tests.** "No free text" and "no parse
error on screen" are absence claims, which rot silently. Both are pinned by
tests that would fail the moment the absence stops holding — a rendered-lines
scan for the forbidden substrings, and a key sweep over the printable range.

## Risks and their step

| Risk | Step | Mitigation |
| --- | --- | --- |
| The reason layer placed wrong in `handle_key` → Esc closes the modal, or `j`/`k` move the hidden ticket cursor | 2 | `reason_step_navigation_moves_only_its_own_cursor`, `esc_on_the_reason_step_returns_to_the_ticket_list` |
| Byte-length padding breaks the box on em dashes | 1 | `reason_step_rows_fit_the_box` |
| `operator_modal_targets` retargeted by a shared cursor | 2 | separate cursor by construction; step 3's seal test would fail if outcome targeting broke |
| A verdict shape nobody listed still dead-ends | 3 | `every_listed_ticket_leads_somewhere` sweeps all four |
| Clippy fails on the removed `#[allow(dead_code)]` if a caller is missed | 2 | the attribute removal and the caller are in the same commit |

## Deviation protocol

Any departure from this plan is written into `progress.md` — what was planned,
why it does not hold, what was done instead — **before** the code changes.
