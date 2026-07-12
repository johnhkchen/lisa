# Research: cross-policy deadline regression

## Ticket and story boundary

- Ticket `T-039-04-03` is the final ticket in story `S-039-04`.
- The story covers six deadline families in the plugin.
- They are acknowledgement, transition, review, health, session, and stale-thread.
- The predecessor centralized their eligibility decisions in `deadline.rs`.
- This ticket asks only for regression tests.
- It does not ask to change a timeout, action, exemption, or production interface.
- The acceptance criterion explicitly names deadline actions and exemptions.
- The suite and Clippy must remain green.
- Live-seat timing evidence is outside this story and deferred to `S-039-06`.

## Relevant source boundary

- `crates/lisa-plugin/src/deadline.rs` owns pure deadline evaluation.
- `crates/lisa-plugin/src/lib.rs` owns state-machine effects for emitted actions.
- `deadline.rs` is private to the plugin crate.
- Its types use `pub(crate)` visibility where `lib.rs` needs them.
- Its inline test module can inspect every private input, action, and field.
- `lib.rs` has broader characterization tests for each deadline path.
- No separate integration-test crate is needed to access evaluator internals.
- No public API or serialized format is involved.

## Evaluator clock model

- `Clock::now` returns `SystemTime`.
- `SystemClock` supplies the production wall clock.
- Tests use `FixedClock` and the `evaluator(now_secs)` helper.
- `DeadlineEvaluator::new` samples its clock once.
- All calculations in an evaluator instance use that sampled instant.
- The local `elapsed` helper saturates future timestamps to zero.
- Acknowledgement compares an absolute deadline inclusively.
- Review, health, session, and stale thresholds are inclusive.
- Transition converts elapsed time to whole seconds and uses strict `>`.
- These boundary differences are existing behavior.

## Acknowledgement policy

- `acknowledgements` accepts typed candidates with pane, state, and deadline.
- It emits `AcknowledgementAction` for expired candidates.
- The action preserves pane and captured state.
- Candidate input order is preserved.
- It has no active-session or awaiting-human input.
- Consequently neither condition is an evaluator exemption.
- `lib.rs` revalidates captured assignment state before applying an action.
- Assignment variant determines retry, recovery, or terminal effects.
- Existing characterization proves awaiting-human does not prevent recovery.

## Transition policy

- `transitions` accepts slot state and a shared `TransitionPolicy` value.
- `WaitingForExit` can emit `ExitReady`.
- `WaitingForStop` can emit `StopTimedOut`.
- `WaitingForClear` can emit `ClearTimedOut`.
- Exit eligibility depends only on its started clock and exit grace.
- Recent activity does not exempt an expired exit transition.
- Awaiting-human does not exempt an expired exit transition.
- Stop and clear additionally require wind-down quietness.
- Awaiting-human exempts stop and clear.
- Missing `started` exempts every transition state.
- Each action carries only the identity needed by its state-layer effect.

## Review policy

- `reviews` receives a timeout and wind-down duration.
- A zero timeout disables the policy.
- Only running threads in Review are eligible.
- `already_prompted` is an idempotence exemption.
- `awaiting_human` is an exemption.
- Recent phase change is an exemption.
- Recent activity is an independent exemption.
- Eligible input emits `ReviewAction` with ticket and pane identity.
- The state layer sends the provider-specific finish-up prompt.

## Health policy

- `health` is observational and emits one observation per candidate.
- Failed thread status maps immediately to `Failed`.
- A running thread at or beyond the activity threshold maps to `Stuck`.
- Other candidates map to `Healthy`.
- The input intentionally has no awaiting-human field.
- Recent activity prevents `Stuck` by producing `Healthy`.
- Awaiting-human is not an exemption from an observational stuck result.
- The state layer owns cache updates and transition logging.
- Health produces no destructive reclaim action.

## Session policy

- `sessions` checks both global and per-phase budgets.
- A nonzero expired global budget takes precedence.
- Otherwise a nonzero expired phase budget can trigger the policy.
- Non-running threads are exempt.
- Pending-completion threads are exempt.
- If no budget expires, no action is emitted.
- Expiry constructs a `SessionDeadline` with ticket, pane, elapsed, and phase.
- Hard silence plus no awaiting-human emits `Reclaim`.
- Recent activity converts the same budget expiry into `Warn`.
- Awaiting-human also converts the same budget expiry into `Warn`.
- These are advisory-versus-destructive distinctions, not total suppression.

