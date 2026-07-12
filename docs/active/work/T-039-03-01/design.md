# Design: failure and reclaim state-machine map

## Goal

Produce a durable before-refactor map which lets T-039-03-02 name transition
outcomes without accidentally changing lease, seat, thread, pane, provenance, or
retry authority. The map must be verifiable against the unmodified tree.

## Option 1: one normalized teardown table

A single table could reduce every path to booleans such as lease retained, slot
retained, and thread retained. It is compact and makes differences visible.

Its weakness is loss of transition context. Startup recovery revokes a
predecessor, mints a successor, and later revokes the successor; recording only
the final absence of a lease hides the ordering contract. Assignment recovery
has a one-successor intermediate state even though its terminal failure retains
that successor. Session timeout and stale reclaim share final teardown but have
different outcomes and dashboard evidence.

## Option 2: prose-only path narratives

Narratives can preserve ordering and explain special cases. Existing comments in
`lib.rs` already provide much of this description.

Prose alone makes it hard to compare all six authorities across seven paths. A
later refactor could preserve the narrative impression while changing one field,
such as emitting provenance for a retained failure or keeping a pane reusable
after hard silence.

## Option 3: transition map plus authority matrix

Combine a compact state-transition map with a field-by-field invariant matrix.
For each path, name the trigger, intermediate edge, terminal state, and retry
mode. Then pin the final effects under lease, seat, thread, pane, provenance,
and retry columns. Add an ordering note where final state alone is insufficient.

Link every row to existing deterministic tests. Where current tests do not pin a
field directly, mark that honestly rather than treating code inspection as test
evidence. Run the named tests and the complete plugin library suite on the
unmodified source tree.

## Decision

Choose Option 3. It preserves the distinct authorities required by story
S-039-03 while remaining reviewable as a comparison matrix. It also creates a
stable contract for the next ticket: named transition outcomes may alter code
shape, but the matrix must remain unchanged.

The authoritative deliverable will live in `progress.md`, the required Implement
artifact. Research, design, structure, and plan support the result but do not
duplicate the full matrix. No production or test source will be edited in this
spike; “passes on the unmodified tree” means the matrix is validated by existing
fixtures before later refactoring begins.

## State notation

- `S:` denotes `SeatAssignmentState`.
- `Th:` denotes thread status or presence.
- `Sl:` denotes physical slot reservation/transition state.
- `L:` denotes current attempt lease authority.
- `P:` denotes provenance output.
- `R:` denotes who may initiate the next attempt.

State names will be copied exactly from the current enums. Automatic reclaim is
not represented as a seat terminal state because the seat assignment is removed.
Instead its terminal scheduler shape is `L:none / S:none / Th:absent / Sl:free`
with the old pane fenced or preserved according to the path.

## Path classification

Operator-retained failures:

- delivery exhaustion ends at `DeliveryFailed`;
- assignment fallback exhaustion ends at `RecoveryFailed`;
- unrecoverable initial startup ends at `StartupFailed` with the pane retained;
- exhausted same-pane startup recovery ends at `StartupFailed` with the pane
  fenced.

Automatic reclaims:

- ordinary error signal removes the thread and releases a non-fenced pane;
- session timeout removes the thread after lease-revoke and pane-fence ordering;
- stale-thread reclamation does the same with a failed rather than timed-out
  provenance outcome.

## Retry interpretation

There are three separate bounds, which the map must not conflate:

1. Chat delivery allows the initial send plus one retry in the same generation.
2. Reused-Codex assignment recovery allows one freshly minted successor attempt.
3. SessionStart recovery allows one same-pane replacement relaunch.

Terminal operator-retained states do not automatically re-enter scheduling.
Automatic reclaim paths remove the thread and reservation so the DAG can offer
the ticket again. The next dispatch mints above `lease_high_water`.

## Provenance interpretation

The matrix records whether the teardown invokes `emit_provenance`, its outcome,
and its `fenced` argument. This is distinct from whether a record is physically
written: native tests with an empty ledger intentionally make emission a no-op.
Retained terminal states do not invoke provenance because their thread remains.

## Test evidence policy

Each matrix row will identify a primary fixture. Supplemental fixtures may pin
guards or exact ordering. A test counts as evidence only for fields it asserts or
drives through an asserted composed state. Code inspection will be labeled as
such. The verification commands will be recorded with their exit result.

The focused tests will cover delivery, startup recovery, error signal, session
timeout, and stale reclaim. The full `lisa-plugin` library suite will cover
assignment recovery code reachability and detect unintended environmental
failures. A noted gap remains if no existing test directly asserts the complete
`RecoveryFailed` terminal vector.

## Rejected changes

- Do not introduce a new enum or transition result; T-039-03-02 owns that work.
- Do not add tests; T-039-03-03 owns after-refactor named-state regression.
- Do not merge timeout and stale deadline evaluation; S-039-03 excludes C-07.
- Do not change retries, fencing, signal deletion, or provenance semantics.
- Do not treat a retained failed thread as automatically retryable.
- Do not infer pane closure from thread failure; the paths differ deliberately.

## Success criteria

The deliverable succeeds when all seven paths have an explicit transition chain,
all six required authorities plus retry behavior are shown side by side, ordering
constraints are called out, the primary deterministic fixtures pass without
source edits, the plugin library suite passes, and gaps are visible to the next
two tickets.
