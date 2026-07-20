# T-053-02-01 — Plan: ordered implementation

Six commits, each through `lisa commit-ticket` with exact `--include` paths. Every step
names what verifies it. `just check` = `cargo check -p lisa-plugin --target wasm32-wasip1`
+ `fmt-check` + `clippy` + `cargo test --workspace` (justfile:53-57).

---

## Step 1 — the core projection grows what the expanded card needs

**Files:** `crates/lisa-core/src/parking.rs`, `crates/lisa-cli/src/status.rs`

1. Extract the ledger walk in `latest_park_attempt_leases` into a private
   `latest_park_records` returning the whole latest Park record per ticket. Re-express
   `latest_park_attempt_leases` over it (signature and behavior unchanged).
2. Add `pub fn latest_park_stamps(&Path) -> HashMap<String, u64>` returning `ended_at`.
3. Add `pub steps: Vec<String>` to `ParkedRemedy`; populate from
   `ReviewDisposition::Block { steps, .. }` with `unwrap_or_default()`.
4. Add `steps: Vec::new()` to the four `ParkedRemedy` literals in parking.rs's own tests and
   the five in status.rs's tests.

**Tests (new, in parking.rs):**
- `park_stamps_track_the_latest_park_and_clear_on_unpark` — extends the existing Park/Unpark
  fixture; stamp present after Park, absent after Unpark.
- `collects_a_blocks_prepared_steps` — a block with `steps` surfaces them; one without
  surfaces an empty vec.

**Verify:** `cargo test -p lisa-core -p lisa-cli`. This is the one step that is not green
in isolation until 1.4 lands, which is why 1.4 is in the same commit.

**Commit:** `feat(core): carry a block's steps and its park stamp to the dashboard`
`--include crates/lisa-core/src/parking.rs crates/lisa-cli/src/status.rs`

---

## Step 2 — desk types, the fourth preset, the shared ask copy

**File:** `crates/lisa-plugin/src/ui.rs`

1. `DeskCardKind`, `DeskCard`, `DeskDetail`, `DeskState` next to `WaitingItem`/`NoteItem`.
2. `WaitingItem` gains `steps: Vec<String>` and `check: Option<String>`.
3. `ViewPreset::Present` between `Operations` and `Dag`; `next()` and `label()` updated;
   module doc "three preset views" → four.
4. `PluginState.desk: DeskState`, added to the hand-written `Default` impl.
5. Lift the two fail-closed sentences out of `ask_header_lines` into
   `pub(crate) const NO_REVIEW_ASK` / `UNREADABLE_REVIEW_ASK`; `ask_header_lines` references
   them. Behavior identical — its existing tests are the regression check.

At the end of this step ui.rs compiles with the desk carried but unrendered, and the
existing `WaitingItem` literals in ui.rs and lib.rs need the two new fields; that churn is
part of this commit.

**Verify:** `just check-wasm && cargo test --workspace`. `ask_header_lines`'s existing tests
must pass untouched — that is the proof step 2.5 was a pure lift.

**Commit:** `feat(plugin): give the dashboard a desk card and a fourth preset`

---

## Step 3 — the desk renders; Operations stops printing paragraphs

**File:** `crates/lisa-plugin/src/ui.rs`

1. Add `desk_card_lines`, `render_present_view`, `render_desk_pointer` per structure.md §2f.
2. Delete `render_waiting_on_you` and `render_notes_for_you` and their six tests.
3. Rename `render_attention_banner` → `render_health_alerts`; remove the review-row
   collection, the `parked_by_ticket` lookup, the review-row loop, and the
   `"Press [d] to mark done"` hint; early-return on `state.alerts.is_empty()`.
4. Rewrite the banner tests that asserted review rows; keep and rename the alert-only ones.
5. `render_operations_view`: `render_desk_pointer` + `render_health_alerts` in place of the
   three deleted/moved calls. `render_dashboard_lines` dispatches `Present`.

**Tests (new):**

| test | asserts |
|---|---|
| `desk_renders_five_collapsed_cards_with_no_staff_work_visible` | five cards, each exactly 3 lines; joined output contains no criterion quote, no evidence path, no reason text, no check command (AC 1) |
| `a_card_without_a_stamp_shows_the_dash` | the note card and a stampless block render `—`, and no digits appear in their age cell (AC 1) |
| `expanding_reveals_staff_work_for_the_selected_card_only` | with `selected: 1, expanded: true`, card 1 shows reason + criterion + evidence + step + check; cards 0/2/3/4 still 3 lines each (AC 2) |
| `collapsing_restores_the_three_line_shape` | same fixture with `expanded: false` renders byte-identically to the collapsed test (AC 2) |
| `operations_shows_pointer_lines_not_paragraphs` | `render_operations_view` output contains the pointer with true counts and none of `Reviewer's note:`, `Criterion:`, `Evidence:`, `Waiting on you`, `Notes for you`, `Press [d] to mark done` (AC 3) |
| `empty_desk_is_one_calm_sentence` | `render_present_view` on a default state emits exactly `["Nothing needs you."]` — no box chars, no header (AC 4) |
| `collapsed_lines_carry_no_mechanism_vocabulary` | tokenized absence of `disposition`, `frontmatter`, `dag`, `seal` across every collapsed line of the fixture (AC 5) |
| `asks_render_verbatim_from_their_disposition_fields` | each card's ask appears character-for-character on line 2, including the field jargon wall specimen (AC 5) |

