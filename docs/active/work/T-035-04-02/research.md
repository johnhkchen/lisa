# T-035-04-02 Research — recover dquote in the same pane

## Ticket boundary

- The ticket begins in Research and requires all remaining RDSPI phases.
- The observed failure occurred before either native provider existed.
- The shell was left in zsh's `dquote>` continuation state.
- Text intended as a provider command had an unmatched double quote.
- A provider `/exit` at that point is only additional quoted shell input.
- Recovery is limited to the already reserved physical pane.
- Recovery must be bounded to one relaunch.
- The failed attempt must lose authority before the relaunch.
- The replacement attempt must use a strictly greater attempt ID.
- Process start and chat assignment acceptance remain separate evidence.
- Started providers and genuinely owned providers must not receive the shell interrupt.
- E-033 acknowledgment and E-034 lease fencing are compatibility constraints.
- T-035-02-01, which depends on this ticket, owns the broader committed deterministic
  real-Zellij delivery-boundary harness.

## Recent implementation history

- Commit `ae2fd95` added a finite deadline to fresh `Starting`.
- A missing process-start signal currently ends in `StartupFailed`.
- That implementation deliberately performed no relaunch.
- Commit `1bd6c35` split fresh launch from chat assignment.
- It added `ReadyForAssignment` and `Delivering`.
- It also made fresh provider commands bare.
- Full attempt instructions now live in attempt-private `assignment.md`.
- Fresh process start no longer implies ownership.
- The current source is the completion of T-035-04-01.

## Primary scheduler location

- `crates/lisa-plugin/src/lib.rs` contains the scheduler and pane lifecycle.
- `State` owns slots, threads, leases, signals, timers, and dashboard projection.
- `AgentSlot` binds one physical pane to an optional ticket and attempt lease.
- `AgentSlot::transition_state` describes `/clear` and `/exit` transport transitions.
- `State::seat_assignments` separately describes assignment truth.
- The separation permits a pane transition and an ownership transition to coexist.
- Native provider differences are resolved through `adapter.rs`.
- The recovery required by this ticket is provider-neutral until relaunch construction.

## Current assignment states

- `Starting { generation, start_deadline }` means a fresh launch was submitted but no
  matching process-start evidence was admitted.
- `ReadyForAssignment { generation }` means the exact provider process started.
- `Delivering { generation, ack_deadline, retries }` means the bounded chat reference
  was submitted and awaits `UserPromptSubmit` evidence.
- `AssignedPendingAck` is the inherited reused-Codex path.
- `Recovering` is the inherited one-fresh-Codex fallback after reused prompt failure.
- `Owned` alone satisfies `seat_is_owned`.
- `StartupFailed`, `DeliveryFailed`, and `RecoveryFailed` are terminal visible states.
- No current state represents an in-progress shell reset.
- No current state remembers whether a fresh `Starting` launch is the original or its
  one allowed same-pane replacement.

## Fresh dispatch sequence

- `schedule_ready_tickets` performs admission and provider-cap checks first.
- It selects a compatible or recyclable physical slot.
- It mints from `lease_high_water` only after those gates.
- The new lease is installed in `lease_high_water` and `current_leases`.
- For a clean fresh pane, it writes `pane-<id>.lease` before launch.
- It prepares complete instructions in the attempt-private work directory.
- It prepares an atomic `.lisa-launch-<pane>.sh` file.
- It sends only `sh <attempt-private-launch-script>` into the pane.
- It stamps the slot and thread with the exact lease.
- It records `Starting` and arms the process-start deadline.
- The launch line uses deferred Enter submission.

## Input transport

- `send_line_to_pane` writes characters immediately.
- It queues Enter for `ENTER_DELAY_SECS`, currently two seconds.
- `PendingEnter` contains the pane and an absolute ready time.
- `flush_pending_enters` writes byte 13 only after the deadline.
- This solves provider TUI character/Enter coalescing.
- It is also usable for short shell probe commands.
- There is no existing helper for raw control characters such as Ctrl-C.
- Zellij's `write_to_pane_id` already accepts arbitrary bytes.
- A Ctrl-C shell interrupt is byte 3.
- Pending Enter entries can be retained or removed by pane ID.

