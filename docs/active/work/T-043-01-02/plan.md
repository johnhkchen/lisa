# Plan: pane-time ownership lookup

## Objective

Land a plugin-local pane-time ownership contract over terminal execution
provenance records and prove recycled-pane behavior without changing capture
writing or usage consumption.

## Step 1: confirm the implementation baseline

Inspect the ordinary working tree before source edits.

Expected unrelated/Lisa-managed changes:

- `docs/active/tickets/T-043-01-01.md` phase transition;
- `docs/active/tickets/T-043-01-02.md` phase transition.

Confirm neither planned source path has pre-existing modifications.

Verification:

- `git status --short` identifies only known pre-existing changes and private
  attempt artifacts remain ignored;
- `git diff -- crates/lisa-plugin/src/lib.rs` is empty;
- `crates/lisa-plugin/src/ownership.rs` does not already exist.

## Step 2: register the ownership module

Modify `crates/lisa-plugin/src/lib.rs` in the existing local-module declaration
group.

Add only:

```rust
mod ownership;
```

Do not import the lookup into `lib.rs` yet because no production consumer is
part of this ticket.

Verification:

- module declaration is positioned consistently with sibling modules;
- no other plugin state or behavior changes appear in the diff.

## Step 3: implement `owner_at`

Create `crates/lisa-plugin/src/ownership.rs`.

Add module and function documentation explaining:

- provenance execution rows are the durable interval source;
- matching keys on pane and time;
- endpoints are inclusive;
- different-ticket overlaps return no confident owner.

Implement the lookup over an `IntoIterator<Item=&ProvenanceRecord>`.

Use one optional borrowed ticket candidate and a linear scan.

Do not allocate, sort, parse files, or read scheduler state.

Verification:

- the return lifetime is tied to input records;
- same-ticket repeated matches retain one answer;
- different-ticket repeated matches return `None`;
- irrelevant panes and times are skipped.

## Step 4: add deterministic fixtures

Inside `ownership.rs`, add a native-only test module.

Create a fixture helper that accepts ticket, attempt, pane, start, and end, then
constructs a complete valid `ProvenanceRecord`.

Use deterministic route, outcome, authority, and empty usage fields for all
non-ownership data.

Verification:

- fixture lease ticket agrees with record ticket;
- timestamps and pane are explicit at call sites;
- no filesystem or wall clock is used.

## Step 5: prove the recycled-pane acceptance case

Create ticket A and ticket B records for the same pane with separated windows.

Assert the owner for:

- a timestamp inside A;
- A's start endpoint;
- A's end endpoint;
- a timestamp inside B;
- B's start endpoint;
- B's end endpoint.

Assert `None` for:

- a timestamp before A;
- a timestamp between A and B;
- a timestamp after B;
- a different pane during A's window.

This one test directly covers every acceptance criterion and locks the endpoint
contract.

## Step 6: prove ambiguity behavior

Create two overlapping intervals for the same ticket and assert the ticket is
returned.

Add an overlapping interval for a different ticket and assert `None` in both
forward and reversed input order.

Verification:

- lookup does not implement first-row or last-row wins;
- duplicate append evidence for one identity does not erase a confident owner.

## Step 7: format the source unit

Run normal workspace formatting after edits.

Command:

```text
cargo fmt --all
```

Then verify formatting produces no additional unexpected file changes.

Verification:

```text
cargo fmt --all -- --check
```

## Step 8: run focused tests

Run the new unit-test filter first for fast feedback.

Command shape:

```text
cargo test -p lisa-plugin owner_at
```

If the test names do not include the function name, use the ownership module or
specific test-name filter.

Verification criteria:

- recycled-pane test passes;
- ambiguity policy test passes;
- no ignored or flaky behavior;
- compilation works for the plugin's native test target.

## Step 9: run package tests

Run all native `lisa-plugin` tests.

```text
cargo test -p lisa-plugin
```

Verification criteria:

- all existing plugin tests pass;
- no scheduler, provenance, completion, signal, deadline, or UI regression;
- no new warnings tied to the ownership module.

## Step 10: run workspace verification

Run:

```text
cargo test --workspace
```

This exercises core and CLI consumers in addition to the plugin.

If an unrelated pre-existing failure occurs, capture the exact command, failure,
and evidence that the focused/package tests remain green in `progress.md` and
`review.md`. Do not hide or silently dismiss failures.

## Step 11: inspect the source diff

Review exact changes for both owned paths.

Expected diff:

- one module declaration in `lib.rs`;
- one new focused module containing documentation, lookup, fixture helper, and
  tests.

Check for accidental changes to:

- provenance schema;
- legacy usage reader;
- capture writer;
- ticket files;
- Cargo manifests or lockfile;
- unrelated formatting.

## Step 12: update implementation progress

Write `.lisa/attempts/T-043-01-02/1/work/progress.md` before the source commit.

Record:

- completed plan steps;
- source paths;
- lookup and ambiguity semantics;
- tests and their results;
- any deviation from this plan;
- exact intended commit ownership.

Private phase artifacts are not included in the source transaction.

## Step 13: commit the meaningful source unit

Commit the two mutually dependent source paths in one isolated Lisa transaction.

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-043-01-02 \
  --message "feat(plugin): add pane-time ownership lookup" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ownership.rs
```

Do not use `git add`, `git add -A`, ordinary `git commit`, or broad includes.

Verification criteria:

- command succeeds;
- commit contains exactly the two owned paths;
- neither owned path remains modified or untracked;
- ordinary index remains untouched;
- ticket frontmatter changes remain outside the commit.

## Step 14: verify the committed state

Inspect:

- `git show --stat --oneline HEAD` or the commit reported by Lisa;
- `git show --name-only --format=... <commit>`;
- `git status --short`;
- `git diff -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ownership.rs`;
- `git diff --cached -- ...` for owned paths.

If the shared branch advances concurrently, locate the reported commit rather
than assuming it remains `HEAD`.

The source unit is clean only when neither owned path is staged, modified, nor
untracked.

## Step 15: produce Review artifacts

Write `review.md` summarizing:

- the two source paths;
- the API and interval behavior;
- ambiguity policy;
- acceptance-criterion mapping;
- focused, package, and workspace verification;
- source transaction evidence;
- known limitations and downstream integration boundary.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

Use `block` instead only if an actionable correctness, test, or source-ownership
problem remains unresolved.

## Step 16: final hygiene and stop

After Review artifacts exist, perform a final status check.

Confirm:

- all ticket-owned source is committed through Lisa;
- no owned source remains staged, modified, or untracked;
- Review artifacts exist in the private attempt work directory;
- no phase/status field was manually edited;
- no shared work artifact was written directly.

Remain on `T-043-01-02` and stop. Lisa owns publication, completion commit, and
seat release.
