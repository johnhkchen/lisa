# T-053-02-01 — Research: the desk view

Descriptive map of what exists today. No solutions proposed here.

## 1. What the Operations screen is right now

`render_dashboard_lines` (crates/lisa-plugin/src/ui.rs:1313) prints a title bar plus a
separator, then dispatches on `state.active_view` to one of three preset renderers
(ui.rs:1324-1328). `render_operations_view` (ui.rs:1334) composes, in order:

1. `render_waiting_on_you` (ui.rs:527)
2. `render_notes_for_you` (ui.rs:561)
3. `render_attention_banner` (ui.rs:589)
4. `render_threads` (ui.rs:944)
5. a separator, then `render_filtered_activity_log`

The first three are the memo pile the ticket names. Their exact emitted shapes matter,
because the acceptance criteria assert on rendered lines.

### `render_waiting_on_you` (ui.rs:527-558)

Emits nothing when `state.waiting_items` is empty. Otherwise a bold `Waiting on you`
header, then **per item**:

- with no triage proposal: `"{ticket_id}  {ask}{suffix}"` where `suffix` is
  `" — Lisa checks on its own."` when `checks_on_own`
- with a proposal: four lines — `First responder:`, `Suggested:`, one `Prepared:` per
  step, `Original ask:`
- always a trailing `"       Reviewer's note: {reason}"`

So today's minimum is 2 lines per item and the reason paragraph is unconditional. The
`reason` field is the raw technical companion — the pinned field specimen in the tests
(ui.rs:2216) is the 60-word codesign/`.appex` wall the epic was written against.

### `render_notes_for_you` (ui.rs:561-576)

Header `Notes for you ({n})`, then **three lines per note**:

- `"{ticket_id}  {summary}"`
- `"       Criterion: “{criterion_quote}”"`
- `"       Evidence: {evidence_citation}"`

The criterion quote and evidence path are printed unconditionally on the default screen.

### `render_attention_banner` (ui.rs:589-798)

Triggered by any Review-phase ticket **or** any health alert. Draws a `╔═╗`/`║`/`╚═╝` box
with a `⚠ ATTENTION NEEDED` header, one row per Review-phase ticket
(`{id:<10} {title:<20} {artifact:<14} {wait:>8}`), then health-alert rows with suggested
actions, then a hardcoded hint row `"Press [d] to mark done"` (ui.rs:771) — the epic's
type specimen for N3.

Note the coupling: the banner also carries **health alerts** (stuck / failed / idle /
timeout sessions), which are *not* pending decisions in the memo sense. The ticket's card
sources name parked remedies, Review-phase tickets, notes, and fail-closed blocks — alerts
are not in that list. So the banner cannot simply be deleted; its review-row duty and its
alert duty are separable and only the first belongs on the desk.

## 2. The data behind the cards

All four card classes reduce to fields already present in `PluginState` or reachable from
`State` in lib.rs.

### 2a. Parked blocks — `collect_parked_remedies`

`lisa_core::parking::collect_parked_remedies` (crates/lisa-core/src/parking.rs:110) filters
tickets to `TicketStatus::Blocked`, parses `docs/active/work/<id>/review-disposition.json`,
and keeps **only** `ReviewDisposition::Block`. Everything else — `Pass`, `Note`, and
crucially `Invalid` — returns `None` from the `filter_map` (parking.rs:122-132). Result:
`ParkedRemedy { ticket_id, remedy_owner, ask, reason, check, proposal }`, sorted by id.

An unstructured legacy block gets `LEGACY_BLOCK_ASK` (parking.rs:16) substituted for its
ask; the raw reason is preserved untouched.

The plugin projects this into `ui::WaitingItem` at lib.rs:9355-9378, dropping
`RemedyOwner::Agent` entirely and folding `Operator`/`World` into a `checks_on_own` bool.
**`ParkedRemedy` carries `check` but `WaitingItem` does not** — the check command is
currently discarded before it reaches the UI. AC 2 requires the check command in the
expanded card, so that projection has to stop dropping it.