## Positive process-start evidence

- Native startup hooks atomically write `pane-<id>.started`.
- The payload is the exact serialized `AttemptLease`.
- `check_process_start_signals` consumes every matching filename once.
- Malformed content is discarded.
- `acknowledge_process_start` accepts only current `Starting`.
- The state generation must match the candidate attempt ID.
- Slot ticket and lease must exactly match the candidate.
- The candidate must still be authoritative in `current_leases`.
- Successful admission reaches `ReadyForAssignment`, not `Owned`.
- A stale start signal is therefore inert after lease rotation.

## Assignment delivery evidence

- `deliver_ready_assignments` snapshots ready states at poll start.
- It runs before process-start scanning.
- Newly admitted readiness remains visible for one complete poll boundary.
- `deliver_assignment_to_pane` rechecks slot and current lease authority.
- It requires the exact attempt-private `assignment.md`.
- It sends a bounded path reference plus a structured attempt marker.
- It enters `Delivering` with an Enter-aware deadline.
- The first missed deadline retries the same bounded reference once.
- The second missed deadline enters `DeliveryFailed`.
- `check_codex_ack_signals` handles both provider payloads despite its historic name.
- `acknowledge_codex_assignment` admits only a matching current generation.
- A stale acknowledgment cannot promote a replacement attempt.

## Existing startup timeout

- `check_assignment_ack_timeouts_at` scans absolute deadlines.
- Expired `Starting` currently calls `fail_startup` immediately.
- `fail_startup` changes the state to `StartupFailed`.
- It fails the logical thread.
- It records a deduplicated error alert.
- It retains the reservation and current lease for operator reset.
- It sends no input.
- It does not distinguish incomplete shell input from a slow or hookless provider.
- It does not revoke the failed attempt.
- It does not mint a replacement.
- This is the precise extension point for the ticket.

## Existing reused-session recovery

- `begin_assignment_recovery` applies only to `AssignedPendingAck`.
- It validates the reused Codex predecessor lease.
- It mints a strict successor from the predecessor.
- It replaces current, slot, and thread lease stamps.
- It enters `Recovering` before sending provider input.
- It sends graceful `/exit` to an actually resident Codex TUI.
- `WaitingForExit` gives the provider a finite teardown grace period.
- The later launch is bare and now re-enters `Starting`.
- That path depends on a provider having existed.
- Reusing it for `dquote>` would type `/exit` into the unfinished shell quote.

## Lease authority and revocation

- `lease_high_water` retains the latest minted predecessor for monotonicity.
- `current_leases` is the active publication authority registry.
- `revoke_current_lease` removes authority without discarding high water.
- `release_slot_for_ticket` also revokes before releasing a slot.
- `revoke_and_fence_attempt` revokes before closing the pane.
- Fencing changes the transition state to `Fenced`.
- It removes assignment state, human-attention state, and queued Enter actions.
- It closes the Zellij terminal outside native unit tests.
- It is currently logged as a hard-silence operation but its mechanics are reusable.
- Same-pane recovery cannot release the slot between attempts.
- It must instead replace lease stamps in place.

## Stale lifecycle and artifact rejection

- Heartbeats carry `AttemptLease` and require exact slot/current equality.
- Start evidence carries `AttemptLease` and requires exact slot/current equality.
- Assignment acknowledgments carry ticket and generation and require current equality.
- Attempt artifacts live under `.lisa/attempts/<ticket>/<attempt>/work`.
- `admit_artifact` validates the candidate lease before copying to shared work.
- Completion requests retain an explicit attempt authority.
- Completion publication revalidates that authority.
- Rotating all slot, thread, and current lease stamps makes prior lifecycle state inert.
- Old signal files are consumed, not replayed.
- A shell-ready signal will need the same exact admission discipline.

