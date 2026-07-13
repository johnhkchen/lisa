# Research: Gate completion on explicit pass

## Ticket boundary

T-040-01-03 connects the Review disposition contract to scheduler completion.
The predecessor T-040-01-02 already supplies a fail-closed parser and typed
outcome in `lisa-core`; this ticket owns consumption in `lisa-plugin`.

The required safety property is that an automated Review transition cannot
prepare Done unless the current attempt explicitly wrote a valid passing
disposition. A valid block must leave the ticket actionable, and invalid or
missing evidence must refuse completion.

The ticket names two primary consumers in `crates/lisa-plugin/src/lib.rs`:

- `check_artifact_advances`, which polls attempt artifacts;
- `auto_complete_review`, which reacts to a stopped Review session.

Both currently call `request_completion` without checking a disposition.

## Existing disposition model

`crates/lisa-core/src/disposition.rs` exports:

```rust
pub enum ReviewDisposition {
    Pass,
    Block { reason: String },
    Invalid { reason: String },
}
```

`parse_review_disposition(path)` reads the complete file and validates JSON.
Only `{"disposition":"pass","reason":null}` returns `Pass`.
A block requires a nonblank string reason and preserves that reason.
Missing files, malformed JSON, wrong types, unknown values, missing fields,
and contradictory reason relationships all return `Invalid` with a diagnostic.

The parser deliberately returns a domain outcome rather than a `Result`.
Callers must exhaustively distinguish approval, refusal, and untrusted input.
There is no default and no convenience conversion to a boolean.

## Attempt artifact boundary

Production agents write phase artifacts beneath the current lease directory:

```text
.lisa/attempts/{ticket}/{attempt}/work/
```

`State::attempt_work_dir` derives this path from an `AttemptLease`.
Tests without a configured attempt root use a deterministic fallback beneath
the configured work directory.

`State::admit_artifact` is the publication and authority boundary.
For a leased attempt it requires the candidate lease to match the ticket and
the current lease map. It reads the staged file and atomically publishes the
bytes into the canonical work directory.

For historical unleased fixtures, admission only succeeds when no current
lease exists and the canonical artifact already exists.

The canonical destination is:

```text
{config.work_dir}/{ticket_id}/{artifact_name}
```

The completion command later passes that canonical ticket work directory to
`lisa complete-ticket`. Consequently, a disposition that is only parsed in the
private directory would not necessarily be included in final publication.

## Artifact-driven completion

`check_artifact_advances` scans running threads until no phase advances remain.
For leased threads it carries the thread's attempt lease through artifact
admission and completion authority.

For Review, `Phase::artifact_filename()` identifies `review.md`.
Once that file is admitted, `current_phase.next()` is `Done`.
The current implementation immediately calls `request_completion` with source
`CompletionSource::Artifact` and attempt authority.

`request_completion` does not update the ticket to Done itself.
It verifies current authority, checks dependencies, captures prior state,
records `PendingCompletion`, constructs the isolated native command, and
launches the transaction. Thread, slot, and ticket remain in Review until a
verified successful result is handled.

Therefore “no request_completion” is directly observable as absence from
`pending_completions`; no host command needs to run in native tests.

## Stopped-session completion

`handle_stopped_signal` recognizes an idle slot whose assigned ticket is in
Review and whose thread is not completed. It delegates to
`auto_complete_review`.

`auto_complete_review` resolves the attempt lease from the matching pane slot
and currently calls `request_completion` with source
`CompletionSource::Stopped(pane_id)`.

This path does not itself admit `review.md`; Review may have been entered by an
earlier artifact or idle transition. It nevertheless has the same current
attempt authority and the same private artifact directory available.

## Additional automated Review edges

`check_idle_signals` contains two calls that can also request Review completion:

- Implement receives idle, moves to Review, and finds an already-written
  `review.md` in the same cycle;
- a thread already in Review receives idle with its review artifact.

These calls use `CompletionSource::Idle` and current attempt authority.
Although the acceptance criterion names the polling and stopped-session sites,
these are semantically automated Review-to-Done edges and share the same risk.

Manual completion is separate. `mark_ticket_done` calls `request_completion`
with `CompletionSource::Manual` and `CompletionAuthority::Operator`.
The ticket asks to gate Review artifacts, not remove explicit operator power.

Observed externally completed tickets are also a reconciliation path rather
than agent Review approval and are outside the named boundary.

## Dependency behavior

The DAG reads `depends_on` from ticket frontmatter.
`request_completion` already refuses a ticket whose own dependencies are not
Done. Dependents become schedulable only after their dependency ticket is
observably Done in the DAG.

If a blocked Review never enters `pending_completions`, its ticket remains
Review/open in both the thread state and on disk. A dependent ticket therefore
continues to see an unfinished dependency and remains blocked.

The strongest regression fixture should contain a Review ticket and a second
ticket depending on it. This makes the downstream scheduling invariant visible
instead of only inferring it from pending state.

## Operator-visible diagnostics

The dashboard derives recent activity from `State::activity_log`.
`log_activity` retains a bounded sequence of `ActivityEvent` values.
Existing completion refusals use `Warning` for authority problems and `Error`
for dependency or missing-ticket problems.

Both `Block` and `Invalid` carry displayable reasons.
A block reason is agent-authored actionable evidence; an invalid reason is a
parser diagnostic such as malformed JSON or a missing file.
Logging those reasons makes refusal visible without changing thread ownership.

There is no dedicated Review-block alert collection in `State` today.
Adding one would expand UI state and rendering beyond the acceptance criterion;
the activity log is the established operator-visible diagnostic surface.

## Test organization

Most scheduler unit tests are embedded in the `#[cfg(test)]` module at the end
of `crates/lisa-plugin/src/lib.rs`.
Helpers create temporary ticket directories, scan them into a `Dag`, construct
threads and slots, and call `install_current_attempt` to stamp matching leases.

`test_check_artifact_advances_review_to_done` currently writes only `review.md`
and expects a pending completion. Under the new contract it must also write a
valid passing disposition.

`test_auto_complete_review_updates_ticket_and_cleans_up` likewise currently
expects pending completion without configuring a work directory or writing a
disposition. It must be updated to represent explicit approval.

Codex lifecycle tests also write `review.md` and expect pending completion.
Every positive automated completion fixture must supply the newly required
artifact or intentionally assert refusal.

## Repository state and ownership

The worktree already contains Lisa-managed ticket/provenance changes and a
concurrent ticket's work artifacts. They are outside this ticket and must be
preserved.

The expected ticket-owned source path is only:

```text
crates/lisa-plugin/src/lib.rs
```

The existing parser module is already committed by the dependency ticket and
does not need modification. Attempt phase artifacts stay in the private work
directory and are not included in the ticket source transaction.

## Constraints

All ticket-owned source changes must be committed with `lisa commit-ticket`
using exact repository-relative includes. The ordinary Git index must remain
untouched.

The plugin test target is native and provides a no-op host command stub, so the
scheduler state machine can be tested without Zellij. Workspace tests and the
WASM check remain relevant verification surfaces for the integration.

The ticket frontmatter is Lisa-managed. Its phase and status must not be edited
by this attempt.
