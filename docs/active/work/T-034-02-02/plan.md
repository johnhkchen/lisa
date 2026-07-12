# Plan: T-034-02-02 gate completion on current lease

## Objective

Require exact current attempt authority before any completion request can enter
the existing native commit transaction, while preserving current-attempt
completion behavior and all T-031 durability guarantees.

## Step 1 — carry authority in pending state

Add `CompletionAuthority::{Attempt, Operator}` and make `PendingCompletion` own
the validated authority.

Change its derive from `Copy` to `Clone` and update the result handler lookup
accordingly.

Verification:

- plugin compilation accepts the non-Copy pending value;
- all pending masks and retry reinsertion retain the same record;
- no result-path ordering changes.

## Step 2 — gate `request_completion`

Add `authority: Option<CompletionAuthority>` to the request method.

After pending deduplication, require a source lease whose `is_current` check
passes against `current_leases[ticket_id]`.

On rejection, log a warning and return false before dependency checks, pending
insertion, command construction, or host launch.

On acceptance, store the authority in `PendingCompletion` and continue through
the existing code. Accept Operator only for Manual requests.

Verification:

- missing source authority returns false;
- stale source authority returns false;
- exact current authority continues to pending state;
- no lease maps are mutated.

## Step 3 — thread artifact identity through completion

Extend `check_artifact_advances`' active-thread snapshot with the thread's
optional lease.

Pass that lease only at the Review-to-Done request boundary.

Keep all intermediate phase updates byte-for-byte equivalent aside from tuple
shape and loop destructuring.

Verification:

- Research through Review catch-up still works;
- Review completion with a leased thread creates a pending transaction;
- an unleased thread cannot request Done.

## Step 4 — thread idle identity through completion

Before each idle-triggered Done request, clone the active thread lease.

Pass the identity with `CompletionSource::Idle`.

Verification:

- Implement plus existing `review.md` reaches Review and requests completion;
- Review plus `review.md` requests completion;
- signal files are still consumed;
- ticket frontmatter remains non-Done while pending.

## Step 5 — bind stopped completion to its slot

In `auto_complete_review`, find the exact slot for the supplied pane and ticket
and clone its lease.

Pass that lease with `CompletionSource::Stopped(pane_id)`.

Verification:

- matching leased slot requests completion;
- missing, unstamped, or differently assigned slot is rejected centrally;
- no `/clear` transition behavior changes.

## Step 6 — thread manual and observed-Done identity

For manual completion, clone the selected active thread lease. When no thread
exists, use explicit Operator authority to preserve manual recovery; never
convert an unleased existing thread into an operator.

For observed Done, snapshot each active thread's lease beside its ticket ID.

Pass both through the same request boundary.

Verification:

- leased manual completion retains existing pending/retry behavior;
- observed Done cannot bypass lease admission;
- dependency and stale-slot reconciliation stay ordered as before.

## Step 7 — add test lease setup helper

Add `install_current_attempt` inside the test module.

Mint from the test state's high-water value, insert into both scheduler maps,
and stamp any matching thread and slot.

Verification:

- helper returns one exact value shared by every representation;
- repeated helper calls mint strictly increasing attempts;
- production code does not depend on the helper.

## Step 8 — add stale/current boundary regression

Create a focused Review ticket fixture with a current successor lease.

Call `request_completion` first with its predecessor and then with the current
lease.

Verify stale rejection has no pending state or lifecycle mutations and current
acceptance creates a pending record carrying the accepted lease.

This is the direct acceptance-criterion test.

## Step 9 — repair completion fixtures

Run the plugin tests and identify fixtures that correctly expect completion but
still model pre-lease scheduler state.

Install a current attempt after their thread/slot setup.

Do not blanket-stamp unrelated fixtures. Each update should document that the
test is modeling an authoritative scheduled attempt.

Verification:

- prior completion tests retain their semantic assertions;
- tests not concerned with completion remain unchanged;
- no test bypasses the production gate.

## Step 10 — focused verification

Run:

```text
cargo test -p lisa-plugin request_completion_rejects_stale_attempt_and_accepts_current_lease
cargo test -p lisa-plugin completion
cargo test -p lisa-plugin auto_complete_review
cargo test -p lisa-plugin idle_signal_review
```

Inspect any failures as either a real regression or an unfaithful pre-lease
fixture.

Acceptance:

- stale test passes;
- all relevant current-attempt completion paths pass;
- no current-path test changes its expected transaction behavior.

## Step 11 — broad verification

Run formatting and full validation:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p lisa-plugin --target wasm32-wasip1
```

If repository baseline or environment prevents a command, record exact output
and distinguish it from ticket-caused failures.

Acceptance:

- workspace tests pass;
- plugin builds for WASM;
- formatting is clean;
- no new Clippy warning is introduced.

## Step 12 — inspect ownership and diff

Review:

```text
git diff -- crates/lisa-plugin/src/lib.rs
git diff --check -- crates/lisa-plugin/src/lib.rs
git status --short
```

Confirm:

- only intended source hunks changed;
- ticket frontmatter phase/status is untouched;
- unrelated worktree changes remain outside the source diff;
- no ordinary-index staging was created.

## Step 13 — commit the source unit

Use Lisa's isolated transaction with the exact owned source path:

```text
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-02-02 \
  --message "Gate completion on current attempt lease" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add` or ordinary `git commit`.

Verify the source path is no longer modified and unrelated working-tree/index
state is preserved.

## Step 14 — record implementation progress

Write `progress.md` with:

- completed steps;
- source behavior implemented;
- test fixture updates;
- commands and results;
- isolated commit ID;
- deviations and rationale;
- remaining work, if any.

## Step 15 — self-review

Inspect the committed diff and re-evaluate:

- whether every request caller supplies source identity;
- whether stale/missing authority stops before command launch;
- whether current completion still uses the unchanged transaction;
- whether the new pending lease is preserved on retry paths;
- whether tests prove both halves of the acceptance criterion;
- whether artifact attribution limitations are explicitly deferred to
  T-034-02-03.

Write `review.md` summarizing files, behavior, coverage, limitations, and open
concerns. Stop after the artifact is complete; Lisa owns final phase/status and
artifact publication.
