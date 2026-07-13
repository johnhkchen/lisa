# Progress: Nested-monorepo path regression

## Status

Implementation and verification are complete. The source unit is ready for its
isolated Lisa commit.

## Completed: CLI transaction library boundary

Created `crates/lisa-cli/src/lib.rs` and exported the existing
`commit_transaction` module.

Changed the transaction request, result, error, and entry-function visibility
from crate-private to public. No transaction logic changed.

Changed `crates/lisa-cli/src/main.rs` to consume
`lisa_cli::commit_transaction`, so the executable and regression use the same
compiled implementation rather than parallel module copies.

## Completed: test-only dependency

Added `lisa-cli` as a path dev-dependency of `lisa-plugin`.

`Cargo.lock` gained one dependency edge from the plugin package to the existing
CLI package. No package version or resolved dependency changed.

Verified with the normal-dependency-only WASM tree that `lisa-cli` is not in the
plugin's production WASM graph.

## Completed: connected regression

Added
`nested_monorepo_completion_command_drives_real_transaction` beside the
existing completion command tests in `crates/lisa-plugin/src/lib.rs`.

The test creates a real temporary Git repository with the Lisa project at
`games/midsummer`. Its baseline contains:

- a Review ticket at the nested project path;
- a root-level `docs/root-sentinel.md`;
- an initial Git commit.

It then creates a nested `review.md` work artifact without adding it through the
ordinary index.

The test represents the field-recorded pre-fix argv with `--path
games/midsummer` and `docs/active/...` arguments. The fixture contract rejects
that argv and requires the error to identify the incorrect `--path`.

The same test constructs a real plugin `State`, calls
`State::build_completion_command`, validates its Git-root and nested path
arguments, and checks the `lisa_completion` context correlation.

It decodes that real argv into the same `CompleteTicketRequest` built by the
CLI entry point and calls the real exported `complete_ticket` transaction.

The postconditions prove:

- the transaction returns a commit id;
- returned commit equals HEAD;
- HEAD's parent equals the recorded baseline, so exactly one commit advanced;
- committed paths are exactly the nested ticket and nested review artifact;
- nested ticket status and phase are Done;
- nested review content is committed;
- root-level docs sentinel content is unchanged;
- root-level ticket/work paths do not exist.

## Deviation: native Zellij export

The first connected run aborted while spawning Git. Backtrace showed the native
test binary's Zellij-generated `pipe` export interposed on the operating
system's `pipe(2)` call used by `std::process::Command`. Existing plugin tests
had not spawned subprocesses and therefore had not encountered the collision.

The plugin registration macro is now gated to `target_arch = "wasm32"`.
Registration is a WASM runtime concern; native unit tests use direct Rust calls
and retain their existing no-op host function stub. This prevents symbol
interposition in native subprocess tests without changing the WASM artifact.

After the gate, the test reached an assertion that searched the full Git tree
with substring matching. The nested path naturally contained the root-level
suffix, producing a false failure. The assertion was corrected to compare
individual tree lines exactly for the ticket and by root-level prefix for work.

Both deviations were diagnosed before proceeding and are covered by the full
native and WASM verification below.

## Verification

Focused connected regression:

`cargo test -p lisa-plugin nested_monorepo_completion_command_drives_real_transaction --no-fail-fast`

Passed: 1; failed: 0.

Focused CLI transaction suite:

`cargo test -p lisa-cli commit_transaction --no-fail-fast`

Passed: 13 transaction tests; failed: 0. Other filtered targets succeeded.

Full workspace:

`cargo test --workspace --no-fail-fast`

Passed. This includes 13 CLI library transaction tests, 267 CLI binary tests,
CLI integration tests, 192 core unit tests, both core integration regressions,
and 344 plugin tests. The declared real-Zellij environment test remained
ignored.

WASM lint:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

Passed.

CLI all-target lint:

`cargo clippy -p lisa-cli --all-targets -- -D warnings`

Passed.

Formatting and diff hygiene:

- `cargo fmt --all -- --check` passed;
- `git diff --check` passed;
- normal WASM dependency tree contains no `lisa-cli`;
- ordinary Git index was empty before the source commit.

## Source commit

Committed through Lisa's isolated transaction:

`bff1b3bbdaec4bea4185f871c542a65e37d1f965`

Message:

`test: connect nested completion command to transaction`

The commit contains exactly the six planned source paths. All six are clean
after the transaction, and the ordinary Git index is empty.

## Remaining

Write Review artifacts and remain on this ticket for Lisa's completion gate.
