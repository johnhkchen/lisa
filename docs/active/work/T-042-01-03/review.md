# Review: level-triggered eligibility reconciliation

## Disposition

Pass.

T-042-01-03 adds an explicit level-triggered Review completion path to the
real plugin adapter.

Every poll now re-derives the obligation from current-attempt `review.md`
admission, E-040's structured disposition, and honest adapter aggregate state.

The plugin-load boundary invokes the same authority-gated collector.

Eligible Pass produces one completion launch effect.

Requested/pending, durable Confirmed/Done, missing evidence, and Block produce
none.

Review timeout delivery no longer injects a generic finish-up prompt when the
current attempt's Review is already admitted or completion is pending.

All focused, package, workspace, formatting, native lint, WASM lint, and
release WASM build gates pass.

No blocking issue remains.

## Source commit

Implementation commit:

```text
27bddc142fd269418fb5dc463f36637fe0a0b5ef
```

Message:

```text
fix(plugin): reconcile Review completion eligibility
```

The commit was created through `lisa commit-ticket` with the exact include:

```text
crates/lisa-plugin/src/lib.rs
```

`git diff-tree` confirms this is the only committed path.

No source file was created or deleted.

No core, CLI, UI, manifest, lockfile, ticket, provenance, or shared artifact
path was included.

## Completion adapter changes

The private adapter vocabulary now includes:

- `CompletionInput::Reconcile` carrying ticket ID and AttemptLease;
- `CompletionSource::Reconcile` retained in PendingCompletion diagnostics.

Callers cannot supply a disposition or choose aggregate eligibility.

The adapter derives those facts internally.

`dispatch_completion` remains the only caller of
`execute_completion_effect`.

Its Reconcile branch calls E-041's pure `reconcile` API.

Its Artifact, Stopped, Idle, ObservedDone, and Manual branches continue to
construct typed Request events and call E-041's pure reducer.

Both branches converge on one common optional EffectCommand and one executor
call site.

The structural single-gateway regression passes.

T-042-01-04's newly committed correlated rejection rendering is preserved.

## Durable input derivation

`review_completion_inputs` admits private `review.md` through the supplied
exact current lease.

It creates CurrentLeaseArtifactAdmission only after successful admission.

The admission binds the lease generation as AttemptId and ticket ID as
CompletionId.

The helper separately admits `review-disposition.json` through the same lease.

It parses the canonical admitted document with the existing structured E-040
parser.

Pass and Block remain typed ReviewDisposition values.

Missing, stale, unreadable, or invalid evidence fails closed.

No private path is trusted from directory naming alone.

No lease is fabricated.

Admission errors are visible in activity and cannot produce a launch effect.

## Aggregate state derivation

`reconciliation_state` maps the facts available to this adapter:

- pending map membership -> Requested;
- durable DAG `phase: done` plus `status: done` -> Confirmed;
- otherwise -> Eligible.

Pending takes precedence over Done, consistent with the existing pending mask
and command-result verification boundary.

The plugin does not pretend to reconstruct CommandInFlight or Rejected without
retained correlation/rejection state.

Event-driven completion deliberately keeps its prior mapping of pending to
Requested and everything else to Eligible.

That distinction matters for ObservedDone: external Done frontmatter still
enters the isolated transaction instead of being accepted as confirmation.

The existing Done-between-polls regression passes and guards this compatibility
point.

## Level-triggered collector

`reconcile_review_completions` snapshots current candidates from thread state.

Each candidate must be non-completed and carry the exact current lease.

The lease ticket ID must match the thread map key.

Review observed in either thread or DAG state makes the ticket a candidate.

Done remains observable briefly so Confirmed suppression can occur before
normal audit removes the thread.

Every candidate enters the typed Reconcile adapter input.

The collector performs no direct command launch.

Repeated calls while the pending map derives Requested return no second effect.

Blocked dispositions likewise remain non-eligible on every call.

## Poll and load wiring

`poll_tick` invokes level-triggered reconciliation after artifact and idle
phase advancement.

It runs before transition timeout and Review timeout policy.

An artifact present before Implement-to-Review is therefore visible after the
phase observation even if the original completion edge did not succeed.

`State::load` invokes the same collector after initial DAG construction.

A completely fresh State normally has no restored thread/current lease and the
collector safely does nothing.

This ticket intentionally does not infer authority from stale private
directories or pane marker filenames.

