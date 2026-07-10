# T-029-02 Design — Deadline-aware deferred Enter delivery

## Decision

Replace `VecDeque<PaneId>` with `VecDeque<PendingEnter>`, where each entry owns
the pane and its absolute `ready_at: SystemTime` deadline. On any Timer event,
partition the queue: return only entries whose deadline is due and retain all
future entries. Send Enter only to the returned panes.

## Options considered

### 1. Increase the global delay

Changing two seconds to a larger constant does not fix the race. An unrelated
timer can still flush the queue immediately. Rejected because it changes the
window without enforcing it.

### 2. Give Codex a separate input method

A provider-specific delay or send routine would mask the generic bug and leave
Claude vulnerable to the same timer violation. It would also split a pane I/O
contract that both native TUIs need. Rejected.

### 3. Track timer identities

Zellij's Timer event does not return the identity of the timeout request, so
Lisa cannot reliably match callbacks to queued lines. Rejected as unsupported
by the host API.

### 4. Store absolute deadlines on pending Enter entries

This makes callback identity irrelevant. Every timer event can safely drain
only entries whose deadlines are satisfied. The dedicated timeout created by
`send_line_to_pane` guarantees another event near the deadline. Selected.

## Detailed behavior

`PendingEnter` contains:

- `pane_id: PaneId`
- `ready_at: SystemTime`

`send_line_to_pane` computes `now + ENTER_DELAY_SECS`, appends the record, then
arms the same two-second timeout used today.

`take_due_pending_enters(now)` walks the existing queue once. Due pane IDs are
returned in queue order; future records are pushed into a replacement queue in
their existing order. The state queue is then replaced.

`flush_pending_enters(now)` performs the Zellij host writes for the due IDs.
The Timer handler passes `SystemTime::now()`.

The comparison treats `now >= ready_at` as due. `SystemTime` is already used
throughout scheduler state, so this introduces no new clock abstraction.

## Correctness properties

- No Enter is delivered before its own deadline.
- An unrelated poll, exit-grace, or line-submit timer cannot force early input.
- Every Enter still has a timer armed for eventual delivery.
- Multiple entries remain independent.
- Delivery order among simultaneously due entries is stable.
- Future entries retain their original queue order.
- The clear handshake and adapter interfaces do not change.

## Testing strategy

Add host-free unit tests for the selector:

1. Early `now` returns an empty list and retains the entry.
2. At/after deadline returns the pane and removes the entry.
3. A queue with due/future/due entries returns the two due panes in order and
   leaves the future pane queued.

Existing tests that inspect `pending_enters.len()` remain valid. No live Codex
process is needed to prove the timer invariant; the installed-client smoke test
already established that correctly delayed text plus Enter works.

## Risk

The main risk is starving an Enter if the dedicated Timer event fires slightly
before `SystemTime` reaches the computed deadline. Zellij's timeout is expected
to be no earlier than requested, but unrelated scheduling and clock precision
can vary. The persistent periodic poll timer provides a backstop. To make the
guarantee explicit without busy re-arming, entries remain queued and will be
visited by the next Timer event.
