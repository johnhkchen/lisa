# T-034-01-01 Plan — attempt lease core type

## Step 1 — add the mint failure contract

Modify `crates/lisa-core/src/types.rs` near `TicketId`.

Add `AttemptLeaseMintError` with ticket-mismatch and attempt-exhaustion variants.
Implement `Display` and `std::error::Error` without adding dependencies.

Verification:

- the enum is public through `lisa_core::types`;
- messages identify the affected ticket(s);
- variants are equality-testable;
- `cargo check -p lisa-core` compiles the trait implementations.

Atomicity: this lands with the lease value because the public mint signature
depends on it; it is not useful as a separate commit.

## Step 2 — add `AttemptLease`

Define the serializable, hashable value with public `ticket_id: TicketId` and
`attempt_id: u64` fields.

Document that minted attempt IDs start at 1 and strictly increase for a ticket.
Do not add a default implementation because a default lease would imply a
valid-looking ticket/attempt without a mint operation.

Verification:

- the value derives whole-identity equality;
- it is cloneable and serializable for later thread/event integration;
- no existing type or serialized record is modified.

## Step 3 — implement fail-closed minting

Add `AttemptLease::mint(ticket_id, previous)`.

Behavior:

- `None` predecessor returns attempt 1;
- same-ticket predecessor returns its checked successor;
- different-ticket predecessor returns `TicketMismatch`;
- `u64::MAX` predecessor returns `AttemptIdExhausted`;
- no branch saturates, wraps, reads time, or mutates global state.

Verification:

- focused unit assertions cover every branch;
- Clippy reports no needless clone or ownership issue.

## Step 4 — implement exact-current validation

Add `AttemptLease::is_current(&self, current)` using complete value equality.

Behavior:

- exact ticket and attempt match returns true;
- stale predecessor returns false;
- different ticket with equal attempt returns false;
- absent authoritative lease returns false.

Verification:

- tests make the authoritative reference change from generation 1 to 2;
- the old lease changes from accepted to rejected without being mutated;
- revocation semantics are represented by `None`.

## Step 5 — add acceptance tests

In the existing `types.rs` test module, add a test that interleaves minting for
two tickets through at least three attempts each.

Assertions:

- both first attempts are 1;
- each successor is exactly one greater than its same-ticket predecessor;
- sequences are independently `[1, 2, 3]`;
- current successor validation rejects every retained prior lease.

This is the direct ticket acceptance proof.

## Step 6 — add defensive edge tests

Add focused tests for:

- predecessor ticket mismatch and its exact error payload;
- exhaustion at `u64::MAX` and its exact error payload;
- equal numeric attempts across different tickets;
- absence of an authoritative current lease.

These tests protect the strict and fail-closed semantics that downstream fence
and rejection sites will rely upon.

## Step 7 — focused verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-core attempt_lease
cargo test -p lisa-core
cargo clippy -p lisa-core --all-targets -- -D warnings
```

If formatting check fails because the new code is unformatted, run
`cargo fmt --all`, inspect the diff for unrelated formatting, then repeat the
check. Only ticket-owned changes may remain in `types.rs`.

Focused success criteria:

- all lease tests pass;
- the full core package remains green;
- no warnings are admitted;
- formatting is stable.

## Step 8 — shared-crate regression verification

Because `lisa-core::types` is consumed by the CLI and plugin, run:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-core/src/types.rs
```

The workspace suite detects accidental API or serialization interactions. The
WASM check confirms derives and error implementations remain compatible with
the deployed plugin target.

If the WASM target is unavailable, record that environmental gap in
`progress.md` and `review.md`; do not claim the check passed.

## Step 9 — inspect and commit the implementation unit

Inspect:

```text
git diff -- crates/lisa-core/src/types.rs
git diff --cached --name-only
git status --short
```

Commit the one meaningful source unit through the isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-034-01-01 \
  --message "feat: add attempt lease core contract" \
  --include crates/lisa-core/src/types.rs
```

If the installed binary lacks the command, locate or build the repository CLI
and invoke its equivalent with the same exact include path. Do not use ordinary
`git add` or `git commit`.

Record the resulting commit ID and command in `progress.md`.

## Step 10 — final ownership audit

Confirm:

- `crates/lisa-core/src/types.rs` has no remaining modified or untracked change;
- it is not staged in the ordinary index;
- unrelated pre-existing worktree changes remain present and untouched;
- ticket frontmatter phase/status fields were not edited;
- all ticket-owned source work is present in the isolated commit.

Use exact-path status/diff commands so the repository's unrelated dirty state
does not obscure the audit.

## Step 11 — write implementation and review artifacts

Write `progress.md` with:

- completed plan steps;
- exact source changes;
- tests and outcomes;
- implementation deviations;
- isolated commit evidence;
- worktree ownership audit.

Then write `review.md` with:

- change summary by file;
- acceptance-criterion evaluation;
- test coverage and gaps;
- compatibility assessment;
- open concerns and deferred integration work;
- critical human-review issues, if any.

Do not edit the ticket's phase or status. Stop after `review.md`; Lisa owns the
phase transitions and completion transaction.