Durable command/correlation journal and broader restart reconstruction remain
the explicitly separate S-042-02 boundary.

Any future authority-preserving restoration path already has the same
reconciliation entry point available at load.

## Finish-up suppression

`review_completion_suppresses_finish_up` runs before any timeout follow-up or
pane I/O.

A pending completion suppresses the action immediately.

Otherwise it requires the thread's exact current lease and re-admits
`review.md`.

An admitted Review suppresses the generic prompt regardless of Pass or Block.

That policy is intentional: Block needs its actionable disposition handled,
not a false instruction to write an artifact that already exists.

An admission error logs the actionable error and also suppresses the misleading
prompt.

A genuinely absent Review retains every existing deadline, wind-down,
awaiting-human, and prompt behavior.

Suppressed actions do not populate `finish_up_sent` or mutate activity/phase
clocks.

## Acceptance regression

The new test is
`poll_then_reload_reconciles_review_once_without_finish_up`.

It uses the real State adapter and temporary ticket/private work directories.

The current leased thread begins in Implement.

Private `review.md` exists before the Implement-to-Review observation.

The structured disposition is deliberately absent on that first edge, so the
old event opportunity produces no pending command.

After exact Pass appears, the level-triggered poll collector derives the
obligation and records one exact LaunchCompletion effect.

The test checks the attempt generation, ticket completion identity, Reconcile
source, and pending map entry.

It ages the thread beyond Review timeout and wind-down, then proves no
FinishUpPromptSent event or sent marker exists.

A second collector call simulates reload observation and proves Requested keeps
the effect count exactly one.

The fixture then reconstructs durable Done without pending state and proves
Confirmed adds no effect.

Finally it restores Review with an explicit Block disposition and proves both
zero additional effects and zero finish-up prompts.

The test fails if durable reconciliation, pending/confirmed suppression, Block
gating, or timeout suppression is removed.

## Verification results

Focused acceptance regression:

```text
cargo test -p lisa-plugin --lib \
  poll_then_reload_reconciles_review_once_without_finish_up --no-fail-fast
```

Passed: 1; failed: 0.

Existing external Done compatibility regression:

```text
cargo test -p lisa-plugin --lib test_done_ticket_detected_between_polls
```

Passed: 1; failed: 0.

Plugin suite:

```text
cargo test -p lisa-plugin --lib --no-fail-fast
```

Passed: 348; failed: 0.

Workspace suite:

```text
cargo test --workspace --no-fail-fast
```

All executed tests passed; the existing real-Zellij environment test remained
ignored by its declared contract.

Formatting and native lint:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Both passed.

WASM lint and optimized build:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Both passed.

`git diff --check` passed.

## Repository preservation

The source unit was committed only through Lisa's isolated transaction.

The ordinary Git index is empty.

The ticket-owned source path is clean after commit.

No ordinary add, broad add, or ordinary commit was used.

During implementation an overlapping T-042-01-04 modification appeared in the
same source file.

Work paused until that ticket committed `e322a75`, its focused test passed, and
the file became clean.

This ticket then layered on the committed interface and did not absorb another
attempt's dirty bytes.

Lisa-managed provenance/frontmatter, unrelated T-042-02-02 work, shared
published artifacts, and untracked plugin documentation remain untouched.

## Open concerns and limitations

No blocking concern was found.

The load call is authority-gated and does not independently reconstruct a
fresh plugin process's threads, leases, pending commands, or correlations.

Inventing those facts here would violate attempt fencing and expand this task
into durable journal/restart work.

The implemented boundary still ensures every poll is level-triggered and any
honestly restored load state uses the same decision path.

Repeated Review/disposition admission republishes canonical bytes atomically
at the poll interval.

That cost already exists in artifact scanning and remains bounded.

An invalid or missing disposition reconciles silently to no effect after its
existing edge/admission diagnostics; richer retained rejected-state rendering
belongs to the adjacent correlation/UI work.

No scheduler rewrite, lease-policy change, or core-domain change was made.

## Critical issues requiring human attention

None.

## Human review focus

A reviewer should verify that the distinction between reconciliation Confirmed
and event-driven ObservedDone preserves transaction safety, and that the
authority-safe load limitation matches the intended separation from durable
restart reconstruction.

Review is complete. This attempt remains on T-042-01-03 for Lisa to admit the
Review artifacts, prepare the completion commit, and release the seat only
after confirmation.
