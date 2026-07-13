# Research: level-triggered eligibility reconciliation

## Assignment boundary

Ticket T-042-01-03 belongs to story S-042-01, the plugin completion
effect-adapter and reconciliation slice.

The ticket starts in Research and requires the complete RDSPI sequence through
Review in one attempt.

The acceptance criterion is a plugin-adapter regression rather than a new
completion-domain contract.

It names a current attempt whose private Review already exists before the
ticket crosses from Implement to Review.

It also names polling, reload, exact-one request behavior, pending and
confirmed suppression, blocked disposition suppression, and the absence of a
second finish-up injection.

The attempt-private artifact directory is
`.lisa/attempts/T-042-01-03/1/work/`.

Lisa, rather than this agent, owns ticket frontmatter transitions and final
publication of admitted phase artifacts.

## Story and predecessor context

S-042-01 owns the plugin adapter that consumes E-041's pure completion domain.

The story explicitly keeps the E-041 reducer read-only.

T-042-01-01 introduced the typed adapter seam for Artifact and Stopped
completion sources.

T-042-01-02 routed Idle, ObservedDone, and Manual through the same typed seam
and removed the legacy boolean completion gateway.

T-042-01-02 deliberately left level-triggered poll/load eligibility to this
ticket.

Later S-042-01 tickets own UI rendering and other adapter concerns.

S-042-02 owns durable completion journaling and replay idempotency; no durable
journal is present in the plugin state today.

The field note records the original failure: Review existed, the ticket became
Review, no completion transaction appeared, and a later timeout injected a
finish-up prompt over the complete artifact.

## Pure completion contract

The completion domain lives in `crates/lisa-core/src/completion.rs`.

`CurrentLeaseArtifactAdmission` records an admitted attempt ID and completion
ID.

`DurableCompletionInputs` combines optional current-lease artifact admission
with a parsed `ReviewDisposition`.

`CompletionState` has Eligible, Requested, CommandInFlight, Rejected, and
Confirmed variants.

Requested represents an accepted request whose launch effect was emitted.

Confirmed represents the authoritative successful terminal state.

`EffectCommand::LaunchCompletion` contains an AttemptId and CompletionId.

`reduce` folds explicit typed events through the aggregate.

An Eligible Request returns Requested plus one LaunchCompletion effect.

Requests received in Requested or Confirmed are rejected as AlreadyPending.

`reconcile` is the domain's level-triggered API.

It borrows durable inputs and the current aggregate state.

Admitted Pass plus Eligible returns a LaunchCompletion effect.

Missing admission, Block, or Invalid disposition returns no effect.

Requested and Confirmed return no effect even when durable eligibility remains.

CommandInFlight returns a correlation-bearing actionable reconciliation
result.

The plugin currently imports `reduce` but does not import or call `reconcile`.

## Plugin completion adapter

The plugin implementation is concentrated in `crates/lisa-plugin/src/lib.rs`.

`CompletionInput` is the exhaustive scheduler/operator input enum.

Its variants are Artifact, Stopped, Idle, ObservedDone, and Manual.

`CompletionSource` preserves the diagnostic origin stored in a pending
transaction.

`CompletionAuthority` distinguishes an Attempt lease from Operator authority.

`dispatch_completion` maps a CompletionInput to ticket, source, authority, and
optional Review-admission lease.

Artifact, Stopped, and Idle carry Review-admission leases.

ObservedDone and Manual do not repeat Review admission.

For a leased Review source, `dispatch_completion` calls
`admit_passing_review`.

It maps membership in `pending_completions` to Requested and absence to
Eligible.

It constructs `CompletionEvent::Request` and calls `reduce`.

Only the reducer's returned effect can reach `execute_completion_effect`.

`execute_completion_effect` is the sole completion host-command launch
boundary.

It checks effect identity, duplicate pending state, current authority,
dependencies, and ticket lookup before inserting PendingCompletion.

The pending record preserves prior phase/status, source, and authority.

The native test build records accepted effects in
`launched_completion_effects`.

## Artifact admission

`attempt_work_dir` resolves a lease to the private
`.lisa/attempts/{ticket}/{attempt}/work` directory in production.

Direct native fixtures use a deterministic work-directory fallback.

`admit_artifact` is the attempt-fencing publication boundary.

A supplied lease must identify the ticket and equal the current lease.

The private artifact must be a file.

Admission copies its bytes atomically into the canonical shared work
directory.

The historical unleased branch reads canonical artifacts only when no current
authority exists.

