# T-033-01-04 Research — bounded acknowledgment recovery

## Ticket boundary

The ticket completes the last part of story `S-033-01`: a recycled physical
seat assigned to Codex already enters `AssignedPendingAck`, receives a
ticket/generation marker, and becomes `Owned` only after an exact native
`UserPromptSubmit` acknowledgment. This ticket covers the missing finite wait
and fresh-session fallback when that acknowledgment never arrives.

The acceptance criterion requires one scheduler test to establish all of these
facts:

- the wait has a finite configurable deadline;
- withholding acknowledgment transitions the seat from pending to recovering;
- recovery launches at most one fresh Codex session for the same ticket;
- the original unacknowledged attempt is abandoned without two owners;
- recovery failure reaches a named actionable error;
- the ticket cannot disappear, retry forever, or remain silently pending.

The story explicitly excludes dashboard rendering and changes to Claude's
handoff contract. The implementation boundary is therefore scheduler state,
configuration transport, and deterministic tests.

## Current assignment state model

`crates/lisa-plugin/src/lib.rs` defines scheduler-owned seat truth separately
from the pane transport state:

```rust
enum SeatAssignmentState {
    AssignedPendingAck { generation: u64 },
    Owned,
    Recovering,
}
```

`Recovering` was introduced as a named prerequisite state but is unused and
has a dead-code allowance. The enum is stored in
`State::seat_assignments: HashMap<u32, SeatAssignmentState>`, keyed by physical
terminal pane ID. Absence means no assignment.

`AgentSlot::ticket_id` retains the reservation and routing key during a
handoff. It is not ownership truth. `seat_is_owned` returns true only for the
`Owned` enum value, so both pending and recovering already report not-owned.

`State::next_assignment_generation` allocates process-local positive `u64`
identities. `pending_assignment_generation` exposes the generation only while
the state is `AssignedPendingAck`. The generation is embedded into Codex
prompts through `SpawnContext::assignment_generation`.

`acknowledge_codex_assignment` reads the pending generation and the slot's
current ticket, asks `codex_ack::detect_codex_ack` for an exact match, and
performs the single pending-to-owned edge. Duplicate or stale payloads are
inert because an owned seat no longer exposes a pending generation.

There is no timestamp or deadline associated with a pending assignment. A
missing hook or lost payload therefore leaves the seat pending indefinitely.

## Scheduling and prompt-delivery timing

`schedule_ready_tickets` selects an eligible slot and resolves its provider.
A Codex assignment gets a generation only when the physical seat already has a
resident session. Fresh empty-pane Codex launches and all Claude paths are
immediately `Owned` under the existing contract.

A reused Codex prompt is not necessarily delivered when the reservation is
created. Native Codex uses `ResetStrategy::ClearHandshake`:

1. scheduling sends `/clear`;
2. the slot enters `TransitionState::WaitingForClear`;
3. a `.cleared` signal calls `handle_cleared_signal`;
4. only then is the generation-tagged prompt typed and submitted.

If `.cleared` is absent, `check_transition_timeouts` sends the prompt after the
90-second clear timeout, subject to the pane quiet guard. Consequently, a
deadline started at reservation time could expire before Codex has received
the prompt.

Cross-provider recycling has another delayed delivery path:

1. scheduling sends the resident provider's `/exit`;
2. the slot enters `WaitingForExit` and is marked as having no live session;
3. after `AGENT_EXIT_GRACE_SECS` (8 seconds),
   `check_transition_timeouts` launches the incoming provider command;
4. the generation must survive this delay and is reconstructed from seat state.

The existing tests explicitly prove that clear-timeout and exit-grace delivery
preserve the pending generation without granting ownership. They do not attach
a wait start time.

The acknowledgment clock therefore has a meaningful start boundary only when
the generation-tagged prompt or launch command is actually submitted to the
pane, not when the scheduler first reserves the seat.

## Pane transport state machine

`TransitionState` is independent from assignment truth:

- `Idle` means no reset/exit transport is outstanding;
- `WaitingForStop` and `WaitingForClear` implement in-place reset;
- `WaitingForExit` waits before treating a pane as a shell.

`check_transition_timeouts` already provides a bounded, one-shot shape for
leaving an interactive client and launching a new command. Once its
`WaitingForExit` action runs, it resets the slot to `Idle`, clears the transition
timestamp, marks `has_session = true`, and sends one launch command. Later poll
ticks cannot repeat that launch unless another caller creates a new transition.

That machinery currently assumes cross-provider recycling. It resolves the
ticket route at launch time and reconstructs `SpawnContext` from the slot and
pending assignment generation. It can also represent exiting a failed Codex
reuse and starting Codex again in the same pane.

`send_line_to_pane` writes characters immediately and queues one deadline-
bearing Enter event. It has no host acknowledgment or return value. It can
decline injection only when the pane is recorded in `awaiting_human`; that path
logs an informational message and queues no Enter.

The interactive Codex command writes `pane-<id>.error` if the Codex process
exits nonzero. `check_error_signals` currently treats any such signal as a
normal failed run: it fails the thread, emits provenance, releases the slot,
removes the thread, records an alert, and allows the ready ticket to be
scheduled again. That generic retry behavior is not bounded specifically for a
recovery attempt.

