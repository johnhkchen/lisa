# Structure: level-triggered eligibility reconciliation

## Change inventory

Modify one ticket-owned source file:

- `crates/lisa-plugin/src/lib.rs`

Create no production module.

Modify no `lisa-core` source.

Modify no CLI source or manifest.

Create the required RDSPI artifacts only under the assigned private attempt
directory.

Do not modify ticket phase/status frontmatter.

## Import boundary

Extend the existing `lisa_core::completion` import list in `lib.rs`.

Add aliases or direct imports for:

- `reconcile`;
- `CurrentLeaseArtifactAdmission`;
- `DurableCompletionInputs`;
- `Reconciliation`.

Retain the existing `reduce as reduce_completion` import.

Use a distinct alias such as `reconcile as reconcile_completion` so dispatcher
code makes the two core decision modes visually explicit.

No new dependency is required.

## Completion source shape

Extend private `CompletionSource` with:

```text
Reconcile
```

This value is stored in `PendingCompletion` when a level-triggered obligation
launches the transaction.

It remains private to the plugin.

Its Debug spelling participates in existing activity messages.

No UI enum change belongs in this ticket.

## Completion input shape

Extend private `CompletionInput` with:

```text
Reconcile {
    ticket_id: TicketId,
    source_lease: AttemptLease,
}
```

The input carries only scheduler evidence.

It does not carry a pre-parsed disposition or a caller-selected core state.

The adapter derives those facts itself so call sites cannot bypass admission
or choose eligibility.

Artifact, Stopped, Idle, ObservedDone, and Manual variants remain unchanged.

## Review durable-input helper

Add a private State helper near `admit_passing_review` and
`dispatch_completion`.

Suggested interface:

```text
fn review_completion_inputs(
    &mut self,
    ticket_id: &str,
    source_lease: &AttemptLease,
) -> DurableCompletionInputs
```

The helper admits `review.md` with the supplied lease.

On `Ok(true)`, populate `artifact_admission` with the lease attempt generation
and ticket completion identity.

On `Ok(false)`, leave artifact admission absent.

On admission error, log a ticket-specific error and leave admission absent.

Then admit `review-disposition.json` with the same lease.

On admitted disposition, parse the canonical path.

On missing or admission error, construct an Invalid disposition with a clear
adapter-owned reason.

Block and Pass retain their parser-produced values.

The helper performs no completion launch.

## Aggregate-state helper

Add a small private State helper near the durable-input helper.

Suggested interface:

```text
fn completion_state(&self, ticket_id: &str) -> CompletionState
```

State precedence is:

1. Pending map membership -> Requested;
2. durable DAG phase/status Done -> Confirmed;
3. otherwise -> Eligible.

Pending takes precedence so a scanned Done written by an in-flight command
cannot be treated as confirmed before result verification.

This matches `rebuild_dag`'s existing pending mask.

The helper does not create CommandInFlight or Rejected because the plugin does
not retain those states yet.

## Dispatcher organization

Keep `dispatch_completion` as the only function that calls
`execute_completion_effect`.

Split its internal decision into two branches.

### Reconcile branch

For `CompletionInput::Reconcile`:

- set source to Reconcile;
- set authority to the supplied Attempt lease;
- derive durable inputs through the helper;
- derive aggregate state through the helper;
- call `reconcile_completion`;
- map `Reconciliation::Effect` to the common optional effect;
- map None to no effect;
- log correlation-bearing actionable output if the exhaustive third variant
  is encountered.

### Event branch

For all existing variants:

- retain the existing tuple mapping;
- retain leased Review admission for Artifact, Stopped, and Idle;
- retain AttemptId construction;
- retain `CompletionEvent::Request` and `reduce_completion` behavior;
- retain typed rejection logging.

Both branches end at one common match over the optional effect.

Only `Some(effect)` calls the existing executor.

The structural test should continue to find exactly one textual executor call
inside dispatch.

## Candidate collector

Add a private State method near phase/artifact processing.

Suggested interface:

```text
fn reconcile_review_completions(&mut self)
```

Snapshot candidates into a vector before dispatching.

Each candidate contains TicketId and AttemptLease.

Candidate authority requirements:

- thread has an attempt lease;
- lease ticket matches the map key;
- lease is current in `current_leases`;
- thread is not Completed.

