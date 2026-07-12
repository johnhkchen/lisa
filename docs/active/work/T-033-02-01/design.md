# Design: surface handoff state in the dashboard

## Objective

Expose scheduler-owned seat assignment truth in the Operations dashboard so an
operator can distinguish a recycled Codex delivery awaiting acceptance, an
acknowledged owner, and a timed-out assignment undergoing recovery.

The display must be a projection of `State::seat_assignments`. It must not
derive ownership from terminal contents, pane names, route strings, thread
existence, or transport transition state.

## Option 1: infer labels inside `ui.rs`

The renderer could combine `ActiveThread.route`, `SlotInfo.transitioning`, and
thread status to guess whether a Codex pane is pending or recovering.

Advantages:

- no additional UI data type;
- minimal change to `State::to_ui_state`;
- labels can be added only in the renderer.

Disadvantages:

- neither the route nor `transitioning` encodes provider acknowledgment;
- pending and recovery can share transport states;
- owned and pending seats both have running threads and ticket reservations;
- route strings are presentation values, not scheduler identity;
- any guess would violate the ticket's explicit scheduler-source requirement;
- future transport changes could silently change dashboard meaning.

Decision: rejected. The required distinctions cannot be reconstructed from the
current UI fields without inventing false equivalences.

## Option 2: preformat a status string in `lib.rs`

`State::to_ui_state` could look up `SeatAssignmentState` and place a ready-made
string such as `assigned-pending-ack` on `SlotInfo`.

Advantages:

- direct scheduler sourcing;
- small implementation;
- the UI renderer only prints a string.

Disadvantages:

- presentation vocabulary and scheduler projection become mixed in `lib.rs`;
- arbitrary strings permit invalid states and make exhaustive handling
  impossible;
- color and precedence logic would depend on string comparisons or additional
  preformatted fields;
- tests could pass malformed labels that production can never emit;
- future assignment variants would not force a compiler-visible UI decision.

Decision: rejected in favor of a typed UI boundary.

## Option 3: expose the scheduler enum to `ui.rs`

The private `SeatAssignmentState` could be made visible and stored directly in
`ui::SlotInfo`.

Advantages:

- no translation enum;
- exact state, including generation and deadline, reaches the UI;
- exhaustive matching is possible.

Disadvantages:

- presentation becomes coupled to scheduler transport details;
- generations and absolute deadlines are unnecessary for the at-a-glance
  indicator;
- the UI would depend on a runtime state type defined by its parent module;
- changing deadline representation would churn the UI despite no display
  change;
- it weakens the existing projection boundary established by `to_ui_state`.

Decision: rejected. The dashboard needs semantic state, not scheduler payload.

## Option 4: typed UI assignment status projected by `to_ui_state`

Add a small enum in `ui.rs` containing the dashboard-relevant assignment
states. Add an optional instance to `SlotInfo`. Map the pane-keyed scheduler enum
to that type while constructing slot data.

Advantages:

- scheduler truth is the sole source;
- UI remains independent of generation/deadline mechanics;
- variants are exhaustive and label/color decisions remain presentation-owned;
- tests can inspect the projection and the rendered dashboard separately;
- `None` continues to represent an unassigned or legacy slot;
- future scheduler variants cause an explicit mapping decision.

Disadvantages:

- introduces a second, intentionally narrower enum;
- every `SlotInfo` test fixture gains one field;
- mapping logic must be maintained when assignment variants change.

Decision: selected. The duplication is a useful anti-corruption boundary
between scheduler mechanics and dashboard semantics.

## UI status vocabulary

The UI-facing enum will use semantic variants and exact stable labels:

- `AssignedPendingAck` -> `assigned-pending-ack`
- `Owned` -> `owned`
- `Recovering` -> `recovering`
- `RecoveryFailed` -> `recovery-failed`

The acceptance criterion names the first three. Surfacing the fourth avoids
turning an actionable terminal scheduler state back into generic idle/running
presentation.

The enum owns its label method. Scheduler code maps variants but does not
format dashboard text.

## Placement in the thread table

