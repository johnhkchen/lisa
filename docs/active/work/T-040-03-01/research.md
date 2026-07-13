# Research: blocking Review regression boundary

## Ticket intent

T-040-03-01 turns the T-039-06-02 field incident into a deterministic plugin
regression. The safety claim is narrow: when the current attempt has produced
`review.md` and an explicit blocking Review disposition, Lisa must retain the
assignment, avoid every Done preparation/publication effect, and keep tickets
that depend on the reviewed ticket blocked.

The ticket begins after T-040-01-03. That predecessor already changed the
production scheduler to gate automated Review completion on an explicit pass.
This ticket is therefore test-owned: it pins the complete hostile boundary in
one scenario rather than adding another production mechanism.

## Relevant source location

Scheduler implementation and native plugin tests share
`crates/lisa-plugin/src/lib.rs`. Its `#[cfg(test)]` module contains temporary
filesystem fixtures that instantiate the real `State`, `Dag`, threads, attempt
leases, agent slots, artifacts, and provenance ledger.

No separate integration-test crate exists for the plugin state machine. Native
unit tests are the established way to drive host-independent scheduler logic.
The Zellij command launcher is stubbed under test, allowing a completion
request to be observed as scheduler state without running an external process.

## Review artifact flow

`State::check_artifact_advances` polls every running thread and admits the
artifact associated with its current phase. For a Review thread, `review.md`
is the artifact at the Review-to-Done boundary.

Artifact admission is attempt-scoped. `install_current_attempt` gives the
thread and its slot a matching `AttemptLease`, records the lease in
`current_leases`, and enables tests to write authoritative private artifacts
under `.lisa/attempts`-shaped temporary paths.

`State::attempt_work_dir` identifies that private location. On admission, the
artifact bytes are copied to the configured canonical work directory only if
the supplied lease is current.

T-040-01-03 introduced `State::request_review_completion`. The helper admits
`review-disposition.json`, parses it through `lisa-core`, and exhaustively
matches `ReviewDisposition`.

A `Pass` delegates to `request_completion`. A `Block` logs the supplied reason
and returns without calling the transaction boundary. Invalid or unavailable
evidence also returns without requesting completion.

## Completion preparation boundary

`State::request_completion` does not immediately publish Done. It validates
authority and dependencies, captures prior state, inserts a
`PendingCompletion`, builds the exact native `complete-ticket` command, and
launches it.

Consequently, `pending_completions` is the deterministic in-memory observation
for “prepared to commit Done.” Absence of the ticket from this map proves that
the blocking disposition never reached the completion command boundary.

This observation is especially important for the requested historical
regression. Before T-040-01-03, the Review branch in
`check_artifact_advances` called `request_completion` unconditionally after
admitting `review.md`. A fixture containing a block disposition would still
have inserted a pending completion. An assertion that the map remains empty is
therefore known to fail against the pre-fix path.

## Done publication and provenance

Done publication is commit-gated. A successful native command result is
verified before Lisa changes the ticket/DAG, emits completion activity,
releases the slot, and appends authoritative Done provenance.

`State::ledger_path` points to the append-only JSONL provenance ledger.
`emit_provenance` writes `RunOutcome::Done` only at the successful result
publication boundary for a current lease.

A blocked Review should never reach that boundary. A test can configure the
ledger path inside its temporary directory and assert either that no ledger
was created or that parsed records contain no authoritative Done row for the
ticket. Since other fixture behavior need not create provenance, nonexistence
is the strongest and simplest expected state in this scenario.

The absence of pending completion and absence of a ledger complement each
other. The former proves no Done transaction was prepared; the latter proves
no Done provenance was published.

## Assignment state

The active assignment is represented in several coordinated places:

- `threads[ticket_id]` retains the running `Thread`;
- the thread remains in `Phase::Review`;
- `agent_slots` retains the ticket ID on its pane;
- the slot retains the current `AttemptLease`;
- `current_leases[ticket_id]` remains the same lease;
- ticket frontmatter on disk remains `status: review` and `phase: review`.

The acceptance criterion says the ticket stays assigned. The pane slot binding
is the direct scheduling assertion, while thread and lease checks guard
against partial cleanup that could make the binding misleading.

## Dependent blocking

The fixture DAG can contain two tickets: a Review ticket and a ready ticket
whose `depends_on` lists the Review ticket. `Dag::all_dependencies_done`
computes readiness from ticket status in the DAG.

Because blocking Review does not publish Done, the dependent must continue to
return false from that query. The dependent should also remain absent from the
running thread map, demonstrating that the poll did not schedule downstream
work as a side effect.

## Existing adjacent coverage

`review_disposition_gates_artifact_completion_and_dependents` already exercises
block, pass, and invalid documents in a table. Its block case checks the
pending map, thread phase/status, slot assignment, lease, on-disk Review state,
dependent readiness, admitted evidence, and visible reason.

`test_auto_complete_review_block_retains_assignment_with_visible_reason`
covers the stopped-session caller directly. Other positive tests write a pass
disposition and prove that the existing pending transaction behavior remains.

The table test does not explicitly name T-039-06-02 and does not configure or
assert the provenance ledger. The ticket asks for a deterministic historical
regression with all named effects together, so a dedicated test adds distinct
value despite overlapping individual assertions.

## Historical failure discriminator

An assertion limited to “ticket file is not Done” would not necessarily fail
against the old code, because the isolated completion command is asynchronous
and the native test host does not publish its successful result during the
same call.

An assertion limited to “no Done provenance” has the same weakness: provenance
is emitted after command success, not when the request is prepared.

The load-bearing pre-fix discriminator is therefore that the reviewed ticket
is absent from `pending_completions`. Combining it with the durable state,
provenance, assignment, and dependency assertions captures both the historical
fault and the full desired safety boundary.

## Repository and workflow constraints

The worktree contains Lisa-managed ticket/provenance edits and unrelated work
from another ticket. They must remain untouched.

The expected ticket-owned source unit is only
`crates/lisa-plugin/src/lib.rs`. Phase artifacts belong only in this attempt's
private work directory until Lisa admits them.

Any source edit must be committed through `lisa commit-ticket` with the exact
repository-relative include. Ordinary `git add` and `git commit` are forbidden.
The ticket frontmatter phase and status are Lisa-managed and must not be edited.

## Verification surfaces

The smallest verification is a filtered native plugin test using the new
regression name. The complete `lisa-plugin` library suite checks interaction
with the monolithic scheduler test module. `cargo test --workspace` checks all
native crates, and the WASM target check ensures the shared production source
still compiles for its deployed target.

No browser, live Zellij pane, network service, or filesystem outside a
temporary fixture is required to reproduce this boundary.
