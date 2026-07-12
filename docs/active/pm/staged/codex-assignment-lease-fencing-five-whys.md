---
title: codex-assignment-lease-fencing
status: chained
chain_after: T-033-03-02
agent: codex
materialized_epic: E-034
chained_at: 2026-07-11
evidence: T-031-02
---

# Codex assignment lease fencing — five whys

## Decision

E-033 is necessary but not sufficient. It makes a newly recycled Codex seat
wait for positive acknowledgement and adds bounded recovery when that new prompt
is missed. It does not fully remove a timed-out old pane's authority, identify
execution attempts independently of tickets and panes, or prevent late work from
the old attempt from advancing the replacement attempt.

Stage a separate epic after E-033. Do not add it to the current loop: the loop's
loaded plugin predates both fixes, and extending its DAG would exercise more
Codex handoffs through the known-buggy runtime.

## Field evidence

The current lisa provenance ledger records two attempts for T-031-02:

- pane 2: `timed-out` after 3,876 seconds;
- pane 1: `done` after 270 seconds.

That proves timeout followed by cross-pane reassignment. The observed terminal
state supplies the missing operational detail: the old pane continued finishing
while the replacement Codex pane held the scheduler assignment but did not
receive/submit its prompt.

The responsible timeout path explicitly releases the logical slot and thread
without killing the provider process because it "may still be doing useful
work." That salvage policy becomes unsafe once the same ticket is retried.

## Five whys

### 1. Why can the pane shown as assigned differ from the pane doing the work?

Because lisa times out a quiet ticket attempt, releases its pane association,
and schedules the still-non-done ticket on another pane while the original Codex
process can remain alive and later resume.

### 2. Why is the original Codex process allowed to continue after reassignment?

Because session timeout currently means "remove the scheduler thread and free
the slot," not "revoke this execution attempt." The code deliberately does not
terminate the process so potentially useful late work is not discarded.

### 3. Why can late work from the old pane affect the new assignment?

Because artifacts are addressed only by ticket ID under
`docs/active/work/<ticket-id>/`, while scheduler ownership is currently a mutable
ticket-to-pane association. There is no execution-attempt generation or lease
attached to artifact admission, phase advancement, completion, or provenance.

### 4. Why does the replacement pane's prompt miss make this split-brain state
persist silently?

Because the native Codex reuse path historically treated prompt injection as
ownership without a positive ticket-scoped acknowledgement. E-033 addresses
that half by adding pending acknowledgement and bounded fallback, but a correct
new-pane handshake alone does not fence the already-running old attempt.

### 5. Why was there no fence between attempts?

Because lisa's original lifecycle model conflated four identities that are equal
on the happy path: ticket, scheduler thread, physical pane, and provider process.
Timeout/retry makes them diverge. Without an attempt-scoped single-writer lease,
the scheduler cannot authoritatively say which process may emit signals, publish
artifacts, advance phases, or complete the ticket.

## Root cause

Lisa lacks an attempt-scoped execution lease and revocation/fencing protocol.
The Codex prompt miss exposes the gap, but the deeper defect is that timeout can
create two live writers for one ticket while all durable artifacts remain keyed
only by ticket ID.

## Required epic contract

- Mint a unique, monotonic attempt identity whenever a ticket is assigned or
  reassigned. Ticket ID and pane ID alone are not sufficient.
- Revoke the old attempt before admitting a retry. Stop, exit, close, or
  quarantine the old provider process with a bounded acknowledgement; inability
  to fence it is a named blocking error, not permission to schedule a second
  writer.
- Carry the current attempt identity through Codex acknowledgement and lifecycle
  events. Stale heartbeats, cleared/stopped/error events, and duplicate acks from
  an older attempt cannot claim or advance the replacement.
- Ensure only the current lease holder can cause artifact-driven phase advance,
  completion commit, slot release, or `done` provenance. If direct artifact
  attribution cannot be made safe, isolate attempt output until publication.
- Preserve E-033's pending-ack state and one-shot fresh-session fallback as the
  new-attempt admission mechanism; extend it rather than creating a parallel
  handoff state machine.
- Record attempt identity and predecessor/retry linkage in activity and
  provenance so a future field report can reconstruct ownership without relying
  on dashboard observation.
- Keep Claude behavior unchanged unless a provider-neutral lease invariant can
  be added without changing its proven handshake.

## Required regression

Reproduce the T-031-02 sequence deterministically:

1. Codex attempt A owns a ticket and becomes silent past timeout.
2. Attempt A is slow rather than dead and later tries to resume.
3. Lisa prepares replacement attempt B on another pane.
4. Attempt B's initial prompt acknowledgement is deliberately dropped.
5. Attempt A emits late activity/artifacts.

The fixed system must never have two authoritative writers. It must either fence
A before admitting B, or block B with a named fence failure. A's late events and
artifacts cannot advance B. B must follow E-033's bounded acknowledgement/fallback
path. Exactly one attempt may produce the completion commit and `done` provenance.

After the deterministic proof is green, run a fresh installed lisa loop with a
forced timeout/prompt-miss leg and verify the same invariants from pane activity,
commit trees, work artifacts, and provenance.

## Chain command — run only after the current loop exits

```bash
vend chain "$(cat docs/active/pm/staged/codex-assignment-lease-fencing.signal.txt)" \
  --after T-033-03-02 \
  --agent codex
```

Preflight before running it:

```bash
lisa status
vend doctor
git status --short
```

Confirm the current loop is no longer running, T-033-03-02 is durably done, and
the working tree contains no unexplained loop residue before chaining.
