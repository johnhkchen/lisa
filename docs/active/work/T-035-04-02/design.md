# T-035-04-02 Design — bounded same-pane shell recovery

## Decision summary

Extend the fresh-start state machine with an explicit attempt-scoped shell-reset state.
When the original `Starting` deadline expires, revoke its lease, mint and install one
strict successor, cancel residual pane submission, send Ctrl-C, and submit a short shell
probe. The probe atomically writes the successor lease to a pane-scoped `shell-ready`
signal. Only exact admission of that signal permits Lisa to publish the successor pane
lease marker and submit T-035-04-01's bare provider launcher in the same pane.

The replacement re-enters ordinary `Starting` with a recovery marker. It must still
produce exact process-start evidence, receive the bounded chat assignment reference, and
produce exact assignment acknowledgment before `Owned`. A missing shell-ready signal or
a missing replacement start exhausts the sole recovery and permanently fences the pane.
Started `ReadyForAssignment`/`Delivering` providers retain bounded chat retry, and
hard-silent `Owned` providers retain E-034 fencing.

## Goals

- Escape unfinished shell input without assuming a provider exists.
- Prove the shell boundary positively rather than by elapsed time.
- Preserve one physical pane throughout the successful recovery.
- Revoke the failed attempt before reset input.
- Mint exactly one strictly newer attempt.
- Prevent predecessor signals and artifacts from affecting the replacement.
- Preserve the two-stage start/chat ownership boundary.
- End every recovery branch within a finite bound.
- Keep actually started and owned providers out of the shell-interrupt path.

## Non-goals

- General shell prompt detection.
- Screen scraping or parsing `dquote>` text.
- Retrying arbitrary provider startup failures indefinitely.
- Replacing the existing E-033 reused-Codex recovery.
- Replacing the existing E-034 owned hard-silence policy.
- Creating the broader real-Zellij harness assigned to dependent T-035-02-01.
- Standardizing provider hook internals beyond their current shared evidence contract.

## Option 1 — retain terminal StartupFailed

Leave `Starting` timeout unchanged and require operator reset.

Advantages:

- No new pane input or state transitions.
- Preserves the prior fail-closed behavior.
- Lowest implementation risk.

Disadvantages:

- Does not escape `dquote>` automatically.
- Consumes operator attention for the reproduced field failure.
- Does not meet same-pane recovery or successor-lease criteria.
- Cannot prove one bounded relaunch.

Decision: rejected because it is the current behavior and misses the ticket's core need.

## Option 2 — send `/exit`, wait, then relaunch

Reuse `WaitingForExit` and the existing fresh Codex fallback.

Advantages:

- Reuses a mature transition and grace timer.
- Already rotates leases for reused Codex assignment recovery.
- Provider adapters already expose `/exit`.

Disadvantages:

- There is no provider in the reproduced failure.
- `/exit` becomes more characters inside the unfinished quoted command.
- Elapsed grace does not prove shell readiness.
- Could launch more text into an unresolved quote.
- Conflates a started provider transition with incomplete shell input.

Decision: rejected. The ticket explicitly identifies `/exit` as invalid pre-provider.

## Option 3 — blind Ctrl-C followed by a fixed delay

Write byte 3 to the pane, sleep or wait a configured grace, then relaunch.

Advantages:

- Ctrl-C is the conventional way to cancel zsh continuation input.
- Small implementation surface.
- Likely repairs the observed case under normal timing.

Disadvantages:

- Ctrl-C receipt is not observable.
- A fixed delay is not positive shell evidence.
- The replacement might be typed into a provider TUI or still-broken shell.
- Slow terminal processing makes correctness timing-dependent.
- It fails an explicit acceptance criterion.

Decision: rejected because elapsed time cannot establish the safety boundary.

## Option 4 — scrape pane contents for a known shell prompt

Inspect terminal screen state after Ctrl-C and require a prompt pattern.

Advantages:

- Could visibly distinguish `dquote>` from an ordinary prompt.
- Does not require an extra shell command.

Disadvantages:

- The plugin does not currently obtain or parse pane text.
- Shell prompts are user-configurable, themed, multiline, and ambiguous.
- A prompt-looking string in output is not proof of command readiness.
- Adds Zellij API and terminal parsing scope.
- Couples recovery to zsh presentation rather than shell semantics.

Decision: rejected as broad, brittle, and less authoritative than execution evidence.

## Option 5 — Ctrl-C plus an attempt-scoped shell probe

After authority rotation, send Ctrl-C and a bounded command that atomically publishes the
successor lease to `pane-<id>.shell-ready`. Admit only an exact signal from an explicit
reset state, then relaunch.

Advantages:

- The probe can execute only at a functioning shell command boundary.
- It works regardless of the configured visual prompt.
- The existing signal directory and lease schema are reusable.
- Exact lease admission rejects stale predecessor proof.
- The successor marker can remain unpublished until the shell is proven safe.
- Missing proof naturally ends at an absolute deadline.
- Native tests can deterministically exercise admission and timeout.

