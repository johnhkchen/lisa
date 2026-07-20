# T-053-02-01 — Structure: file-level blueprint

Four files change. No files are created or deleted. Ordering matters and is stated at the
end.

---

## 1. `crates/lisa-core/src/parking.rs`

### 1a. Share the ledger scan (replaces the body of `latest_park_attempt_leases`)

The existing scan (parking.rs:86-104) keeps only the attempt lease. Extract the walk into
one private helper that keeps the whole latest Park record per ticket, and express both
public views over it.

```
fn latest_park_records(ledger_path: &Path) -> HashMap<String, ParkingTransitionRecord>
    // same Park-inserts / Unpark-removes rule as today, unchanged

pub fn latest_park_attempt_leases(&Path) -> HashMap<String, AttemptLease>   // signature unchanged
pub fn latest_park_stamps(&Path) -> HashMap<String, u64>                    // new: ticket_id -> ended_at
```

`latest_park_attempt_leases` keeps its exact signature and semantics — crates/lisa-cli/src/proposal.rs:84
and the existing test `latest_unpark_removes_the_current_park_lease` (parking.rs:412) both
continue to compile and pass untouched.

`latest_park_stamps` returns `ended_at` (the epoch second the park was recorded,
lib.rs:6916/6932). A ticket with no Park record is simply absent from the map.

### 1b. `ParkedRemedy` gains `steps`

```
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub reason: String,
    pub steps: Vec<String>,   // NEW — empty when the block supplied none
    pub check: Option<String>,
    pub proposal: Option<TriageProposal>,
}
```

Populated in `collect_parked_remedies` by destructuring `steps` out of
`ReviewDisposition::Block` (disposition.rs:86) and `unwrap_or_default()`-ing the `Option<Vec>`.
`Vec<String>` rather than `Option<Vec<String>>`: "no steps" and "empty steps" are the same
fact to a renderer, and it keeps the four existing `ParkedRemedy` literals in this file's
tests to a one-line addition.

### 1c. Tests added here

- `park_stamps_track_the_latest_park_and_clear_on_unpark` — extends the existing
  Park/Unpark fixture to assert the stamp appears and disappears alongside the lease.
- `collects_a_blocks_prepared_steps` — a block disposition carrying `steps` surfaces them;
  one carrying none surfaces an empty vec.

---

## 2. `crates/lisa-plugin/src/ui.rs`

### 2a. New types (near `WaitingItem`/`NoteItem`, ui.rs:166-183)

`DeskCardKind`, `DeskCard`, `DeskDetail`, `DeskState` exactly as spelled in design.md §D2,
plus:

```
pub struct DeskState {
    pub cards: Vec<DeskCard>,
    pub selected: usize,   // index into cards; clamped by the renderer
    pub expanded: bool,    // whether the selected card shows its staff work
}
```

`DeskState` derives `Debug, Clone, Default`. `DeskCard`/`DeskDetail` derive
`Debug, Clone, PartialEq` (`TriageProposal` already derives `PartialEq`, per parking.rs's
own assertions). `DeskDetail` gets a `Default` so `Note`/`ReviewWait` cards stay terse to
construct in fixtures.

### 2b. `ViewPreset` gains `Present` (ui.rs:376-405)

```
pub enum ViewPreset { Operations (default), Present, Dag, Activity }
next():  Operations -> Present -> Dag -> Activity -> Operations
label(): "Present"
```

The module doc comment at ui.rs:3-6 ("three preset views") is updated to four.

### 2c. `PluginState` gains `desk: DeskState` (ui.rs:409-446)

Added to the struct and to the hand-written `Default` impl (ui.rs:428-446).

### 2d. Deleted: `render_waiting_on_you` (ui.rs:527-558), `render_notes_for_you` (ui.rs:561-576)

Their `Waiting on you` / `Notes for you` section banner is gone from the default screen.

`WaitingItem` and `NoteItem` themselves survive: they remain the projections `to_ui_state`
builds, and the desk assembler consumes them. `WaitingItem` grows `steps: Vec<String>` and
`check: Option<String>` so the expanded card has them (research §2a found `check` was being
dropped at lib.rs:9362-9375).

### 2e. Renamed and narrowed: `render_attention_banner` → `render_health_alerts` (ui.rs:589-798)

Removed from it: the Review-phase ticket collection (ui.rs:591-595), the
`parked_by_ticket` lookup (ui.rs:605-609), the review-row loop (ui.rs:641-688), and the
`"Press [d] to mark done"` hint row (ui.rs:770-786). Retained unchanged: the box chrome, the
`⚠ ATTENTION NEEDED` header, the alert rows with their suggested actions, and the
`... and N more alerts` overflow. Its early return becomes `if state.alerts.is_empty()`.