The existing STATUS column is the correct at-a-glance surface:

- it already communicates Running, Awaiting, Parked, Winding Down, and Idle;
- operators scan it for lifecycle conditions;
- adding another column would widen the dashboard and duplicate status meaning;
- the ticket asks for an indicator, not detailed generation/deadline data.

For active threads, an assignment status replaces generic `Running`. Thus a
normal acknowledged seat displays `owned`, while recycled handoff states display
their more specific labels.

The longest label is 20 characters. Increase the STATUS field width from 14 to
20 and update the separator width so labels do not collide with TIME.

## Precedence rules

Active row precedence will be:

1. awaiting-human presentation;
2. explicit assignment status;
3. legacy generic `Running` fallback.

Awaiting-human remains first because it describes immediate operator action
and mirrors the scheduler exemption from reclamation. Codex recycled handoff
does not produce `.awaiting`, so this precedence does not obscure the ticket's
three required states.

Parked rows retain `Parked`, since parking is the thread's operative condition.
Unoccupied rows retain winding-down/idle behavior unless a later ticket expands
failed-seat presentation. The three required handoff states all retain a
running thread and therefore use the active-row path.

## Color decisions

- `assigned-pending-ack`: yellow, indicating an incomplete bounded handshake;
- `owned`: green, indicating accepted normal operation;
- `recovering`: bright yellow, indicating active fallback;
- `recovery-failed`: red, indicating operator action;
- legacy `Running`: green, unchanged;
- `Awaiting`: cyan, unchanged.

The exact text is the accessibility and snapshot contract; color is secondary.

## Scheduler projection

While enumerating `agent_slots`, `State::to_ui_state` will:

1. take the physical `pane_id` from each slot;
2. call `seat_assignment(pane_id)`;
3. map the returned scheduler variant to the UI enum;
4. store it in that slot's `SlotInfo.assignment_status`.

This lookup is pane-keyed and independent of terminal contents. It preserves the
same physical-seat identity used by acknowledgment and timeout transitions.

No inference from `ticket_id` is necessary. `ticket_id` remains useful for row
content and occupancy counts, but it does not prove ownership.

## Snapshot test strategy

The acceptance test should be scheduler-integrated rather than a UI-only fixture.
It will use the existing resident-Codex scheduling fixture to create real
assignment states.

Branch A:

1. schedule a ready ticket onto a resident Codex pane;
2. render the thread table through `to_ui_state`;
3. capture `assigned-pending-ack`;
4. inject an exact matching acknowledgment;
5. render again and capture `owned`.

Branch B:

1. create the same pending assignment;
2. deliver the prompt to arm its deterministic deadline;
3. advance timeout evaluation to that deadline;
4. render again and capture `recovering`.

The test will combine stable, ANSI-free row text into one literal snapshot. A
small test-only ANSI stripping helper avoids coupling the expected output to
escape sequences. The snapshot should contain the full row fields so it proves
that one recycled Codex pane and ticket changes status, not merely that labels
exist somewhere.

## API visibility for testing

The renderer's thread-section function can be `pub(crate)` so the scheduler
test in the parent module renders the same production table without printing to
stdout or exposing a public external API.

Alternatively the full-dashboard line renderer could be exposed, but it adds
dynamic status, activity, and timing content unrelated to this criterion. A
snapshot of the dashboard's Threads section is more stable and directly targets
the at-a-glance indicator.

## Compatibility

- Existing slots without a `seat_assignments` entry use the `Running` fallback.
- Existing UI fixtures will set `assignment_status: None`.
- Claude and fresh Codex assignments already record `Owned`; their active row
  will say `owned`, accurately reflecting scheduler state.
- Awaiting-human, parked, idle, and winding-down semantics remain unchanged.
- No configuration, persistence, ticket parsing, or Zellij protocol changes are
  required.

## Decision summary

Introduce a narrow UI assignment-status enum, project it from the scheduler's
pane-keyed assignment map in `to_ui_state`, render it in the existing STATUS
column, and prove the three required states with one scheduler-driven dashboard
thread snapshot test.