## Poll ordering

`poll_tick` runs every five seconds and currently orders relevant work as:

1. heartbeat and awaiting-human signal consumption;
2. Codex acknowledgment consumption;
3. artifact, idle, and transition signals;
4. provider error signals;
5. transition timeout fallbacks;
6. session health and stale-thread handling;
7. DAG rebuild and new scheduling.

The acknowledgment scanner intentionally precedes timeout evaluation. A new
ack-deadline evaluator belongs after ack consumption so a matching payload at
the boundary wins rather than being discarded by recovery first.

Timer callbacks are not correlated to individual requests. The regular poll
timer is the reliable evaluator, so absolute `SystemTime` deadlines fit the
existing scheduler style and deterministic native tests.

## Acknowledgment signal boundary

`check_codex_ack_signals` reads each `pane-<id>.ack` file, removes it regardless
of validity, and calls the exact detector. Successful promotion bumps pane
activity and logs one informational event. Invalid, stale, unreadable, or
duplicate files are consumed without mutation.

The prompt marker contains both ticket ID and generation. Recovery can allocate
a new generation to fence the abandoned reused-session delivery from the fresh
fallback. Reusing the original generation would permit a late acknowledgment
from the abandoned prompt to claim the replacement attempt.

`CodexAdapter` already attaches the marker to both bare reuse prompts and full
interactive launch commands whenever `SpawnContext` carries a generation. No
adapter interface change is needed to tag a recovery launch with a new identity.

## Configuration pipeline

Scheduler settings travel through four representations:

1. `crates/lisa-cli/src/config.rs` deserializes optional `[scheduling]` TOML
   keys into `SchedulingConfig`, validates known keys, and resolves defaults
   into `ResolvedConfig`;
2. `crates/lisa-cli/src/loop_cmd.rs` writes resolved values into the generated
   Zellij KDL plugin block;
3. `crates/lisa-core/src/types.rs` parses the KDL string map into
   `PluginConfig` used by the WASM scheduler;
4. `crates/lisa-cli/src/init.rs` and `default_config_toml` merge commented
   examples into new and existing project configuration.

Existing timeout settings (`review_timeout_secs`, `session_timeout_secs`, and
`wind_down_secs`) use this complete path and provide the local pattern for a
configurable acknowledgment deadline. Core config tests cover defaults and map
parsing; CLI tests cover TOML parsing/resolution, known-key validation, default
template content, and generated layout output.

`setup_guide.rs` describes scheduling keys using the generated default config.
README's compact table currently documents only a subset of the settings.

## Failure and ownership constraints

The old assignment must stop being ack-eligible before `/exit` is submitted.
Otherwise a late payload could promote it during recovery. Replacing pending
state with `Recovering` first makes the original generation unavailable to the
current detector.

A fresh recovery launch needs a distinct generation if its success is to be
positively acknowledged. The recovery state must retain that identity while
remaining not-owned. A matching recovery acknowledgment can then be the only
edge to `Owned`; a late original acknowledgment is fenced by generation.

One finite deadline is insufficient if it only starts recovery. The fresh
fallback can also fail to acknowledge. Without a second terminal boundary the
seat would remain `Recovering` indefinitely, which violates the acceptance
criterion's no-silent-stall requirement.

The scheduler already surfaces `ActivityEvent::Error` entries and failed thread
status. A terminal recovery failure can retain the ticket/seat association for
operator inspection instead of releasing it into unbounded automatic retry.
Manual ticket reset is the existing operator action for retrying a failed or
stuck assignment.

## Existing verification surface

Relevant plugin tests already cover:

- recycled Codex scheduling enters generation-bearing pending state;
- no acknowledgment is not owned;
- stale ticket and stale generation payload rejection;
- exact acknowledgment and duplicate idempotence;
- acknowledgment signal file consumption;
- clear-timeout prompt delivery preserves pending state;
- cross-provider exit grace launches once and preserves pending state;
- Claude reuse remains owned;
- pane release removes assignment truth.

Core config tests cover every existing timeout default and map override. CLI
config/layout/init tests offer direct places to prove a new setting reaches the
plugin without a live Zellij or Codex process.

## Worktree and transaction constraints

The repository has unrelated modified and untracked files, including
`crates/lisa-cli/src/agent_exec.rs`, project hooks, stories, tickets, knowledge
documents, and other work artifacts. Ticket-owned edits must avoid those paths
where possible and must be committed with exact `lisa commit-ticket --include`
arguments. The ordinary index must remain untouched.

The preceding ticket's source commit is `74afa61` and all acknowledgment-gating
source files are currently clean. The current ticket file and RDSPI artifacts
are Lisa-owned completion inputs and must not be included in the source commit.

## Research conclusions

The missing behavior is local but crosses scheduler state and configuration.
The critical boundary is prompt submission: waiting must not begin during
`/clear` or `/exit` transport. Recovery can reuse the existing one-shot exit
grace, but needs a fresh generation to fence late evidence and a second bounded
wait to turn fallback failure into an explicit terminal condition. Claude,
fresh ordinary Codex assignments, adapter text, and dashboard rendering do not
need semantic changes.