## Shell-readiness evidence gap

- The plugin does not read pane screen contents.
- A fixed wait after Ctrl-C would not prove that zsh accepted the interrupt.
- Blind Ctrl-C alone would not prove a shell boundary.
- A bounded shell command can positively prove the boundary by writing a signal file.
- Such a command cannot complete while zsh remains inside the unfinished quote.
- It also cannot complete inside a functioning provider TUI.
- The signal payload can be the scheduler-minted successor lease.
- The filename can be pane-scoped like existing native signals.
- Admission must require the reset-specific state, slot stamp, and current successor.
- The successor pane lease marker must not be published before this proof.
- Publishing it early could let a still-running predecessor hook copy successor identity.

## Relaunch preparation

- Every attempt owns a distinct private work directory.
- The replacement therefore needs a new `assignment.md`.
- It also needs a launch script under the successor directory.
- The adapter can reconstruct both from the ticket, route, pane, and successor ID.
- `prepare_assignment` and `prepare_fresh_launch` are already atomic.
- The launcher remains bare because it comes from `adapter.launch_command`.
- After shell readiness, publishing the successor pane lease marker is safe.
- The replacement then enters the ordinary `Starting` evidence boundary.
- The replacement must be marked so another start timeout cannot relaunch again.

## Started and owned behavior

- `ReadyForAssignment` never enters the startup timeout branch.
- `Delivering` receives only the bounded same-process chat retry.
- Neither state currently receives `/exit` during a missed fresh assignment.
- The ticket permits bounded chat retry or graceful `/exit` for a started provider.
- The existing one-chat-retry policy satisfies that disjunction.
- `Owned` is evaluated by the established session and stale-thread timeouts.
- Those paths require hard silence before `revoke_and_fence_attempt`.
- This ticket must not weaken that threshold or redirect Owned into shell reset.

## Dashboard and diagnostics

- `to_ui_state` exhaustively maps assignment variants.
- `ui.rs` owns visible labels and colors.
- Existing `StartupFailed` is red.
- A reset-in-progress state needs an operator-visible label if added.
- Failed reset/replacement evidence needs named actionable logging.
- Tests can inspect `activity_log`, assignment state, lifecycle events, and pane status.

## Test infrastructure

- Most scheduler behavior is tested inline in `crates/lisa-plugin/src/lib.rs`.
- Injected `SystemTime` avoids real sleeps for deadline tests.
- Test host calls are suppressed for pane close.
- Existing helpers construct scheduled Claude and Codex panes.
- `start_and_deliver_fresh_recovery` exercises the strengthened successor boundary.
- Existing tests cover missing start, bounded chat retry, E-033 recovery, E-034 fencing,
  stale signals, pane naming, and provider parity.
- Test-only lifecycle events verify revoke-before-fence-before-release ordering.
- There is no committed real-Zellij harness in the repository today.
- T-035-02-01 explicitly depends on this ticket and owns that larger harness.

## Constraints surfaced by the code

- Recovery must not use the resident-provider `/exit` adapter for `Starting`.
- Recovery must not publish the successor lease marker until shell proof.
- Recovery must preserve the physical slot and pane ID.
- Recovery must rotate authority before any reset action.
- Recovery needs a new explicit state to reject unrelated signals during reset.
- Recovery must cancel any queued Enter for the failed launch before Ctrl-C.
- The reset command must be bounded, shell-safe, and attempt-scoped.
- A missing reset signal must terminate and fence rather than loop.
- A missing replacement start must terminate and fence rather than relaunch again.
- A matching replacement start still reaches only `ReadyForAssignment`.
- A matching replacement chat acknowledgment is still required for `Owned`.
- Ticket-owned source commits must use exact paths through `lisa commit-ticket`.
