# T-029-02 Review — Codex consecutive-session prompt parity

## Outcome

Fixed the race that made reused Codex panes intermittently fail to submit the
next ticket prompt. Deferred Enter delivery now honors each line's own deadline
instead of allowing any Zellij Timer event to flush the entire queue.

This restores the intended native-TUI sequence:

1. send `/clear` and wait for Codex's clear lifecycle signal;
2. type the next ticket prompt;
3. wait the full composer-settle delay;
4. submit that prompt with Enter.

Claude follows the same shared path and retains its existing behavior.

## Files changed

- `crates/lisa-plugin/src/lib.rs`
  - adds deadline-bearing `PendingEnter` state;
  - adds a host-free due-entry selector;
  - gates Timer-event Enter delivery by deadline;
  - adds two regression tests.
- `docs/active/stories/S-029-codex-integration.md`
  - lists the discovered release-candidate bug ticket.
- `docs/active/tickets/T-029-02-codex-reuse-prompt-timer-race.md`
  - records context and acceptance criteria.
- `docs/active/work/T-029-02/`
  - contains the complete RDSPI artifact set.

No adapter, CLI, hook template, prompt text, routing rule, or public interface
changed.

## Correctness assessment

The previous implementation's comment promised a two-second delay but its data
model stored only a pane ID. Because Timer callbacks are anonymous, the handler
could not know whether the line's timer had fired and drained everything.

The new invariant is local and explicit: `ready_at <= now` is required before a
pane leaves the queue. This remains true no matter which timer caused the event.
The selector preserves the order of both due and future entries.

The dedicated per-line timer remains in place for liveness. The periodic poll
timer is an additional backstop if a host callback arrives at an unusually early
clock boundary.

## Test coverage

New unit coverage directly exercises the regression without a live Zellij host:

- early unrelated timer does not flush;
- entry becomes due at its deadline;
- multiple entries have independent deadlines;
- stable ordering is preserved.

Existing tests cover Codex's `ClearHandshake`, bare native reuse prompt, Codex
context-file selection, finish-up prompt delivery, and scheduler transitions.
The entire workspace suite passes: 621 tests, zero failures.

The optimized `wasm32-wasip1` release build passes, proving the deadline code is
valid in the deployed plugin target. Production plugin Clippy and format/diff
checks pass.

## Open concerns

The fix proves Lisa will not submit early; it does not automate a real Zellij +
Codex second-ticket run in CI. The installed Codex 0.144.x PTY verification in
`docs/knowledge/codex-client/09-native-tui-parity.md` already proves that a
properly delayed text-plus-Enter submission works. A final live loop is still a
valuable release smoke check.

`clippy --all-targets -D warnings` remains red on five pre-existing test-only
style warnings outside this delta. The production plugin lint gate is green.

## Human review focus

Confirm the use of wall-clock `SystemTime` is acceptable for this short UI
deadline. It matches existing scheduler timing state and compiled for WASM. A
future broad timer refactor could use a monotonic abstraction, but that is not
needed for the release-candidate parity fix.

No critical unresolved issue was found in the implementation.
