# Research: recorded Review livelock regression

## Ticket and story boundary

- T-041-02-01 turns one recorded field incident into deterministic test evidence.
- The parent story S-041-02 is the proof layer for the completion contract.
- The story says its work lives in lisa-core test modules and Cargo dev-dependencies.
- This ticket needs no new dependency; the generated follow-on owns proptest dependencies.
- The story explicitly says there are no production source changes.
- The settled S-041-01 completion module is consumed read-only.
- Plugin adapter wiring and a live Arcade replay belong to E-042.
- This test is a fixture transcription, not a live provider or scheduler test.

## Recorded incident evidence

- `docs/active/pm/staged/review-completion-livelock-field-note.md` records the incident.
- The evidence repository was Arcade's nested `games/midsummer` project.
- The evidence ticket was T-009-01-01, attempt 1.
- The private Review artifact was written before completion occurred.
- Lisa advanced the ticket only as far as Review.
- No pending or successful completion transaction existed at that point.
- At 16:21:39 PDT the roughly ten-minute Review timeout fired.
- The timeout injected a generic finish-up prompt despite the artifact already existing.
- The agent revalidated the same artifact and stopped again.
- The ticket remained in Review.
- The dashboard manual-done path did not recover it.
- A later direct `lisa complete-ticket` invocation succeeded.
- Completion commit `f64df75` proved that the ticket and artifacts were committable.
- The field evidence does not isolate the exact runtime guard that missed completion.

## Required trace shape

- Review is present before the phase advances to Review.
- A stopped signal follows in an inconvenient ordering.
- A Review timeout follows after about ten minutes.
- Plugin/session state is reloaded.
- A later manual command result confirms authoritative completion.
- The expected aggregate evidence is exactly one completion request.
- The expected terminal evidence is exactly one Confirmed transition.
- No finish-up action is allowed while the Review artifact is already present.
- No second request is allowed while the first transaction is pending or confirmed.

## Completion module location

- `crates/lisa-core/src/completion.rs` owns the pure completion domain.
- `crates/lisa-core/src/lib.rs` publicly exposes it as `lisa_core::completion`.
- The module has no scheduler, Zellij, WASM, filesystem, process, or clock dependency.
- Its unit tests are colocated inside the production source file.
- A public integration test can exercise only the published contract.
- The workspace currently has no `crates/lisa-core/tests` directory.
- Adding an integration test keeps proof code separate from settled production code.

## Durable input contract

- `DurableCompletionInputs` contains artifact admission and Review disposition.
- `CurrentLeaseArtifactAdmission` carries attempt and completion identities.
- Admission is a positive fact supplied only after the adapter verifies current authority.
- `ReviewDisposition::Pass` is the only disposition eligible for completion.
- Missing admission, Block, and Invalid dispositions reconcile to no request.
- Phase is not part of `DurableCompletionInputs`.
- The future adapter determines when the admitted Review facts apply to completion.
- The deterministic fixture must therefore retain phase observation as harness state.

## Aggregate state contract

- `CompletionState::Eligible` can accept a request.
- `Requested` records that the request effect has already been emitted.
- `CommandInFlight` requires a correlation ID.
- `Rejected` retains a typed reason and retryability.
- `Confirmed` is the authoritative terminal success state.
- The state enum itself prevents an in-flight command without correlation.
- The test can retain aggregate state across the synthetic reload event.

## Reducer contract

- `reduce` accepts owned state and event values.
- Eligible plus Request produces Requested and one LaunchCompletion effect.
- Requested plus CommandLaunched produces CommandInFlight and no effect.
- Matching CommandInFlight plus CommandSucceeded produces Confirmed and no effect.
- Duplicate Request in Requested, CommandInFlight, or Confirmed is rejected.
- Mismatched results return a typed correlation rejection.
- The reducer performs no external action.
- A fixture driver must count effects and confirmations explicitly.

## Reconciliation contract

- `reconcile` re-derives completion obligation from durable facts and state.
- Admitted Pass plus Eligible returns one LaunchCompletion decision.
- Requested and Confirmed suppress additional work.
- CommandInFlight returns a correlation-tagged actionable result, not a retry effect.
- Retryable rejection may return a fresh effect.
- Action-required rejection returns no effect.
- Repeated reconciliation is the level-triggered behavior under test.
- Reload can call reconciliation again without relying on a one-time artifact edge.

## Finish-up boundary

- Finish-up prompting is a plugin adapter concern, not a core effect command.
- `EffectCommand` has only the isolated completion launch variant.
- The fixture can model timeout policy as an observable counter.
- Artifact presence is sufficient for this trace to suppress the synthetic finish-up action.
- The test must not imply that lisa-core itself sends or suppresses pane prompts.
- The later E-042 adapter regression will test the real runtime boundary.

## Naive edge-triggered behavior

- A naive implementation can request only when the artifact-created edge arrives.
- If the artifact edge occurs before phase Review, that implementation ignores it.
- The later phase edge does not revisit the durable artifact fact.
- Stop does not repair the missed edge.
- Timeout injects finish-up because no request is pending.
- Reload also loses the unconsumed edge.
- The recorded order therefore strands the ticket until external manual recovery.
- This is the comparison behavior the regression fixture must reject.

## Test conventions and verification

- Workspace integration tests use Rust's standard `#[test]` harness.
- Public types can be imported from `lisa_core::completion` and `disposition`.
- Exact equality assertions are common throughout completion tests.
- A local recorded-event enum can preserve event names and ordering visibly.
- A local observation struct can count requests, confirmations, finish-ups, and re-requests.
- The naive model can return an observation without touching production code.
- The aggregate driver can use only public reducer/reconciler APIs.
- `cargo test -p lisa-core --test recorded_livelock_regression` is the focused gate.
- `cargo test --workspace` is the acceptance-wide regression gate.
- `cargo fmt --all -- --check` and clippy protect test quality.

## Repository state and ownership

- Lisa has modified the active ticket and provenance as orchestration state.
- `crates/lisa-plugin/docs/` is unrelated and untracked.
- Those paths must remain outside the ticket source commit.
- Phase artifacts belong only in this attempt-private work directory.
- The owned source unit will be the new integration test file only.
- The source unit must be committed with exact `lisa commit-ticket --include` ownership.
- The ordinary index must remain untouched.

## Constraints and assumptions

- The test must not change production completion semantics.
- The test must not add plugin dependencies to lisa-core.
- The test must not claim end-to-end command or commit execution.
- The manual-result event is modeled as the matching successful command result.
- Reload preserves aggregate transaction state in this pure-contract proof.
- Durable persistence and idempotent journal recovery are follow-on runtime concerns.
- The fixture must use provider-neutral events despite originating from a Codex incident.
- Exactly-one claims apply to observed domain request and Confirmed transition.

