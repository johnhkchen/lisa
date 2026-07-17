# Progress — T-049-04-02

## Status

Implementation and verification are complete.

All ticket-owned source changes are committed through Lisa's isolated
transaction.

No ticket-owned source file is staged, modified, or untracked.

The root `.lisa-commit.lock` is absent.

Review remains.

## Completed: transaction lock ownership

Refactored `TransactionLock` in
`crates/lisa-cli/src/commit_transaction.rs`.

Added a stable advisory guard at `<git-dir>/lisa-commit.guard`.

The guard remains a reusable inode inside repository metadata.

Retained the root `.lisa-commit.lock` advisory lock for compatibility with
older Lisa processes and existing external holders.

The root path is now an ephemeral operator marker rather than the serialization
inode.

Added JSON owner metadata with schema version, PID, and acquisition Unix time.

New transactions acquire the stable guard before opening the visible marker.

Live guard contention returns without touching the root marker.

Live visible-marker contention returns owner details when readable.

The live-holder error explicitly says the lock was not stolen.

Marker cleanup occurs while the stable guard remains held.

Cleanup removes the root marker, releases its advisory lock, and releases the
stable guard.

All cleanup steps are attempted and their errors are combined.

`Drop` uses the same cleanup path best-effort for early exits.

## Completed: stale lock diagnosis and recovery

Added Unix PID liveness inspection using `kill(pid, 0)`.

`ESRCH` is treated as an absent recorded holder.

Success and `EPERM` are treated as a present holder.

Other outcomes remain conservative and are not auto-stolen.

An existing marker with an absent recorded PID is removed under the stable
guard.

The recovery returns one actionable error before retrying transaction work.

That error names:

- the stale commit transaction lock;
- the exact marker path;
- its measured age;
- the recorded PID;
- the absent/no-such-process fact;
- successful recovery.

Malformed legacy empty markers are also recovered under both advisory locks and
reported as having no recorded holder.

This behavior was exercised against the actual repository's legacy empty
marker during the first ticket commit attempt.

The observed error named a 512989-second-old marker and removed it.

The subsequent exact-path ticket transaction succeeded.

Concurrent old-binary transactions later left two additional empty markers.

Both were named and recovered by the new binary before retry, while no root
marker remained after the new transaction completed.

## Completed: empty-history discovery

Split Git execution into raw-output and strict-status helpers.

The strict helper retains all existing error formatting.

Added exact unborn-HEAD inspection:

- `git symbolic-ref --quiet HEAD` identifies a symbolic current branch;
- `git show-ref --verify --quiet <head-ref>` determines whether that branch ref
  exists.

Completion-key discovery returns `Ok(None)` when the symbolic current branch
has no ref.

Normal history still uses the existing current-ancestry `git log --grep` and
exact commit-message verification.

Unexpected Git failures remain failures rather than being mapped to absence.

The transaction body now names the real parent-commit precondition for an
unborn branch: it cannot resolve HEAD because the current branch has no commits.

This retains the production history/identity classifier phrase needed for the
bounded operator policy.

## Completed: CLI regression coverage

Expanded transaction tests from 14 to 16 in the focused library run.

The live-holder fixture now writes real owner metadata, takes the visible
advisory lock, runs a competitor, and verifies:

- acquisition fails;
- the error identifies a live PID;
- the error says the lock was not stolen;
- marker bytes remain identical;
- the held pathname remains present until fixture teardown.

The stale-holder fixture starts and reaps a real child to obtain an absent PID.

It writes owner metadata dated roughly 120 seconds earlier.

The first transaction asserts stale, age, PID, absent/no-such-process, and
recovered text.

It asserts the marker is gone.

The second transaction succeeds, proving recovery rather than diagnosis alone.

The age assertion accepts elapsed test time while still requiring a
seconds-valued operator report.

The unborn-history fixture directly asserts discovery is `None`.

It then runs full completion and asserts the error comes from resolving HEAD,
not prior-completion discovery.

It verifies byte-exact non-Done ticket restoration and no residual marker.

Existing matching-key history coverage still proves idempotent short-circuit.

Marker-absence assertions now cover successful completion, idempotent replay,
staged overlap, unchanged paths, identity failure, completion rollback, nested
projects, and compensating rollback.

## Completed: preserved field replay

