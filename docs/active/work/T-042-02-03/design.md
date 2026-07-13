# Design: bounded reconciliation replay convergence

## Goals

Reconnect the already durable completion identity to the already idempotent CLI
transaction after a plugin loses the host command result.

Keep every automatic replay inside one durable, absolute deadline.

Converge a successful same-key replay on the commit that already published
Done.

Stop automatic work in a named action-required rejection when confirmation
cannot be obtained before the deadline.

Preserve current lease fencing, exact-path commit isolation, journal atomicity,
and authoritative provenance ordering.

## Non-goals

Do not redesign the isolated Git transaction.

Do not introduce a new completion generation policy.

Do not infer completion from Done frontmatter alone.

Do not treat a reachable matching commit as confirmed inside the WASM plugin
without passing through the CLI result boundary.

Do not add a live-provider or Zellij process test.

Do not move all scheduler deadlines into `deadline.rs`.

Do not change ticket or provenance schemas.

## Option 1: immediately reject every reconstructed in-flight command

On reload, append an action-required Rejected record as soon as
CommandInFlight is observed.

This is bounded and simple.

It does not satisfy replay convergence.

The prior commit could be safely discoverable by the CLI, but the plugin would
never ask the CLI to discover it.

The operator would have to recover a normal lost-result event manually.

Rejected.

## Option 2: infer confirmation from Done bytes or Git history

On reload, inspect the ticket or search commit messages directly from the
plugin and append Confirmed.

This could avoid launching another host command.

Done bytes are intentionally masked while completion is uncertain and are not
an adequate authority by themselves.

Reimplementing the CLI's exact commit-key discovery in the plugin would split
the transaction authority and make verification rules drift.

The plugin's WASI filesystem view also differs from the host Git boundary.

Rejected.

## Option 3: replay forever with the same idempotency key

Every poll without a result could launch the same `complete-ticket` command.

The CLI would not create a duplicate commit because it discovers the key.

However, a broken host result channel or repeatedly failing host process would
produce an unbounded loop.

It would violate the ticket's explicit livelock-closed requirement.

Rejected.

## Option 4: bounded same-key replay with durable absolute deadline

Persist an absolute reconciliation deadline in CommandInFlight.

Before the deadline, reconciliation may relaunch the exact same command only
when no live invocation is already pending.

The replay uses the journal's original completion key and correlation rather
than creating a new generation.

The CLI either performs the original transaction or discovers and returns the
prior matching commit.

A successful replay result follows the existing Done verification, Confirmed
journal append, provenance, release, and dependent scheduling path.

At or after the deadline, reconciliation appends an action-required Rejected
record and stops relaunching.

Chosen.

## Durable time representation

Add an opaque `CompletionDeadline` value to `lisa-core`.

It contains Unix epoch milliseconds as `u64`.

Absolute wall-clock time is necessary because the deadline must survive plugin
restart.

`SystemTime` itself is not serialized into the core journal model.

The newtype exposes construction, its raw durable value, and an inclusive
`is_expired_at(now)` comparison.

The adapter converts `SystemTime` to epoch milliseconds at its boundary.

Sub-millisecond precision is irrelevant to a poll interval measured in
seconds.

Times before the Unix epoch fail closed to zero rather than panicking.

The production timeout is a named plugin constant.

Sixty seconds gives multiple five-second poll opportunities without leaving a
completion command indefinitely unresolved.

Tests use explicit timestamps and do not sleep.

## Core state and decisions

Extend `CompletionState::CommandInFlight` with `deadline` alongside the existing
mandatory correlation.

Extend `CompletionEvent::CommandLaunched` with that deadline.

The reducer remains the only creator of in-flight aggregate state.

Change core reconciliation to accept the current `CompletionDeadline` value.

For in-flight state before the deadline, return a named replay decision carrying
the exact correlation and deadline.

At or after the deadline, return a distinct named deadline-exceeded decision
carrying the same identity.

The pure reconciler does not mutate state or manufacture an I/O failure.

The plugin translates deadline exceeded into the existing typed
`Rejected { retryability: ActionRequired }` transition through the journal.

Eligible, Requested, Confirmed, and both Rejected behaviors remain otherwise
unchanged.

## Journal compatibility

Add `reconciliation_deadline_unix_ms` to new CommandInFlight JSON records.

Keep schema version 1 because the change is additive at the record level.

Deserialize the field as optional for histories written by T-042-02-02.

An older in-flight record without a deadline reconstructs with deadline zero.

Deadline zero is already expired for real current time.

This makes an unbounded legacy in-flight state terminate action-required on its
next eligible reconciliation instead of silently granting a fresh retry window.

Newly written records always contain the field.

Requested, Rejected, and Confirmed record shapes do not change.

An action-required Rejected aggregate continues to reconstruct through the
existing reducer path.

