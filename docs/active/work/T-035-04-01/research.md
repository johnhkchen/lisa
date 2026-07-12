# T-035-04-01 Research — split start from chat assignment

## Ticket boundary

The ticket strengthens the fresh native-provider ownership contract introduced by
S-035-01. It applies to both Claude Code and Codex interactive sessions. A successful
provider process start must no longer imply that the provider received its ticket.
Fresh delivery must become two positively observed stages: provider readiness, then
chat assignment acceptance.

The ticket starts in `research`. Its predecessor, T-035-01-04, is complete and leaves a
bounded `Starting` state plus a terminal `StartupFailed` state. This ticket consumes that
foundation rather than replacing its lease fencing or startup timeout behavior.

## Primary implementation files

`crates/lisa-plugin/src/lib.rs` owns the scheduler, attempt leases, pane input, signal
consumption, assignment state, timeouts, recovery, and native tests.

`crates/lisa-plugin/src/adapter.rs` owns provider-specific launch commands and prompt
construction. Both native adapters implement the same `AgentAdapter` trait.

`crates/lisa-plugin/src/codex_ack.rs` owns the existing attempt-tagged prompt marker and
`UserPromptSubmit` payload classifier. Despite its name, its parsed event shape is a
minimal `hook_event_name` plus optional `prompt` envelope.

`crates/lisa-plugin/src/ui.rs` owns the reduced dashboard assignment statuses, labels,
colors, and rendering tests.

`crates/lisa-cli/src/templates.rs` owns the generated Claude and Codex lifecycle hook
configuration plus the shared `on-ack.sh` script.

`crates/lisa-cli/src/init.rs` installs `on-ack.sh` already. No new hook script is needed
for prompt evidence if Claude can use the same raw-payload transport.

## Attempt-scoped assignment text

`ticket_prompt` in `lib.rs` constructs the complete RDSPI assignment. It resolves the
ticket file, names the provider context file, points phase artifacts at the exact private
attempt work directory, prohibits frontmatter edits and ordinary Git commits, and tells
the agent to stop after Review.

`State::attempt_work_dir` maps a current `AttemptLease` to
`.lisa/attempts/<ticket>/<attempt>/work`.

The current fresh launch preparation writes `.lisa-launch-<pane>.sh` in that work
directory using a temporary file followed by same-directory rename. The prepared script
contains the provider command, and therefore currently also contains the complete ticket
prompt.

The private attempt work directory is ignored by `.lisa/.gitignore`. Lisa later admits
phase artifacts from it only after exact current-lease validation.

There is no separate assignment file today. The complete assignment exists only as a
constructed string embedded in either a launch command or an in-process reuse prompt.

## Fresh provider launch shapes

`build_claude_command` accepts ticket directory, ticket ID, pane ID, attempt ID, model,
Lisa binary, and artifact directory. It calls `ticket_prompt` and appends the shell-quoted
prompt as Claude's positional argument.

`CodexAdapter::interactive_line` calls `assignment_prompt`, which calls `ticket_prompt`
and optionally appends the `LISA_ASSIGNMENT` marker. It appends that shell-quoted prompt
as Codex's positional argument.

Both launch lines carry lifecycle identity through `LISA_PANE_ID`, `LISA_TICKET_ID`, and
`LISA_ATTEMPT_ID`. Codex additionally carries `LISA_AGENT_CLIENT=codex` and `LISA_BIN`;
Claude carries `LISA_BIN` only when configured.

Claude launch flags are `--dangerously-skip-permissions` plus optional `--model`.
Codex launch flags are `--dangerously-bypass-approvals-and-sandbox`,
`--dangerously-bypass-hook-trust`, plus optional `--model`.

The Codex launch line also writes `.error` if the provider process exits unsuccessfully.
This shell-side error fallback is independent of the positional prompt.

`State::prepare_fresh_launch` atomically writes the complete launch script and returns a
bounded `sh '<path>'` pane command. The pane command length is independent of payload
length, but the script payload still grows with the assignment and violates this ticket's
stronger bare-provider-launch requirement.

## Existing assignment state machine

`SeatAssignmentState::Starting` stores the attempt generation and an optional process
start deadline. `None` means the delayed launcher has not yet been submitted; `Some`
means the bounded wait for `.started` is armed.

`AssignedPendingAck` stores a generation and optional acknowledgement deadline. It is
currently used only for reused Codex prompts.

`Owned` is the only state for which `seat_is_owned` returns true.

`Recovering` represents the one fresh Codex fallback after a reused delivery misses its
acknowledgement. `RecoveryFailed` retains a terminal reservation after that fallback
also fails. `StartupFailed` does the same for a missed process-start signal.

There are no `ReadyForAssignment` or `Delivering` variants yet.

Fresh Claude and fresh Codex both enter `Starting`. Same-process Codex reuse enters
`AssignedPendingAck`. Same-process Claude reuse enters `Owned` immediately under the
older contract.

## Process-start evidence

Generated Claude and Codex configurations both bind `SessionStart[startup]` to
`.lisa/hooks/on-start.sh`.

The start hook compares immutable launch environment identity with the scheduler-owned
`pane-<id>.lease` marker. Only exact pane/ticket/attempt equality copies the marker to an
atomic `pane-<id>.started` file.

`check_process_start_signals` parses each `.started` file as an `AttemptLease`, removes
the file, and calls `acknowledge_process_start`.