Added a native scheduler regression in `crates/lisa-plugin/src/lib.rs`.

The test embeds the preserved 2026-07-16 journal at:

`docs/active/work/T-046-06-03/cbt-0716-211915-variant-xdg/demo-completion-journal.jsonl`.

It parses the JSONL and requires exactly 80 T-001 rejected rows naming the old
discovery failure and unborn branch.

It creates a real temporary Git repository for the existing plugin completion
fixture.

The fixture explicitly verifies HEAD is unborn.

It explicitly verifies `GIT_AUTHOR_IDENT` cannot be resolved after empty local
identity configuration.

It dispatches the production completion generation.

For each scheduler host attempt, it invokes
`lisa_cli::commit_transaction::complete_ticket` directly.

The resulting real CLI failure reaches the HEAD precondition and contains the
history classifier phrase.

The test feeds those actual failure bytes into production scheduler result
handling.

It observes exactly two launch effects: initial command plus one retry.

It observes two durable `failure-observed` rows: 1/2 retry-scheduled and 2/2
park.

It observes exactly one Park provenance row with count 2 and limit 2.

It verifies the canonical block is structured, operator-owned, and carries the
exact `HISTORY_IDENTITY_ASK` sentence.

It verifies Review/Blocked ticket state, seat release, thread removal, and no
pending completion.

It invokes reconciliation again and proves no third command launches.

It verifies the real CLI transaction left no `.lisa-commit.lock` after each
failure and after the complete replay.

## Source commits

Commit `148523b076a7fa5385de987a1617e84fdf66706c`:

`fix(cli): clean completion locks and handle unborn history`

Exact include:

`crates/lisa-cli/src/commit_transaction.rs`

Commit `49f98a36cb24ea4500536e5b6ad9373c34befec1`:

`test(plugin): replay the bounded completion incident`

Exact include:

`crates/lisa-plugin/src/lib.rs`

Commit `8a69720001c169365aea15e7dd6b73a16827df3b`:

`test(cli): make stale lock age assertion robust`

Exact include:

`crates/lisa-cli/src/commit_transaction.rs`

All three commits used `target/debug/lisa commit-ticket` with ticket ID
T-049-04-02.

No ordinary `git add`, `git commit`, or broad include was used.

## Verification

Passed `cargo fmt --all -- --check`.

Passed `git diff --check`.

Passed `cargo test -p lisa-cli commit_transaction --lib` with 16 transaction
tests before later workspace feature unification.

Passed `cargo test -p lisa-cli` including all CLI unit and integration tests.

Passed `cargo test -p lisa-plugin completion_failure --lib` with 3 focused
tests.

Passed `cargo test -p lisa-plugin field_journal_replay --lib`.

Passed `cargo test --workspace`.

The final workspace run included 21 lisa-cli library tests, 345 CLI binary
tests, 228 core tests, 422 plugin tests, and all executed integration tests.

The existing real-Zellij delivery test remained ignored under its declared
environment requirement.

Passed `just check`.

That included `cargo check -p lisa-plugin --target wasm32-wasip1` and a second
complete workspace run.

Passed `cargo clippy -p lisa-cli --lib -- -D warnings`.

Passed `cargo clippy -p lisa-plugin --lib --target wasm32-wasip1 -- -D warnings`.

## Repository hygiene

`git diff --cached --name-only` is empty.

Both ticket-owned source paths are clean relative to HEAD.

The root `.lisa-commit.lock` is absent.

Remaining worktree changes belong to Lisa journals, active ticket bookkeeping,
and another concurrently scheduled ticket.

None were included in this ticket's source commits.

## Deviations from plan

The plan expected one coupled source commit.

Concurrent T-049-03-01 implementation modified the same plugin source while the
field replay was being added.

Both attempts detected the exact-path ownership collision.

This ticket committed its independent CLI module first.

It then temporarily withdrew only its two plugin test hunks, allowing
T-049-03-01 to publish its plugin coordinator and journal-seal units.

The field replay hunks were reapplied on the new clean baseline, retested, and
committed as the second exact-path unit.

This avoided committing another ticket's uncommitted source and preserved both
implementations without reset or destructive checkout.

The stale-lock age assertion received a final small robustness commit after
reviewing timing sensitivity.

No behavioral design deviation remains.
