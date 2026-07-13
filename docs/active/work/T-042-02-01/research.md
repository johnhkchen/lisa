# Research: Completion generation idempotency

## Assignment boundary

T-042-02-01 is the first ticket in Story S-042-02, the durability layer for
completion.

Its acceptance criterion is limited to identity and transaction idempotency:

- a completion request carries an identity bound to ticket, attempt, and
  completion generation;
- replaying the same identity returns the commit already created for it;
- replay does not create a second commit; and
- a different identity does not match that prior commit.

The later story tickets own the durable intent/result journal, reconstruction
on plugin load, deadlines, and replay convergence across hostile scheduler
orderings.

This ticket does not change Review disposition authority, dependency gates,
seat release, provenance, or the completion reducer's lifecycle transitions.

## Core completion domain

`crates/lisa-core/src/completion.rs` contains provider-neutral completion
vocabulary and the pure reducer.

The file defines string-backed identity newtypes through the `string_id!`
macro:

- `AttemptId` identifies the execution attempt claiming authority;
- `CompletionId` identifies a completion aggregate, currently populated with
  the ticket id by the plugin; and
- `CorrelationId` matches an asynchronous launch with its command result.

Each generated type owns a `String`, exposes `new` and `as_str`, implements
`Display`, and supports conversion from `String` and `&str`.

`CompletionEvent::Request` carries an `AttemptId` and `CompletionId`.
The accepted transition emits `EffectCommand::LaunchCompletion` carrying the
same pair.

The reducer deliberately performs no I/O. It neither creates commit messages
nor knows about Git, the CLI, Zellij, ticket paths, or persistence.

There is currently no completion-generation identity in lisa-core.
`CompletionId` alone is ticket-scoped in production, while `AttemptId` is
attempt-scoped. Neither distinguishes multiple completion transaction
generations for the same ticket attempt.

`crates/lisa-core/src/types.rs` separately defines `AttemptLease`.
An attempt lease contains a ticket id and a positive `u64` attempt id.
Attempt ids increase per ticket and only the full ticket/attempt pair is an
authority boundary.

The plugin converts the numeric lease attempt id to the string-backed
completion-domain `AttemptId` at the adapter boundary.

## Plugin completion adapter

`crates/lisa-plugin/src/lib.rs` owns the production effect adapter.

`CompletionInput` represents scheduler evidence admitted to the typed adapter.
The currently migrated variants are artifact and stopped inputs, both carrying
the source `AttemptLease`.

`State::dispatch_completion` validates the Review disposition, derives an
eligible or requested state, and constructs `CompletionEvent::Request` with:

- the lease attempt number converted to `AttemptId`; and
- the ticket id converted to `CompletionId`.

The reducer returns `EffectCommand::LaunchCompletion`.
`State::execute_completion_effect` is the single command-launch boundary.
It checks that effect identity agrees with the ticket and source authority,
checks the current lease and dependencies, records a `PendingCompletion`, and
launches the host command.

Legacy completion origins enter the same executor through
`State::request_completion`. Operator authority is represented with the
literal attempt identity `operator`; the follow-on operator-authority story
owns broader changes to that path.

`State::build_completion_command` currently accepts only a ticket id and ticket
file. It creates argv for:

`lisa complete-ticket --path ... --ticket-id ... --message ... --ticket-file ... --work-dir ...`

The builder uses the enclosing Git root and Git-root-relative ticket/work
paths. T-042-01-05 and T-042-01-06 established and connected this nested
monorepo behavior.

The command context contains `lisa_completion=<ticket-id>` so asynchronous
results can be attributed to the pending ticket.

No attempt or completion generation currently crosses the process boundary.
The effect executor has the attempt identity available in the effect before it
calls the builder.

The plugin has many native unit tests in the same `lib.rs` module. Existing
completion-command tests assert exact argv values, so adding command options
will require updating their expectations and test-local argv decoding.

## CLI command surface

`crates/lisa-cli/src/main.rs` defines the Clap `CompleteTicket` command.

Its current options are repository path, ticket id, message, ticket file, and
work directory. The command constructs `CompleteTicketRequest` and prints the
returned commit id on success.

