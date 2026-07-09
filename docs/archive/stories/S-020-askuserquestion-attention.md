---
id: S-020
title: askuserquestion-attention
status: open
---

## AskUserQuestion Attention — Precise "needs human input" Signal

S-019 notifies on loop completion, permission prompts, and idle-without-artifact.
The one moment it can't reliably catch is an agent **asking a clarifying question**:
in most phases that surfaces as "idle without artifact" (caught heuristically), but
in the Implement phase idle is read as completion (`lib.rs:765`), so the question is
missed — and worse, lisa may type over it.

The precise signal is the **`AskUserQuestion` tool invocation** itself. A
`PreToolUse` hook matched on that tool fires at the exact moment the agent asks,
with the question text in the hook payload — no confusion with phase-completion,
in any phase. This story is to **figure out** whether that mechanism works in
lisa's `--dangerously-skip-permissions` agent context and, if so, wire it to the
`on-notify attention` hook from S-019.

### Why this needs the deferred correctness fix

An `AskUserQuestion` call **blocks quietly**: while it waits for an answer, the
agent emits no tool calls, so no `PostToolUse` heartbeat fires and the pane looks
idle. lisa's auto-injection (`/clear`, next-ticket prompt, finish-up) is gated only
on heartbeat-quietness, which does **not** protect a question-blocked pane — so lisa
can type a `/clear` straight over the agent's question. Notifying the human is
pointless if lisa then clobbers the prompt before they arrive. So this story owns
the **"awaiting human" suppression** that S-019 explicitly deferred: a per-pane flag
that pauses `send_line_to_pane` (and the Implement-idle auto-advance) for that pane
until the agent resumes (next heartbeat).

### Open questions to resolve (the spike)

1. Does invoking `AskUserQuestion` fire a `PreToolUse` hook, and what exact tool-name
   string does the matcher need? (lisa binds no `PreToolUse` today.)
2. Do lisa's `claude --dangerously-skip-permissions` agents ever actually invoke
   `AskUserQuestion`, or does that mode/headless-ish pane context suppress it? If
   agents never ask via the tool, this whole signal is moot — establish this first.
3. `PreToolUse` payload shape: can we extract the question text (POSIX `sh`, no `jq`)
   to pass as the `attention` detail?
4. Does the question pane reliably resume with a `PostToolUse` heartbeat once
   answered, so the "awaiting human" flag can be cleared on the existing
   `check_heartbeat_signals` path (`lib.rs:679`)?
5. Suppression design: a new signal file (e.g. `pane-$ID.awaiting`) the plugin reads
   to set the flag, vs. inferring from the PreToolUse event. What's the smallest change
   to `scheduler`/`lib.rs` that pauses injection without destabilizing the heartbeat
   liveness model? (See [[liveness-heartbeat-design]].)
6. Interaction with timeouts: should an awaiting-human pane be exempt from review/
   transition timeouts (`check_review_timeouts` at `lib.rs:1199`, transition timeouts
   ~`lib.rs:1138`) so it isn't reclaimed mid-question?

### Likely shape (pending spike — not committed)

- New `PreToolUse[AskUserQuestion]` Claude hook → writes `pane-$ID.awaiting` and/or
  fires `on-notify attention "<question>"` with `LISA_REASON=question`.
- Plugin reads the await signal, sets a per-pane "awaiting human" flag that suppresses
  `send_line_to_pane` + Implement-idle auto-advance + timeout reclamation for that pane.
- Flag cleared on the next heartbeat (agent resumed after the human answered).
- Reuses the S-019 `on-notify` contract; no new user hook.

### Scope

- Spike: answer the open questions above; produce a design that defines the
  implementation tickets. **Q2 is the gate** — if skip-permissions agents never use
  `AskUserQuestion`, the story pivots or closes.
- Implementation tickets (T-020-02+) are **TBD, derived from the spike's design**.

### Dependencies / relationship

- Builds on S-019's `on-notify` hook and `run_command` plumbing (T-019-01, T-019-02).
- Owns the "awaiting human" suppression deferred by S-019.

### Tickets

- **T-020-01** — Spike: AskUserQuestion → PreToolUse detection + awaiting-human design
- T-020-02+ — TBD from spike output
