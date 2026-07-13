# Plan — T-045-03-01 claim is ownership proof

## Objective

Make a valid current assignment claim sufficient for the scheduler to publish
`Owned`, while keeping the preceding delivered state observable and requiring no
provider hook signal.

## Scope guard

Modify only the plugin signal/scheduler path and its focused tests.
Do not implement:

- hook/artifact evidence ranking;
- a delivered-awaiting-claim state;
- timeout or reinjection changes;
- launcher or assignment prompt changes;
- CLI producer changes;
- nonce revocation or ticket-boundary exit;
- dashboard label additions.

## Step 1 — establish the baseline

Run focused tests before editing:

```text
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
```

Record pass/fail counts in `progress.md`.
If a baseline failure is unrelated, inspect and document it before continuing.

Verification criteria:

- signal ingestion is green before the new family;
- source-order characterizations match the current poll;
- no ticket-owned source file is already modified or staged.

## Step 2 — add the typed claim signal family

Modify `crates/lisa-plugin/src/signal.rs`.

Actions:

1. import `AssignmentClaim`;
2. add `SignalRequest::Claims`;
3. add `SignalRecord::Claim`;
4. recognize exact `.claim` pane filenames;
5. read and deserialize the shared JSON type;
6. remove every recognized claim after acquisition attempt;
7. return no record for malformed bodies.

Tests:

- valid claim produces the exact typed record;
- recognized path is deleted;
- malformed claim is deleted and produces no record;
- malformed filename remains unowned by this consumer.

Run:

```text
cargo test -p lisa-plugin signal::tests
```

Verification criteria:

- no local wire type is introduced;
- full `u128` nonce survives parsing;
- raw hook payload handling remains unchanged;
- one-shot behavior matches other typed signals.

## Step 3 — implement authoritative scheduler admission

Modify `crates/lisa-plugin/src/lib.rs`.

Add `admit_assignment_claim(pane_id, claim)` near the current assignment
acknowledgement method.

Evaluate without mutation:

1. seat is not already owned;
2. seat has an active assignment generation;
3. pane resolves to a slot with ticket and lease;
4. claim ticket equals slot ticket;
5. claim attempt equals active generation;
6. slot lease equals the claim's ticket and attempt;
7. slot lease is current in `current_leases`;
8. retained assignment reference exists for the ticket;
9. retained assignment lease equals the slot lease;
10. retained nonce equals the claim nonce.

Then and only then insert `Owned` and return true.

Focused unit checks:

- exact claim succeeds;
- wrong nonce fails;
- wrong ticket fails;
- wrong attempt fails;
- missing retained reference fails;
- already-owned does not report another transition.

Verification criteria:

- rejected cases make no state mutation;
- path existence is not used as authority;
- current lease registry remains required;
- method is provider-neutral.

## Step 4 — wire claim consumption into polling

Modify `crates/lisa-plugin/src/lib.rs`.

Actions:

1. add `check_claim_signals` near other consumers;
2. ingest `SignalRequest::Claims`;
3. call scheduler admission for each typed claim;
4. bump activity only after success;
5. log claim-specific successful admission;
6. call the consumer after shell-ready and before hook acknowledgement;
7. add `claim` to lifecycle cleanup suffixes.

Verification criteria:

- claim evidence is processed before timeout evaluation;
- a corrected later claim can succeed after a rejected one;
- no hook file is created or required;
- stale cleanup cannot itself grant ownership.

## Step 5 — extend signal contract regressions

Modify `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

Actions:

1. add the exact typed claim request/record case;
2. assert final file deletion;
3. preserve strict vs broad recognition distinctions;
4. add claim consumer to the poll-operation order.

Run:

```text
cargo test -p lisa-plugin signal_ingestion_regression
```

Verification criteria:

- every request variant is exercised;
- claim parsing remains ingestion-only;
- poll order places claims after delivery/lifecycle prerequisites and before
  provider acknowledgement and timeouts.

## Step 6 — extend scheduler consumer characterization

Modify `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.

Actions:

1. update the expected consumer order;
2. add malformed recognized claim to the one-shot matrix;
3. add legacy claim filename to the idle-only legacy matrix;
4. construct a delivered current attempt with retained exact assignment;
5. prove wrong nonce is consumed but not admitted;
6. prove exact claim promotes and bumps activity;
7. assert a claim-specific activity message.

Run:

```text
cargo test -p lisa-plugin signal_consumer_characterization
```

Verification criteria:

- signal consumer policy is explicitly characterized;
- semantic rejection is distinct from ingestion success;
- current authority plus exact nonce is required.

## Step 7 — add the ticket acceptance test

Modify the high-level tests in `crates/lisa-plugin/src/lib.rs`.

Use a scheduling fixture to execute:

1. fresh dispatch;
2. exact process start;
3. delivery;
4. visible `delivering` output;
5. unowned assertion;
6. exact claim file publication;
7. claim consumer execution;
8. visible `owned` output.

Explicitly assert the hook path does not exist before claim admission.
Use the actual current lease and retained nonce produced by scheduling.

Suggested test filter:

```text
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
```

Verification criteria:

