# Design: Connected command-and-transaction regression

## Goal

Create one deterministic native regression that obtains argv from the real
plugin completion builder and feeds its values into the real CLI completion
transaction over an Arcade-shaped temporary repository.

## Option 1: Keep the tests separate

The existing plugin argv test and CLI nested transaction test collectively
cover most values. Extending their assertions would require no crate boundary
change.

This does not meet the ticket's defining condition. A future builder and
transaction mismatch could let both isolated tests pass while their composition
fails. It also cannot truthfully say that the fixture drove the real builder.

Rejected.

## Option 2: Run the CLI as an external process

A plugin test could build argv and invoke a `lisa` executable. This resembles
the production process boundary closely.

Cargo does not guarantee a sibling package binary path to this crate's unit
tests. Depending on `target/debug/lisa` makes a clean checkout fail unless the
binary happened to be built first. Launching nested Cargo during tests adds
locking, latency, target-directory, and environment dependencies.

Rejected as nondeterministic for the default workspace suite.

## Option 3: Duplicate or source-include the transaction

The plugin test could reproduce transaction behavior or include the CLI source
file with a path attribute.

Duplication would not exercise the real transaction. Source inclusion would
compile the same file in an unnatural module context, require mirroring CLI
dependencies in plugin dev-dependencies, and make visibility and maintenance
confusing.

Rejected.

## Option 4: Export the CLI transaction as a library API

Add a minimal `lisa-cli` library target that publicly exports
`commit_transaction`. Make the request, result, error, and two transaction
functions public. Change the binary entry point to import that module from its
package library instead of compiling a second private copy.

Add `lisa-cli` as a plugin dev-dependency. The plugin native regression can then
call `State::build_completion_command` and `lisa_cli::commit_transaction::complete_ticket`
in one process.

This uses the exact production builder and transaction implementations without
subprocess assumptions. It also removes duplicate binary/library compilation of
the transaction module by having the binary consume the library.

Chosen.

## Argv fixture adapter

The production boundary between plugin and CLI is argv plus process execution.
The regression will use a small test-only decoder for the fixed, known
`complete-ticket` option set. It will assert the executable/subcommand and
extract `--path`, `--ticket-id`, `--message`, `--ticket-file`, and `--work-dir`.

The decoder is deliberately test-local. Production Clap parsing is separately
covered by CLI surface tests; exposing the private Clap command model would
broaden the change without improving the path regression.

The decoded strings populate `CompleteTicketRequest`, matching what `main.rs`
does after parsing.

## Historical failure assertion

The test will define the field-recorded legacy argv:

- `--path games/midsummer`;
- `--ticket-file docs/active/tickets/<id>.md`;
- `--work-dir docs/active/work/<id>`.

A shared fixture assertion will require repository-root `--path` and both
`games/midsummer/...` path prefixes. The legacy argv must fail this assertion.
This pins why the old contract is invalid before any transaction runs.

The assertion will then accept argv produced by the real builder. This makes a
regression back to the historical form fail at the command boundary with a
focused diagnostic instead of merely failing later inside Git.

## Transaction execution

The accepted fixed argv is decoded into the public request and passed directly
to `complete_ticket`. The fixture uses an absolute temporary Git root in State,
so the builder's `--path` is canonical enough for the transaction's own
canonicalization.

The ticket input uses `/host/docs/...`, matching the plugin's WASI view. The
configured work root also uses `/host/docs/...`. Thus the regression covers the
real sandbox-to-host mapping, not a handcrafted fixed request.

## Exact safety assertions

The test records HEAD before execution and requires:

- returned `commit_id` equals new HEAD;
- new HEAD differs from old HEAD;
- `HEAD^` equals old HEAD, proving one commit was produced;
- `committed_paths` contains only the nested ticket and nested review artifact;
- the nested committed ticket has Done status and phase;
- the nested review artifact exists in the commit;
- root `docs/root-sentinel.md` retains its original committed content;
- no root-level ticket or work path is introduced.

The test uses a fresh repository and makes no ordinary-index claim about the
real Lisa repository.

## API tradeoff

The CLI transaction becomes a public Rust API even though the executable
remains its primary consumer. This is a narrow, provider-neutral operation with
already explicit request/result types. Exporting it is preferable to test-only
source inclusion and allows future deterministic adapters to exercise the same
transaction.

No transaction semantics, CLI flags, plugin production dependencies, or WASM
binary contents change. The dependency is dev-only for native testing.
