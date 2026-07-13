# Research: Durable completion journal reconstruction

## Assignment boundary

Ticket T-042-02-02 is the middle task in story S-042-02.

Its required behavior is durable persistence and reconstruction of the
completion aggregate.

The named states are Requested, CommandInFlight, and Confirmed.

The persistence must use the plugin publication module's atomic
sibling-temporary pattern.

The acceptance test must simulate a plugin restart and prove that reconstructed
state equals the state before restart, or is unambiguously rebuilt from other
durable facts.

The existing provenance ledger must remain backward-compatible.

The attempt-private artifact directory is
`.lisa/attempts/T-042-02-02/1/work/`.

Lisa owns ticket phase changes, artifact admission, final Done publication, and
seat release.

## Story context

S-042-02 owns completion durability, identity, and bounded reconciliation.

T-042-02-01 is complete and introduced the generation/idempotency identity.

T-042-02-03 follows this ticket and owns lost-result replay convergence plus a
bounded CommandInFlight reconciliation deadline.

This ticket therefore needs to retain the state and identity that the next
ticket will reconcile, without implementing the deadline policy itself.

The story explicitly separates this journal from operator `[d]one` authority
and from the later live hostile-order harness.

## Completion domain

The pure completion domain is in `crates/lisa-core/src/completion.rs`.

It has no filesystem, scheduler, Zellij, or process dependency.

`AttemptId` identifies the completion authority.

`CompletionId` identifies one completion aggregate and is populated with the
ticket ID by the production adapter.

`CorrelationId` identifies an asynchronous launch/result pair.

All three are opaque string newtypes with constructors, accessors, Display,
and value semantics.

`CompletionGenerationId` binds CompletionId, AttemptId, and a numeric
generation.

Its stable Display representation is
`v1:<ticket-hex>:<attempt-hex>:<generation>`.

The component fields are private and it currently has no parser or serde
implementation.

`CompletionState` has Eligible, Requested, CommandInFlight, Rejected, and
Confirmed variants.

CommandInFlight contains a CorrelationId.

Rejected contains a structured CompletionRejection and Retryability.

Requested contains no identity fields; identity currently remains in the
effect or plugin adapter.

`CompletionEvent::Request` carries AttemptId and CompletionId.

An Eligible Request transitions to Requested and returns exactly one
`EffectCommand::LaunchCompletion`.

`CompletionEvent::CommandLaunched` moves Requested to CommandInFlight with its
correlation.

A matching CommandSucceeded moves CommandInFlight to Confirmed.

Mismatched result correlations are rejected.

The reducer performs no I/O and does not itself retain an event history.

The core `reconcile` function is level-triggered over admitted durable inputs
plus an aggregate state.

Requested and Confirmed require no new effect.

CommandInFlight returns a correlation-bearing action-required decision; the
bounded policy for that result belongs to the next ticket.

## Generation identity from the predecessor

The plugin constructs generation 1 at its sole effect execution boundary.

The exact completion and attempt identities come from LaunchCompletion.

The command builder passes ticket ID, attempt ID, and completion generation to
`lisa complete-ticket`.

`CompleteTicketRequest` carries the typed CompletionGenerationId.

The completion transaction embeds the stable key in the commit message as an
exact `Lisa-Completion-Key:` line.

Replay with the same key searches reachable commits under the repository lock
and returns the original commit ID.

That Git metadata is durable result evidence independent of the plugin
process, but the plugin does not currently reconstruct aggregate state from it.

## Plugin adapter state

The plugin implementation is concentrated in
`crates/lisa-plugin/src/lib.rs`.

`PendingCompletion` retains prior ticket phase/status, diagnostic source, and
CompletionAuthority.

`CompletionAuthority` is either an exact AttemptLease or Operator.

`pending_completions` is a ticket-indexed HashMap on State.

Its presence masks scanned Done frontmatter until the attributed command result
is validated.

The map exists only in plugin memory and is empty in a fresh default State.

`dispatch_completion` converts scheduler/operator input into a pure-domain
decision.

`execute_completion_effect` is the only completion command-launch boundary.

It checks effect identity, current lease, dependency completion, and ticket
path before inserting PendingCompletion.

It constructs one CompletionGenerationId and builds the host command.

In production the command is launched only after pending memory has been
inserted.

The adapter does not currently fold CommandLaunched through the pure reducer.

It therefore represents a launched command with pending map membership, which
its aggregate-state helper presently maps to Requested.

`handle_completion_result` looks up the pending record by ticket context.

Stale authority removes pending state, rebuilds the DAG, and rejects the
result.

Command failure removes pending state, rebuilds the DAG, and logs a retryable
launch failure.

Success first validates exit status and commit-ID syntax.

It then removes pending state, rebuilds the DAG, and verifies durable ticket
frontmatter is both phase Done and status Done.

