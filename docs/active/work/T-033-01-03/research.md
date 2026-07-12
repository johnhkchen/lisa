# Research: gate ownership on acknowledgment

## Ticket boundary

- Ticket `T-033-01-03` starts in `research`.
- Its acceptance criterion is scheduler-visible behavior for a recycled seat.
- A recycled Codex seat must remain not-owned until matching acknowledgment.
- A stale acknowledgment must not claim the seat.
- A duplicate acknowledgment must not perform a second ownership transition.
- Timeout and recovery behavior belongs to dependent ticket `T-033-01-04`.
- UI projection of the state belongs to later story work.
- Ticket frontmatter phase and status are Lisa-owned and must not be edited here.

## Prerequisite assignment model

- `crates/lisa-plugin/src/lib.rs` defines `SeatAssignmentState`.
- Its variants are `AssignedPendingAck`, `Owned`, and `Recovering`.
- `State::seat_assignments` maps physical pane IDs to assignment state.
- `AgentSlot::ticket_id` separately records the scheduler reservation.
- The split prevents a reservation from automatically meaning acknowledged ownership.
- `State::seat_assignment` returns the exact state for a pane.
- `State::seat_is_owned` is true only for the `Owned` variant.
- Release removes the assignment map entry.
- Pending and recovering states therefore already report not-owned.

## Assignment classification at scheduling

- `State::schedule_ready_tickets` resolves the ticket route per assignment.
- It selects a compatible seat or a cross-provider recycle seat.
- `reused_seat` captures whether the physical pane hosted a session before handoff.
- A recycled or reused Codex seat is inserted as `AssignedPendingAck`.
- Fresh Codex launches remain immediately `Owned` under the prerequisite contract.
- Every Claude assignment remains immediately `Owned`.
- Cross-provider recycling first sends the resident provider's `/exit` command.
- Same-provider reuse sends `/clear` and waits for `.cleared`.
- Both transition paths retain the pending assignment state.
- Clear and exit timeouts deliberately do not promote ownership.

## Transition state separation

- `TransitionState` models transport, not assignment acceptance.
- `WaitingForClear` means the old context is being reset.
- `WaitingForExit` means the old provider is leaving the pane.
- `Idle` means no reset transport is outstanding.
- None of these states is evidence that Codex accepted the new ticket.
- Existing tests assert that timeout-driven prompt delivery leaves Codex pending.
- This boundary should remain intact when acknowledgment is connected.

## Existing Codex acknowledgment detector

- `crates/lisa-plugin/src/codex_ack.rs` was added by `T-033-01-02`.
- `CodexAssignmentRef` contains a borrowed ticket ID and a `u64` generation.
- `tag_codex_assignment` appends structured metadata to a prompt.
- The canonical marker begins with `LISA_ASSIGNMENT ` on its own line.
- Marker JSON contains `ticket_id` and `generation`.
- JSON serialization prevents ambiguous ticket interpolation.
- `detect_codex_ack` parses a minimal lifecycle payload.
- It requires `hook_event_name == "UserPromptSubmit"`.
- It requires a prompt containing a valid marker line.
- It compares both ticket ID and generation.
- Invalid JSON and malformed markers return false.
- Non-submit events return false.
- Previous-ticket and previous-generation fixtures return false.

## Missing scheduler identity

- `SeatAssignmentState::AssignedPendingAck` currently carries no data.
- `State` currently has no assignment-generation counter.
- A ticket can be retried or reassigned more than once.
- Ticket identity alone cannot distinguish delayed evidence from an older attempt.
- The detector already requires a generation, but the scheduler does not allocate one.
- The pending state must make the expected generation available at acknowledgment time.
- Generation uniqueness only needs to hold within the live scheduler state.
- The plugin does not persist live seat assignment state across restarts today.

## Prompt construction boundary

- `crates/lisa-plugin/src/adapter.rs` owns provider-specific prompt delivery.
- `SpawnContext` currently carries ticket directory, ticket ID, and pane ID.
- `CodexAdapter::reuse_prompt` returns the bare ticket prompt.
- `CodexAdapter::interactive_line` embeds the same prompt in the launch command.
- `ClaudeCodeAdapter` uses the shared ticket prompt without Codex metadata.
- The scheduler calls adapter methods at each actual delivery boundary.
- Cross-provider recycle defers launch command delivery until exit grace expires.
- Same-provider reuse defers reuse-prompt delivery until `.cleared` arrives.
- The generation must therefore travel with `SpawnContext` through deferred delivery.
- Tagging inside the Codex adapter covers fresh-command and bare-prompt representations.
- Claude can ignore the optional Codex generation without output changes.

## Existing Codex lifecycle hooks

