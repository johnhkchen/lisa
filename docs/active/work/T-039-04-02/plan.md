# Plan: clock-injected deadline evaluator

## 1. Establish baseline

- Run the six T-039-04-01 characterization tests unchanged.
- Record the result in `progress.md`.
- Inspect ordinary-index and worktree state before editing.
- Preserve Lisa-managed ticket and provenance changes.

Verification:

- All six characterization tests pass.
- No ticket-owned source path is already modified or staged.

## 2. Add the evaluator module

- Create `crates/lisa-plugin/src/deadline.rs`.
- Define `Clock`, `SystemClock`, and `DeadlineEvaluator<C>`.
- Define typed inputs and actions for all six policies.
- Implement pure evaluation with one clock sample per call.
- Preserve inclusive and strict boundary differences.
- Preserve active-session and awaiting-human rules.

Verification:

- Module compiles once wired into the crate.
- Unit tests prove exact deterministic firing with a fixed clock.

## 3. Integrate acknowledgement evaluation

- Route acknowledgement candidate selection through the evaluator.
- Retain `check_assignment_ack_timeouts_at(now)`.
- Preserve seat-state revalidation and action ordering.
- Keep the production wrapper sampling the system clock.

Verification:

- Existing acknowledgement characterization passes unchanged.
- Existing assignment deadline tests pass.

## 4. Integrate transition evaluation

- Replace the slot traversal with typed transition inputs.
- Apply exit, stop, and clear actions through existing bodies.
- Move quietness and awaiting-human eligibility into the evaluator.
- Preserve strict whole-second threshold semantics.

Verification:

- Existing transition characterization passes unchanged.
- Existing transition timeout and awaiting-human tests pass.

## 5. Integrate review evaluation

- Replace Review candidate filters with evaluator inputs.
- Preserve disabled configuration and idempotence.
- Preserve adapter follow-up effects and activity updates.
- Use the evaluator's sampled instant for the phase-clock reset where possible.

Verification:

- Existing Review characterization passes unchanged.
- Existing finish-up prompt tests pass.

## 6. Integrate health evaluation

- Replace health traversal calculations with evaluator observations.
- Preserve transition logging, first observation insertion, and cache pruning.
- Keep awaiting-human visible as stuck.

Verification:

- Existing health characterization passes unchanged.
- Existing health tests pass.

## 7. Integrate session evaluation

- Build session inputs from running threads and phase timeout configuration.
- Replace budget and hard-silence traversal with evaluator actions.
- Preserve warning idempotence.
- Preserve reclaim effect ordering and returned outcomes.

Verification:

- Existing session characterization passes unchanged.
- Global, per-phase, disabled, warning, exemption, and fencing tests pass.

## 8. Integrate stale evaluation

- Build stale inputs from thread activity and exclusions.
- Replace stale traversal with evaluator reclaim actions.
- Preserve failure, fencing, provenance, release, removal, logging, and outcomes.

Verification:

- Existing stale characterization passes unchanged.
- Existing stale and fencing tests pass.

## 9. Format and focused verification

- Run `cargo fmt --all`.
- Inspect formatting changes and discard no user work.
- Run evaluator unit tests.
- Run the unchanged `characterizes_` filter.
- Run focused deadline/session/stale/health tests.

Verification:

- All focused tests pass.
- No characterization test was edited.

## 10. Full verification

- Run `cargo test -p lisa-plugin`.
- Run `cargo test --workspace`.
- If available and proportionate, run `just check`.
- Record exact results and any environment-gated skips.

Verification:

- All required gates are green.
- Failures are investigated rather than silently ignored.

## 11. Review the diff

- Compare T-039-04-01 characterization lines against `HEAD` to confirm unchanged.
- Confirm all six production paths call `DeadlineEvaluator`.
- Check boundary semantics against Research and Design.
- Inspect for unrelated edits.
- Update `progress.md` with deviations and final state.

Verification:

- Diff contains only the evaluator and integrations.
- Ticket-owned paths are ready for isolated commit.

## 12. Commit the meaningful source unit

Run:

```text
lisa commit-ticket --ticket-id T-039-04-02 \
  --message "refactor(plugin): centralize deadline evaluation" \
  --include crates/lisa-plugin/src/deadline.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Verification:

- Lisa reports a successful isolated transaction.
- Both source paths are clean afterward.
- Neither path is staged in the ordinary index.
- Lisa-managed ticket/provenance changes remain outside the source commit.

## 13. Review artifact

- Write `review.md` in the attempt-private directory.
- Summarize architecture, files, behavior, tests, and commit.
- Identify gaps, open concerns, and critical issues.
- Do not edit ticket phase/status.
- Stop on this ticket after the artifact is complete.
