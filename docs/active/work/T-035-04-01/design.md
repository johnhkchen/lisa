# T-035-04-01 Design — two-stage fresh chat assignment

## Decision summary

Fresh native assignments will use three independent artifacts and two positive provider
signals:

1. Lisa atomically writes the complete RDSPI instructions to the exact attempt's private
   `assignment.md`.
2. Lisa atomically writes and submits a bare provider launch script containing only
   lifecycle environment, provider flags, model selection, and the existing process-exit
   error handler.
3. Exact `SessionStart[startup]` evidence moves `Starting` to
   `ReadyForAssignment`.
4. On the following scheduler poll, Lisa injects a bounded chat message that references
   `assignment.md` and carries the exact attempt marker, moving the seat to `Delivering`.
5. Exact provider `UserPromptSubmit` evidence moves `Delivering` to `Owned`.

A missed fresh chat acknowledgement retries the same bounded chat reference once. A
second miss moves the retained reservation to a named terminal `DeliveryFailed` state.
The retry is finite, does not rotate the attempt, does not relaunch the provider, and
never routes the unowned seat through owned-agent silence handling.

## Goals

- Make process readiness distinct from ticket ownership.
- Keep full assignment bytes out of provider argv and launch scripts.
- Publish complete assignment instructions atomically before provider launch.
- Attribute both readiness and chat acceptance to the exact current attempt lease.
- Give Claude and Codex the same scheduler-visible fresh state sequence.
- Preserve provider flags, routed models, pane identity, and usage/error environment.
- Preserve E-033 recycled-Codex acknowledgement and recovery behavior.
- Preserve E-034 lease fencing and stale-evidence rejection.
- Bound missing chat acceptance without an indefinite retry loop.
- Surface every operationally meaningful state on the dashboard.

## Non-goals

- Repairing a shell stuck in `dquote>`; T-035-04-02 owns that recovery.
- Replacing the existing `/clear` protocol for resident sessions.
- Changing the terminal fence for hard-silent owned providers.
- Scraping terminal text to infer provider state.
- Adding a second assignment identity beside `AttemptLease`.
- Making assignment files shared or publishable workflow artifacts.
- Changing the phase/status frontmatter transition mechanism.
- Changing follow-up prompts for stalled Review sessions.

## Option 1 — retain prompt-bearing launch, delay ownership

Lisa could keep passing the full prompt positionally but change `.started` to a pending
state and wait for `UserPromptSubmit`.

Advantages:

- smallest scheduler change;
- existing adapters already build the launch lines;
- prompt evidence can reuse Codex's marker.

Disadvantages:

- violates the explicit no-assignment-in-launch criterion;
- launch script size still grows with the ticket prompt;
- shell quoting remains part of assignment delivery;
- a provider start and initial prompt are still one transport operation;
- does not isolate whether startup or chat delivery failed.

Decision: reject. It changes state labels without splitting the transport contract.

## Option 2 — bare launch followed by full prompt injection

Lisa could start the provider with no positional prompt, then type the complete RDSPI
instructions into the TUI after `.started`.

Advantages:

- launch command is bare and bounded;
- prompt acknowledgement naturally follows startup;
- no provider needs file-reading behavior beyond the existing instructions.

Disadvantages:

- pane input length still grows with the full assignment;
- control- and quote-heavy assignment text remains a large PTY write;
- the attempt-private assignment has no durable complete copy before injection;
- duplicates the failure mode that prompted the launch indirection work.

Decision: reject. The ticket requires a bounded chat reference to an atomic assignment
file, not merely moving the long payload to a later terminal write.

## Option 3 — atomic assignment file plus bounded tagged chat reference

Lisa writes the complete assignment to the attempt directory, starts the provider bare,
then submits a short instruction naming that file and carrying the attempt marker.

Advantages:

- both shell and chat transports are bounded;
- complete instructions exist before the process starts;
- atomic rename prevents partial assignment observation;
- the attempt directory and lease provide a natural ownership namespace;
- the existing marker classifier can correlate prompt evidence;
- both providers can expose the same scheduler states;
- stale signal rejection remains exact and simple.

Disadvantages:

