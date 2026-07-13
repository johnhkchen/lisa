# Structure: Completion generation key

## Modified files

### `crates/lisa-core/src/completion.rs`

Add the pure domain identity `CompletionGenerationId` beside `AttemptId`,
`CompletionId`, and `CorrelationId`.

The type has private component fields:

- `completion_id: CompletionId`;
- `attempt_id: AttemptId`;
- `generation: u64`.

Its public interface contains:

- `new(completion_id, attempt_id, generation)`;
- `completion_id(&self) -> &CompletionId`;
- `attempt_id(&self) -> &AttemptId`;
- `generation(&self) -> u64`; and
- `Display` for the stable versioned encoding.

The type derives the value traits needed by request, journal, collection, and
test code: debug, clone, equality, ordering, and hashing.

An internal byte-to-hex formatter supports collision-free Display without a
new dependency.

Add focused identity unit tests in the existing completion test module.

Do not change the reducer event/effect shapes in this ticket. The effect's
existing ticket and attempt identities are sufficient for the adapter to mint
the initial generation key.

### `crates/lisa-cli/src/main.rs`

Extend `Commands::CompleteTicket` with two required options:

- `attempt_id: String` from `--attempt-id`;
- `completion_generation: u64` from `--completion-generation`.

In the command dispatch arm, construct a `CompletionGenerationId` from the
ticket id, attempt id, and numeric generation before constructing the request.

Pass the typed value as `completion_key` on `CompleteTicketRequest`.

The command continues to print exactly the returned commit id on stdout.

### `crates/lisa-cli/src/commit_transaction.rs`

Import `CompletionGenerationId` from lisa-core.

Extend `CompleteTicketRequest` with:

`pub completion_key: CompletionGenerationId`.

Add a stable marker prefix constant local to the transaction module.

Add private helpers for:

- forming the exact commit-message marker line from a key;
- appending the marker to a human commit message;
- discovering candidate reachable commits by fixed-string marker search;
- reading candidate commit messages; and
- accepting only a candidate containing the exact marker line.

Refactor the public `commit_ticket` wrapper around one private transaction
entry that accepts `Option<&CompletionGenerationId>`.

The private entry retains all existing validation, repository discovery,
locking, alternate-index cleanup, unlock, and compensating rollback behavior.
Its only new branch runs after lock acquisition and before alternate-index
reservation:

- no key: proceed exactly as today;
- key with exact reachable match: return the prior id without mutation;
- key without match: reserve the index and create the commit.

For a new completion commit, pass a request whose message contains the marker
to the existing transaction body. Do not teach `run_transaction_body` about
completion-specific types.

Update `complete_ticket` to:

- validate that the key's `CompletionId` matches `request.ticket_id`;
- remove the clean-Done/current-HEAD shortcut;
- retain path normalization and exact include construction;
- retain original-ticket byte rollback;
- append no marker itself; and
- invoke the keyed internal transaction entry.

Update all request literals in transaction tests with explicit keys.

Add the primary idempotency regression to this module's real-Git tests.

### `crates/lisa-plugin/src/lib.rs`

Import `CompletionGenerationId` with the existing completion-domain imports.

Change `State::build_completion_command` to accept a reference to the typed
completion key in addition to ticket file input. The builder can derive the
ticket id from `key.completion_id()` and adds:

- `--attempt-id <key.attempt_id()>`;
- `--completion-generation <key.generation()>`.

At the start of `execute_completion_effect`, build generation `1` from the
effect's existing completion and attempt identities.

Use that same key for authority comparison and command construction. The
pending map and command-result context remain ticket-indexed in this slice.

Update direct builder call sites in native tests.

Update exact argv expectations to include the new options.

Update the nested-monorepo test's argv decoder so it constructs the typed key
exactly as `main.rs` does. The test continues to execute the exported real CLI
transaction.

## Created files

No production source file is created.

The six phase artifacts are created only under:

`.lisa/attempts/T-042-02-01/1/work/`

Lisa owns their later admission and publication.

## Deleted files

None.

## Component flow

The completed production flow is:

1. scheduler evidence carries a current `AttemptLease`;
2. adapter constructs the existing typed request event;
3. reducer emits ticket/attempt effect identity;
4. effect executor binds those components to completion generation `1`;
5. command builder emits ticket, attempt, and generation CLI components;
6. CLI constructs `CompletionGenerationId`;
7. `CompleteTicketRequest` carries the typed key;
8. transaction acquires the repository lock;
9. transaction searches reachable commit metadata for the exact key;
10. an exact match returns its prior commit id; otherwise one marked commit is
    created through the existing alternate-index transaction.

## Transaction layering

`run_transaction_body` stays provider- and completion-neutral. It receives a
normal `CommitTransactionRequest` with a fully prepared message.

The keyed wrapper owns only discovery and message decoration. It uses the same
lock as creation, making “observe absent then create” one serialized critical
section.

`commit_ticket` remains the unkeyed public source-commit API.
`complete_ticket` is the keyed final-publication API.

## Error boundaries

A request whose typed key names a different completion/ticket id fails before
ticket mutation.

Git discovery failures are transaction failures and do not fall through to
commit creation.

An exact prior match is success even if it is not HEAD.

A different key never returns the prior key's commit. With no changes it gets
the existing no-changes error; with new exact-path changes it creates its own
commit.

Ticket byte restoration remains active only for errors after the original
ticket snapshot. A discovered replay is a successful non-mutating result.

## Tests and verification boundaries

Core tests prove identity construction and encoding independently.

CLI unit tests prove real Git persistence, exact discovery, one-commit replay,
different-key isolation, nested project behavior, failure restoration, and
ordinary-index preservation.

Plugin tests prove typed effect identity reaches argv and the connected
nested-project transaction.

Workspace compilation finds every request literal and builder call that must
adopt the new required contract.

WASM Clippy proves the adapter changes compile in the production target.

## Ownership and commit units

The implementation is one meaningful cross-crate contract unit:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/commit_transaction.rs`; and
- `crates/lisa-plugin/src/lib.rs`.

These paths will be committed together through `lisa commit-ticket` with exact
includes because intermediate subsets do not satisfy the required request
contract across crates.

No active ticket, provenance file, shared work artifact, or unrelated plugin
fixture is an implementation include.