Disadvantages:

- Requires a new state and signal scanner.
- Requires carefully ordered raw Ctrl-C and deferred Enter input.
- A provider that started but failed to emit its hook will receive Ctrl-C.
- Such a provider cannot execute the shell probe, so the path fails closed and fences.

Decision: selected. It is the smallest positive proof compatible with current boundaries.

## State model

Add a reset state:

```text
ResettingStartup {
    generation: u64,
    reset_deadline: SystemTime,
}
```

`generation` is the newly minted successor attempt, not the revoked predecessor. This
makes every accepted signal in the state refer to the only lease that can proceed.

Extend `Starting` with a finite relaunch count or boolean:

```text
Starting {
    generation: u64,
    start_deadline: Option<SystemTime>,
    relaunches: u8,
}
```

Initial dispatch uses zero. Shell-ready relaunch uses one. No code path creates a value
greater than one. On expiry, zero begins recovery; one exhausts recovery and fences.

## Positive transition graph

```text
Starting(attempt N, relaunches 0)
  -- start deadline --> revoke N, mint N+1
ResettingStartup(attempt N+1)
  -- exact shell-ready --> publish N+1 marker, bare relaunch
Starting(attempt N+1, relaunches 1)
  -- exact process-start --> ReadyForAssignment(N+1)
ReadyForAssignment(N+1)
  -- bounded chat reference --> Delivering(N+1, retry 0)
Delivering(N+1)
  -- exact UserPromptSubmit --> Owned
```

The pane ID never changes on the positive path.

## Failure graph

```text
ResettingStartup -- missing shell-ready --> StartupFailed + revoke + fence
ResettingStartup -- preparation/publish error --> StartupFailed + revoke + fence
Starting(relaunches 1) -- missing start --> StartupFailed + revoke + fence
Delivering -- first missed ack --> Delivering(retry 1)
Delivering(retry 1) -- missed ack --> DeliveryFailed
Owned -- genuine hard silence --> existing E-034 revoke + fence + release
```

`DeliveryFailed` retains its started provider reservation for explicit reset. The ticket
allows bounded chat retry instead of mandatory `/exit`, so no new graceful-exit branch is
needed for a process that has already reported startup.

## Authority rotation order

The original `Starting` timeout must perform these mutations in order:

1. Revalidate state generation, slot ticket/lease, current lease, and high water.
2. Remove the predecessor from `current_leases`.
3. Mint a successor from the retained high-water predecessor.
4. Install successor in high water and current authority.
5. Replace the slot and thread attempt stamps.
6. Enter `ResettingStartup` with an absolute deadline.
7. Remove stale pane lifecycle files and queued Enter actions.
8. Send Ctrl-C.
9. Send the successor-scoped shell probe.

Revocation before input satisfies the ticket and makes late predecessor evidence inert.
State replacement before input also prevents a synchronous or next-poll stale signal from
traversing the original `Starting` edge.

## Successor marker isolation

Do not call `write_pane_lease_marker` while beginning reset. A provider process might
have started without publishing `.started`; if it remains alive, its heartbeat hook must
not be able to copy successor authority.

The shell probe embeds serialized successor identity directly in its command. That value
is scheduler-created and is admitted only from `ResettingStartup`. After exact proof,
Lisa publishes the normal `pane-<id>.lease` marker immediately before relaunch.

## Shell probe command

Use a short POSIX-compatible command shaped as:

```text
command printf '%s' '<successor-json>' > '<tmp>' && command mv '<tmp>' '<ready>'
```

Paths and JSON are passed through the established `shell_quote` helper. The temporary and
destination files are in `.lisa/signals`. The rename makes publication atomic. The probe
contains no ticket prompt or provider command. Its only dynamic content is bounded pane,
attempt identity, and nonce/path text.

Production commands execute in the project root, so host-facing relative `.lisa/signals`
paths are appropriate. Native tests use configured absolute paths to inspect command
construction and direct signal admission without host execution.

## Input ordering

Before Ctrl-C, delete every queued `PendingEnter` for the pane. This prevents a delayed
Enter belonging to the failed launch from racing the reset.

Then write raw byte 3 with `write_to_pane_id`. Submit the probe through
`send_line_to_pane`, retaining its existing deferred Enter discipline. Shell readiness is
not inferred from these writes; it exists only after the probe output is scanned.

## Signal admission

Add `check_shell_ready_signals` with the established consume-first pattern:

- scan only `pane-<id>.shell-ready`;
- parse exactly one `AttemptLease`;
- remove the file regardless of validity;
- require current `ResettingStartup` generation;
- require exact slot ticket and lease;
- require exact current authority;
- only then prepare and submit the successor launch.

