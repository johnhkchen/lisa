# T-029-02 Research — Codex reuse prompt timer race

## Scope

The observed defect is limited to the second and later ticket assigned to a
resident Codex TUI pane. Fresh Codex launch prompts work. Claude session reuse
works. The relevant boundary is therefore the shared pane-input and native-TUI
reset machinery in `crates/lisa-plugin/src/lib.rs`, not prompt construction or
ticket routing.

## Existing reuse flow

`schedule_ready_tickets` resolves an adapter per ticket. When a compatible pane
already has a resident session, both native adapters currently advertise
`ResetStrategy::ClearHandshake`.

The scheduler sends `/clear`, marks the slot `WaitingForClear`, and waits for
`pane-<id>.cleared`. `handle_cleared_signal` resolves the incoming ticket's
adapter, builds `reuse_prompt`, sends it through `send_line_to_pane`, then moves
the slot back to `Idle`.

Codex and Claude prompt construction are already parallel:

- Claude references `CLAUDE.md`.
- Codex references `AGENTS.md`.
- Both use the shared `ticket_prompt` body.
- Both follow-ups are `FollowUp::TypeIntoPane`.

The installed Codex CLI is 0.144.1. The repository's native-TUI verification
documents that `/clear`, `SessionStart[source=clear]`, delayed text submission,
and inherited Lisa environment variables work on 0.144.0. That makes the
adapter contract credible and focuses the defect on Lisa's timing.

## Pane input implementation

`send_line_to_pane` writes all text immediately. It appends only the `PaneId` to
`pending_enters`, arms a two-second Zellij timer, and increments the global
timer count.

`Event::Timer` does not identify which requested timeout fired. Its handler
currently calls `flush_pending_enters`, which drains the entire queue, before
handling the timer count and optional poll tick.

Lisa also arms the periodic scheduler timer. Consequently, the sequence can be:

1. a scheduler poll sends the reused Codex prompt;
2. `send_line_to_pane` queues Enter for two seconds later;
3. a previously armed scheduler timer fires milliseconds later;
4. the generic Timer handler drains the Enter queue immediately;
5. Codex receives Enter before its composer commits the pasted text.

This violates the function's documented two-second guarantee. The race is more
visible in Codex because its paste-burst/composer processing needs the intended
delay; Claude happens to tolerate more early submissions.

## Constraints

- Zellij Timer events carry elapsed time, not a caller-controlled timer ID.
- Timer callbacks therefore cannot be correlated by identity.
- The pending queue must carry its own absolute deadline.
- Any Timer event may be used as an opportunity to deliver entries whose
  deadlines have passed, but must leave future entries queued.
- A dedicated Enter timer is still needed so a due entry is eventually visited
  when there are no other events.
- Native unit tests cannot call Zellij pane-write host functions. Deadline
  selection must be factored into a pure, host-free operation.
- Existing user work is present only as an untracked `.codex/` directory and
  must remain untouched.

## Test surface

The core regression can be tested with synthetic `SystemTime` values:

- an unrelated early timer selects no pending Enter;
- a later timer selects the due Enter;
- mixed deadlines select only due entries while preserving future order;
- equal/decreasing insertion order remains deterministic.

Existing scheduler and adapter tests cover Codex clear-handshake selection,
Codex reuse prompt construction, and queued Enter creation after provider
recycling. No host-level Zellij write is required for the new regression.

## Root cause

`pending_enters` stores no deadline, and `flush_pending_enters` treats every
Timer event as if it were every Enter timer. The defect is a shared timer
bookkeeping bug exposed reliably by Codex reused-session input.