A shared `fn desk_fixture() -> DeskState` builds the five-card fixture once. The verbatim
test deliberately feeds the pinned field specimen (the 60-word codesign wall already in
ui.rs's tests) as one block's ask, so the "no summarization pass" boundary is executable:
the desk truncates for width but never rewrites.

**Verify:** `just check`. AC 3's "the paragraph renderers no longer run" is proven by the
functions not existing — a compile-level guarantee, plus the negative string assertions.

**Commit:** `feat(plugin): render the desk and collapse Operations to a pointer`

---

## Step 4 — the plugin assembles real cards

**File:** `crates/lisa-plugin/src/lib.rs`

1. `to_ui_state`'s `waiting_items` projection stops dropping `steps`/`check`.
2. `fn desk_state(...) -> ui::DeskState` — the four groups in fixed order, park stamps read
   once from `latest_park_stamps`.
3. `fn fail_closed_desk_cards(...)` — blocked ticket ids absent from `waiting`, run through
   `observed_override_state`, keeping only the two fail-closed shapes.
4. `State.desk_selected` / `State.desk_expanded`; `desk` wired into the returned
   `ui::PluginState`.

**Tests (new, in lib.rs):**
- `desk_state_gives_a_blocked_ticket_with_no_disposition_its_own_card` — a temp work dir with
  one blocked ticket whose disposition file is missing and another whose file is malformed;
  both get cards, wearing `NO_REVIEW_ASK` and `UNREADABLE_REVIEW_ASK` respectively, and
  neither ask contains the raw parse error.
- `desk_state_ages_a_parked_block_from_the_ledger_stamp` — a Park record in the ledger
  yields a non-`—` age; no record yields `—`.
- `desk_state_orders_cards_deterministically` — blocks, fail-closed, review waits, notes,
  each group by ticket id.

**Verify:** `just check`.

**Commit:** `feat(plugin): assemble desk cards from parks, reviews, notes, and silent blocks`

---

## Step 5 — the desk's own keys

**File:** `crates/lisa-plugin/src/lib.rs`

A `Present`-guarded branch in `handle_key`, placed after modal handling and before the
existing `j`/`k` scroll branch: Up/`k` and Down/`j` move `desk_selected` (clamped to the
live card count, collapsing `desk_expanded`), Enter toggles `desk_expanded`.

**Tests (new):**
- `desk_keys_select_and_expand_only_on_the_present_view` — under `Present`, Down moves the
  selection and leaves `scroll_offset` at 0; Enter toggles expansion. Under `Operations`,
  the same keys scroll and leave `desk_selected`/`desk_expanded` untouched.
- `desk_selection_is_clamped_to_the_card_count` — Down at the last card is a no-op; a
  selection past the end renders without panicking.

**Verify:** `just check`.

**Commit:** `feat(plugin): let the desk select a card and open it one keypress deep`

---

## Step 6 — Review

Write `review.md` (change summary, coverage, open concerns, and the AC 5 card-copy check
recorded explicitly) and `review-disposition.json`, then run
`lisa check-disposition T-053-02-01`.

---

## Testing strategy

**Unit, ui.rs** — pure renderer tests over hand-built `DeskState` fixtures. This is where
every card-shape criterion (AC 1, 2, 4, 5) is proven, because they are all statements about
rendered lines and none of them needs a filesystem.

**Unit, lib.rs** — assembly tests over a temp work dir and ledger. This is where the fifth
class and the age sourcing are proven, because both are statements about reading the world.

**Unit, parking.rs** — the two new core behaviors, in the file that owns them.

**No integration test.** Every criterion is reachable at one of the two seams above, and
the plugin has no harness that drives a real Zellij render.

**Known coverage gap, to be stated in review.md rather than papered over:** nothing asserts
that the desk and the Operations pointer agree at *runtime* — the pointer counts the same
`state.desk.cards` the desk renders, so they agree structurally, but that is an argument
from construction, not a test.

## Risks

- **Deleting tests loses coverage.** Mitigation: every assertion in the six deleted tests
  has a named successor in the step 3 table — the world-waiting suffix, the legacy-ask
  substitution, and the note's summary/criterion/evidence triple all reappear as desk-card
  assertions.
- **Test churn in status.rs.** Bounded to five literals; if it turns out larger,
  `#[derive(Default)]` on `ParkedRemedy` plus `..Default::default()` is the fallback.
- **`[d]` on a card is honest only while the T-053-01 chain holds.** If `open_mark_done_modal`
  ever stops listing blocked tickets, the action line becomes an N3 violation. Nothing in
  this ticket can pin that; it is named in review.md as an open concern.