`crates/lisa-cli/src/lib.rs` exports `commit_transaction` as a small library
surface. The plugin uses this only as a dev-dependency for the connected nested
repository regression.

The CLI help-surface integration test verifies that `complete-ticket` exists,
but it does not enumerate all options.

## Completion transaction

`crates/lisa-cli/src/commit_transaction.rs` implements both ticket source
commits and final completion commits.

`CompleteTicketRequest` currently carries:

- `repo_root`;
- `ticket_id`;
- `message`;
- `ticket_file`; and
- `work_dir`.

It has no idempotency field.

`complete_ticket` discovers the enclosing repository, normalizes ticket and
work paths to Git-root-relative paths, validates the path shapes, reads the
original ticket bytes, and prepares Done frontmatter.

It then delegates to `commit_ticket` with exact ticket/work include paths.
If the transaction fails, it restores the ticket's original bytes.

`commit_ticket` validates input, discovers the repository, acquires a
repository-scoped transaction lock, reserves an alternate Git index, runs the
transaction body, cleans the index, and releases the lock.

The body snapshots the ordinary index, reads HEAD into the alternate index,
stages only exact includes, rejects overlap with ordinary staged paths, writes
a tree, creates a commit with `git commit-tree`, advances HEAD with an
old-object compare-and-swap, reconciles committed paths in the ordinary index,
and verifies the ordinary staged snapshot is unchanged.

The transaction result exposes the created commit id and committed paths.
It retains the previous commit id privately for compensating rollback.

## Existing already-Done behavior

Before mutating frontmatter, `complete_ticket` checks whether the ticket bytes
already contain both Done fields.

If the ticket and work include paths have no worktree/index changes, it returns
the current HEAD and an empty committed-path list without creating a commit.

The test `already_committed_done_ticket_returns_verified_head_without_new_commit`
pins that behavior.

This shortcut is state-based, not request-identity-based. It does not prove
that current HEAD is the completion commit for the replaying request. If an
unrelated commit follows completion, the shortcut returns that unrelated HEAD.
It also cannot distinguish a different completion identity.

## Commit discoverability

Completion commits currently use only the caller-provided message, normally
`Complete <ticket-id>`.

The commit object contains its tree, parent, author/committer metadata, and
message. No structured completion identity is currently recorded in the
commit.

Git can search reachable commit messages with `git log --grep`, select an exact
format with `--format`, and limit results. Fixed-string matching avoids regular
expression interpretation of identity values.

The repository transaction lock serializes commit creation, but current
already-Done inspection happens before entering `commit_ticket` and therefore
outside that lock.

The acceptance test expressly requires two invocations. Existing transaction
tests use temporary real Git repositories, configure deterministic author
identity, and can count commits or inspect commit messages.

## Path and repository constraints

Completion paths may be supplied relative to a nested Lisa project while Git
operations occur at the enclosing repository root. The normalization behavior
from T-042-01-05/06 must remain intact.

Transaction-owned paths must remain exact. The ordinary index must remain
untouched except for reconciliation of the exact committed paths after HEAD
advances.

The worktree contains Lisa-managed modifications to provenance and active
tickets plus an unrelated untracked `crates/lisa-plugin/docs/` fixture. Those
paths predate this ticket and are outside its ownership.

Ticket source changes must be committed with `lisa commit-ticket`, exact
repository-relative includes, and no ordinary-index staging.

Attempt artifacts belong under the assigned private work directory. Lisa will
admit and publish them after Review; they are not implementation commit inputs.

## Test boundaries

The CLI transaction unit suite is the direct location for a two-invocation
real-Git idempotency regression.

Core unit tests can validate construction, access, formatting, and equality of
any new identity type without involving Git.

Plugin native tests can validate that the effect identity becomes the expected
CLI argv. The connected nested-monorepo regression decodes real builder argv
into `CompleteTicketRequest`, so it also detects drift between the adapter and
transaction request.

Workspace tests cover downstream call sites because Rust struct literals must
provide every new request field.

No live provider, Zellij process, network service, or token-bearing agent run
is necessary for this ticket's acceptance criterion.
