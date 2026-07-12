# Lisa — Charter

Lisa is worth changing when the change makes concurrent, artifact-driven agent
work safer, more reliable, or easier for an operator to understand. Prefer
small provider-neutral scheduler contracts backed by field evidence and
repeatable tests.

## Product principles

- **P1 — Preserve the repository.** Lisa must not silently destroy user-owned
  content, expose secrets, or mix one ticket's changes into another commit.
- **P2 — State means what it says.** Published ticket, phase, slot, and
  completion states must correspond to durable reality; failure stays visible
  and recoverable rather than being mislabeled as success.
- **P3 — Provider parity at the contract boundary.** Claude and Codex may need
  different mechanisms, but assignment, liveness, completion, and recovery
  guarantees must be equivalent from the scheduler and operator perspective.
- **P4 — Operable at a glance.** A person supervising several panes should be
  able to identify ownership, progress, idle capacity, and failures without
  reverse-engineering terminal contents.
- **P5 — Field evidence becomes regression evidence.** A real failure is not
  closed by a plausible patch alone; preserve a deterministic reproduction and
  prove the lifecycle boundary that failed.

## Non-goals

- **N1 — Accidental provider uniformity.** Do not force Claude and Codex through
  an identical handshake when their supported lifecycle semantics differ.
- **N2 — Infinite retries or silent babysitting.** Recovery must be bounded,
  observable, and end in a named actionable state when it cannot succeed.
- **N3 — Dashboard-only correctness.** UI labels and activity events report
  state; they never substitute for correct scheduler transitions.
- **N4 — Broad rewrites for local defects.** Preserve stable paths and isolate
  the smallest contract that removes the failure mode.
