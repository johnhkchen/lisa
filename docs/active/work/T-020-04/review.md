# T-020-04 Review — timeout-exemption-surfacing

Handoff for a human reviewer. This feature stops the two wall-clock reclaimers from
killing a pane that is legitimately blocked on an `AskUserQuestion`, and surfaces
those panes on the dashboard so an exempt pane is never invisible. It is the final
ticket of S-020 and builds directly on the `awaiting_human` set from T-020-03.

## What changed

**Two files, both in `crates/lisa-plugin/src/`. Additive only** — no signatures
removed, no public interfaces changed, nothing deleted.

| # | Change | Location |
|---|--------|----------|
| 1 | `check_session_timeouts` kill branch gated on `!awaiting_human.contains(&t.pane_id)` | `lib.rs` ~1540 |
| 2 | `detect_stale_threads` `stale` selection filters out awaiting panes | `lib.rs` ~1590 |
| 3 | `to_ui_state` sets `awaiting: self.is_pane_awaiting(t.pane_id)` per active thread | `lib.rs` ~2713 |
| 4 | `ui::ActiveThread` gained `pub awaiting: bool` | `ui.rs` ~140 |
| 5 | `render_threads` renders `T-xxx [AWAITING]` + "Awaiting"/CYAN status when awaiting | `ui.rs` ~720 |
| 6 | 6 existing `ActiveThread` fixtures updated `awaiting: false` | `ui.rs` |
| 7 | 6 new tests (5 in `lib.rs`, 1 in `ui.rs`) | both |

Net: ~+150 lines (impl + tests), all additive.

## Acceptance-criteria coverage

- ✅ `!self.awaiting_human.contains(&pane_id)` added to **both** reclamation filters
  (`check_session_timeouts` and `detect_stale_threads`). An awaiting pane is not
  reclaimed on the hard-silence clock by either path.
- ✅ Injection timeouts (`check_transition_timeouts` / `check_review_timeouts`) are
  **not** exempted from running — untouched here; they keep their T-020-03
  skip-the-write guards and resume normally once the flag clears. Only the **kill**
  is exempt.
- ✅ Budget/stuck **warnings** still log for an awaiting pane: in
  `check_session_timeouts` the awaiting over-budget pane falls into the existing
  `over_budget_active` → `Warning` branch instead of the kill list. Only the
  reclamation is suppressed.
- ✅ Dashboard surfacing: awaiting panes render as `T-xxx [AWAITING]` with a distinct
  CYAN "Awaiting" status in the Threads table.
- ✅ Tests (native): an awaiting pane past `2×stuck_threshold_secs` is not reclaimed
  by either reclaimer; the paired flag-cleared tests prove normal reclamation resumes
  once the flag is gone; a `to_ui_state` test and a `render_threads` test confirm the
  marker.
- ✅ `just check` passes (WASM check + full workspace suite).

## Test coverage & how it's verified

6 new tests, all green:
- `test_session_timeout_skips_kill_when_awaiting` — over-budget + silent + flagged →
  thread survives.
- `test_session_timeout_kills_after_flag_clears` — identical fixture, no flag →
  thread reclaimed. (The pair isolates the exemption as the sole cause.)
- `test_detect_stale_skips_when_awaiting` — silent past hard timeout + flagged →
  survives.
- `test_detect_stale_kills_after_flag_clears` — identical fixture, no flag → removed.
- `test_to_ui_state_marks_awaiting_thread` — only the flagged pane's `ActiveThread`
  has `awaiting == true`; proves the marker is a pure projection of the same
  `awaiting_human` set used by the exemption (anti-divergence).
- `test_render_threads_marks_awaiting` — output contains `[AWAITING]` and "Awaiting"
  and **not** "Running" for an awaiting row.

Full suite: lisa-plugin **177** (was 171), lisa-cli 172, lisa-core 106 — all pass.
The reclaimer tests are direct (no zellij host calls on these paths), so a regressed
guard fails loudly rather than passing silently.

**Key property:** the four reclaimer tests are matched pairs differing only in
`awaiting_human` membership and producing opposite outcomes — this is the core
evidence that the exemption is both effective (flagged → survives) and narrow
(unflagged → still killed, preserving the v0.2.11 liveness invariant).

## Design notes for the reviewer

- **Single source of truth.** Both the exemption and the marker read the same
  `awaiting_human: HashSet<u32>`. The UI value is set in `to_ui_state` as a live
  projection (`is_pane_awaiting`), so "exempt-but-invisible" — the specific bad state
  the ticket calls out — cannot occur.
- **Warn-not-silent for session timeout.** Routing the awaiting pane into the
  existing `over_budget_active` branch reuses the one-shot `Warning` log for free and
  satisfies "warnings may still log." `detect_stale_threads` has no warn path, so it
  exempts silently — visibility there comes from the dashboard marker.
- **Borrow handling.** `check_session_timeouts` uses an inline disjoint field borrow
  inside its `for` loop; `detect_stale_threads` binds `let awaiting = &self.awaiting_human`
  before the iterator chain to avoid closure-capture ambiguity. Both compiled first try.

## Gaps & open concerns

1. **Marker width.** The `[AWAITING]` token is appended to the 12-wide TICKET cell,
   so a long ticket id on an awaiting row can overflow its column and nudge alignment.
   Deliberate (design B2): legibility of a rare, important state over strict column
   alignment. If undesirable, widen the column or move the token to the STATUS cell.
2. **Stale-but-abandoned awaiting pane.** If an agent asks a question and the session
   then dies (no heartbeat ever clears the flag), the pane stays both exempt and
   marked `[AWAITING]` indefinitely. This is the intended trade for never killing a
   live question (S-020's whole purpose), and the dashboard marker keeps it visible so
   an operator can intervene manually. There is no auto-timeout on the exemption by
   design — flagged here so it reads as deliberate, not an oversight. A future
   enhancement could add a much longer "awaiting-abandoned" ceiling if dead-question
   panes prove to accumulate in practice.
3. **No attention-banner entry.** Awaiting is shown only in the Threads table, not the
   health-alert banner (it is intentional waiting, not an alert). Reasonable future
   add-on; out of scope per design B2/B3.
4. **Not committed.** Consistent with prior S-020 tickets, the working-tree changes
   are left for the operator's normal commit flow.

## Risk assessment

Low. Blast radius is two one-token-ish reclaimer guards plus one UI field and one
render branch. The exemption is strictly scoped to the `awaiting_human` set — a pane
that never received an `AskUserQuestion` signal is completely unaffected, so the
v0.2.11 "silence kills" invariant is preserved for every normal pane. Both reclaimers
are covered by paired survive/kill tests.
