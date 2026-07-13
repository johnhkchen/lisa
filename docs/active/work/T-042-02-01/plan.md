# Plan: Completion generation idempotency

## 1. Add the core identity

Implement `CompletionGenerationId` in `lisa-core` with private ticket/attempt/
generation components, accessors, value traits, and stable versioned Display.

Add unit tests that change each component independently and exercise delimiter
and Unicode input through the collision-free encoding.

Focused verification:

`cargo test -p lisa-core completion_generation --no-fail-fast`

Completion criteria:

- the type cannot be confused with any existing string identity;
- every component remains available to adapters;
- equal values format identically; and
- distinct component tuples do not collapse under the chosen examples.

## 2. Extend the CLI request contract

Add required `--attempt-id` and `--completion-generation` arguments to
`complete-ticket`.

Construct `CompletionGenerationId` from CLI components and add it to
`CompleteTicketRequest`.

Update all CLI transaction test literals so the package compiles before
idempotency behavior changes.

Focused verification:

`cargo test -p lisa-cli --no-run`

Completion criteria:

- Clap accepts the new typed numeric option;
- `main.rs` binds the existing ticket id into the key;
- no completion request can be constructed without the typed identity.

## 3. Add exact commit marker formation

Add helpers in `commit_transaction.rs` to form the exact marker and append it
to the human message.

Keep the marker machine-readable and on its own line.

Add small tests for marker/message shape if coverage through the integration
regression does not make failures sufficiently local.

Completion criteria:

- the original message remains visible;
- one exact `Lisa-Completion-Key` line is present; and
- arbitrary opaque ticket/attempt bytes cannot introduce message newlines
  because Display is ASCII encoded.

## 4. Discover prior keyed commits under the lock

Add a helper that uses fixed-string Git history search to narrow candidates,
then verifies an exact message line for each candidate.

Refactor the transaction wrapper to accept an optional key and perform lookup
after acquiring the repository lock but before reserving an alternate index.

Return a read-only transaction result on an exact match.

Keep public `commit_ticket` unkeyed.

Completion criteria:

- lookup and creation share one serialization boundary;
- a search error fails closed;
- a prefix-only marker does not match; and
- a prior reachable commit can be returned even when it is not HEAD.

## 5. Make `complete_ticket` identity-specific

Validate that `request.ticket_id` agrees with the key's `CompletionId`.

Remove the state-only clean-Done shortcut.

Prepare Done frontmatter as before, then enter the keyed transaction path.
Decorate only newly created completion commits with the marker.

Retain exact includes, nested-root normalization, original-byte restoration,
alternate-index isolation, compare-and-swap HEAD update, and rollback.

Completion criteria:

- same-key replay succeeds without mutation;
- different-key calls do not return the first key's commit;
- unkeyed source commits behave exactly as before.

## 6. Add the primary real-Git regression

Create a transaction test that invokes `complete_ticket` with key A, records
its commit, and verifies the marker.

Create an unrelated follow-up commit so HEAD no longer equals the completion
commit.

Replay key A and assert:

- returned id equals the first completion commit;
- returned id differs from current HEAD;
- committed paths are empty; and
- total commit count does not increase.

Modify the exact work artifact and invoke key B. Assert:

- key B returns a different new commit;
- commit count increases once;
- its message contains only key B's exact marker; and
- replaying key A still returns key A's first commit.

Also test ticket/key mismatch fails without changing HEAD or ticket bytes.

Focused verification:

`cargo test -p lisa-cli commit_transaction --no-fail-fast`

## 7. Thread the key through the plugin adapter

Import `CompletionGenerationId` in `lisa-plugin`.

Construct generation `1` at the effect execution boundary using the effect's
typed ticket and attempt identities.

Pass the key into `build_completion_command` and emit the new CLI options.

Continue using ticket id for command result context and pending-map lookup.

Update direct builder tests, exact argv assertions, and authority mismatch
coverage.

Completion criteria:

- one effect identity deterministically creates one key;
- a new attempt creates a different command key;
- no scheduler path invents an unbound string token.

## 8. Update the connected nested-project regression

Extend the test-local argv decoder with `--attempt-id` and
`--completion-generation`.

Build the request's typed key exactly as CLI `main.rs` does.

Keep all existing assertions for Git-root path arguments, exact nested ticket
and work paths, root docs preservation, and a single returned commit.

Focused verification:

`cargo test -p lisa-plugin nested_monorepo_completion_command_drives_real_transaction --no-fail-fast`

## 9. Run package and workspace verification

Run:

- `cargo fmt --all -- --check`;
- `cargo test -p lisa-core --no-fail-fast`;
- `cargo test -p lisa-cli --no-fail-fast`;
- `cargo test -p lisa-plugin --lib --no-fail-fast`;
- `cargo test --workspace --no-fail-fast`;
- `cargo clippy -p lisa-core --all-targets -- -D warnings`;
- `cargo clippy -p lisa-cli --all-targets -- -D warnings`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `git diff --check`.

Inspect the normal WASM dependency tree only if the implementation introduces a
manifest change; none is planned.

## 10. Review repository ownership

Inspect `git status --short`, ticket-owned diffs, and ordinary staged entries.

Preserve the pre-existing Lisa-managed provenance/ticket modifications and the
untracked plugin docs fixture.

Confirm the only ticket-owned source paths are the four planned Rust files.

Document any plan deviation in `progress.md` before committing.

## 11. Commit the meaningful source unit

Use the repository-built CLI if necessary:

`target/debug/lisa commit-ticket --ticket-id T-042-02-01 --message "feat: make completion commits generation-idempotent" --include crates/lisa-core/src/completion.rs --include crates/lisa-cli/src/main.rs --include crates/lisa-cli/src/commit_transaction.rs --include crates/lisa-plugin/src/lib.rs`

Do not use ordinary `git add` or `git commit`.

After the command, confirm all four owned paths are clean and the ordinary
index contains no ticket-owned entries.

## 12. Complete Review artifacts

Update `progress.md` with implementation, verification, deviations, and exact
source commit id.

Write `review.md` mapping the result to acceptance criteria, listing tests and
open limitations.

Write exactly one valid `review-disposition.json`. Use pass only if all required
behavior and ownership checks succeed; otherwise use an actionable block.

Remain on T-042-02-01 after Review. Do not update ticket phase/status, publish
shared artifacts, or invoke final completion.