### 2f. New: the desk renderers

```
fn desk_card_lines(card: &DeskCard, selected: bool, expanded: bool,
                   current_time: Duration, width: usize) -> Vec<String>
fn render_present_view(state: &PluginState, width: usize, output: &mut Vec<String>)
fn render_desk_pointer(state: &PluginState, output: &mut Vec<String>)
```

**`desk_card_lines` — the ≤3-line contract.** Collapsed emits exactly three lines and
never reads `card.detail`:

```
▸ T-015-02-02 · signed-build · 2h ago
    Sign into Xcode with an Apple ID, then re-run the signed build.
    → [d] mark it done
```

Line 1 is `{marker}{id} · {title} · {age}`, marker `"▸ "` when selected and `"  "`
otherwise, age from `format_age_bucket(card.age_stamp.unwrap_or(Duration::ZERO), now)` —
which already yields `UNKNOWN_AGE` (`"—"`) for the absent case (ui.rs:475-489), so `None`
needs no separate branch. Line 2 is the ask, fitted to width with the existing
`fit_modal_line` (ui.rs:1436). Line 3 is the action line from design.md §D5.

Expanded appends, for the selected card only, and only for the fields that are present:
`Reason:`, `Criterion:`, `Evidence:`, one `Step:` per entry, `Check:`, and the first-responder
block (`First responder:` / `Suggested:` / `Prepared:`) when `detail.proposal` is `Some`.

**`render_present_view`.** Empty cards → pushes exactly one line, `"Nothing needs you."`,
and returns (AC 4: no header, no box, no counts). Otherwise a plain bold `Your desk` header,
a blank line, then each card's lines with a blank line between cards. `selected` is clamped
with `.min(cards.len().saturating_sub(1))` so a stale index can never panic or select
nothing.

**`render_desk_pointer`.** One line, counts derived from `state.desk.cards` by kind:
`"5 waiting, 2 notes — [p]"`. "waiting" counts `Block | NoReviewOnFile | ReviewWait`; "notes"
counts `Note`. A zero count drops its clause; both zero emits nothing.

### 2g. `render_dashboard_lines` / `render_operations_view` (ui.rs:1313-1358)

- dispatch gains `ViewPreset::Present => render_present_view(state, width, output)`
- `render_operations_view` replaces its first three calls with
  `render_desk_pointer` then `render_health_alerts`

### 2h. Tests

Deleted with their subjects: `waiting_section_preserves_operator_ask_and_explains_world_waiting`,
`legacy_field_block_never_puts_the_raw_reason_first`, `waiting_section_is_empty_without_human_or_world_items`,
`notes_section_leads_with_summary_then_citations`, `notes_section_is_empty_without_active_notes`,
`notes_are_distinct_from_urgent_waiting_and_precede_operations`, and the review-row half of
the `render_attention_banner` tests (ui.rs:3044-3300).

Added — one per acceptance criterion, plus the shape guards:

| test | criterion |
|---|---|
| `desk_renders_five_collapsed_cards_with_no_staff_work_visible` | AC 1 |
| `a_card_without_a_stamp_shows_the_dash` | AC 1 |
| `expanding_reveals_staff_work_for_the_selected_card_only` | AC 2 |
| `collapsing_restores_the_three_line_shape` | AC 2 |
| `operations_shows_pointer_lines_not_paragraphs` | AC 3 |
| `empty_desk_is_one_calm_sentence` | AC 4 |
| `collapsed_lines_carry_no_mechanism_vocabulary` | AC 5 |
| `asks_render_verbatim_from_their_disposition_fields` | AC 5 |

A module-local `fn desk_fixture() -> DeskState` builds the exact five-card fixture AC 1
names (two blocks, one review wait, one note, one fail-closed block) so AC 1, AC 2, and AC 5
assert against one shared fixture rather than three drifting ones. The mechanism-vocabulary
test tokenizes like `catalog_copy_passes_the_kitchen_table_read`
(crates/lisa-core/src/operator_override.rs:260) — substring matching would false-positive on
"disposition" inside a legitimate evidence path, and the collapsed lines contain no paths
anyway, which is itself part of what AC 1 asserts.

---

## 3. `crates/lisa-plugin/src/lib.rs`

### 3a. `State` gains two fields (near `view_preset`, lib.rs:986-987)

```
/// Which card the desk has selected (index into the rendered card list).
desk_selected: usize,
/// Whether the selected desk card is showing its staff work.
desk_expanded: bool,
```

`State` derives `Default` (lib.rs:900), so every existing construction site is unaffected.

### 3b. `to_ui_state` (lib.rs:9280-9608)

- the `waiting_items` projection (lib.rs:9355-9378) stops dropping `steps`/`check`
- a new `let desk = self.desk_state(&waiting_items, &note_items, &tickets, &parked_threads);`
- `desk` joins the returned `ui::PluginState`