Candidate phase requirements:

- thread phase is Review; or
- DAG phase is Review; or
- DAG phase/status is Done for terminal suppression observation.

The collector does not inspect file existence itself.

For every candidate, call the typed Reconcile input.

The collector ignores the returned bool because effect and activity state are
already retained by the adapter.

## Poll integration

In `poll_tick`, insert a call to `reconcile_review_completions` after:

- `check_artifact_advances`;
- `check_idle_signals`.

Place it before:

- transition-signal side effects where practical;
- `check_review_timeouts` in all cases.

The critical ordering is phase observation -> durable reconciliation ->
timeout policy.

Keep the later ObservedDone reconciliation unchanged.

Repeated calls in later polls see Requested and emit no second effect.

## Load integration

At the end of successful initial DAG setup in `load`, call
`reconcile_review_completions` before PluginStarted is logged or immediately
after initialization.

The method is authority-gated and therefore normally a no-op on a completely
fresh State.

Do not scan attempt directories to infer a lease.

Do not reconstruct panes, threads, pending commands, or correlations.

The load call establishes the adapter boundary for any state restoration path
without expanding this ticket into S-042-02.

## Timeout suppression helper

Add a focused State predicate near `check_review_timeouts`.

Suggested interface:

```text
fn review_completion_suppresses_finish_up(&mut self, ticket_id: &str) -> bool
```

Return true immediately when `pending_completions` contains the ticket.

Otherwise obtain the thread's attempt lease.

Require it to be the current lease.

Call `admit_artifact(ticket_id, Some(&lease), "review.md")`.

Return true only for `Ok(true)`.

Log an admission error and return true or false according to fail-closed prompt
policy; the chosen policy is true because a present-but-unpublishable Review
needs an actionable error rather than a generic write-Review prompt.

Return false for a genuinely absent artifact.

Inside the loop over deadline actions, evaluate this predicate before adapter
resolution or pane I/O.

Suppressed actions do not bump activity, change phase clocks, populate
`finish_up_sent`, or log FinishUpPromptSent.

## Acceptance regression

Add a new `#[test]` in the completion/artifact test region of `lib.rs`.

Use a temporary ticket directory and work directory.

Create a ticket in Implement and a matching running thread.

Install the current attempt with the existing test helper.

Write private `review.md` and Pass disposition before phase advancement.

Age the thread's phase/activity clocks beyond both Review timeout and wind-down.

Drive the production artifact advance and explicit reconciliation boundary.

Assert:

- thread and disk observe Review while the commit is pending;
- exactly one recorded LaunchCompletion effect exists;
- the effect carries exact attempt and ticket identity;
- pending map contains the ticket;
- timeout evaluation emits no prompt record;
- repeated reconciliation simulating reload leaves effect count at one.

Add blocked and confirmed assertions in the same test or nearby focused tests.

For Block, use a fresh state or clear only test-owned transaction facts before
writing a block disposition.

For Confirmed, rebuild the DAG from durable Done frontmatter while retaining
the fixture thread/lease long enough to call Reconcile.

Both must leave the recorded effect vector empty or unchanged.

## Existing-test adjustments

Preserve all predecessor tests where possible.

If source assertions expect Artifact, update only assertions whose production
path is intentionally now Reconcile.

Do not loosen exact effect identity checks.

Do not remove the single-gateway structural test.

Add explicit zero-FinishUp assertions rather than relying only on
`finish_up_sent`.

## Verification boundaries

Run formatting before source commit.

Run the new focused acceptance test.

Run existing artifact, disposition, timeout, and typed-gateway focused tests.

Run the complete lisa-plugin library test suite.

Run workspace tests.

Run Clippy for lisa-plugin native/all targets as supported and wasm32-wasip1
with warnings denied.

Run the release WASM build or project `just check` when practical.

Inspect the exact isolated commit and repository status.

## Commit boundary

The meaningful source unit is the complete adapter reconciliation change in
`crates/lisa-plugin/src/lib.rs`.

Commit it once tests are green using:

```text
lisa commit-ticket --ticket-id T-042-01-03 \
  --message "fix(plugin): reconcile Review completion eligibility" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not include attempt artifacts, ticket files, provenance, unrelated work, or
untracked documentation in the source commit.
