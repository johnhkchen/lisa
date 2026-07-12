# Design — T-037-01-03 delayed-send-and-prompt-miss-regression

## Decision

Add **two** new `#[test]` functions to the existing test module in
`crates/lisa-plugin/src/lib.rs`, immediately after the T-037-01-02 grace tests
(~lib.rs:9839). Reuse the established harness (`pane_name_schedule_state`,
`acknowledge_assignment`, injected `check_assignment_ack_timeouts_at`). No
production code changes. No new helpers unless a step repeats verbatim.

1. `codex_delayed_send_reaches_owned_only_on_current_attempt_ack`
2. `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`

Rationale: the acceptance criterion names exactly these two injected-time tests
and requires the existing green tests to stay green. The behaviour already
exists (T-037-01-02); the deliverable is regression evidence, so the lowest-risk
design is additive tests in the same module using the same idioms.

## What each test must prove (mapped to acceptance criteria)

Acceptance: *"a delayed-send test (Codex grace→Delivering directly, never
synthetic ReadyForAssignment, then Owned only on the exact current-attempt
UserPromptSubmit) and a prompt-miss test (grace elapses, no matching ack →
bounded retry then named recycle/DeliveryFailed, never Owned, stale-attempt
signals rejected)."*

### Test 1 — delayed-send → Owned only on exact ack

| Claim in AC | Assertion strategy |
|---|---|
| Codex is grace-mode | `seat_readiness_mode(10) == Some(Grace)` |
| the send is *delayed* (paced) | poll `check_assignment_ack_timeouts_at(before_deadline)` → seat still `Starting`, nothing delivered (no `Delivering`/`ReadyForAssignment`/`Owned`; delivery-log count 0). This is the new coverage the happy-path test lacks. |
| grace→Delivering **directly** | at `grace_deadline`: seat is `Delivering { generation == lease, retries: 0 }` |
| never synthetic ReadyForAssignment | assert the seat is never `ReadyForAssignment` at any observed step; UI status goes `Starting`→`Delivering`, never `ReadyForAssignment` |
| Owned only on exact current-attempt ack | stale generation (`attempt_id + 1`) rejected → not owned; wrong ticket id rejected → not owned; exact `(T-NAME, attempt_id)` ack → `Owned` |

### Test 2 — prompt-miss → bounded named DeliveryFailed, never Owned

| Claim in AC | Assertion strategy |
|---|---|
| grace elapses, no matching ack | drive grace elapse → `Delivering{0}`; never send a matching ack |
| bounded retry | elapse `Delivering{0}` ack_deadline → `Delivering{1}`; delivery-log count == 2 (initial + one retry) |
| stale-attempt signals rejected | during `Delivering{1}`, a stale-generation ack returns false, not owned |
| named recycle / DeliveryFailed | elapse `Delivering{1}` ack_deadline → `DeliveryFailed`; UI status `DeliveryFailed`; thread `Failed`; reservation + current lease retained for operator reset |
| never Owned | assert `!seat_is_owned` at every step, and a late exact-generation ack after `DeliveryFailed` still returns false (terminal — `active_assignment_generation` is `None`) |
| E-034 fencing intact | out of scope to re-test here; covered by run of full suite staying green |

## Options considered

**A. Two additive tests in lib.rs's test module (chosen).**
Matches the AC's exact wording, reuses the proven harness, zero production
risk, and keeps the grace-mode prompt-miss coverage next to the happy path.

**B. Extend the existing `codex_startup_grace_paces_...` test in place.**
Rejected: the AC calls for *two new* tests and distinguishes delayed-send from
prompt-miss as separate regressions (P5 — the field deadlock becomes two named
regressions). Folding them loses the independent failure signal and muddies
which behaviour broke when one fails.

**C. Parameterise both providers through one table-driven test.**
Rejected: the Claude/SessionStart retry→fail path already has its own test
(`test_missing_fresh_chat_ack_...`) entered via a different seam
(`.started` signal). Unifying would couple two intentionally distinct entry
paths and reduce readability. The story explicitly scopes the *grace* path.

**D. Add a shared helper `advance_grace_to_delivering(state, pane, lease)`.**
Deferred unless the exact block repeats. Both tests need "grace elapse →
Delivering{0}", which is ~6 lines. If duplicated verbatim it earns a small
private helper; otherwise inline for locality. Decision: inline first; extract
only if it reads cleaner (judged in Structure/Implement).

## Why this is grounded in the research

- The grace pace is `Starting → Delivering` with no `ReadyForAssignment` node
  (research §"Where grace behaviour lives" #2), so "never synthetic
  ReadyForAssignment" is checkable by asserting the state sequence.
- `active_assignment_generation` returns `None` for `DeliveryFailed`
  (research §"Ownership gate"), so a post-failure ack provably cannot own —
  this is how "never Owned" is pinned as terminal, not merely momentary.
- Injected `check_assignment_ack_timeouts_at` + deadlines read from state give
  full determinism with no sleeps (research §"Injected time").
- `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1` fixes the exact miss sequence
  (Delivering{0}→Delivering{1}→DeliveryFailed), so counts are exact, not
  approximate.

## Risks & mitigations

- *Risk:* `deliver_assignment_to_pane` fails if `assignment.md` is absent,
  turning the grace elapse into `DeliveryFailed` in Test 1. *Mitigation:*
  `schedule_ready_tickets` stages it; the happy-path test already relies on
  this, so it holds.
- *Risk:* clock granularity making `before_deadline` flaky. *Mitigation:* use
  `grace_deadline - Duration::from_secs(1)` (grace is 8s), an unambiguous
  strictly-earlier instant, not a now-relative value.
- *Risk:* overlap with existing tests reduces value. *Mitigation:* Test 1 adds
  the pre-deadline quiescence + wrong-ticket rejection the happy path omits;
  Test 2 is the only grace-entered prompt-miss.
