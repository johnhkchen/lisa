---
id: T-020-04
story: S-020
title: timeout-exemption-surfacing
type: feature
status: open
priority: medium
phase: done
depends_on: [T-020-03]
---

## Context

Stop the wall-clock reclaimers from killing a pane that is legitimately waiting on
a human, and make awaiting panes visible on the dashboard so an exempt pane is
never invisible. A human may take many minutes to answer an `AskUserQuestion` —
longer than hard-silence — and reclaiming mid-question is exactly the failure
S-020 exists to prevent (spike design Q6). Builds on the `awaiting_human` flag from
T-020-03. Touches `crates/lisa-plugin/src/lib.rs` and `crates/lisa-plugin/src/ui.rs`.

Key anchors (verify before editing; from spike design Q6):
- `check_session_timeouts` reclaim filter — `lib.rs:1399` (reclaims at
  `2×stuck_threshold_secs` hard silence).
- `detect_stale_threads` filter — `lib.rs:1484` (same bar).
- Dashboard rendering — `ui.rs` (thread/pane status markers).

## Acceptance Criteria

- Add `!self.awaiting_human.contains(&pane_id)` to the reclamation filters in **both**
  `check_session_timeouts` (`lib.rs:1399`) and `detect_stale_threads` (`lib.rs:1484`) so an
  awaiting pane is not reclaimed on the hard-silence clock.
- Do **not** exempt the *injection* timeouts (`check_transition_timeouts` /
  `check_review_timeouts`) from running — they are already guarded by T-020-03 (they skip the
  write) and must resume normally once the flag clears. Only the **kill** is exempt.
- Budget/stuck **warnings** may still log for an awaiting pane (visibility is good); only the
  reclamation/kill is suppressed.
- Surface awaiting panes in the dashboard (`ui.rs`): a distinct "awaiting human" marker on the
  thread/pane (e.g. `T-024-01 [AWAITING]`), so an exempt-from-reclamation pane is clearly shown
  rather than silently parked.
- Tests (native): an awaiting pane past `2×stuck_threshold_secs` of silence is **not** reclaimed
  by either reclaimer; once its flag clears (heartbeat), normal reclamation applies again; UI
  snapshot/state includes the awaiting marker.
- `just check` passes.

## Implementation notes

- This preserves the v0.2.11 liveness invariant: the exemption is scoped to panes explicitly
  flagged `awaiting_human` (set only by a real `AskUserQuestion` PreToolUse signal), not a
  blanket relaxation of stall detection. A pane that never got the signal is unaffected.
- Keep the exemption and the marker driven off the same `awaiting_human` set so they can never
  disagree (exempt-but-invisible is the specific bad state to avoid).