Action-required rejection must mask already-written Done bytes because the
command outcome remains uncertain and scheduler release still requires a
correlated confirmation.

Retryable rejection continues not to mask because an explicit failure permits
a new request.

## Initial launch deadline

When the sole effect executor accepts a new completion request, compute one
deadline from the current adapter time plus the named timeout.

Persist Requested first as today.

Persist CommandInFlight with the exact correlation and deadline before the host
call.

Store the same deadline on the live pending entry for diagnostic and timeout
handling consistency.

The host command and context remain the existing completion command.

The deadline does not need to be passed to the CLI.

## Replay adapter

Add a focused plugin method for a reconciliation replay.

It requires a current attempt lease.

It looks up the reconstructed aggregate by ticket.

It verifies that aggregate ticket, attempt, correlation, and in-flight state
all match the replay decision.

It rejects replay when a live pending command already exists.

It rebuilds argv from the durable original `CompletionGenerationId`.

It creates a live pending entry from the aggregate's prior phase/status and the
current attempt authority.

It marks that pending entry as a reconciliation replay.

It does not append another Requested or CommandInFlight record.

It crosses the same Zellij `run_command` host boundary as the initial launch.

Duplicate stop, artifact, idle, and poll observations cannot launch a second
command while that pending entry exists.

## Replay result semantics

A successful replay result uses the unchanged confirmation path.

The handler verifies a syntactically valid commit ID and durable Done ticket
frontmatter.

It appends one Confirmed record for the original key and correlation.

It then emits authoritative provenance once and releases scheduler state once.

A duplicate result arriving after pending state is removed is ignored as today.

An explicit failure from the original first invocation remains retryable and
uses the current Rejected behavior.

An explicit failure from a reconciliation replay is different: it does not
prove whether the original invocation committed before its result was lost.

Therefore it removes only the live replay pending entry, logs the failure, and
retains the original CommandInFlight journal state.

The next poll may retry under the same absolute deadline.

The failure must never append a retryable transition that grants a new
CommandInFlight deadline.

This preserves boundedness across repeated replay command failures.

## Deadline expiration semantics

At deadline, append a correlated Rejected record with
`Retryability::ActionRequired`.

Use a stable operator-visible reason naming the correlation and expired
deadline.

Remove any live pending replay entry after the durable rejection append.

Rebuild the DAG so its masking derives from the new durable aggregate.

Log the typed launch rejection through the existing activity vocabulary.

Further reconciliation returns None for action-required rejection.

Late host results find no pending entry and cannot override the terminal state.

Done bytes remain masked for this uncertain action-required outcome.

An operator or later ticket can add explicit recovery policy; this ticket does
not silently release authority.

## Test design

Core unit tests use explicit before, equal, and after deadline values.

They prove replay is offered only before the inclusive deadline boundary and
deadline exceeded is a distinct decision.

Reducer tests prove the deadline is retained with the correlation.

Journal tests prove new field round-trip and legacy missing-field recovery to
an expired deadline.

Plugin tests prove initial persistence of the absolute deadline.

A timeout test drives reconciliation at the exact deadline, observes a durable
action-required Rejected state, and repeats reconciliation to prove no relaunch.

The convergence integration test creates a real temporary Git repository.

It drives the production adapter to Requested and CommandInFlight.

It executes the adapter-built request once through
`lisa_cli::commit_transaction::complete_ticket` but deliberately withholds the
result from the plugin.

It constructs a restarted plugin state from the durable journal and current
attempt fixtures.

It presents a duplicate stop observation and reconciliation poll.

Only one replay command intent may be live.

The test executes that same durable key through `complete_ticket` again.

The CLI must return the original commit ID and create no second commit.

The replay result is delivered to the restarted plugin.

Assertions cover one completion commit, one Done ticket blob, one Confirmed
journal record, one authoritative provenance record, no pending command, and
Confirmed aggregate state.

The timeout leg uses explicit adapter time after the same lost-result setup and
asserts named action-required termination without waiting.

## Failure containment

Journal publication still precedes every in-memory authority change.

If deadline rejection cannot be persisted, in-flight state and pending fencing
remain intact.

If replay argv cannot be built, no pending entry is installed and the next poll
may retry until the original deadline.

If the journal is unhealthy, the existing fail-closed gate remains in force.

If lease identity changed, replay is refused by the existing current-attempt
boundary.

If durable Done cannot be verified after a successful result, state remains
blocked as today.

## Decision summary

The durable generation key remains the unit of idempotency.

The durable absolute deadline becomes the unit of boundedness.

The CLI remains the unit that discovers prior commits.

The journal remains the unit of reconstructed aggregate authority.

The plugin remains the only place that launches, correlates, confirms, and
releases scheduler consequences.
