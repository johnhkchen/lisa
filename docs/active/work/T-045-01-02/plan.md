# Plan — T-045-01-02 claim command surface

## Objective

Deliver a hidden `lisa claim` plumbing command that accepts a current
ticket/attempt/nonce only when it matches the pane's E-034 lease marker and exact
published assignment, rejects prior-attempt and wrong-nonce claims with stable named
reasons, and atomically emits typed claim evidence for later scheduler admission.

## Preconditions

- Keep the ticket frontmatter untouched.
- Keep all phase artifacts in the private attempt work directory.
- Preserve unrelated dirty and untracked files.
- Use no ordinary-index `git add` or `git commit` command.
- Commit only exact ticket-owned source paths through `lisa commit-ticket`.
- Do not implement scheduler ownership promotion.

## Step 1 — establish a baseline

Run focused predecessor and CLI tests before source edits:

```text
cargo test -p lisa-plugin assignment::tests
cargo test -p lisa-cli --test help_surface
```

Verification:

- assignment writer tests pass with the existing private filename helper;
- the current help snapshot passes before adding the new command;
- any pre-existing failure is recorded in `progress.md` before implementation.

This step does not mutate source or the Git index.

## Step 2 — add the shared claim identity module

Create `crates/lisa-core/src/claim.rs`.

Implement:

- `AssignmentClaim` with ticket, attempt, and nonce;
- `ClaimRejection` with explicit named reasons;
- `ClaimRejection::name`;
- stable Display descriptions;
- `assignment_file_name`.

Add core unit tests for:

- normal and maximum numeric filename formatting;
- the complete ordered rejection-name table;
- claim JSON round-trip with hostile ticket characters and a nonce above `u64::MAX`.

Verification:

```text
cargo test -p lisa-core claim::tests
```

Criteria:

- all new tests pass;
- JSON round-trip retains the full `u128` nonce;
- reason names exactly match the CLI contract;
- no other core behavior changes.

## Step 3 — export and consume the shared filename contract

Modify `crates/lisa-core/src/lib.rs` to export the claim module.

Modify `crates/lisa-plugin/src/assignment.rs` to:

- import `assignment_file_name` from core;
- remove its private duplicate;
- leave writer and `AssignmentRef` behavior unchanged.

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-core claim::tests
cargo test -p lisa-plugin assignment::tests
```

Criteria:

- formatting is clean;
- core wire tests pass;
- predecessor assignment atomicity tests still pass;
- exact durable filename remains `assignment-{attempt}-{nonce}.md`.

## Step 4 — commit shared identity unit

Inspect only the intended diff:

```text
git diff -- crates/lisa-core/src/claim.rs crates/lisa-core/src/lib.rs crates/lisa-plugin/src/assignment.rs
```

Commit through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-045-01-02 \
  --message "feat(core): define assignment claim identity" \
  --include crates/lisa-core/src/claim.rs \
  --include crates/lisa-core/src/lib.rs \
  --include crates/lisa-plugin/src/assignment.rs
```

Verification:

- command returns a commit ID;
- the three paths have no remaining staged, modified, or untracked source state;
- unrelated worktree entries remain unchanged;
- ordinary index entries, if any, remain untouched.

Record the commit ID and tests in `progress.md`.

## Step 5 — implement the focused CLI validator/publisher

Create `crates/lisa-cli/src/claim.rs`.

Implement request and receipt values.
Implement semantic versus operational errors.
Implement the validation pipeline:

1. parse pane ID;
2. locate and parse the pane lease marker;
3. reject wrong ticket;
4. reject prior or non-current attempt;
5. derive the exact attempt-private assignment path;
6. reject a missing/non-file exact nonce;
7. reread and compare the lease marker;
8. serialize the claim;
9. write sibling temporary;
10. rename to the pane claim signal;
11. return the receipt.

Implementation checks:

- use `is_file`, not mere path existence;
- never scan for the newest assignment file;
- never infer a nonce from directory order;
- never write the final signal directly;
- clean up the command's temporary on publication failure;
- include path/action context in operational errors;
- do not consume or delete the durable lease marker.

## Step 6 — register `lisa claim`

Modify `crates/lisa-cli/src/main.rs`.

Add:

- `mod claim`;
- shared `AssignmentClaim` import;
- hidden `Claim` command with `path`, `ticket_id`, `attempt_id`, and `nonce`;
- dispatch using `LISA_PANE_ID`;
- exact success output;
- standard error/exit behavior;
- one curated plumbing-footer line.

Run the new command's generated help manually:

