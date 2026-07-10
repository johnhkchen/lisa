# T-029-02 Structure — Timer-safe pane submission

## Modified files

### `crates/lisa-plugin/src/lib.rs`

Add an internal `PendingEnter` record beside the existing transition types.
It owns a terminal/plugin `PaneId` and the absolute time at which its Enter may
be delivered.

Change `State.pending_enters` from `VecDeque<PaneId>` to
`VecDeque<PendingEnter>`.

Change `send_line_to_pane` to enqueue a deadline-bearing record after writing
the characters. Preserve the current two-second timeout and timer accounting.

Add `take_due_pending_enters(now) -> Vec<PaneId>` as the host-free queue
selection boundary. It removes only due entries and retains future entries.

Change `flush_pending_enters` to accept `now`, call the selector, and write CR
only to due panes.

Change the Timer event handler to pass the current wall-clock time.

Add unit tests in the existing `mod tests` for early-timer retention and mixed
deadline partitioning. Existing queue-length assertions need no semantic change.

### Board and work artifacts

- `docs/active/tickets/T-029-02-codex-reuse-prompt-timer-race.md`
- `docs/active/stories/S-029-codex-integration.md`
- `docs/active/work/T-029-02/research.md`
- `docs/active/work/T-029-02/design.md`
- `docs/active/work/T-029-02/structure.md`
- `docs/active/work/T-029-02/plan.md`
- `docs/active/work/T-029-02/progress.md`
- `docs/active/work/T-029-02/review.md`

## Public interfaces

None. All changed types and methods remain private to `lisa-plugin`.

## Data flow

`send_line_to_pane(text, pane)` writes text, records `(pane, deadline)`, and
arms a timer. A Timer event asks the queue for entries due at the current time.
Only those panes receive CR. The scheduler then performs its existing timer
accounting and poll behavior.

## Unchanged boundaries

- `AgentAdapter`, `CodexAdapter`, and `ClaudeCodeAdapter`
- `/clear` and `.cleared` signal handling
- ticket prompt text and context-file selection
- provider routing and pane affinity
- timeout constants and Zellij permissions
- CLI hook generation

## Ordering

1. Introduce the record and queue selector.
2. Wire enqueue and timer flush to deadlines.
3. Add selector regression tests.
4. Run formatting and focused tests.
5. Run workspace, WASM, and Clippy gates.
6. Record verification in progress and review artifacts.