Malformed, predecessor, wrong-pane, duplicate, late, and post-failure signals are inert.

## Relaunch construction

On exact shell readiness:

1. Resolve the current ticket adapter and route.
2. Resolve the exact successor attempt directory.
3. Recreate the complete successor `assignment.md` atomically.
4. Create the bare successor launch script atomically.
5. Publish the successor pane lease marker.
6. Send the launch indirection into the same pane.
7. Mark the slot as hosting the resolved client.
8. Enter `Starting { relaunches: 1, start_deadline: Some(...) }`.

Any error before launch is named as startup recovery failure and fences the pane. There is
no fallback to another pane inside this ticket's recovery transaction.

## Terminal recovery failure

Introduce one helper that can fail either `ResettingStartup` or replacement `Starting`.
It should:

- preserve a visible `StartupFailed` assignment state long enough for dashboard/tests;
- fail the thread and emit a deduplicated alert;
- log the exact missing or failed evidence;
- revoke the current successor;
- permanently fence the physical pane;
- retain the ticket reservation rather than auto-rescheduling in the same poll.

Because `revoke_and_fence_attempt` currently removes assignment state, recovery failure
needs either a lower-level pane-fence helper or a mode that preserves `StartupFailed`.
The safer design is to factor physical fencing from assignment removal, then explicitly
restore the terminal assignment state for this retained failed reservation.

## Poll ordering

Use this order near the existing lifecycle consumers:

1. deliver assignments already ready;
2. consume process-start evidence;
3. consume shell-ready evidence;
4. consume assignment acknowledgments;
5. evaluate transition and assignment deadlines.

Signals win over deadlines in the same poll. Shell readiness may launch immediately, but
its new `Starting` deadline is absolute and cannot expire in that same call.

## Provider distinctions

- Original `Starting` for Claude and Codex uses identical shell recovery.
- The relaunch adapter is resolved from the ticket route, preserving models and flags.
- `ReadyForAssignment` and `Delivering` never receive Ctrl-C.
- Their existing provider-neutral bounded reference retry remains unchanged.
- Reused Codex `AssignedPendingAck` continues to use graceful `/exit` recovery.
- `Owned` continues to use provider-neutral hard-silence fencing.

## Testing strategy

Add native deterministic tests for:

- initial fresh `Starting` contains zero relaunches;
- timeout rotates to an exact successor before Ctrl-C/probe state;
- successor ID is strictly greater and the pane ID is unchanged;
- predecessor lease is no longer current immediately;
- exact shell-ready proof is required before relaunch;
- stale shell/start/ack/heartbeat/artifact signals remain inert;
- exact shell readiness creates successor assignment and a bare launcher;
- replacement start reaches only `ReadyForAssignment`;
- replacement chat reaches `Delivering`;
- exact replacement acknowledgment alone reaches `Owned`;
- missing shell readiness ends in named failure and fencing;
- missing replacement start does not relaunch a second time and fences;
- `ReadyForAssignment`/`Delivering` receive no shell recovery;
- `Owned` still follows the existing E-034 hard-silence path;
- Claude and Codex share the positive replacement contract;
- pane naming and existing E-033/E-034 tests stay green.

The dependent T-035-02-01 harness can execute the emitted Ctrl-C/probe/relaunch boundary
under a real Zellij PTY with a stub provider. This ticket supplies the deterministic state
and command contract that harness will drive.

## Risks and mitigations

- Risk: Ctrl-C reaches a provider that started without its hook.
  Mitigation: the shell probe cannot execute there; bounded failure fences rather than
  submitting a replacement into the TUI.
- Risk: predecessor copies successor lease.
  Mitigation: do not publish the normal marker until shell proof.
- Risk: delayed Enter races reset.
  Mitigation: purge pane-specific pending Enter before Ctrl-C.
- Risk: signal replay relaunches twice.
  Mitigation: accept only `ResettingStartup`, then leave that state before input.
- Risk: replacement start timeout repeats recovery.
  Mitigation: explicit relaunch count and terminal branch at one.
- Risk: failure becomes automatic spare consumption.
  Mitigation: retain failed thread/reservation and fence the pane; require operator reset.

## Acceptance mapping

- Incomplete-shell distinction: only original expired `Starting` enters reset.
- Started-provider distinction: Ready/Delivering retain bounded chat retry.
- Owned distinction: unchanged E-034 hard-silence fencing.
- Positive readiness: exact shell probe signal, not delay or Ctrl-C alone.
- Lease order: revoke predecessor before reset, mint strict successor.
- Stale rejection: exact successor state/slot/current checks at every boundary.
- Single relaunch: replacement `Starting` records one and cannot re-enter reset.
- Ownership: successor still requires start plus chat acknowledgment.
- Actionable failures: reset/start/preparation errors are named and fenced.
- Same pane: slot and pane ID are updated in place; no spare is selected.