```text
cargo run -q -p lisa-cli -- claim --help
```

Criteria:

- all required arguments are visible and typed;
- `--path` defaults to `.`;
- no explicit pane argument bypasses scheduler routing context;
- top-level generated operator list still hides the command.

## Step 7 — add command-level acceptance tests

Create `crates/lisa-cli/tests/claim_cli.rs`.

Test 1: accepted current claim.

- marker: ticket T, attempt 2;
- exact assignment: attempt 2, nonce 100;
- invocation: same ticket, attempt, nonce;
- assert success and exact output;
- deserialize `pane-7.claim` as `AssignmentClaim`;
- assert no temporary residue.

Test 2: stale prior attempt.

- marker: ticket T, attempt 2;
- create valid assignment files for attempts 1 and 2;
- invoke the real nonce for attempt 1;
- assert nonzero exit and `[stale-attempt]`;
- assert no claim signal.

Test 3: wrong nonce.

- marker: ticket T, attempt 2;
- exact published assignment uses nonce 100;
- invoke nonce 101;
- assert nonzero exit and `[wrong-nonce]`;
- assert no claim signal.

Use an explicit `env_remove` for unrelated inherited Lisa identity variables where
appropriate, then set only `LISA_PANE_ID`.
Use `--path` to make project-root resolution part of the black-box test.

Run:

```text
cargo test -p lisa-cli --test claim_cli
```

Acceptance criterion is satisfied only if all three paths run through the built
binary and their filesystem effects match.

## Step 8 — update command-surface regression

Modify `crates/lisa-cli/tests/help_surface.rs`.

Update:

- plumbing command count from four to five;
- complete own-command count from 12 to 13;
- categorized arrays;
- exact top-level help snapshot;
- documentation comments carrying the old counts.

Run:

```text
cargo test -p lisa-cli --test help_surface
```

Criteria:

- `claim --help` resolves;
- `claim` appears only in the curated plumbing footer;
- operator command list and jargon checks remain unchanged;
- the exact snapshot passes.

## Step 9 — verify the complete CLI source unit

Run focused and package checks:

```text
cargo fmt --all -- --check
cargo test -p lisa-core claim::tests
cargo test -p lisa-plugin assignment::tests
cargo test -p lisa-cli --test claim_cli
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli
```

If a test exposes a plan deviation, record the deviation and rationale in
`progress.md` before changing the implementation.

Inspect exact diffs:

```text
git diff -- crates/lisa-cli/src/claim.rs crates/lisa-cli/src/main.rs crates/lisa-cli/tests/claim_cli.rs crates/lisa-cli/tests/help_surface.rs
```

Criteria:

- tests pass without a live provider or Zellij;
- errors carry named semantic reasons;
- successful JSON matches the shared type;
- no scheduler ownership code is touched.

## Step 10 — commit the CLI command unit

Commit through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-045-01-02 \
  --message "feat(cli): add lease-bound claim command" \
  --include crates/lisa-cli/src/claim.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/claim_cli.rs \
  --include crates/lisa-cli/tests/help_surface.rs
```

Verification:

- command returns a commit ID;
- all four paths are clean afterward;
- no ordinary-index staging was introduced;
- unrelated `.lisa` and documentation state remains outside the commit.

Record the commit ID in `progress.md`.

## Step 11 — workspace regression

Run the repository-level verification proportional to the cross-crate contract:

```text
cargo test --workspace
```

If available and time permits, run:

```text
just check
```

`just check` includes the WASM check and native tests according to project guidance.
The CLI and core changes are native, while the plugin import must still compile for
the WASM target.

Criteria:

- all workspace tests pass;
- WASM check passes if run;
- no new warnings attributable to the ticket appear;
- `git status --short` shows no ticket-owned source paths modified, staged, or
  untracked.

## Step 12 — Review handoff

Complete `progress.md` with:

- implemented units;
- test commands and results;
- commit IDs;
- deviations;
- remaining work state;
- clean ticket-owned path audit.

Write `review.md` covering:

- shared wire contract;
- command syntax and validation order;
- atomic claim signal publication;
- exact file changes and commits;
- focused and workspace test evidence;
- the deliberate producer-versus-scheduler authority boundary;
- deferred scheduler claim consumption and nonce revocation.

Write exactly one valid `review-disposition.json` shape.
Use pass only if all acceptance tests and relevant regressions succeed and every
ticket-owned source path is committed and clean.
Use block with a non-empty actionable reason if a required verification or contract
remains unresolved.

After both Review artifacts exist, remain on T-045-01-02 and stop.
