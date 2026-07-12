# Structure: cross-policy deadline regression

## Change inventory

Modified:

- `crates/lisa-plugin/src/deadline.rs`

Created or deleted source files:

- none.

Workflow artifacts are created only under:

- `.lisa/attempts/T-039-04-03/1/work/`

No shared active-work path is written directly.

## Module boundary

The production boundary remains unchanged:

```text
State in lib.rs
  -> constructs policy-specific deadline inputs
  -> DeadlineEvaluator in deadline.rs
  -> receives typed policy-specific actions
  -> applies state-machine effects
```

New tests sit inside the existing `#[cfg(test)] mod tests` in `deadline.rs`.
This gives direct access to private evaluator inputs and result fields.

## Existing support retained

Keep `FixedClock(SystemTime)`, its `Clock` implementation, and the
`evaluator(now_secs)` helper. Both new tests use `evaluator(100)`. No additional
clock abstraction is introduced.

## New test component 1

Name:

```text
cross_policy_deadline_actions_remain_distinct
```

Internal organization:

1. construct the fixed evaluator;
2. evaluate acknowledgement expiry;
3. assert captured acknowledgement action;
4. evaluate all transition variants;
5. assert exact ordered transition actions;
6. evaluate an eligible Review input;
7. assert exact review identity;
8. evaluate overdue health;
9. assert exact observation transition;
10. evaluate silent session budget expiry;
11. assert exact reclaim deadline payload;
12. evaluate stale silence;
13. assert exact stale action.

Each typed output stays in a separate variable. There is no generic cross-policy
action wrapper.

## Action fixture identities

Acknowledgement:

```text
pane 11, state "expired", deadline 99, now 100
```

Expected one action retaining pane and state.

Transitions share:

```text
wind_down 10s
exit grace 8s
stop timeout 60s
clear timeout 90s
started at epoch
quiet at epoch
```

Candidates and expected actions:

```text
pane 21 / T-EXIT  -> ExitReady
pane 22 / T-STOP  -> StopTimedOut
pane 23 / T-CLEAR -> ClearTimedOut
```

Review:

```text
T-REVIEW, pane 31, Running, Review, unprompted, non-human,
old phase and activity clocks -> ReviewAction(T-REVIEW, 31)
```

Health:

```text
T-HEALTH, Running, old activity, previous Healthy -> current Stuck
```

Session:

```text
T-SESSION, pane 41, Running, Implement, old start and activity,
global timeout 50s -> Reclaim with elapsed_secs 100
```

Stale:

```text
T-STALE, pane 51, Running, old activity -> StaleAction(T-STALE, 51)
```

## New test component 2

Name:

```text
cross_policy_activity_and_human_exemptions_remain_distinct
```

Organization follows evaluator method order:

1. acknowledgement no-exemption baseline;
2. transition activity/human matrix;
3. review activity/human matrix;
4. health active/quiet observational contrast;
5. session activity/human action-conversion matrix;
6. stale activity/human suppression matrix.

## Shared time vocabulary

Within the test:

```text
now = epoch + 100s
recent = now
quiet = epoch
expired start = epoch
```

This makes strict transition boundaries and inclusive other boundaries
unambiguous.

## Transition exemption component

Create six candidates in order:

```text
exit + recent activity
exit + awaiting human
stop + recent activity
stop + awaiting human
clear + recent activity
clear + awaiting human
```

All transition starts are expired. Expected output contains exactly the first
two exit actions. Stop and clear do not appear. Distinct pane IDs reveal any
unexpected candidate in the assertion.

## Review exemption component

Create three otherwise eligible Review candidates:

```text
T-REVIEW-ACTIVE: recent last_activity
T-REVIEW-HUMAN: quiet, awaiting_human
T-REVIEW-FIRE: quiet, non-human
```

Expected output contains only `T-REVIEW-FIRE`.

## Health exemption component

Create two running observations:

```text
T-HEALTH-ACTIVE: recent activity
T-HEALTH-HUMAN: quiet activity
```

The second name documents the state-layer condition represented. The evaluator
has no human flag, so its quiet result remains Stuck. Expected statuses are
Healthy and Stuck in input order.

## Session exemption component

Create three over-budget running inputs:

```text
T-SESSION-ACTIVE: recent activity, non-human
T-SESSION-HUMAN: quiet activity, awaiting human
T-SESSION-FIRE: quiet activity, non-human
```

Expected ordered actions:

```text
Warn(T-SESSION-ACTIVE)
Warn(T-SESSION-HUMAN)
Reclaim(T-SESSION-FIRE)
```

Every deadline payload retains ticket and pane identity. Exact matching prevents
warning and reclaim from being collapsed.

## Stale exemption component

Create three running inputs:

```text
T-STALE-ACTIVE: recent activity
T-STALE-HUMAN: quiet activity, awaiting human
T-STALE-FIRE: quiet activity, non-human
```

Expected output contains only the `T-STALE-FIRE` action.

## Public interfaces

- No public interface changes.
- No visibility changes.
- No new trait methods.
- No action variants or input fields.
- No serialization changes.
- No configuration changes.

## Production organization

- Evaluator methods remain unchanged.
- State integration in `lib.rs` remains unchanged.
- Existing tests are retained.
- New tests are appended before the test module closes.

## Assertion helpers

No new module-level helper is planned.

Reasons:

- policy structs are intentionally heterogeneous;
- inline fixtures expose exemption-controlling fields;
- a generic helper could encode the collapsing the ticket guards against;
- two tests do not justify a builder layer.

Small local destructuring and `matches!` are acceptable where types do not
derive equality/debug traits, provided every relevant payload field is checked.

## Change ordering

1. add exact-action regression;
2. compile and run it;
3. add exemption-matrix regression;
4. compile and run it;
5. format;
6. run focused and broad gates;
7. inspect diff;
8. commit the source path through Lisa.

## Ownership boundary

Ticket-owned source:

- `crates/lisa-plugin/src/deadline.rs`.

Lisa-managed and not ticket-owned:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-039-04-03.md`;
- attempt-private artifacts awaiting final publication.

## Final source shape

```text
deadline.rs
  production evaluator (unchanged)
  production input/action types (unchanged)
  tests
    FixedClock (existing)
    evaluator helper (existing)
    fixed_clock_drives_all_six_policies_at_their_boundaries (existing)
    policy_specific_exemptions_are_preserved (existing)
    cross_policy_deadline_actions_remain_distinct (new)
    cross_policy_activity_and_human_exemptions_remain_distinct (new)
```

This is the complete architectural impact.