- providers perform an initial file read before the full task is in context;
- generated Claude settings need a `UserPromptSubmit` hook;
- scheduler needs an explicit ready-to-deliver action between polls;
- adapter tests and launch signatures change.

Decision: choose. It satisfies every transport and state requirement with existing
attempt and hook primitives.

## Option 4 — launch a shell wrapper that feeds provider stdin

A wrapper could start the provider and pipe or paste the assignment file after detecting
readiness.

Advantages:

- assignment remains out of argv;
- wrapper could own transport retries.

Disadvantages:

- interactive TUI stdin is the terminal, not a stable message API;
- wrapper readiness detection would duplicate provider hooks;
- input attribution and deferred Enter behavior would move outside the scheduler;
- lifecycle and Zellij state would become split between shell and plugin;
- failure diagnostics would be harder to surface consistently.

Decision: reject. Lisa already owns a tested pane-input seam and provider lifecycle
signals.

## Assignment file contract

The file is named `assignment.md` under
`.lisa/attempts/<ticket>/<attempt>/work/`.

The body is exactly the provider-specific `ticket_prompt` output. Claude's body names
`CLAUDE.md`; Codex's body names `AGENTS.md`. The machine marker is not required in the
file because acknowledgement concerns the short submitted chat message.

Publication uses the existing atomic pattern:

- create the exact attempt work directory;
- write a same-directory uniquely named temporary file;
- rename it to `assignment.md`;
- remove the temporary file on rename failure;
- do not submit any pane input if preparation fails.

The assignment is prepared after the scheduler mints and registers the lease, but before
it writes a launch command or lifecycle input. Failure revokes the just-minted current
lease and leaves the seat unassigned.

The file is runtime state, not a phase artifact. Lisa's admission scanner ignores it.

## Bare launch contract

`AgentAdapter::launch_command` will stop constructing assignment text.

Claude's fresh command contains:

- optional `LISA_BIN`;
- `LISA_PANE_ID`;
- `LISA_TICKET_ID`;
- `LISA_ATTEMPT_ID`;
- `claude --dangerously-skip-permissions`;
- optional routed `--model`.

Codex's fresh command contains:

- `LISA_BIN`;
- `LISA_AGENT_CLIENT=codex`;
- pane, ticket, and attempt identity;
- full-access and hook-trust flags;
- optional routed model;
- the existing `.error` fallback after provider exit.

Neither command contains `ticket_prompt`, `assignment.md`, the marker, or any instruction
to read the ticket. The prepared launch script therefore remains bounded by lifecycle
identity, configured binary/model values, and provider flags.

Ticket ID and attempt ID remain legitimate lifecycle identity. Tests must not mistake
their presence for embedded ticket instructions.

## Bounded chat message

The fresh message is generated only after exact process-start admission. Its semantic
shape is:

```text
Read and follow the complete assignment at <attempt-path>/assignment.md.
LISA_ASSIGNMENT {"ticket_id":"...","generation":...}
```

The path is host-visible, derived from the exact slot lease, and shell-independent
because it is typed into provider chat rather than evaluated by a shell.

The message size depends only on the bounded attempt path and identity, not on ticket
content. The structured marker remains serialized with `serde_json`.

The same message shape works for Claude and Codex. Their lifecycle hook implementations
may emit different extra fields, but the minimal classifier consumes only event name and
prompt.

## Scheduler state machine

The fresh success path is:

```text
Starting --exact .started--> ReadyForAssignment
ReadyForAssignment --bounded pane send--> Delivering
Delivering --exact .ack--> Owned
```

`Starting` retains its existing process-start deadline.

`ReadyForAssignment` stores the attempt generation. It has no deadline because it is an
internal scheduler action state: the next poll either sends the message or fails
explicitly if lease/file/injection preconditions are invalid. Processing ready states
before new `.started` signals guarantees at least one poll boundary where Ready is
observable.

`Delivering` stores generation, acknowledgement deadline, and retry count. The deadline
is armed when the message is typed, with the existing deferred-Enter allowance.

`Owned` remains the sole owned state.

`DeliveryFailed` is terminal, red, and retained for operator reset. It fails the logical
thread, adds one error alert, and logs a reset instruction while retaining ticket, pane,
lease, and attempt files.

