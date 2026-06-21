# T-020-04 Design — timeout-exemption-surfacing

Decisions, with rationale grounded in `research.md`. Two concerns: (A) exempt
awaiting panes from the two kill paths, (B) surface them on the dashboard. Keep both
driven off the single `awaiting_human` set so they cannot disagree.

## A. Reclaim exemption

### A1. Where to apply the exemption

**Decision:** Add `!self.awaiting_human.contains(&t.pane_id)` (via `is_pane_awaiting`
where ergonomic) at the **kill decision** in each reclaimer:

- `check_session_timeouts`: in the silence split, require `silent_for >= hard_silence
  && !awaiting` to enter the `timed_out` (kill) list. An awaiting over-budget pane
  therefore falls into the existing `over_budget_active` branch → it **warns once**
  and is not removed. This is a one-token change that reuses the existing warn path —
  exactly the "warnings may still log; only the kill is suppressed" AC, for free.
- `detect_stale_threads`: add a `.filter(|(_, t)| !awaiting.contains(&t.pane_id))` to
  the `stale` selection so an awaiting pane is never collected for removal. There is
  no warn path here; silent exemption is correct (the marker provides visibility).

**Rejected — exempt at the *action* (skip inside the `for ... in timed_out` /
`for ticket_id in stale` removal loops):** would require re-deriving pane id from the
thread inside the action loop and would either drop the alert silently or need a
parallel warn. Filtering at selection is simpler and keeps the alert/warn semantics
already wired. Rejected.

**Rejected — a single shared `reclaimable()` helper wrapping both:** the two
reclaimers have different shapes (a `for` loop with a two-way split vs. an iterator
chain) and different post-actions (alerts vs. error log). A shared helper would
abstract over too little. Inline guards keyed off the same set are clearer. Rejected.

### A2. Warn vs. silent for the session-timeout case

**Decision:** Let the awaiting over-budget pane take the existing `over_budget_active`
→ `Warning` path. The warn message ("…still active — waiting for it to wind down…")
is approximately true (it is waiting — on a human) and the dashboard marker makes the
real reason explicit. Adding a bespoke "awaiting" warning string is not worth a new
log variant.

**Rejected — suppress the warn too:** the AC explicitly permits warnings for
visibility; suppressing would make an exempt pane quieter, the opposite of intent.

### A3. Do NOT touch the injection timeouts

`check_transition_timeouts` / `check_review_timeouts` keep running and self-skip via
their T-020-03 guards. No change. Confirmed by AC and research: only the kill is
exempt; injectors must resume normally once the flag clears.

### A4. Borrow strategy

- `check_session_timeouts`: the kill split is inside `for (tid, t) in &self.threads`.
  `self.awaiting_human.contains(&t.pane_id)` is a disjoint shared field borrow
  alongside the `&self.threads` loop borrow — compiles directly. Use it inline.
- `detect_stale_threads`: bind `let awaiting = &self.awaiting_human;` before the
  iterator chain and reference it in the closure. Avoids any closure-capture-of-self
  ambiguity and reads cleanly. (Edition 2021's disjoint captures would likely allow
  `self.awaiting_human` directly, but the local binding is unambiguous and free.)

## B. Dashboard surfacing

### B1. Carry awaiting into the UI mirror

**Decision:** Add `pub awaiting: bool` to `ui::ActiveThread`. Populate it in
`to_ui_state` with `self.is_pane_awaiting(t.pane_id)`. This keeps the exemption and
the marker reading from the **same** `awaiting_human` set (the AC's anti-divergence
requirement) — the UI value is a pure projection of the authoritative set at render
time.

**Rejected — recompute awaiting in the UI from a separate source:** there is no other
source; the set is the truth. A second path is exactly what the AC forbids.

**Rejected — only mark parked threads:** an awaiting pane is *Running* (it has not
parked at a review artifact); the question can occur in any phase. The marker must
ride the active-thread row.

### B2. How the marker renders

**Decision:** In `render_threads`, the active-thread branch chooses STATUS text/color
by `active.awaiting`:
- awaiting → STATUS `"Awaiting"` in `CYAN`, plus the ticket id rendered as
  `T-024-01 [AWAITING]` (append ` [AWAITING]` to the TICKET cell) so the state is
  unmistakable and greppable even if color is stripped.
- not awaiting → unchanged (`"Running"`, GREEN).

Using both a status word and an `[AWAITING]` token on the ticket matches the ticket's
own example (`T-024-01 [AWAITING]`) and gives tests a stable substring. The `[AWAITING]`
token is appended to the ticket label before width-padding so alignment degrades
gracefully (an awaiting row may run slightly wider — acceptable for a rare, important
state).

**Rejected — a separate attention-banner entry only:** the banner is for health
alerts (Stuck/Failed/Idle/TimedOut). Awaiting-human is not an *alert* (it is healthy,
intentional waiting); putting it in the threads table where the operator already
looks for per-pane status is the right altitude. A banner entry could be a future
add-on but is out of scope.

**Rejected — a new PHASE-column decoration:** PHASE is the RDSPI phase; overloading it
muddies two orthogonal axes. STATUS is the correct column for "what is this pane
doing right now."

### B3. No new `AlertType`

We are not adding `AlertType::AwaitingHuman`. The marker lives in the threads table,
not the alert banner (B2). Keeps the change additive and avoids touching the alert
pipeline.

## Net shape

- `lib.rs`: 2 small reclaimer edits + 1 field set in `to_ui_state`.
- `ui.rs`: 1 struct field + 1 render branch + fixture/test updates.
- Tests: reclaimer exemption (both paths), flag-clears-restores-kill, UI marker.

All additive; no signatures removed, nothing deleted. Preserves the v0.2.11 invariant
because the exemption is scoped strictly to the `awaiting_human` set.