- `crates/lisa-cli/src/templates.rs` generates `.codex/hooks.json`.
- Current generated hooks cover `PostToolUse`, `Stop`, and `SessionStart[clear]`.
- Their scripts emit heartbeat, stopped, and cleared files.
- No generated `UserPromptSubmit` hook exists.
- No current signal file contains the lifecycle JSON payload.
- Signal filenames are pane-scoped through `LISA_PANE_ID`.
- Hook scripts read event JSON from standard input.
- The plugin polls `.lisa/signals` on each timer tick.
- Signal files are consumed and removed by type-specific scanners.
- Existing script generation is shared by `lisa init` and validation.
- Merge logic preserves user-owned hook groups and is idempotent.

## Hook installation lifecycle

- `init.rs` plans creation or update of `.codex/hooks.json`.
- Hook script constants are written into `.lisa/hooks/` during init.
- Validation checks required script presence and executable mode.
- Codex validation compares current hooks with `merge_codex_hooks` output.
- Adding a Lisa hook to the merge function makes stale installs diagnosable.
- Existing repositories receive updates through the same init planning flow.
- The worktree's generated `.lisa/hooks` files already contain unrelated edits.
- Ticket source changes must avoid claiming those project-instance files.

## Plugin signal consumption patterns

- Each scanner tolerates a missing signal directory.
- Filenames are parsed as `pane-<id>.<suffix>`.
- Files are normally removed immediately to prevent retriggering.
- Heartbeat signals update liveness and question-gating state.
- Transition signals update only reset transport state.
- Error signals reclaim the assigned thread.
- Acknowledgment needs a separate scanner because its file body is semantic.
- The scanner must read payload bytes before removal.
- It must resolve the current slot ticket and pending generation.
- It must invoke `detect_codex_ack` before mutating assignment state.
- It must not infer acceptance from file presence alone.

## Poll ordering

- `poll_tick` consumes heartbeat signals first.
- It then handles awaiting, artifacts, idle, transitions, and errors.
- Transition timeouts run after signal consumers.
- Acknowledgment promotion is independent of phase advancement.
- It can run near other signal consumers before recovery/timeouts are introduced.
- The exact order relative to heartbeat does not change acknowledgment truth.
- Promotion should occur before future acknowledgment deadline evaluation.
- `T-033-01-04` can later place deadline evaluation after this scanner.

## Duplicate and stale behavior

- A matching acknowledgment should mutate only `AssignedPendingAck`.
- After promotion, the seat is `Owned` and no pending generation remains.
- A duplicate payload then has no eligible pending assignment.
- A payload for an unknown or released pane has no eligible assignment.
- A payload for the same pane but previous ticket fails ticket comparison.
- A payload for the same ticket but previous generation fails generation comparison.
- A malformed payload fails the detector and leaves state unchanged.
- Consuming rejected files prevents endless repeated classification.

## Test infrastructure

- Plugin unit tests live in the `#[cfg(test)]` module in `lib.rs`.
- Helpers construct `State`, slots, DAG tickets, and temporary signal directories.
- Existing recycled-Codex tests verify the initial pending state.
- Detector fixtures are available through `include_str!` in `codex_ack.rs`.
- Scheduler tests can construct lifecycle payloads with `serde_json::json!`.
- A focused test can call a direct acknowledgment handler without filesystem timing.
- A signal-consumer test can prove the live file boundary separately.
- State transition counting can be represented by a boolean return from the handler.
- The acceptance criterion requires exactly one transition, not an activity-log count.

## Constraints and assumptions

- The implementation must compile for native tests and `wasm32-wasip1`.
- No new dependency is needed; Serde and filesystem APIs are already used.
- Shell hook writes should avoid exposing partial JSON to the polling plugin.
- Payloads may contain newlines and shell-sensitive bytes.
- Standard-input copying to a temporary file preserves the payload without interpolation.
- Atomic rename within `.lisa/signals` prevents partial reads.
- One pending prompt per pane is expected; later prompt submission supersedes older files.
- Fresh Codex ownership remains unchanged because the prerequisite explicitly retained it.
- Recovery, deadlines, and user-visible state are out of scope.

## Research conclusion

The codebase already separates reservation from ownership and already contains a strict
ticket/generation acknowledgment classifier. The missing connection spans three existing
boundaries: allocate and retain a generation with pending scheduler state, add that identity
to the Codex prompt at the adapter boundary, and carry the native `UserPromptSubmit` JSON
through a pane-scoped signal file into a scheduler handler. Promotion can then be an exact
pending-state transition, making stale and duplicate events inert without coupling ownership
to clear, exit, heartbeat, terminal output, or timeout behavior.