## Missing acknowledgement recovery

The first expired `Delivering` deadline performs one same-attempt redelivery:

- revalidate the slot's exact current lease;
- revalidate that `assignment.md` exists;
- ensure the pane is not blocked on a human question;
- submit the same bounded tagged reference;
- increment retry count;
- replace the absolute deadline.

The second expired deadline moves to `DeliveryFailed` with no further input.

This is a bounded recovery path: one original send plus one retry. Repeated polls after
failure are inert. A late exact acknowledgement cannot promote the terminal state because
the classifier only admits active delivery states.

Attempt identity does not rotate for chat retry. The same live provider and same complete
assignment are being retried; a new lease would imply a new execution attempt and would
conflict with the provider process's immutable launch environment. T-035-04-02 owns
rotation when the provider never started and the shell must be reset.

## Existing recycled Codex behavior

`AssignedPendingAck` and `Recovering` remain for same-process Codex reuse and its one
fresh fallback. Their current exact-generation acknowledgement paths continue to use the
shared classifier.

The fresh fallback launched from `Recovering` will now also start bare. Its process-start
semantics are already represented awkwardly by `Recovering`; changing that predecessor
contract is unnecessary for this ticket's required fresh initial path. The recovery
launch still carries its generation-tagged acceptance deadline and must remain green in
existing E-033 tests.

Same-process Claude reuse remains immediately `Owned`. Extending positive acceptance to
all reused Claude prompts would broaden the ticket beyond fresh assignment and require a
new recovery policy for an established live Claude session.

## Provider hook evidence

Rename the internal classifier module from `codex_ack` to `assignment_ack` and update its
comments/types to provider-neutral names. The wire marker remains `LISA_ASSIGNMENT`, so
existing fixtures and compatibility are preserved.

Install the existing atomic `on-ack.sh` under Claude's `UserPromptSubmit` group as well as
Codex's. `merge_hooks` must add it idempotently without disturbing user-owned entries.

The classifier continues to fail closed on malformed JSON, event mismatch, absent prompt,
and stale identity. Provider-specific extra fields remain ignored.

## Poll ordering

Ready delivery must run before `check_process_start_signals` in a poll. This gives a newly
accepted start one full scheduler boundary in `ReadyForAssignment` and prevents start
handling from immediately collapsing readiness and delivery into one unobservable state.

Prompt acknowledgement signal consumption remains before deadline evaluation, preserving
the positive-evidence-wins boundary rule.

The resulting relevant order is:

1. deliver assignments that were already ready;
2. consume new process-start signals;
3. consume prompt acknowledgement signals;
4. evaluate transition and assignment deadlines.

## Dashboard semantics

Add `ready-for-assignment`, `delivering`, and `delivery-failed` labels.

Starting, ReadyForAssignment, and Delivering are yellow because they are expected bounded
progress states. Owned remains green. DeliveryFailed is red.

Keep the older `assigned-pending-ack` label for reused Codex compatibility. The fresh
path tests must assert the full new sequence through state and rendered dashboard rows.

## Verification strategy

Adapter and command tests prove both launch commands omit assignment text and positional
prompt arguments while retaining lifecycle identity, flags, model selection, and Codex's
error handler.

Atomic-file tests use long, quoted, control-heavy instructions and prove exact persisted
bytes plus bounded launch-script size independent of assignment length.

Native scheduler tests prove:

- dispatch enters armed `Starting`;
- exact `.started` reaches only `ReadyForAssignment`;
- a later ready-delivery action writes bounded chat input and reaches `Delivering`;
- stale ticket/attempt acknowledgement does not promote either provider;
- exact acknowledgement reaches `Owned`;
- missing acknowledgement sends exactly one retry then reaches `DeliveryFailed`;
- terminal failure never becomes owned or relaunches;
- dashboard labels expose the sequence.

Template tests prove Claude and Codex both install one idempotent prompt hook.

Focused predecessor tests cover process start, dropped prompt ack, recycled Codex, and
split-brain fencing. Final verification runs the full workspace suite and `just check` if
the environment has the WASM target.
