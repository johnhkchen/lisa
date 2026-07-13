# Review — T-045-03-03 delivered awaiting claim

## Disposition

Pass.

The scheduler now has a real `DeliveredAwaitingClaim` state for a live, current Codex
TUI whose initial delivery window expires without ownership evidence.

That transition performs no assignment reinjection.
It replaces the old live-Codex retry window with a finite passive wait.

If the passive wait expires, the scheduler reaches the distinct retained terminal state
`ClaimTimedOut`, not `DeliveryFailed`.

The operator can see the state, retrieve its durable pre-ownership record, inspect the
retained pane, and reset the ticket explicitly.

All enabled tests, formatting checks, whitespace checks, and the WASM build check pass.
Ticket-owned source is committed and clean.
No critical issue blocks completion.

## Ticket commit

The isolated source commit is:

```text
88efa9877530db45494ada78e277fd7fb63d311e
feat(plugin): await delivered Codex assignment claims
```

It contains exactly:

- `crates/lisa-cli/src/preownership_status.rs`;
- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

It was created through:

```text
lisa commit-ticket \
  --ticket-id T-045-03-03 \
  --message "feat(plugin): await delivered Codex assignment claims" \
  --include crates/lisa-cli/src/preownership_status.rs \
  --include crates/lisa-core/src/provenance.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

No ordinary `git add`, broad staging, ordinary `git commit`, or broad include was used.

`git show --check HEAD` passes.
All four source paths are clean relative to HEAD.
The ordinary Git index is empty.

## Scheduler state changes

`SeatAssignmentState` gained the active unowned variant:

```text
DeliveredAwaitingClaim {
    generation,
    claim_deadline,
}
```

The state means:

- the assignment is already in the delivered path;
- the scheduler still retains a live current Codex session;
- no admissible ownership evidence arrived in the initial delivery window;
- the scheduler is waiting passively for claim or fallback evidence;
- no duplicate assignment send is authorized.

The state carries the exact attempt generation and one absolute finite deadline.

`SeatAssignmentState` also gained:

```text
ClaimTimedOut
```

This state means the passive evidence window ended without ownership.
It is terminal for the attempt and retains the reservation for operator inspection and
reset.

Only `Owned` still satisfies `seat_is_owned`.

## Scope boundary

The new passive transition applies only when `is_live_codex_delivery` proves:

- matching physical pane;
- retained ticket reservation;
- live session flag;
- Codex as the resident client;
- matching slot lease ticket;
- matching delivery generation;
- exact current lease authority.

This protects the story's smallest contract.

Claude does not enter the new path.
Missing, stale, or non-live delivery state does not enter the new path.
Those cases retain the prior bounded delivery retry/failure behavior.

No launcher, adapter, signal, configuration, claim, assignment-file, or completion
schema changed.

## No-reinjection behavior

The live-Codex `Delivering` timeout branch no longer calls:

- `deliver_assignment_to_pane`;
- `send_line_to_pane`;
- any launch or recovery helper.

It only inserts `DeliveredAwaitingClaim`, computes its deadline, and records a warning.

The acceptance test snapshots three independent send-side observations before the
transition:

- assignment-delivery activity count;
- pending deferred-Enter count;
- session-launch activity count.

All three remain unchanged after the old deadline expires.

This is stronger than checking only a dashboard label or retry counter.

## Bounded timeout resolution

The passive deadline is computed with the existing `assignment_ack_deadline(now)`.

That preserves:

- the configured positive acknowledgement duration;
- the deferred-Enter allowance;
- the existing overflow fallback;
- deterministic injected-time tests.

The existing generic deadline evaluator recognizes `claim_deadline`.
No new timer loop or unbounded wall-clock wait was introduced.

On expiry, `fail_assignment_claim_wait` first inserts `ClaimTimedOut` as its exact-once
guard.

It then:

- marks the logical thread failed;
- appends claim-timeout provenance;
- raises the existing error alert;
- tells the operator to inspect the pane and reset the ticket;
- returns typed `AssignmentClaimTimedOut`.

It does not send input, relaunch, mint an attempt, release the reservation, revoke the
lease, or silently redispatch.

Repeated timeout evaluation is inert.

## Evidence admission

`active_assignment_generation` includes `DeliveredAwaitingClaim`.

All predecessor ownership paths therefore remain admissible during passive waiting:

1. exact assignment claim;
2. supplemental matching `UserPromptSubmit`;
3. bounded current-attempt artifact fallback.

Existing slot/current-lease/nonce validation remains unchanged.
Stale evidence remains fenced.

The exact-claim regression now proves a valid claim can promote the new passive state to
`Owned` without a hook file.

The supplemental hook regression proves a matching current generation can also promote
the passive state and suppress its later deadline.

Once terminal `ClaimTimedOut` is inserted, no active generation is exposed.
A late exact provider acknowledgement cannot resurrect ownership.

## Durable operator vocabulary

`lisa_core::provenance::AssignmentState` gained `ClaimTimedOut`.

The existing serde policy writes:

```text
claim-timed-out
```

The record remains attempt-scoped and contains the exact ticket, lease, pane, provider,
reason, and bounded timestamps.

The acceptance regression reads the actual ledger and requires one assignment-transition
row with:

- the current attempt lease;
- `AssignmentState::ClaimTimedOut`;
- the explicit bounded-deadline reason.

It also requires that repeated timeout evaluation does not append another row.

The CLI pre-ownership status formatter gained the same stable name and a direct test.
This fourth source file was a necessary implementation-plan deviation discovered by an
exhaustive compiler error.

## Dashboard projection

The UI gained two projection values:

- yellow `delivered-awaiting-claim`;
- red `claim-timed-out`.

`State::to_ui_state` maps directly from the private scheduler variants.

No UI inference based on missing hooks, elapsed time, or pane labels was added.
The scheduler remains the authority, satisfying the story's non-cosmetic requirement.

## Acceptance test coverage

The principal regression is:

```text
live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
```

It covers:

- fresh Codex ticket delivery;
- live session and current lease premise;
- absent claim and hook evidence;
- transition from `Delivering` to `DeliveredAwaitingClaim`;
- finite later claim deadline;
- zero duplicate delivery, Enter, or launch;
- scheduler and dashboard state visibility;
- typed terminal outcome;
- `ClaimTimedOut` state;
- retained ticket and attempt lease;
- failed logical thread and error alert;
- actionable inspect/reset log;
- absence of delivery-failure reporting;
- exact durable provenance;
- terminal idempotence.

Historical Codex prompt-miss, dropped-hook, current-generation evidence, and consecutive
pane-reuse regressions were updated to the new passive behavior.

Existing claim, hook, artifact, stale-evidence, startup, recovery, delivery-failure,
Claude, lease fencing, and completion suites remain green.

## Verification

Focused pre-change baselines passed for exact claim, supplemental hook, artifact
fallback/stale fencing, and old bounded delivery retry.

After implementation:

```text
cargo test -p lisa-plugin
```

passed with 393 tests before the CLI formatter test was added.

The final quick check:

```text
just check
```

passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

Observed final principal totals:

- plugin: 394 passed;
- core: 200 passed plus integration suites;
- CLI binary: 270 passed;
- CLI library: 19 passed;
- all enabled integration and doc tests passed;
- real-Zellij delivery test remained environment-gated and ignored as expected.

These additional commands passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-cli preownership_status
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check
git show --check HEAD
```

## Open concerns and limitations

No blocking implementation concern remains.

The story's honest boundary is fixture-level scheduler proof.
This ticket does not claim that an installed real Codex TUI under real Zellij exhibits
the expected timing; S-045-05 owns that field validation.

The live-session premise is scheduler-owned slot lifecycle state because Codex has no
truthful pre-prompt readiness hook.
That matches the ticket's fixture boundary and keeps actual process validation assigned
to the later field-test story.

The existing first delivery remains part of the established grace-paced delivery path.
This ticket removes the later duplicate retry when hook evidence is absent; it does not
redesign startup or launcher transport.

Claude's mechanism remains unchanged.

## Final assessment

The acceptance criterion is satisfied.

A delivered ticket in a live current Codex TUI enters a real bounded
`DeliveredAwaitingClaim` state when hook/claim evidence is slow, receives no duplicate
prompt, and ends in distinct actionable `ClaimTimedOut` rather than silent retry or
false `DeliveryFailed`.

The work is ready for Lisa's completion transaction.
