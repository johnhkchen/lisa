# T-034-01-01 Progress — attempt lease core type

## Outcome

Implementation is complete.

`lisa-core` now exposes a serializable `AttemptLease` value, a fail-closed
`AttemptLease::mint` helper, and an exact `AttemptLease::is_current` helper.
Five unit tests prove per-ticket monotonicity, stale-prior rejection,
cross-ticket isolation, mismatch rejection, and exhaustion rejection.

## Plan progress

### Step 1 — add the mint failure contract

Completed.

Added public `AttemptLeaseMintError` variants:

- `TicketMismatch { ticket_id, previous_ticket_id }`;
- `AttemptIdExhausted { ticket_id }`.

The error implements `Display` and `std::error::Error` without a new
dependency. Both variants retain enough ticket context for a later scheduler
diagnostic.

### Step 2 — add `AttemptLease`

Completed.

Added a public value with:

- `ticket_id: TicketId`;
- `attempt_id: u64`.

It derives `Debug`, `Clone`, complete equality, `Hash`, `Serialize`, and
`Deserialize`. It deliberately does not implement `Default`, so callers cannot
obtain a valid-looking lease without explicitly minting or deserializing one.

### Step 3 — implement fail-closed minting

Completed.

`AttemptLease::mint` creates attempt 1 for a ticket with no predecessor. A
same-ticket predecessor is incremented with `checked_add`.

A predecessor belonging to a different ticket returns `TicketMismatch`.
Attempt `u64::MAX` returns `AttemptIdExhausted`. No code path saturates, wraps,
uses a clock, or mutates global state.

### Step 4 — implement exact-current validation

Completed.

`AttemptLease::is_current` compares the candidate against an optional
authoritative lease using whole-value equality. It therefore requires both the
ticket and attempt ID to match.

`None` rejects every candidate and provides the later revocation representation
without adding revocation behavior in this ticket.

### Step 5 — add acceptance tests

Completed.

`attempt_lease_ids_are_strictly_monotonic_per_ticket` interleaves T-A and T-B
through attempts 1, 2, and 3. It demonstrates that each ticket has its own
strictly increasing sequence and that complete lease identity remains distinct
when two tickets share the same numeric generation.

`prior_attempt_lease_never_validates_as_current` proves that attempt 1 is
current before replacement, becomes stale when attempt 2 is authoritative, and
also fails validation after authoritative state is absent.

### Step 6 — add defensive edge tests

Completed.

Added tests for:

- same numeric attempt on different tickets;
- predecessor ticket mismatch and exact error contents;
- exhaustion at `u64::MAX` and exact error contents;
- absent current authority.

### Step 7 — focused verification

Completed with one repository-wide test-target limitation documented below.

Passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-core attempt_lease
5 passed; 0 failed

cargo test -p lisa-core
155 passed; 0 failed

cargo clippy -p lisa-core --lib -- -D warnings
passed

git diff --check -- crates/lisa-core/src/types.rs
passed
```

The planned `cargo clippy -p lisa-core --all-targets -- -D warnings` reached 12
existing `clippy::unnecessary_to_owned` findings in
`crates/lisa-core/src/dag.rs` tests. None is in the ticket-owned file or caused
by this change. `dag.rs` was left untouched. The library-target Clippy check
passes with warnings denied.

### Step 8 — shared-crate regression verification

Completed.

Passed:

```text
cargo test --workspace
693 tests passed; 0 failed

cargo check -p lisa-plugin --target wasm32-wasip1
passed

git diff --check
passed
```

Workspace test composition after this ticket is 270 CLI unit tests, one CLI
integration test, 155 core unit tests, and 267 plugin unit tests. Doc tests also
passed with no failures.

The WASM check confirms the shared type, Serde derives, and error implementation
compile for the deployed plugin target.

### Step 9 — inspect and commit the implementation unit

Completed.

The globally installed `/opt/homebrew/bin/lisa` did not recognize
`commit-ticket`. The repository-built `target/debug/lisa`, produced during the
workspace verification, exposes the required command. It was used with the
same isolated transaction and exact include semantics:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-034-01-01 \
  --message "feat: add attempt lease core contract" \
  --include crates/lisa-core/src/types.rs
```

Result:

```text
1094e7b91f8b31ec729bf78721d85e34cdde6185
```

Commit summary:

```text
feat: add attempt lease core contract
crates/lisa-core/src/types.rs | 157 insertions
```

### Step 10 — final ownership audit

Completed.

After the isolated commit:

- `crates/lisa-core/src/types.rs` is clean;
- the ordinary Git index is empty;
- the source commit contains only `crates/lisa-core/src/types.rs`;
- no unrelated modified or untracked path was included;
- the ticket remains untracked as it was at ticket start;
- the ticket's phase and status fields were not edited;
- RDSPI artifacts remain for Lisa's completion transaction.

The repository's numerous unrelated pre-existing worktree changes remain
untouched.

## Deviations from plan

There was no design or implementation deviation.

The only verification deviation was narrowing Clippy from all targets to the
library target after the all-target command failed on 12 pre-existing `dag.rs`
test warnings. Full tests and the WASM build still cover every crate consumer.

The repository-built CLI was used instead of the installed CLI because the
installed version predates `commit-ticket`. This follows the workflow's stated
fallback and preserved exact-path isolated commit behavior.

## Files changed

Ticket-owned source commit:

- `crates/lisa-core/src/types.rs`

RDSPI artifacts written for Lisa's later completion transaction:

- `docs/active/work/T-034-01-01/research.md`;
- `docs/active/work/T-034-01-01/design.md`;
- `docs/active/work/T-034-01-01/structure.md`;
- `docs/active/work/T-034-01-01/plan.md`;
- `docs/active/work/T-034-01-01/progress.md`;
- `docs/active/work/T-034-01-01/review.md` after Review completes.

## Remaining work in this ticket

Only the Review artifact remains. Scheduler storage, dispatch stamping,
revocation, fencing, and rejection-site integration remain explicitly deferred
to the dependent tickets.
