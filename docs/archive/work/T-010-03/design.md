# T-010-03 Design: Auto-complete Review tickets on Stop signal

## Options

### Option A: Add `.stopped` signal processing as a new method in `check_idle_signals()` scope

Extend the existing `check_idle_signals()` to also look for `.stopped` files and handle Review auto-complete within the same function.

**Pros:** No new function, minimal diff.
**Cons:** Mixes two distinct signal types in one function. When T-010-02 lands, it will need to factor `.stopped` processing out anyway. Pollutes the idle signal method with unrelated logic.

### Option B: Add a new `check_stopped_signals()` method alongside `check_idle_signals()`

Create a dedicated function that scans for `.stopped` signal files and handles Review auto-complete. Called from `poll_tick()` alongside `check_idle_signals()`.

**Pros:** Clean separation of concerns. When T-010-02 adds the transition state machine, `handle_stopped_signal()` simply gains an additional case (WaitingForStop) without disrupting the Review auto-complete logic. Follows the existing pattern of `check_idle_signals()`.
**Cons:** Slightly more code than Option A.

### Option C: Process `.stopped` signals inside `poll_tick()` inline

Add the signal scanning directly in `poll_tick()` without a helper method.

**Pros:** Simplest possible change.
**Cons:** Makes `poll_tick()` longer and harder to read. Not reusable when T-010-02 lands.

## Decision: Option B

Option B provides clean separation that aligns with T-010-02's eventual design. The new `check_stopped_signals()` method follows the exact same pattern as `check_idle_signals()`:
1. Read `.lisa/signals/` directory
2. Find `pane-{id}.stopped` files
3. Delete signal file immediately
4. Resolve pane ID → slot → ticket
5. Check conditions (ticket in Review, thread Parked)
6. Auto-complete if conditions met

When T-010-02 implements the full transition state machine later, it will refactor `check_stopped_signals()` into `check_transition_signals()` that handles both the WaitingForStop transition case and the Review auto-complete case.

## Rejected

**Option A** rejected because it conflates idle and stopped signals. These are semantically different: idle means "Claude prompted for input," stopped means "Claude finished responding." Mixing them invites subtle bugs.

**Option C** rejected because inlining in `poll_tick()` doesn't scale and won't compose with T-010-02.

## Design details

### Auto-complete conditions

A `.stopped` signal triggers auto-complete when ALL of:
1. The signal file is `pane-{id}.stopped`
2. A slot exists with that `pane_id`
3. The slot has a `ticket_id` assigned
4. The ticket's phase is `Phase::Review` (from the DAG)
5. The thread is `Parked` or `Running` (not already `Completed`)

Condition 5 is a safety check — if the thread was already completed by another path (manual `[d]`, audit sweep), we skip.

### Auto-complete actions (same as `mark_ticket_done()`)

1. Update ticket frontmatter: `phase: done`
2. Update ticket status: `status: done`
3. Mark thread as `Completed`
4. Release slot
5. Remove thread from tracking
6. Log `TicketPhaseChanged` and `Info` events
7. Rebuild DAG and schedule ready tickets

### Signal file handling

- Delete signal file before processing (prevents re-trigger)
- If multiple `.stopped` files exist for the same pane, only the first matters (rest are stale)
- Ignore `.stopped` files for unknown panes (no slot match)
- Ignore `.stopped` files for panes with no ticket assigned

### Non-Review `.stopped` signals

For tickets NOT in Review phase, `.stopped` signals are ignored (deleted silently). This is correct because:
- During active phases (Research, Design, etc.), Stop fires on every turn
- These signals are meaningless for phase advancement
- T-010-02 will later use them for transition state management

### Test strategy

1. **Unit test: Review auto-complete triggers** — Set up state with a ticket in Review, parked thread, and a `.stopped` signal file. Verify ticket phase updated, thread removed, slot released.
2. **Unit test: Non-Review signal ignored** — Set up state with a ticket in Implement phase. `.stopped` signal should be deleted but no auto-complete.
3. **Unit test: No ticket on slot ignored** — `.stopped` signal for a pane with no assigned ticket.
4. **Unit test: Already-completed thread ignored** — Thread already in Completed status, `.stopped` signal should not re-complete.
