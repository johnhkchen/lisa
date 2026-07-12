# Research: T-032-01 Zellij pane lifecycle names

## Ticket scope

- The ticket asks Lisa to own terminal-pane titles across assignment, provider reuse,
  provider switching, successful completion, and idle states.
- Assigned titles must expose the actual resolved agent, ticket ID, and parsed ticket
  title.
- Idle titles distinguish a resident reusable client from an empty shell.
- Title input must be sanitized and bounded without sacrificing the agent or ticket ID.
- Completion is transaction-gated: a pane cannot become idle until the completion commit
  is verified and the slot is actually released.
- Repeated polls must not emit redundant rename operations.
- The ticket begins in `phase: research`; no earlier artifact exists.

## Repository and workflow constraints

- `CLAUDE.md` identifies Lisa as a Rust Zellij WASM plugin plus CLI and core library.
- `AGENTS.md` points all agent clients to `CLAUDE.md` as the project source of truth.
- `docs/knowledge/rdspi-workflow.md` requires all six RDSPI artifacts in one continuous
  pass and prohibits manual ticket phase/status edits.
- Source commits must use `lisa commit-ticket` with exact ticket-owned paths.
- The ordinary worktree is already dirty with unrelated modified and untracked files.
  Those files must remain untouched and must not be included in this ticket's commit.
- The ticket itself and its work directory are currently untracked repository content
  managed by Lisa's eventual completion transaction; source commits should not absorb them.

## Crate boundaries

- `crates/lisa-core/src/types.rs` defines `Ticket`, `Thread`, `PluginConfig`, phases,
  statuses, and activity events.
- `Ticket.title` is a parsed YAML frontmatter string and is the authoritative human title.
- `Thread.client` is the actual client snapshotted at spawn.
- `Thread.route` stores the full `ResolvedRoute`, including requested and actual agent,
  model, substitution flag, and note.
- `crates/lisa-core/src/client.rs` defines the closed client vocabulary: `claude` and
  `codex`. Its `Display` implementation returns these canonical lowercase names.
- `crates/lisa-core/src/route.rs` resolves ticket hints against the loop default.
- Invalid requested agents fall back to the configured default; `ResolvedRoute.agent`
  is explicitly the client that will actually run.
- `crates/lisa-plugin/src/adapter.rs` converts a resolved route into provider-specific
  launch, reset, reuse, follow-up, and exit behavior.
- `crates/lisa-plugin/src/lib.rs` owns scheduler state and all pane lifecycle transitions.
- `crates/lisa-plugin/src/ui.rs` renders the dashboard but does not control terminal panes.
- `crates/lisa-cli/src/loop_cmd.rs` creates twice `max_threads` empty terminal panes in a
  stacked layout; those panes have no Lisa lifecycle name today.

## Scheduler-owned pane state

- `AgentSlot` is the scheduler's record for each pre-created terminal pane.
- `pane_id` is the Zellij terminal pane identifier.
- `ticket_id: Option<TicketId>` distinguishes assigned from released slots.
- `has_session` distinguishes a resident agent TUI from an empty shell.
- `last_client: Option<AgentClient>` identifies the resident or incoming provider.
- `transition_state` represents idle, clear-handshake, and cross-provider exit states.
- `cooldown_until` and `last_activity_at` prevent unsafe immediate reuse.
- `State.agent_slots` is populated once from the first usable `PaneUpdate`.
- Slot discovery filters out plugin panes and initializes each terminal slot as an empty,
  unassigned shell with no resident client.
- `State.threads` maps ticket IDs to active execution records and preserves actual routing.
- `State.dag` owns parsed `Ticket` values, including frontmatter title and routing hints.

## Assignment paths

- `schedule_ready_tickets` is the single scheduler entry point for ready work.
- It resolves the adapter and `ResolvedRoute` before concurrency and slot-selection gates.
- Provider-cap decisions use `route.agent`, so actual routing is already established before
  the pane is mutated.
- `find_slot_for_client` prefers fresh or same-provider slots and otherwise selects a quiet
  opposite-provider session for recycling.
- A fresh pane receives the adapter launch command and then records `has_session = true`.
- A same-provider resident pane receives `/clear`, enters `WaitingForClear`, and receives
  the new prompt only after a `.cleared` signal or safe timeout recovery.
- A cross-provider slot receives the resident adapter's `/exit`, enters `WaitingForExit`,
  and receives the incoming provider's fresh launch after the exit grace period.
- In all three cases, scheduling binds `slot.ticket_id`, stamps `slot.last_client` with
  `route.agent`, and creates a `Thread` whose `client` and route use the actual resolution.
- The current code sends pane input before it records the ticket binding and thread.
- No current assignment path calls a pane rename API.

## Transition and recovery paths

- `handle_cleared_signal` resolves the adapter again from the bound ticket and sends the
  reuse prompt, then returns the slot transition to `Idle`.
