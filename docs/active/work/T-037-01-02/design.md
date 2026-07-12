# Design — T-037-01-02 codex-startup-grace-pacing

## Decision

Add a **named startup grace** to the existing `Starting` state, keyed on the
per-pane `ReadinessMode` recorded at dispatch:

1. A new `STARTUP_GRACE_SECS` constant + `startup_grace_deadline(now)` helper.
2. `start_assignment_ack_wait` arms a **grace** deadline for grace-mode panes and
   the unchanged ack/SessionStart-wait deadline for SessionStart-mode panes.
3. `check_assignment_ack_timeouts_at`'s `Starting { relaunches: 0 }` branch
   becomes provider-aware: grace mode **delivers the assignment directly**
   (`Starting → Delivering`); SessionStart mode keeps `begin_startup_recovery`.
4. `fail_assignment_delivery` is widened to accept a `Starting` origin so a
   grace send that cannot even be submitted resolves in a named `DeliveryFailed`
   state rather than silently staying `Starting`.

Everything downstream of `Delivering` — the bounded retry, `DeliveryFailed`, and
`Owned`-only-on-exact-ack — is **reused unchanged**. That is the whole point: the
grace path merges into the existing acknowledgement-gated `Delivering → Owned`
boundary, so ownership still means "this attempt's prompt was accepted."

## Why this shape

The epic's contract is *provider-specific bootstrap, shared ownership boundary*.
The only truthful difference for Codex is **how the first prompt is paced**: no
pre-prompt readiness hook exists, so a bounded timer paces the send. That maps
to exactly one behavioural fork — the `Starting`-expiry transition — while
`Delivering`, acknowledgement, retry, and failure are already provider-neutral.
Keeping the fork that narrow is what satisfies "no broad scheduler rewrite" (N4)
and "Claude's path unchanged" (N1).

The grace lives on the **existing** `Starting.start_deadline` field. For a
SessionStart seat it means "bound the wait for the process-start signal"
(unchanged); for a grace seat it means "the startup grace before the paced
send." No new state variant, no new field, no ripple through the dozens of
`Starting { .. }` match sites. `Starting` *is* the "named grace" the AC asks for.

## Options considered

### A. Provider-aware `Starting`-expiry transition (CHOSEN)

Fork only in the deadline evaluator + deadline arming, reusing `Delivering`.

- **For:** smallest possible surface; the ownership boundary and all failure
  handling are reused, not reinvented; `Starting`-field-shape untouched so no
  test/match churn beyond the one Codex branch that *defines* the old behaviour;
  the recovery-fresh Codex path (also grace-classified) transparently gains the
  same real-world fix without touching E-034 code.
- **Against:** overloads `start_deadline`'s meaning across two modes — mitigated
  by a doc-comment update stating both readings.

### B. New `StartingGrace` state variant

A dedicated `StartingGrace { generation, grace_deadline }` variant separate from
`Starting`.

- **For:** self-documenting; impossible to confuse the two deadline meanings.
- **Against:** ripples into every `Starting` match arm, the UI status map
  (lib.rs:5659), `check_assignment_ack_timeouts_at`'s collector, and a raft of
  existing tests — precisely the "broad rewrite" N4 forbids. Rejected: the cost
  is real and the benefit (naming) is achievable with a comment.

### C. Reuse the ack timeout as the grace duration (no new constant)

Let the grace equal `assignment_ack_deadline` (≈30 s + enter delay), the value
`start_deadline` already carries.

- **For:** zero new constants; smallest diff.
- **Against:** semantically wrong and poor real behaviour — it stalls ~32 s
  before *ever* attempting the first Codex prompt on every launch. A startup
  grace is "let the TUI accept input" (~seconds), not "the acceptance clock."
  Rejected on faithfulness to "bounded named startup grace"; the two durations
  are conceptually distinct and deserve distinct names.

### D. Guard `acknowledge_process_start` to reject grace-mode seats

Structurally forbid a grace seat from ever reaching `ReadyForAssignment`, even
if a stray process-start signal arrived.

- **For:** literally enforces the ticket's "Reserve `ReadyForAssignment` for
  Claude's SessionStart path."
- **Against:** the recovery-fresh Codex `Starting` (from `begin_assignment_
  recovery` → post-exit relaunch, lib.rs:4021) is grace-classified, and four
  existing E-034/E-035 recovery tests drive it to `Owned` through a *synthetic*
  `acknowledge_process_start`. Guarding would force rewriting those four tests
  and altering the E-034 recovery contract — which the story explicitly does not
  reopen ("Consumes E-034… does not reopen"). **Rejected as out of scope.**
  The reservation is honoured *in practice* anyway: Codex emits no pre-prompt
  process-start signal, so grace expiry is the only route a primary Codex seat
  leaves `Starting`. Time never triggers `acknowledge_process_start` (a signal
  does), so "never `ReadyForAssignment` merely because time passed" holds without
  the guard. Documented as a deliberate boundary in Review.

## Chosen grace duration

`STARTUP_GRACE_SECS = 8`. Rationale: long enough for a cold Codex TUI to reach
input-readiness before we type (a too-short grace merely costs one bounded
`Delivering` retry, not a deadlock), short enough that a launch does not visibly
stall. Mirrors the existing `AGENT_EXIT_GRACE_SECS = 8` "let the TUI settle"
precedent. Deadline checks run on the 5 s poll, so the effective fire is the
next poll boundary after 8 s. It is a `lib.rs` constant, not a `PluginConfig`
field, because config parsing lives in `types.rs` (outside this ticket's file
ownership); promoting it to a knob is noted as future work, not built here.

## Resulting grace-mode lifecycle (Codex)

```
dispatch → Starting{grace_deadline}           (readiness=Grace recorded)
  grace elapses → deliver_assignment_to_pane → Delivering{retries:0}
      UserPromptSubmit(exact gen) → Owned                (only Owned edge)
      ack_deadline, retries<1    → Delivering{retries:1} (bounded retry)
      ack_deadline, retries==1   → DeliveryFailed        (named terminal)
      submit error at grace      → DeliveryFailed        (named terminal)
```

Never `ReadyForAssignment`, never `StartupFailed`-from-time, never
`Owned`-from-time. SessionStart mode (Claude) is byte-for-byte unchanged:
`Starting → (signal) → ReadyForAssignment → Delivering → Owned`, and a Claude
`Starting` deadline still elapses into `begin_startup_recovery`.

## Grounding in research

- `deliver_assignment_to_pane` is state-agnostic on entry and validates the exact
  current lease (research: "Delivery mechanics"), so calling it from `Starting`
  is safe and reuses the same tag/ownership checks.
- The assignment file is written at dispatch for every launch (lib.rs:2609), so a
  paced send finds it.
- `seat_readiness` is populated at both `Starting` dispatch sites (lib.rs:2787,
  4032), so `seat_readiness_mode` is reliably `Some` for every `Starting` seat
  the evaluator inspects — the fork never falls through on a missing record.
- Only one existing test asserts the *old* Codex primary-expiry behaviour
  (`same_pane_replacement…for_both_providers`); it is split so Claude keeps the
  SessionStart-recovery contract and Codex is covered by the new grace test.

## Test strategy (this ticket)

One focused injected-time test proving the AC transition (Codex grace → Delivering
directly → Owned only on exact ack; Claude still signal-gated), plus splitting the
one now-divergent shared test. The full delayed-send and prompt-miss regressions
are T-037-01-03; this ticket lands the transition they will exercise.