### 3c. New assembler, `impl State`

```
fn desk_state(&self, waiting: &[ui::WaitingItem], notes: &[ui::NoteItem],
              tickets: &[ui::TicketNode], parked: &[ui::ParkedThread]) -> ui::DeskState
fn fail_closed_desk_cards(&self, waiting: &[ui::WaitingItem]) -> Vec<ui::DeskCard>
```

`desk_state` builds the card list in a fixed order — blocks, then fail-closed blocks, then
review waits, then notes — each group already sorted by ticket id (`collect_parked_remedies`
sorts; `collect_notes` returns a `BTreeMap` iteration; tickets follow DAG order, so review
waits are sorted explicitly). Determinism is what makes the AC 1 line assertions stable.

It reads park stamps once via `lisa_core::parking::latest_park_stamps(&self.ledger_path)`
and attaches them to block and fail-closed cards.

`fail_closed_desk_cards` is the fifth class. It walks `self.dag.tickets()` for
`TicketStatus::Blocked` ids **not** present in `waiting`, calls the existing
`observed_override_state` (lib.rs:2173), and keeps only `NoReviewOnFile` /
`UnreadableReview` — a `Block` there would mean `collect_parked_remedies` had dropped an
agent-owned remedy, which is a different concern, and `None` means the verdict authorizes
completion. The ask comes from the sentences already pinned in `ask_header_lines`; to keep
one source of truth, those two strings move to named `pub(crate) const`s in ui.rs
(`NO_REVIEW_ASK`, `UNREADABLE_REVIEW_ASK`) that `ask_header_lines` then references. The
evidence citation comes from `inspected_paths` (lib.rs:2192), joined with `", "`.

### 3d. `handle_key` (lib.rs:8567-8738)

One block inserted after the modal handling and **before** the existing `j`/`k` scroll
branch (lib.rs:8707-8714), guarded on `self.view_preset == ui::ViewPreset::Present`:

```
Up | Char('k')   -> desk_selected = desk_selected.saturating_sub(1);  desk_expanded = false
Down | Char('j') -> desk_selected += 1 (clamped to card count - 1);   desk_expanded = false
Enter            -> desk_expanded = !desk_expanded
```

Moving the selection collapses the expansion, so "one keypress deep, never the default"
holds when the operator walks the desk. The card count is recomputed from `self.desk_cards()`
at keypress time rather than cached, so the clamp cannot go stale against a poll that
changed the list. Every other view's `j`/`k` scrolling is untouched because the branch
returns early only under `Present`.

`[p]`'s existing cycle (lib.rs:8675-8679) is unchanged; it now passes through `Present`.

### 3e. Tests added in lib.rs

- `desk_state_gives_a_blocked_ticket_with_no_disposition_its_own_card` — the fifth class,
  built from a real temp work dir with a missing file and a malformed file.
- `desk_state_ages_a_parked_block_from_the_ledger_stamp` — the D3 decision, end to end.
- `desk_keys_select_and_expand_only_on_the_present_view` — Up/Down/Enter move desk state
  under `Present` and leave `scroll_offset` alone; under `Operations` they scroll and leave
  desk state alone.

---

## 4. `crates/lisa-cli/src/status.rs`

Five `ParkedRemedy` struct literals in the test module (status.rs:454, 462, 470, 494, 519)
gain `steps: Vec::new()`. No production change — the CLI's `waiting_on_you_lines` does not
render steps today and is not in this ticket's scope.

---

## Ordering

1. **parking.rs** — `steps` field and `latest_park_stamps`. Compiles alone; breaks
   status.rs tests, so step 2 rides with it in the same commit.
2. **status.rs test literals** — restores a green workspace.
3. **ui.rs types** — `DeskCard`/`DeskDetail`/`DeskState`, `ViewPreset::Present`,
   `PluginState.desk`, and the two extracted ask consts. Compiles with the desk unrendered.
4. **ui.rs renderers** — `desk_card_lines`, `render_present_view`, `render_desk_pointer`;
   delete the two paragraph renderers, narrow and rename the banner, rewire
   `render_operations_view` and the dispatch. This is the commit that must carry the ui.rs
   test rewrite, because deleting a renderer and deleting its tests cannot be separated.
5. **lib.rs assembly** — `desk_state`, `fail_closed_desk_cards`, the `to_ui_state` wiring,
   and the `WaitingItem` projection fix.
6. **lib.rs keys** — the `Present`-scoped navigation branch.

Steps 3 and 4 both edit ui.rs and 5 and 6 both edit lib.rs; they are split for reviewable
commit size, not because they can land independently. `just check` is expected green at the
end of every numbered step except 1.