`ParkedRemedy` also has a second consumer: crates/lisa-cli/src/status.rs:87/122 build
`waiting_on_you_lines` from it, with struct literals in its tests (status.rs:454-519). Any
field added to `ParkedRemedy` is CLI churn.

### 2b. Review waits — Review-phase tickets

`render_attention_banner` derives them from `state.tickets.iter().filter(|t| t.phase ==
Phase::Review)` (ui.rs:591-595). `TicketNode` (ui.rs:129) carries `id`, `title`, `phase`,
`status`, `depends_on` — no ask, no reason. The banner's per-row artifact filename and
wait time come from a `parked_threads` lookup keyed by ticket id (ui.rs:605-609), falling
back to `"—"` when absent (ui.rs:659, 666).

### 2c. Notes — `collect_notes`

`lisa_core::notes::collect_notes` (crates/lisa-core/src/notes.rs:163) reconstructs active
notes from the completion journal minus acknowledged keys in the provenance ledger. It
returns `QueuedNote { key: NoteKey { ticket_id, attempt_id, generation }, note }`. The
`DispositionNote` exposes `summary()`, `criterion_quote()`, `evidence_citation()`.

**There is no timestamp anywhere in this projection.** Neither `NoteKey` nor
`DispositionNote` carries an emit instant. The ticket's "notes carry no stamp at all" is
confirmed. Projected into `ui::NoteItem` at lib.rs:9380-9390.

Note acknowledgment (`acknowledge_note`, notes.rs:179) is wired **only** into the CLI
(crates/lisa-cli/src/notes.rs:71). The plugin has no dismiss key today.

### 2d. The missed class — Blocked with no parseable disposition

This is the class parking.rs:122 drops. The seam that *does* see it already exists on the
plugin side: `State::observed_override_state` (crates/lisa-plugin/src/lib.rs:2173) parses
the same file and maps:

- `Pass | Note` → `None` (verdict already authorizes completion)
- `Block { ask, reason, .. }` → `OverriddenAsk::Block { ask, reason }`
- `Invalid { reason }` → `OverriddenAsk::UnreadableReview { detail: reason }` when the file
  exists on disk, else `OverriddenAsk::NoReviewOnFile`

That existence probe is what distinguishes "no review" from "unreadable review" —
the parser reports both as `Invalid`.

The operator-facing copy for those two states is **already written** in
`ask_header_lines` (ui.rs:1420-1430), used by the reason-step modal T-053-01-02 landed:

- `NoReviewOnFile` → `"No review was left for this ticket."`
- `UnreadableReview { .. }` → `"No review Lisa can read was left for this ticket."`

Both destructure with `{ .. }` on purpose so a raw parse error can never reach the screen.
This is the "no-review-on-file framing" the ticket asks the fifth card to wear.

`inspected_paths` (lib.rs:2192-2213) gives the honest evidence citation per state: the
disposition path plus review.md/progress.md where they exist for a Block, the disposition
path alone for `UnreadableReview`, the work directory for `NoReviewOnFile`.

## 3. Age — what stamps actually exist

E-052's formatter is `format_age_bucket` (ui.rs:486-498), `pub(crate)`:

- zero timestamp → `UNKNOWN_AGE` = `"—"` (ui.rs:475)
- `0..=59s` → `"just now"`, then `"{n}m ago"`, `"{n}h ago"`, `"{n}d ago"`

It already renders exactly the `"—"` the acceptance criteria demand for a missing stamp,
and it already clamps future timestamps (test at ui.rs:1989).

**Correction to the ticket's stated model.** The ticket says parked blocks "carry an
in-memory `parked_at` that resets with the plugin". Research does not bear that out:

- `ui::ParkedThread.parked_at` (ui.rs:162) is populated at lib.rs:9344-9349 from
  `thread.started_at` — the thread's *spawn* time, not its park moment — and only for
  threads whose status is `ThreadStatus::Parked` (lib.rs:9327).
- A ticket that parks **durably** (becomes `Blocked`) has its thread **removed**:
  lib.rs:6058-6070 stamps `parked_at`, emits the parking transition, releases the slot, and
  calls `self.threads.remove(&ticket_id)`. Same shape at lib.rs:3170-3181 and 6204.