`acknowledge_process_start` requires `Starting`, exact generation, exact slot ticket and
lease, and the current authoritative lease. It currently changes the state directly to
`Owned`.

The poll loop consumes process-start signals before evaluating assignment deadlines.
Exact evidence visible at a boundary poll wins over expiration of the prior state.

## Prompt acknowledgement evidence

`codex_ack.rs` serializes an attempt marker as a full prompt line:
`LISA_ASSIGNMENT {"ticket_id":...,"generation":...}`.

`tag_codex_assignment` appends this marker safely using `serde_json`.

`detect_codex_ack` accepts only valid `UserPromptSubmit` JSON with a prompt containing a
whole marker line whose ticket and generation exactly match the expected assignment.
Malformed payloads, other event types, missing prompts, stale tickets, and stale
generations fail closed.

`ON_ACK_HOOK` atomically preserves the complete hook stdin payload as
`pane-<id>.ack`. The hook itself is provider-agnostic, although its comments and current
scheduler classifier describe Codex.

Generated Codex hooks bind `UserPromptSubmit` to `on-ack.sh`. Generated Claude settings
do not currently bind `UserPromptSubmit`.

`check_codex_ack_signals` removes each `.ack`, passes it to
`acknowledge_codex_assignment`, and records activity only on a matching promotion.

`acknowledge_codex_assignment` currently accepts `AssignedPendingAck` or `Recovering`,
validates the exact current lease, runs the marker classifier, then moves to `Owned`.
It does not accept `Starting`.

## Delivery and deadline boundaries

`send_line_to_pane` writes characters immediately and queues Enter for two seconds later.
It refuses all injection while a pane is marked as awaiting a human.

`start_assignment_ack_wait` adds the delayed-Enter allowance to the configured positive
`assignment_ack_timeout_secs`. It arms `Starting`, `AssignedPendingAck`, or `Recovering`
only after the relevant launcher or prompt has actually been sent.

`check_assignment_ack_timeouts_at` is deterministic and accepts injected time. An
expired `Starting` becomes `StartupFailed`; `AssignedPendingAck` begins one fresh Codex
recovery; `Recovering` becomes `RecoveryFailed`.

The existing missing-ack recovery is Codex-specific. It mints a successor attempt,
fences the predecessor, sends `/exit`, launches one fresh Codex process after the exit
grace, and fails terminally if that fresh attempt also misses acknowledgement.

Fresh launch recovery currently arms `Starting` only for process evidence, then becomes
Owned immediately. It has no post-start chat-delivery deadline.

## Session reuse and cross-provider recycling

Same-provider resident sessions use `/clear`, wait for `.cleared`, publish the successor
lease marker immediately before prompt delivery, then send `adapter.reuse_prompt`.

Cross-provider seats send `/exit`, wait for the fixed exit grace, then submit a prepared
fresh launch script. Attempt identity is minted before the exit but the start deadline is
not armed until the launch is actually submitted.

The ticket specifically concerns fresh provider assignment. Existing reused Codex
acknowledgement, lease rotation, and bounded fallback behavior are regression constraints.
Same-process Claude ownership is also a regression constraint unless the implementation
can extend shared evidence without destabilizing its established path.

## Dashboard surface

`ui::SeatAssignmentStatus` mirrors every internal variant. Current labels are
`starting`, `assigned-pending-ack`, `owned`, `recovering`, `recovery-failed`, and
`startup-failed`.

Starting and pending are yellow, Owned is green, Recovering is bright yellow, and
terminal failures are red.

`State::to_ui_state` performs the exhaustive internal-to-dashboard conversion.
Native scheduler tests inspect rendered thread rows, so new state labels can be proven
without a live Zellij instance.

## Existing regression coverage

Fresh-dispatch tests prove exact process-start matching, stale and malformed rejection,
boundary ordering, bounded startup failure, retained reservation, and no relaunch loop.

Codex acknowledgement tests prove exact marker matching, stale ticket/generation
rejection, one fresh recovery, terminal recovery failure, and recovery-generation
promotion.

Consecutive reuse tests exercise ten Codex and ten Claude assignments across two panes.
They encode the existing provider-specific reuse behavior.

Adapter tests assert exact launch command shapes, model quoting, environment variables,
prompt presence, and generation tagging. These expectations must change for bare fresh
launches while reuse-prompt expectations remain relevant.

Template tests assert hook generation and idempotent merge behavior. Claude tests do not
currently expect `on-ack.sh`; Codex tests do.

Workspace verification is `cargo test --workspace`; project guidance also exposes
`just check` for WASM checking plus tests.

## Constraints and assumptions

Attempt authority must remain the `AttemptLease`, not a new independent generation.

Assignment publication must be atomic and occur under the exact attempt directory.

The fresh shell script may contain lifecycle identity and flags, but must never contain
the ticket prompt or an instruction derived from it.

The chat message must be bounded independently of full assignment size. It therefore
needs to reference a stable attempt-specific file instead of reproducing its contents.

Both providers need positively attributable prompt evidence before fresh ownership.
Provider hook payloads may differ, but scheduler-visible states must be the same.

Stale `.started` and `.ack` files are one-shot and must never advance a successor lease.

Missing chat acknowledgement must use one bounded recovery path and must not be routed
through owned-agent hard-silence logic.

Ticket T-035-04-02 owns deterministic recovery from a shell stuck at `dquote>` and the
strictly newer-attempt same-pane relaunch. This ticket should establish clean two-stage
delivery semantics without absorbing that specialized shell-repair scope.
