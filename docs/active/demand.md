# Vend — Demand (the pull board)

Thin demand **signals**, not epics — one line of "what + why it might matter." Epics are
**pulled** from here just-in-time when there's capacity; clearing (signal → epic →
stories/tickets) happens on pull, never ahead of demand. Cleared signals crystallize to
one line in `docs/archive/demand-cleared.md` and are deleted from here.

---

- **Review completion livelock:** make an admitted `review.md` a durable, idempotent
  completion trigger across Codex stop/timeout/relaunch state, and make `[d]one` a
  reliable operator recovery path; Arcade T-009-01-01 remained in Review after both
  paths and completed only when the agent ran `lisa complete-ticket` itself.