- delivery and ownership are distinct scheduler states;
- both are observable through scheduler output;
- no hook method or fixture payload performs the transition;
- the claim alone triggers `Owned`.

## Step 8 — format and focused regression

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
cargo test -p lisa-plugin claim
```

If formatting check fails, run `cargo fmt --all`, inspect all touched diffs, and
rerun the check.

Verification criteria:

- all new and directly adjacent tests pass;
- no warning is attributable to the change;
- formatting touches no unrelated source content.

## Step 9 — inspect the exact source diff

Run read-only audits:

```text
git diff -- crates/lisa-plugin/src/signal.rs
git diff -- crates/lisa-plugin/src/lib.rs
git diff -- crates/lisa-plugin/src/tests/signal_consumer_characterization.rs
git diff -- crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
git diff --check
git status --short
```

Confirm:

- only the four planned source paths are ticket-owned changes;
- unrelated `.lisa` and docs state is preserved;
- no ticket-owned path is staged in the ordinary index;
- no broad mechanical rewrite obscures the change.

## Step 10 — run package regression

Run:

```text
cargo test -p lisa-plugin
```

This is required because the ownership method and signal poll order are exercised by
many historical lifecycle and timeout tests beyond the focused filters.

Verification criteria:

- all native plugin tests pass;
- existing hook, startup, recovery, and UI behaviors remain green;
- no ignored live boundary test is accidentally treated as proof.

If a test reveals that the plan overlaps the next ticket, document the deviation in
`progress.md` before adjusting the source.

## Step 11 — commit the meaningful source unit

After focused and package tests pass, use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-045-03-01 \
  --message "feat(plugin): own assignments from exact claims" \
  --include crates/lisa-plugin/src/signal.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/tests/signal_consumer_characterization.rs \
  --include crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
```

Do not run `git add`, ordinary `git commit`, or any broad include.
Record the returned commit ID in `progress.md`.

Post-commit verification:

```text
git status --short -- <each ticket-owned source path>
git show --stat --oneline HEAD
```

Criteria:

- all ticket-owned source paths are clean;
- commit contains exactly the intended paths;
- unrelated ordinary-index entries, if any, remain untouched.

## Step 12 — run workspace regression

Run:

```text
cargo test --workspace
```

Verification criteria:

- core claim contract still passes;
- CLI claim producer tests still pass;
- plugin claim consumer tests pass;
- no cross-crate wire mismatch exists.

If shared-worktree activity changes source during the run, inspect the race and rerun
on a consistent snapshot rather than attributing unrelated work to this ticket.

## Step 13 — run production-oriented check

Run:

```text
just check
```

This covers the repository's declared quick check, including the WASM target and
native tests.

Verification criteria:

- plugin compiles for `wasm32-wasip1`;
- all workspace tests pass in the check recipe;
- no production-target import or filesystem issue is introduced.

## Step 14 — final source cleanliness audit

Run:

```text
git status --short
git status --short -- \
  crates/lisa-plugin/src/signal.rs \
  crates/lisa-plugin/src/lib.rs \
  crates/lisa-plugin/src/tests/signal_consumer_characterization.rs \
  crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
git diff --cached --name-only
```

Criteria:

- no ticket-owned source path is staged, modified, or untracked;
- no ordinary index state was created by this work;
- existing unrelated workflow/runtime changes are documented but untouched.

## Step 15 — complete progress artifact

Write `progress.md` in the attempt directory.
Include:

- baseline results;
- implementation by component;
- exact admission invariants;
- focused/package/workspace/WASM results;
- isolated commit ID and included paths;
- deviations and rationale;
- cleanliness audit;
- remaining ticket scope, if any.

Do not publish it directly to `docs/active/work`.

## Step 16 — Review

Write `review.md` with:

- disposition;
- change summary;
- file-by-file review;
- claim authority evaluation;
- acceptance coverage;
- regression coverage;
- verification commands/results;
- open concerns and deferred story scope;
- commit and cleanliness evidence.

Write `review-disposition.json` with exactly one valid shape.
Use pass only if implementation, tests, commit, and cleanliness all succeed.

## Failure handling

If authoritative admission cannot be implemented without changing the shared claim
schema or CLI producer:

1. stop source expansion;
2. record the exact missing invariant;
3. use a blocking review disposition with actionable reason.

If package tests expose historical hook assumptions:

1. preserve existing hook behavior in this ticket;
2. confirm claim-only ownership still works;
3. defer evidence ranking to T-045-03-02 as designed.

If `lisa commit-ticket` fails:

1. do not substitute ordinary Git commands;
2. inspect exact path ownership and command output;
3. retry only the isolated transaction when safe;
4. block Review if source cannot be made durable and clean.

## Done criteria

- Exact typed claims are ingested one-shot.
- Full scheduler authority and retained nonce checks gate admission.
- Delivered remains visibly unowned before claim.
- Exact claim alone promotes to visible owned with hook absent.
- Stale or wrong identity remains unowned.
- Focused, package, workspace, and production-oriented checks pass.
- Ticket-owned source is committed only through Lisa and clean.
- Review artifacts are complete in the private attempt directory.
