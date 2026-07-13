# Design — T-045-04-01 clean-exit-revoke-attempt

## Decision summary

Add a successful-completion slot-release seam that preserves generic release
semantics, then converts only a resident Codex slot into an unassigned bounded
`WaitingForExit` transition.

The sequence will be:

1. identify whether the completed slot hosts a resident Codex TUI;
2. run the existing release boundary, which revokes the attempt and clears the
   ticket reservation and seat ownership;
3. submit the resident Codex adapter's `/exit` command;
4. mark the now-unassigned slot `WaitingForExit` and unavailable to scheduling;
5. let the existing exit-grace transition prove the clean-shell boundary;
6. schedule the next ticket only after that transition returns the slot to Idle;
7. launch that ticket through the existing fresh Codex launcher path.

Add a native boundary test that drives claim, successful-ticket cleanup, late
claim rejection, exit grace, and next-ticket launch while printing a stable
scheduler transcript.

## Design goals

Make clean exit a fact of successful Codex completion, not a side effect of
whether another ticket happens to be eligible immediately.

Revoke the predecessor authority before any successor can reserve the pane.

Keep a late predecessor nonce-bearing claim rejected by existing lease and seat
admission checks.

Keep the pane unavailable throughout the asynchronous exit grace.

Reuse the existing `WaitingForExit` deadline and unassigned-slot branch.

Keep Claude's completion release and `/clear` reuse unchanged.

Keep hard-silence fencing, failure release, and startup recovery unchanged.

Make the ordering legible in one deterministic fixture transcript.

## Option 1 — retain current behavior and add only a characterization test

The existing code revokes the completed lease during release.
When a next Codex ticket is scheduled, it sends `/exit` and later launches fresh.
A test could demonstrate that eventual sequence.

Advantages:

- no production source behavior changes;
- existing consecutive-reuse fixtures cover most mechanics;
- minimal regression risk.

Disadvantages:

- the completed TUI does not exit when no successor is ready;
- the successor lease and slot reservation are created before exit completes;
- clean exit remains coupled to scheduling rather than completion;
- it does not satisfy the acceptance wording that the TUI exits on completion;
- the boundary transcript would characterize the gap instead of closing it.

Rejected because the ticket is specifically about the completion boundary.

## Option 2 — change `release_slot_for_ticket` to exit every resident Codex TUI

The shared release method could detect `last_client == Codex`, send `/exit`, and
move every released Codex slot into `WaitingForExit`.

Advantages:

- centralizes Codex exit policy in one release function;
- all successful and unsuccessful releases get a clean process boundary;
- few call-site changes.

Disadvantages:

- release is used by timeout, stale, error, reset, audit, and operator paths;
- some failure paths already fence or recover with different safety semantics;
- a generic release would silently broaden the ticket beyond successful completion;
- release callers do not all mean that a responsive TUI is safe to exit cleanly;
- tests that characterize retry and fencing policy could change indirectly.

Rejected because the shared release boundary carries more meanings than success.

## Option 3 — add a Codex-specific successful-completion release wrapper

Introduce a private method called only after durable completion confirmation.
It snapshots the assigned slot's resident-provider fact, delegates to the existing
release method, and conditionally begins an unassigned clean exit for Codex.

Advantages:

- completion journal, provenance, and generic release remain unchanged;
- only successful completion gains the new process policy;
- Claude continues through the existing release path without an exit;
- hard-silence and failure callers remain untouched;
- the existing unassigned `WaitingForExit` transition already supplies the wait;
- next scheduling is naturally blocked by `transition_state != Idle`;
- no new durable state, enum, timer, or host API is required.

Disadvantages:

- release behavior is split across a generic method and one wrapper;
- the wrapper must snapshot the pane before generic release clears its ticket;
- state written by generic release must be normalized for the exit transition;
- tests must distinguish completion cleanup from ordinary release.

Chosen because it is the smallest boundary-specific change grounded in existing code.

## Option 4 — reserve the successor first and retain current recycle transition

The completion path could explicitly schedule a successor and rely on the current
ticket-bearing `WaitingForExit` transition.

Advantages:

- uses the current launch-after-exit implementation unchanged;
- successor context is immediately available for the post-grace launch;
- resembles existing same-provider Codex reuse.

Disadvantages:

- no successor means no exit;
- the next attempt is minted before the old process reaches shell;
- the outgoing and incoming ticket lifecycles overlap in one slot;
- it weakens the desired transcript ordering between revoke, exit, and fresh launch.

Rejected for the same reason as Option 1.