## Stale-thread policy

- `stale` uses one hard activity timeout.
- Only running threads are eligible.
- Pending-completion threads are exempt.
- Awaiting-human threads are exempt.
- Recent activity is exempt.
- Eligible input emits `StaleAction` with ticket and pane identity.
- The state layer turns it into a fenced stale-thread reclaim outcome.

## Existing test coverage

- `fixed_clock_drives_all_six_policies_at_their_boundaries` covers one firing case per family.
- It asserts exact transition exit action.
- Other policies are mostly asserted by length or broad variant matching.
- `policy_specific_exemptions_are_preserved` covers awaiting-human behavior.
- It covers transition stop suppression.
- It covers review suppression.
- It covers session conversion to warning.
- It covers stale suppression.
- It does not present the policies as one comparative contract.
- It does not cover transition clear and exit in the same exemption matrix.
- It does not compare recent-activity behavior across transition, review, health,
  session, and stale policies.
- It does not assert every emitted action payload exactly.

## Existing state-layer characterization

- Six `characterizes_*` tests live in `lib.rs`.
- They predate the evaluator extraction and remained unchanged through it.
- They prove state effects after evaluator actions are applied.
- Acknowledgement characterization proves fresh-attempt recovery.
- Transition characterization proves exit action and clear exemptions.
- Review characterization proves finish-up action and exemptions.
- Health characterization proves awaiting-human remains observationally stuck.
- Session characterization proves warning retention and fenced timeout reclaim.
- Stale characterization proves suppression and fenced stale reclaim.
- Those tests each construct independent state fixtures.
- The new ticket calls specifically for cross-policy regression coverage.

## Action distinctions visible in types

- Acknowledgement emits a generic captured-state action.
- Transition has three named action variants.
- Review has a dedicated action struct.
- Health returns `HealthObservation`, not a command.
- Session distinguishes `Warn` and `Reclaim` variants.
- Stale has a dedicated reclaim candidate struct.
- Ticket identity is absent from transition stop action.
- Optional ticket identity is retained by transition exit and clear actions.
- Session action carries elapsed seconds and phase.
- These different payload shapes prevent a single generic timeout action today.

## Exemption distinctions visible in inputs

- Recent activity is irrelevant to acknowledgement and transition exit.
- Recent activity suppresses transition stop and clear.
- Recent activity suppresses review prompting.
- Recent activity makes health healthy rather than stuck.
- Recent activity converts session reclaim to warning.
- Recent activity suppresses stale reclaim.
- Awaiting-human is irrelevant to acknowledgement and transition exit.
- Awaiting-human suppresses transition stop and clear.
- Awaiting-human suppresses review prompting.
- Awaiting-human is deliberately absent from health.
- Awaiting-human converts session reclaim to warning.
- Awaiting-human suppresses stale reclaim.

## Repository and workflow constraints

- The worktree already contains Lisa-managed ticket and provenance changes.
- They are not ticket-owned source changes.
- Phase artifacts belong only in this attempt-private work directory.
- Lisa publishes admitted artifacts later.
- Ticket frontmatter phase and status must not be edited by this agent.
- Any test source change must use `lisa commit-ticket`.
- The command must receive exact repository-relative include paths.
- Ordinary `git add` and `git commit` are prohibited for ticket work.
- Review must leave ticket-owned source clean and unstaged.

## Verification surfaces

- Focused evaluator tests run under the native `lisa-plugin` target.
- The complete plugin suite exercises evaluator integration and state effects.
- The workspace suite checks cross-crate regressions.
- Clippy is named explicitly by the ticket acceptance criterion.
- `just check` adds the `wasm32-wasip1` plugin check and workspace tests.
- `cargo fmt --all -- --check` can verify formatting without mutation.
- `git diff --check` can detect whitespace errors.
- The ignored real-Zellij test is environment-gated and unrelated to pure policy evaluation.

## Observed scope conclusion

- The ticket-owned implementation surface is test code.
- The narrowest location is the inline test module in `deadline.rs`.
- That location has deterministic time and direct access to exact action values.
- Existing `lib.rs` characterization remains the state-effect safety net.
- Production evaluator and state-machine code need no behavioral change.