So there is no surviving in-memory stamp for a blocked ticket at all — `parked_threads`
holds threads parked awaiting review, not durable blocks.

There *is* a durable stamp: `ParkingTransitionRecord` (crates/lisa-core/src/provenance.rs:229)
carries `started_at`/`ended_at`/`wall_clock_secs` as unix seconds, appended on every Park.
`latest_park_attempt_leases` (parking.rs:86-104) already scans the whole ledger for exactly
these records, keeping the latest Park per ticket and dropping it on Unpark — it just
discards everything except the attempt lease. Its other caller is
crates/lisa-cli/src/proposal.rs:84.

So the honest age sources available are: the ledger Park record for blocked tickets, the
in-memory `ParkedThread` for review waits, and nothing for notes.

## 4. Presets and keys

`ViewPreset` (ui.rs:376-405) is a three-variant enum — `Operations` (default), `Dag`,
`Activity` — with `next()` and `label()`. The plugin holds `view_preset: ui::ViewPreset`
(lib.rs:987), copies it into `active_view` at lib.rs:9606, and cycles it on `[p]` at
lib.rs:8675-8679 (also zeroing `scroll_offset`).

The status line (ui.rs:1289-1303) hardcodes `[p] view  [space] {pause}  [d] done  [r] reset`.

`handle_key` (lib.rs:8567) is modal-first: quit-confirm, then MarkDone's pending-outcome
layer, then the reason step (lib.rs:8608-8627), then the shared list handling. Normal mode
follows: `p`, space, `d` → `open_mark_done_modal` (lib.rs:8741), `r`, `j`/`k` scroll,
`D` snapshot, `q`. Note `j`/`k` and Up/Down are currently **scroll** in normal mode
(lib.rs:8707-8714) — the desk's selection keys collide with that, but the key estate is
T-053-02-02's scope, not this ticket's.

`[d]` today opens a modal listing every non-Done ticket; Enter on one either seals it
(verdict already authorizes) or opens the reason step (`override_choices_for` →
`open_reason_step`, lib.rs:8654-8656). That chain is live as of T-053-01-02, so a card
advertising `[d]` is advertising something that works.

There is no `[s]` send-back and no in-plugin note dismissal. Advertising either on a card
would be the exact N3 sin this epic exists to correct.

## 5. Test conventions

UI tests live in `mod tests` at the bottom of ui.rs and call renderers directly with a
`PluginState { field: ..., ..PluginState::default() }` literal and a `Vec<String>` sink
(ui.rs:2180-2246 are the model). Assertions run against exact line equality
(`assert_eq!(output[1], ...)`) or `output.join("\n").contains(...)`, and negative assertions
on absence (`assert!(!full.contains("remedy_owner"))`) are established practice. A
`strip_ansi` helper exists at ui.rs:3673 for width assertions.

Plugin-side state tests build a `State` and call `to_ui_state()` (lib.rs:10536, 10617).

## 6. Constraints and assumptions surfaced

- **Two crates, no new dependency direction.** Cards can be assembled in lib.rs from
  existing lisa-core reads; ui.rs stays a pure renderer over `PluginState`.
- **`check` is dropped before the UI.** Expanded cards need it back (AC 2).
- **Health alerts are not decisions.** The attention banner has a second job the four card
  sources do not cover; collapsing Operations must not silently delete alerts.
- **No new prose.** Every collapsed line must be a field a disposition already carries, or
  copy already pinned in `ask_header_lines`/`OverrideReason::summary`.
- **Mechanism-word check is precedented.** `catalog_copy_passes_the_kitchen_table_read`
  (crates/lisa-core/src/operator_override.rs:260) already asserts
  `disposition`/`frontmatter`/`dag`/`seal` absence by tokenized match — AC 5's copy check
  has a working shape to copy.
- **`just check`** = `check-wasm` (cargo check on wasm32-wasip1) + `fmt-check` + `lint` +
  `cargo test --workspace` (justfile:53-57).