`admit_passing_review` admits `review-disposition.json`, then parses the
canonical document through E-040's structured disposition parser.

Pass returns true.

Block logs its reason and returns false.

Invalid or missing disposition logs an error and returns false.

Review eligibility therefore depends on two separate admissions today:
`review.md` at the caller and the disposition inside dispatch.

## Artifact phase scanning

`check_artifact_advances` scans running threads in a fixpoint loop.

Each iteration snapshots ticket ID, thread phase, and optional attempt lease.

Research, Design, Structure, Plan, and Review use their phase artifact names.

Implement uses `review.md` as the phase-completion edge because `progress.md`
is a living artifact.

The method admits the selected artifact before taking a phase edge.

Non-Done edges update ticket frontmatter and the in-memory thread phase.

When Review's next phase is Done, it dispatches an Artifact input if the
thread has a lease.

Because the loop repeats, a Review file observed during Implement can advance
the ticket to Review and be inspected again as Review in the same method call.

That incidental second inspection currently provides some retry behavior.

The completion obligation is nevertheless embedded in the artifact-edge
scanner rather than represented by an explicit reconciliation pass.

## Poll ordering

`poll_tick` ingests liveness and assignment signals before artifact processing.

It calls `check_artifact_advances`, then `check_idle_signals`, then transition,
error, and timeout handlers.

It calls `check_review_timeouts` after those phase and completion edges.

It rebuilds the DAG later in the poll.

After rebuilding, it dispatches ObservedDone for running threads whose scanned
ticket phase is Done.

The pending map masks scanned Done frontmatter during a live transaction.

The poll later reconciles thread phases, sweeps stale slots, audits threads,
and schedules ready work.

There is no named pass that re-derives passing-Review completion obligations
from durable inputs on every poll.

## Plugin load

`State::load` parses configuration and converts configured paths to the WASI
host mount.

It initializes signal, attempt, ledger, and provider-usage directories.

It subscribes to Zellij events and requests permissions.

It scans tickets, runs diagnostics, builds the DAG, snapshots phases, and logs
PluginStarted.

It does not call artifact advancement or completion reconciliation.

Fresh plugin state has no threads, current leases, pending transactions, or
discovered pane slots at the end of `load`.

Lease authority is minted later during scheduling and stamped into thread and
slot state.

The scheduler also writes a per-pane lease marker for native hooks, but load
does not currently reconstruct scheduler state from those markers.

Durable journal/restart reconstruction is outside this ticket's story.

## Review timeout behavior

`check_review_timeouts` delegates timing decisions to `DeadlineEvaluator`.

Its inputs include thread status and phase, prior prompt state, human-waiting
state, phase-change time, and activity time.

The deadline input does not contain Review artifact presence, disposition, or
completion transaction state.

Every returned action resolves the attempt work directory and sends the
adapter's finish-up follow-up into the pane.

The method then updates activity time, records the ticket in
`finish_up_sent`, and logs FinishUpPromptSent.

A passing Review can become pending earlier in the same poll and still satisfy
the current timeout evaluator's inputs.

A blocked Review can also satisfy those inputs.

Artifact existence by itself does not currently suppress the prompt.

## Existing tests

Plugin unit tests are colocated in `lib.rs`.

`test_check_artifact_advances_review_to_done` asserts one effect for an already
Review ticket and duplicate suppression when Stopped follows.

`test_codex_dag_advances_all_phases_via_artifacts` covers Review existing at
the Implement edge and observes a pending completion.

Disposition tests cover Pass, Block, missing, malformed, stale, and publication
behavior.

Review timeout tests cover elapsed time, idempotence, disabled configuration,
thread states, wind-down, and awaiting-human suppression.

No existing plugin test drives an explicit poll/load reconciliation boundary
using the core `reconcile` API.

No existing test jointly asserts exact-one completion effect and zero
finish-up injection for the recorded artifact-before-phase sequence.

## Repository constraints

The working tree contains Lisa-managed modifications to provenance and ticket
frontmatter plus unrelated ticket work and untracked plugin documentation.

Those paths are not owned by T-042-01-03 and must remain untouched.

The likely ticket-owned source surface is `crates/lisa-plugin/src/lib.rs`.

Any meaningful source unit must be committed with `lisa commit-ticket` and an
exact repository-relative include path.

The ordinary Git index must not be used.

The normal verification surface includes plugin tests, workspace tests,
formatting, Clippy, and the wasm32-wasip1 build/check boundary.
