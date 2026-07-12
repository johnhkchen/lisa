# Design — T-037-01-01 provider-readiness-capability

## Decision

Add a two-variant `ReadinessMode` classification to the adapter boundary and a
trait method `AgentAdapter::readiness_mode(&self) -> ReadinessMode`. Claude
returns `SessionStart`; Codex returns `Grace`. At launch dispatch the scheduler
reads `adapter.readiness_mode()` and records it per pane in a new observational
map `State::seat_readiness: HashMap<u32, ReadinessMode>`, exposed via a
`seat_readiness_mode(pane_id)` accessor. Nothing else changes: no
`SeatAssignmentState` variant is touched, no transition or deadline moves.

## What the mode means

- `ReadinessMode::SessionStart` — the provider emits a truthful pre-prompt
  process-start signal; readiness is *proven* by positive evidence
  (`acknowledge_process_start` → `ReadyForAssignment`). Claude's existing path.
- `ReadinessMode::Grace` — no truthful pre-prompt readiness hook exists; the
  first prompt must be *paced* by a bounded named startup grace, and elapsed
  time is never treated as readiness or ownership. Codex. (The transition that
  consumes this is T-037-01-02; here it is only classified.)

This names exactly the fork E-037 identified and gives 02 a settled shape to
branch on without re-deriving provider identity inside the state machine.

## Options considered

### Option A — trait method returning a `ReadinessMode` enum; scheduler records it per pane (CHOSEN)

Mirrors the two existing capability descriptors on the trait (`signals` →
`SignalCapabilities`, `reset_strategy` → `ResetStrategy`). The enum is a `Copy`
closed set; each adapter answers for itself. The scheduler reads it at the
canonical fresh-`Starting` dispatch sites and stores it in a map disjoint from
`seat_assignments`.

- **Pros:** Idiomatic — indistinguishable in shape from `reset_strategy()`.
  Adapter-testable in isolation. The recorded map is exactly what a scheduler
  test can assert against ("the scheduler reads it"), and exactly what 02 will
  consume when it makes the `Starting` deadline provider-aware. Fully disjoint
  from the state machine; zero behavior change. Extensible to a third provider
  by adding one enum arm + one adapter answer.
- **Cons:** Adds a `State` field now whose only consumer this ticket is a test.
  Acceptable: the AC demands the scheduler *read* the mode at dispatch, which
  requires an observable sink, and 02 is the imminent real consumer.

### Option B — trait method only; scheduler does not store it (REJECTED)

Define `readiness_mode()` and call it at dispatch into a `let _mode = ...`
(or a log line) with no stored record.

- **Rejected because:** "the scheduler reads that mode at launch dispatch" is an
  acceptance criterion a native test must *prove*. A discarded read or a log
  string is not machine-assertable without scraping activity text — brittle and
  against the grain of how seat state is tested (`seat_assignment(pane)`). A
  recorded classification is the honest, testable form of "reads it."

### Option C — add the mode as a field on `SeatAssignmentState::Starting` (REJECTED)

Carry `readiness: ReadinessMode` inside the `Starting` variant.

- **Rejected because:** The story reserves the `lib.rs` `Starting` region for
  T-037-01-02/03 and explicitly places this ticket "in `adapter.rs`, disjoint
  from the state machine." Threading a field through every `Starting`
  construction/match site (12+ sites, incl. tests) *is* a state-machine change
  and would collide head-on with 02's edits. Deferred to whoever actually
  branches on it.

### Option D — derive the mode from `AgentClient` / route at each use site (REJECTED)

Skip the trait method; `match route.agent { Claude => .., Codex => .. }` wherever
readiness matters.

- **Rejected because:** It scatters provider lifecycle knowledge across the
  scheduler instead of keeping it behind the adapter boundary — the exact
  coupling the adapter trait exists to prevent (module docs, "everything that
  differs per integration method"). A future ACP/bridge leg would have to be
  special-cased in the scheduler rather than by implementing one trait method.

## Why record at dispatch rather than lazily

The AC ties the read to *launch dispatch*. Recording when the fresh `Starting`
state is created (a) matches the wording, (b) guarantees any pane that reaches
`Starting` already has its mode classified for 02 to consume, and (c) keeps the
adapter (already in hand at those sites) as the single source. The map is
overwritten on every launch dispatch for a pane, so a stale prior entry can
never be read before a fresh classification replaces it.

## Scope guardrails honored

- No `SeatAssignmentState` variant added or changed; no transition edited.
- No launch-line, deadline, log, or ordering change — behavior byte-identical.
- Claude's `SessionStart` evidence path is untouched (only *labeled*).
- Codex grace *transition* is NOT implemented here — only the classification it
  will later key on.

## Test strategy

1. Adapter unit tests (`adapter.rs`): `claude_reports_session_start_readiness`,
   `codex_reports_grace_readiness` — direct per-adapter assertions, mirroring
   the `signals()` tests.
2. Scheduler test (`lib.rs`): dispatch a Codex ticket via
   `pane_name_schedule_state`, assert `seat_readiness_mode(10) ==
   Some(Grace)` and the seat is still `Starting` (no behavior change); dispatch a
   Claude ticket, assert `Some(SessionStart)`. This proves the scheduler reads
   and records the mode at launch dispatch.
