# Plan: Nested command/transaction regression

## 1. Expose the existing transaction implementation

Create `crates/lisa-cli/src/lib.rs` exporting `commit_transaction`.

Change the transaction's request/result/error types and entry functions from
crate-private to public. Keep implementation details private.

Change the binary entry point to import the library module rather than declare
its own module.

Verification:

`cargo test -p lisa-cli commit_transaction --no-fail-fast`

This proves the transaction's existing unit suite still passes through the
library target and the binary compiles against the exported API.

## 2. Connect plugin tests to the CLI library

Add a path dev-dependency from `lisa-plugin` to `lisa-cli`.

Do not add a normal dependency. The WASM plugin must not gain CLI, Clap, Git
process, or filesystem-lock production code.

Verification:

`cargo tree -p lisa-plugin --target wasm32-wasip1`

Inspect that normal WASM dependencies do not include `lisa-cli`.

## 3. Build the two-level temporary fixture

In the plugin test module, initialize a temporary Git repository and configure
a deterministic author identity.

Create the nested Review ticket at:

`games/midsummer/docs/active/tickets/T-009-02-01.md`

Create a root-level sentinel at `docs/root-sentinel.md`, commit the baseline,
then create the nested work artifact at:

`games/midsummer/docs/active/work/T-009-02-01/review.md`

Record the baseline HEAD.

## 4. Pin the historical argv failure

Construct the field-recorded pre-fix argv using `--path games/midsummer` and
project-relative `docs/...` values.

Run it through a test-only nested-fixture contract assertion. Require a failure
whose diagnostic identifies `--path` or the missing nested path prefix.

This is a non-mutating command-boundary assertion; do not pass invalid argv to
the transaction.

## 5. Drive the real builder

Construct real `State` roots from the temporary fixture:

- project root `<temp>/games/midsummer`;
- Git root `<temp>`;
- WASI work root `/host/docs/active/work`.

Call `State::build_completion_command` with the `/host` ticket path.

Assert the fixture contract accepts its exact `--path`, ticket, and work
values. Assert the completion correlation context remains correct.

## 6. Drive the real transaction

Decode the accepted argv's option values into `CompleteTicketRequest` and call
the exported real `complete_ticket` function.

Assert:

- exactly one new commit separates baseline HEAD and returned HEAD;
- the returned commit id is the repository HEAD;
- committed paths are exactly the nested ticket and review artifact;
- nested ticket frontmatter is Done;
- nested review content is committed;
- root sentinel content is unchanged;
- root-level ticket/work paths do not exist in the commit.

Focused verification:

`cargo test -p lisa-plugin nested_monorepo_completion_command_drives_real_transaction --no-fail-fast`

## 7. Quality verification

Run:

- `cargo fmt --all -- --check`;
- `cargo test -p lisa-cli --no-fail-fast`;
- `cargo test -p lisa-plugin --lib --no-fail-fast`;
- `cargo test --workspace --no-fail-fast`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

Review the source diff and confirm no unintended manifest or lock changes.

## 8. Commit the source unit

Use the repository-built CLI if the installed binary lacks the command:

`target/debug/lisa commit-ticket --ticket-id T-042-01-06 --message "test: connect nested completion command to transaction" --include crates/lisa-cli/src/lib.rs --include crates/lisa-cli/src/main.rs --include crates/lisa-cli/src/commit_transaction.rs --include crates/lisa-plugin/Cargo.toml --include crates/lisa-plugin/src/lib.rs`

Confirm all five ticket-owned paths are clean and the ordinary index remains
untouched.

## 9. Review handoff

Write `progress.md`, `review.md`, and a passing `review-disposition.json` only
after all source checks and ownership checks succeed. Document any deviation
before implementation proceeds.
