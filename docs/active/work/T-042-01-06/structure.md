# Structure: Nested command/transaction fixture

## Modified files

### `crates/lisa-cli/src/lib.rs`

Create the package library root.

It publicly exposes the existing `commit_transaction` module. No other CLI
module is moved into the library.

### `crates/lisa-cli/src/main.rs`

Remove the binary-local `mod commit_transaction` declaration.

Import `lisa_cli::commit_transaction` so the executable continues to construct
requests and invoke the same functions through the library target.

Command definitions and command-line behavior remain unchanged.

### `crates/lisa-cli/src/commit_transaction.rs`

Change visibility only for the composition surface:

- `CommitTransactionRequest`;
- `CompleteTicketRequest`;
- `CommitTransactionResult`;
- `CommitTransactionError`;
- `commit_ticket`;
- `complete_ticket`.

Request and result fields needed by callers remain public. The prior commit id,
repository implementation, normalization helpers, rollback helpers, and all
internal transaction machinery remain private.

No transaction algorithm changes.

### `crates/lisa-plugin/Cargo.toml`

Add `lisa-cli` as a path dev-dependency. It is available only to native tests
and does not enter the plugin's WASM production dependency graph.

### `crates/lisa-plugin/src/lib.rs`

Add one native unit test near the existing completion command tests.

The test contains local helpers for:

- initializing and querying a temporary Git repository;
- writing fixture files;
- locating option values in completion argv;
- asserting the nested fixture command contract;
- converting accepted argv into `CompleteTicketRequest`.

The test builds a `State` with the temporary repository root as `git_root`, the
nested project as `project_root`, `/host/docs/active/work` as configured work
root, and a stable dummy Lisa binary name.

## Created files

Only `crates/lisa-cli/src/lib.rs` is created as production source.

Attempt-private workflow artifacts are also created under the assigned work
directory but are published later by Lisa and are not source commit inputs.

## Deleted files

None.

## Component flow

The regression follows this sequence:

1. temporary repository fixture creates the nested project and root sentinel;
2. historical argv is checked and rejected by the fixture contract;
3. `State::build_completion_command` generates current argv;
4. the same fixture contract accepts current argv;
5. the test-local adapter decodes argv fields;
6. `lisa_cli::commit_transaction::complete_ticket` executes;
7. Git and result assertions verify the single isolated commit.

## Boundaries preserved

The plugin builder remains private because it is production-internal and the
test lives in the same module.

The test adapter remains private to the test. It does not become a second
production parser.

The CLI transaction is the only newly public boundary. The executable consumes
that boundary, ensuring the tested function is also the command's real
implementation.

The plugin's normal dependency list remains unchanged. Only native test code
can link `lisa-cli`.

## Ordering

First expose the CLI library surface and make the binary consume it. Verify the
CLI package before adding the plugin dev-dependency.

Then add the connected regression and run its focused test. Finally run package
and workspace verification, formatting, and source ownership checks.

## Ownership

The meaningful source unit includes all five paths because the regression does
not compile or connect without the library exposure, manifest edge, and test.
They will be committed together through `lisa commit-ticket` with exact
repository-relative includes.

Existing Lisa-managed changes to ticket/provenance files and the unrelated
untracked plugin docs fixture are outside this ticket's source ownership.