- `check_transition_timeouts` recovers missing stop, clear, and exit signals only after
  bounded deadlines and required quiet periods.
- The clear-timeout path submits the same pending ticket prompt without releasing it.
- The exit-timeout path launches the incoming provider and marks its session resident.
- Awaiting-human guards suppress pane input and preserve the assignment.
- Timeout warnings and recovery do not release a ticket slot.
- Error-signal handling marks a thread failed, emits provenance, releases the slot, and
  removes the thread. This is a release path, but it is not successful completion.
- Session-budget reclaim similarly has teardown behavior elsewhere in the scheduler and
  must be understood separately from commit-gated Done publication.

## Completion and release paths

- Completion requests enter `pending_completions` and launch the isolated host completion
  command.
- While pending, `rebuild_dag` masks observed Done frontmatter back to prior phase/status.
- `handle_completion_result` rejects a nonzero exit, malformed commit ID, or inability to
  verify durable Done state.
- All failure branches retain the thread and assigned slot.
- Only the verified-success branch completes the thread, emits provenance, calls
  `release_slot_for_ticket`, removes the thread, and schedules dependents.
- `release_slot_for_ticket` clears `ticket_id`, preserves `has_session`, sets a cooldown,
  and logs release.
- `sweep_stale_slots` also calls the same release helper, but excludes pending completions.
- Thus the release helper is the common boundary for a slot becoming scheduler-idle.
- The resident provider remains in `last_client` after normal release.
- A clean shell can arise during provider-exit recovery if the incoming ticket disappears;
  that path explicitly clears `has_session` and `last_client`.

## Zellij API and permissions

- `crates/lisa-plugin/Cargo.toml` depends on `zellij-tile = "0.43"`; the lockfile resolves
  version 0.43.1.
- The installed `zellij-tile` shim exposes
  `rename_terminal_pane(terminal_pane_id, new_name)`.
- The API emits `PluginCommand::RenameTerminalPane` through the Zellij host boundary.
- The plugin already requests `PermissionType::ChangeApplicationState`, alongside stdin,
  application-state reading, and command execution.
- Slot discovery already receives the pane IDs needed by the rename call.
- Zellij host calls cannot be meaningfully executed in native unit tests, matching the
  existing pattern around stdin writes.

## Existing test organization

- Most plugin scheduler tests are colocated in the large `#[cfg(test)]` module in
  `crates/lisa-plugin/src/lib.rs`.
- Test helpers construct `State`, `AgentSlot`, ticket directories, DAGs, and mixed-provider
  scenarios directly.
- Existing tests cover provider affinity, provider caps, fresh scheduling behavior,
  clear handshakes, clear/exit timeout recovery, mixed-provider recycling, completion
  commit success, and completion commit failure.
- Existing completion tests explicitly assert that slots remain assigned on failure and
  are released only after verified success.
- Native tests avoid some Zellij host functions; pure state transitions and queued input
  are asserted instead.
- Adapter tests already verify actual-vs-requested route fallback and provider-specific
  launch commands.
- Workspace verification commands are documented as `cargo test --workspace`, the WASM
  release build, and `just check`; the ticket additionally requires plugin Clippy.

## Input and display constraints

- YAML parsing yields a Rust `String`; it may contain Unicode and escaped control
  characters.
- Terminal-pane titles are operator-facing strings passed across a host API.
- The ticket explicitly identifies control characters as unsafe input.
- The ticket does not prescribe whether the display limit is bytes, scalar values, or
  grapheme clusters, only that it be documented and deterministic.
- Stable scan keys are the complete canonical agent name and complete ticket ID.
- Only the human title may be shortened during truncation.
- The examples use a middle dot with surrounding spaces as the field separator and the
  exact idle forms `<resident-agent> · idle` and `lisa · idle`.

## Observed gaps

- There is no pane-name formatter in any crate.
- `AgentSlot` has no last-applied title cache.
- Slot discovery does not label empty shells.
- Assignment does not rename before command or prompt submission.
- Release does not rename retained sessions to their resident-provider idle form.
- Clean-shell recovery does not restore `lisa · idle`.
- Existing routing tests prove fallback resolution but do not connect it to pane titles.
- Existing lifecycle tests do not observe a host-independent rename intent.
- No live mixed Claude/Codex pane-name evidence is present for this ticket.

## Constraints carried into Design

- The formatter must consume actual `AgentClient`, ticket ID, and parsed `Ticket.title`.
- Scheduler lifecycle paths are authoritative; adapters should not independently name panes.
- Rename deduplication requires state associated with a physical pane.
- Assignment naming must precede or coincide with every initial prompt submission path.
- Idle naming must follow actual slot release and reflect resident-session truth.
- Non-release states must retain their assigned name.
- Native tests need an observable seam that does not require a live Zellij host.
- Live Zellij validation may depend on the availability of an active mixed-provider loop and
  cannot be fabricated from unit-test output.