## Successful-completion helper contract

The helper accepts only a ticket ID, matching `release_slot_for_ticket`.
Before release it searches for the slot assigned to that ticket.
It records the pane ID only when all of these facts hold:

- the slot is not fenced;
- `has_session` is true;
- `last_client == Some(AgentClient::Codex)`.

It resolves the exit spelling through the resident Codex adapter.
The adapter abstraction remains the owner of `/exit` syntax.

It then calls `release_slot_for_ticket` exactly once.
That call revokes current authority before any exit or future scheduling.
It clears ticket lease, ticket reservation, and seat assignment as today.
It emits the established lease-revoked and slot-released test events.

For a snapshotted Codex pane, the helper submits `/exit` via `send_line_to_pane`.
It mutates the released slot to:

- `transition_state = WaitingForExit`;
- `transition_started_at = Some(now)`;
- `has_session = false`, matching the existing recycle branch;
- `cooldown_until = None`, because the transition itself is the gate;
- `last_client = Some(Codex)` until shell-ready cleanup identifies an empty slot.

The slot remains unassigned: no ticket ID, attempt lease, or seat state is restored.
The helper removes attention markers already cleared by release and makes no launch.
It logs a completion-boundary message naming the ticket and pane.

For Claude, an empty shell, a fenced pane, or a missing slot, the helper adds no
process action beyond the existing release.

## Exit-grace behavior

`find_idle_slot` and `find_slot_for_client` both require `TransitionState::Idle`.
Therefore the immediate `schedule_ready_tickets` call in completion handling finds
no reusable slot while the completion exit is pending.

On a later poll, `check_transition_timeouts` derives `ExitReady`.
Because the slot has no ticket reservation, its existing unassigned branch:

- sets the transition to Idle;
- clears transition time;
- keeps `has_session` false;
- clears `last_client`;
- removes any seat state;
- renames the pane to empty `lisa · idle`.

The scheduler later in the same poll may then reserve it.
Because `has_session` is false, Codex takes the fresh-pane launch branch.
No `/clear`, same-process prompt, or incoming-ticket recycle is used.

## Nonce revocation semantics

No assignment file is deleted.
No `assignment_refs` entry needs to be removed to enforce authority.
The predecessor claim is rejected because release removes current lease authority,
slot lease identity, slot reservation, and active seat generation.

The boundary test will retain the exact predecessor `AssignmentClaim` value and
call scheduler admission after completion cleanup.
It must return false before and after the successor is launched.
The successor's new `AssignmentRef` must not equal the predecessor reference.
This demonstrates that immutable historical bytes do not constitute authority.

## Transcript evidence

Extend the native `AttemptLifecycleEvent` trace with a successful clean-exit event.
The trace will distinguish a completion exit request from hard pane fencing.
The event carries the predecessor ticket and pane.

The boundary test will combine safety-order observations with scheduler activity:

- claimed predecessor ticket and nonce;
- lease revoked;
- slot released;
- clean exit requested;
- late claim rejected;
- exit grace reached an empty shell;
- next ticket reserved;
- new launch script submitted;
- fresh attempt awaiting ownership.

Stable `println!` rows make the transcript visible with `--nocapture` while exact
state assertions, rather than text alone, enforce correctness.

## Test fixture

Create two Codex tickets with an explicit dependency.
Use one physical pane and zero wind-down seconds.
Start from an empty shell so the first ticket uses a genuine fresh launch.
Advance its startup grace into assignment delivery without sleeping.
Build an exact `AssignmentClaim` from the retained nonce and admit it.
Treat the claimed seat as the fixture's completed work state.

Update the first ticket to durable Done for DAG readiness.
Run the successful-completion release helper and remove the completed thread,
matching the cleanup portion of verified completion without asserting the
completion-record invariant owned by `T-045-04-02`.

Attempt the retained predecessor claim and require rejection.
Call scheduling and prove the next ticket cannot reserve the exiting slot.
Inject an expired exit transition timestamp and evaluate transition timeouts.
Schedule again and require the dependent next ticket to take the empty slot.
Inspect its launch script and `Starting` seat state as fresh-TUI evidence.

## Regression verification

Run the focused new boundary test with `--nocapture`.
Run existing Codex consecutive-reuse tests.
Run existing Claude consecutive-reuse and clear-handshake tests.
Run attempt-lease and hard-fencing focused tests.
Run the full `lisa-plugin` native suite.
Run `cargo test --workspace` if focused verification is green.

No CLI, core, adapter, config, UI, or fixture-file change is expected.
