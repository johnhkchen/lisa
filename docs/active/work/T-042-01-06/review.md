# Review: Nested-monorepo path regression

## Disposition

Pass.

T-042-01-06 now connects the real plugin completion command builder to the real
CLI `complete-ticket` transaction in one deterministic Arcade-shaped temporary
Git repository. The field-recorded legacy argv is explicitly rejected, while
the current argv produces one commit containing only the nested Lisa ticket and
work artifact.

All focused, workspace, formatting, dependency-boundary, CLI Clippy, and WASM
Clippy checks pass. No blocking issue remains.

## Source commit

Implementation commit:

`bff1b3bbdaec4bea4185f871c542a65e37d1f965`

It was created through `lisa commit-ticket` with exact repository-relative
include paths. It contains:

- `Cargo.lock`;
- `crates/lisa-cli/src/commit_transaction.rs`;
- `crates/lisa-cli/src/lib.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-plugin/Cargo.toml`;
- `crates/lisa-plugin/src/lib.rs`.

No ticket-owned source remains staged, modified, or untracked. The ordinary Git
index is empty.

## Changes reviewed

### CLI transaction reuse

`lisa-cli` now has a minimal library root exporting `commit_transaction`.
The existing request, result, error, `commit_ticket`, and `complete_ticket`
items are public so deterministic adapters can call the production transaction.

The CLI binary imports the library module. It no longer compiles a separate
binary-private copy. Command flags and transaction semantics are unchanged.

### Plugin test dependency

`lisa-plugin` depends on `lisa-cli` only under dev-dependencies. `Cargo.lock`
records that edge without changing resolved versions.

The normal WASM dependency tree was inspected and does not contain `lisa-cli`.
Therefore CLI, Clap, Git subprocess, and lock implementation code do not enter
the production plugin artifact.

### Native plugin registration boundary

`register_plugin!(State)` is now compiled only for `wasm32`, the runtime target
where Zellij registration is required.

This was necessary because the macro's native `pipe` export interposed on the
operating system `pipe(2)` used by subprocess spawning. The connected test must
spawn Git through the real transaction. Native tests call Rust methods directly
and keep the existing no-op host function stub, so they do not require plugin
registration exports.

WASM compilation and lint prove the production registration remains present.
The full native plugin suite proves the test target remains intact.

## Regression behavior

The new
`nested_monorepo_completion_command_drives_real_transaction` test initializes a
temporary Git repository and places its Lisa project at `games/midsummer`.

The baseline tree contains the nested Review ticket and a root-level docs
sentinel. A nested Review work artifact is then created as the transaction's
pending input.

The fixture first evaluates historical argv with:

- `--path games/midsummer`;
- `--ticket-file docs/active/tickets/T-009-02-01.md`;
- `--work-dir docs/active/work/T-009-02-01`.

The nested command contract rejects it and identifies the incorrect `--path`.
This pins the field failure independently of later Git errors.

The fixture then constructs a real plugin `State` and invokes the existing
private `State::build_completion_command`. Ticket and work inputs use their
real `/host/docs/...` WASI representation.

The generated argv must contain:

- the temporary repository root as `--path`;
- `games/midsummer/docs/active/tickets/T-009-02-01.md`;
- `games/midsummer/docs/active/work/T-009-02-01`.

A test-local argv decoder constructs `CompleteTicketRequest`, matching the CLI
entry point's request construction, and calls the exported production
`complete_ticket` implementation.

## Acceptance mapping

### Temporary nested Git repository

Satisfied. Every test run creates and configures a fresh repository and a Lisa
project exactly two directories below it at `games/midsummer`.

### Real command builder

Satisfied. The test invokes `State::build_completion_command`; it does not
reimplement path mapping or hard-code the fixed request.

### Real complete-ticket transaction

Satisfied. The CLI binary and test both invoke the same exported
`lisa_cli::commit_transaction::complete_ticket` function.

### Pre-fix argv fails

Satisfied. The literal field-recorded `games/midsummer` plus `docs/...` argv
fails the fixture contract before mutation.

### Fixed argv targets repository root

Satisfied. The contract compares `--path` to the temporary Git root and checks
both nested Git-root-relative arguments.

### Commits only nested ticket and work directory

Satisfied. `committed_paths` must equal exactly the nested ticket and nested
`review.md`. Git tree inspection additionally rejects root-level ticket/work
entries.

### Leaves root-level docs untouched

Satisfied. `HEAD:docs/root-sentinel.md` retains its exact baseline content.

### Returns one commit id

Satisfied. Returned id equals HEAD, differs from baseline HEAD, and `HEAD^`
equals the baseline. This proves exactly one transaction commit advanced the
fixture.

## Test coverage

Focused connected regression:

`cargo test -p lisa-plugin nested_monorepo_completion_command_drives_real_transaction --no-fail-fast`

Passed: 1; failed: 0.

Focused transaction tests:

`cargo test -p lisa-cli commit_transaction --no-fail-fast`

Passed: 13; failed: 0.

Full workspace:

`cargo test --workspace --no-fail-fast`

Passed. The run included 13 CLI library transaction tests, 267 CLI binary
tests, CLI integration coverage, 192 core tests plus integration regressions,
and 344 plugin tests. The real-Zellij test remained ignored under its declared
environment requirement.

Lint and formatting:

- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` passed;
- `cargo clippy -p lisa-cli --all-targets -- -D warnings` passed;
- `cargo fmt --all -- --check` passed;
- `git diff --check` passed.

## Open concerns and limitations

No blocking concern exists.

The test-local argv decoder intentionally covers only the fixed
`complete-ticket` option contract. Clap parsing remains exercised by the CLI's
own command surface tests. This regression targets composition of path values
and the transaction rather than duplicating general parser coverage.

The regression directly calls the transaction rather than launching a sibling
binary, avoiding build-order and target-path assumptions. Because `main.rs`
now consumes the exported library function, the direct call is the command's
real implementation after parsing.

The public transaction API is a broader Rust visibility surface than before,
but its types are narrow and provider-neutral. No stability guarantee is added
beyond ordinary workspace use.

## Repository preservation

Lisa-managed changes in `.lisa/provenance.jsonl`, the active ticket, and
published workflow artifacts were not included in the source commit.

The pre-existing untracked `crates/lisa-plugin/docs/` tree was preserved.

## Critical issues requiring human attention

None.

Review is complete. This attempt remains on T-042-01-06 for Lisa to validate
the disposition, publish admitted artifacts, prepare the completion commit, and
release the seat.
