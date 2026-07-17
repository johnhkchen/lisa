# Design: bounded park on completion failure

## Decision summary

Completion command failures will be classified at the plugin adapter boundary.

The completion journal will record every failed command observation with its
one-based count, fixed limit, full technical reason, class, and consequence.

Known operator-owned failures will retry within the fixed bound and then park.

Unknown failures will park immediately with an unstructured raw-reason remedy.

Transient failures will consume the same fixed command-attempt bound without
parking as a direct consequence of that classification.

After transient exhaustion, the generation remains in flight without further
launches until its existing absolute reconciliation deadline parks it with an
uncertain-completion ask.

Deadline expiry will use the same park helper as classified failures.

Parking will write a canonical blocking disposition, restore Review
frontmatter, set blocked status, append E-048 provenance, and release the seat.

Ordinary `lisa unblock` will therefore be the only new recovery entry point
needed.

## Retry bound

Use a private fixed `MAX_COMPLETION_FAILURES` value of two.

The value bounds failed completion command observations for one durable
in-flight generation.

The first known operator failure records failure 1/2 and relaunches the exact
idempotent generation.

The second known operator failure records failure 2/2 and parks.

An unknown failure parks on failure 1/2 because guessing and repeated churn add
no safety.

A transient failure at 1/2 relaunches the exact generation.

A transient failure at 2/2 records exhaustion but does not relaunch or park
from the transient classification.

The original absolute deadline remains unchanged throughout.

Once exhausted, reconciliation suppresses further replay before that deadline.

At deadline the ordinary uncertain-result policy parks the ticket.

This preserves the ticket's explicit “transient retries within bounds, no
park” mapping while ensuring the scheduler cannot exceed the bound.

## Option 1: count retryable Rejected rows

The smallest apparent change is to count existing rejected journal rows and
park after the count reaches a limit.

This preserves the current Requested → InFlight → Rejected loop.

It does not preserve a single absolute deadline because every fresh request
creates another deadline.

It also makes transient exhaustion difficult: a retryable Rejected state is
level-triggered and will immediately request again.

An ActionRequired state would suppress churn but recreate the unrecoverable
state this ticket removes unless it always parks.

This option is rejected.

## Option 2: keep failures inside CommandInFlight

Add journal audit rows that retain the reducer's CommandInFlight state while
recording each observed failed attempt.

Retries then replay the same idempotent generation and retain the original
deadline.

The journal aggregate can durably retain failure count, limit, and exhaustion
across plugin restarts.

A final operator-owned or unknown failure appends the existing ActionRequired
Rejected transition immediately before parking.

Transient exhaustion can remain in-flight without another launch until the
deadline.

This option makes the limit durable and enforceable at every replay boundary.

It is selected.

## Option 3: put classification in lisa-core

The core module could define Git failure classes and retry policy.

That would make classification easy to unit-test independently.

It would also introduce Git stderr vocabulary, Review disposition construction,
and parking concerns into a provider-neutral reducer.

The core currently owns completion lifecycle facts, not host adapter failure
syntax.

This option is rejected to preserve the existing domain boundary.

## Failure classes

Use a private serializable journal class vocabulary:

- operator-history-or-identity;
- operator-repository-unwritable;
- operator-stale-lock;
- transient-contention;
- unrecognized;
- deadline-expired.

Classification uses lowercase substring matching over the raw stderr detail.

The history/identity class recognizes only established Git phrases.

The repository-writable class recognizes explicit permission and read-only
diagnostics, not generic I/O wording.

The stale-lock class recognizes Lisa's completion lock and messages explicitly
naming a stale or dead lock owner.

The transient class recognizes Git index-lock contention that says another
process is running, plus narrow temporary-resource wording.

Stale-lock recognition runs before transient lock recognition.

Everything else is unrecognized.

## Remedy construction

History and identity share the ticket's required forwardable ask verbatim:

`Lisa needs a name for recording finished work. Run: git config user.name ...`

The persisted string retains Markdown backticks because status and dashboard
already display arbitrary ask text.

Repository unwritable failures receive a short sentence asking the operator to
make the repository writable and then unblock the named ticket.

Stale-lock failures receive a short sentence asking the operator to remove the
named stale completion lock and unblock the ticket.

Deadline failures receive a sentence saying Lisa could not confirm finished
work and asking the operator to check history before unblocking.

These are structured operator-owned block documents.

Unrecognized failures persist only disposition and raw reason.

The existing disposition parser converts that legacy shape to an operator ask
and marks it `unstructured: true`.

No raw technical reason is discarded.

## Journal shape

Advance the completion journal schema additively.

Add a `failure-observed` row with:

- completion and attempt identity;
- generation and correlation;
- full reason;
- stable failure class;
- failure count and failure limit;
- consequence: retry-scheduled, retry-exhausted, or park.

The fold accepts the row only for the matching in-flight aggregate.

Counts must start at one and increase by exactly one.

The limit must be positive and stable for the generation.

The row does not call the core reducer and leaves CommandInFlight unchanged.

The aggregate retains failure count, limit, and whether launch retries are
exhausted.

Requested and a different generation reset those fields.

Rejected and Confirmed preserve them for audit/restart inspection.

Older schema journals load with zero failures and no exhaustion.

## Parking transaction

Create one plugin helper for completion parks.

It takes ticket ID, generation key, correlation, technical reason, class,
structured or unstructured ask data, and optional retry progress.

It appends an ActionRequired Rejected transition first.

It atomically publishes the canonical blocking disposition.

It restores the aggregate's prior phase and sets the ticket status blocked.

It appends a ParkingTransitionType::Park row using the current attempt.

It removes pending completion state, releases the seat, removes the thread,
and rebuilds the DAG.

The action-required aggregate may continue to mask stray Done bytes, but the
ticket itself is durably Review/blocked.

After `lisa unblock`, a newly minted attempt has a different completion key.

The journal fold already permits a different key to start from rejected state.

That new Review attempt can write a fresh pass disposition and complete.

## Surface behavior

The journal reason remains the complete technical failure envelope.

Retry activity uses a plain lead sentence and places technical detail after it.

Park activity leads with the canonical ask.

The dashboard's Waiting on you row reads the canonical ask, not the journal.

Operator MarkDone feedback continues to receive a structured rejection event,
but classified failure detail will begin with the plain sentence.

## Failure safety

Journal publication remains the authority for retry count.

If a failure-observed row cannot be persisted, no retry launch occurs and the
pending command remains retained for another observation boundary.

If final rejection cannot be persisted, parking does not proceed.

If the disposition or ticket update fails after final rejection, the error is
made visible; tests focus on the normal durable path.

No successful completion semantics change.

No journal-only completion semantics change.

## Verification

Pure fixture tests cover classification and action selection.

Journal tests cover exact rows, strict counts, restart reconstruction, and
retry exhaustion.

Scheduler fixtures cover known operator retry then park, transient bounded
retry without immediate park, unrecognized immediate unstructured park, and
deadline park/unpark recovery.

Provenance assertions reuse the mixed-ledger parser and E-048 schema.

Focused plugin tests run before the full workspace suite.