If durable Done cannot be verified, it restores pending state so scheduling
remains blocked.

If durable Done is verified, later code publishes completion activity,
provenance, and releases scheduler resources.

None of those transitions currently writes a completion-specific durable
journal.

## Plugin load and restart boundary

`State::load` resolves config paths into the WASI `/host` mount.

It initializes `.lisa/signals`, `.lisa/attempts`, `.lisa/provenance.jsonl`, and
provider usage paths.

It records the native project root and enclosing Git root separately.

It scans ticket frontmatter and constructs the initial DAG.

It does not currently initialize a completion journal path.

It does not currently read persisted completion state.

A fresh plugin State also has no reconstructed threads, current leases, or
PendingCompletion records.

The active T-042-01-03 change adds a level-triggered reconciliation call at
load, but correctly treats a fresh state without authority as a no-op.

Durable journal reconstruction is the missing source of completion aggregate
truth for this ticket.

## Publication mechanism

`crates/lisa-plugin/src/publication.rs` centralizes atomic publication.

`PublicationPath` contains a destination and a typed sibling temporary-name
policy.

`TemporaryName` supports nonce, attempt-plus-nonce, and exact filenames.

Resolution rejects absolute or multi-component temporary names.

`RustPublication` writes the entire body to the resolved temporary path.

It then renames the sibling temporary over the destination.

On rename failure it removes the complete temporary and preserves the prior
destination.

Its caller supplies site-specific write and publish error labels.

The helper does not create parent directories.

Payload serialization and authority remain caller responsibilities.

Tests cover replacement, cleanup, hostile paths, and complete-byte visibility.

An append-only logical journal can use this mechanism by reading the existing
complete history, appending one serialized record in memory, and atomically
publishing the new complete history.

That differs from `OpenOptions::append`: the logical history is append-only,
while filesystem visibility is an atomic whole-file replacement.

## Existing provenance ledger

Provenance schema and I/O live in `crates/lisa-core/src/provenance.rs`.

`.lisa/provenance.jsonl` currently mixes terminal execution records and
assignment-transition records.

`ProvenanceLedgerRecord` is an untagged enum so schema-v2 execution rows remain
readable beside schema-v3 assignment rows.

The provenance writer uses true filesystem append through OpenOptions.

Provenance records describe execution history and assignment failures, not
completion aggregate transitions.

Changing their required fields, enum tagging, schema interpretation, or file
contents would risk backward compatibility.

The ticket can preserve compatibility by using a separate completion journal
and making no provenance schema or parser change.

## Serialization facilities

`lisa-plugin` already depends on serde with derive and on serde_json.

No new dependency is required for compact JSONL records.

The plugin's native tests already depend on tempfile.

Rust's standard filesystem APIs and the existing publication module are
available to plugin-native tests.

WASI compilation constrains implementation to APIs supported by the plugin's
existing std filesystem usage.

## Test boundaries

Plugin tests are colocated in `lib.rs` and module-local tests exist in
`publication.rs`.

Native plugin tests can construct State directly, use temporary paths, execute
adapter methods without a real Zellij host, and inspect recorded effects.

A dedicated journal module can test serialization, atomic append behavior,
malformed/truncated input handling, and reconstruction without depending on
the large scheduler fixture.

An adapter integration test is still needed to prove transition write ordering
at request, launch, result, and simulated restart boundaries.

The acceptance criterion does not require a live provider or real Zellij
reload.

## Concurrency and repository state

Multiple tickets run on the same branch and working tree.

T-042-01-03 currently has uncommitted changes in
`crates/lisa-plugin/src/lib.rs` for level-triggered reconciliation.

That ticket and this one overlap the plugin adapter despite lacking a dependency
edge.

Its source cannot be included in this ticket's isolated commit.

The working tree also contains Lisa-managed provenance and ticket changes,
another ticket's shared work artifacts, and an unrelated untracked plugin docs
tree.

All of those paths are externally owned and must remain untouched.

This ticket may create a dedicated source module independently, but any
`lib.rs` integration must wait until the concurrent ticket has committed or be
isolated without consuming its changes.

Meaningful source units must be committed only with `lisa commit-ticket` and
exact repository-relative includes.

The ordinary Git index must remain unused.

## Constraints carried into Design

The journal needs an explicit schema and deterministic fold behavior.

Records need enough identity to reconstruct Requested, CommandInFlight, and
Confirmed without fabricating attempt authority.

The generation key must survive reload in component form or in a safely parsed
stable encoding.

Journal publication must occur before the corresponding externally visible
effect when the state claims that effect is durable.

A failed journal publication must prevent the state/effect transition from
being treated as accepted.

Confirmed must not be journalled before durable Done and correlated command
success have both been verified.

The separate provenance ledger should remain byte- and schema-compatible.

The next ticket needs correlation-bearing CommandInFlight state available after
reload for bounded reconciliation.
