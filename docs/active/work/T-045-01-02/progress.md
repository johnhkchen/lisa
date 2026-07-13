# Progress — T-045-01-02 claim command surface

## Status

Implementation is complete in two meaningful committed source units.
Focused, package, workspace, and WASM-inclusive verification pass.
The ticket-owned path audit is complete.
Only Review artifacts remain.

## Baseline verification

Before source changes, ran:

```text
cargo test -p lisa-plugin assignment::tests
cargo test -p lisa-cli --test help_surface
```

Results:

- 2 assignment tests passed;
- 5 help-surface tests passed;
- no failures or warnings blocked implementation.

The baseline established that T-045-01-01's atomic writer and the then-current
command snapshot were green.

## Shared assignment/claim identity

Created `crates/lisa-core/src/claim.rs`.

Added `AssignmentClaim` with:

- `ticket_id: TicketId`;
- `attempt_id: u64`;
- `nonce: u128`.

The type derives Serde serialization and exact equality.
It is the provider-neutral JSON body written by the CLI now and available to the
later plugin claim consumer.

Added `ClaimRejection` with eight typed reasons:

- pane unavailable;
- lease unavailable;
- invalid lease;
- wrong ticket;
- stale attempt;
- attempt mismatch;
- wrong nonce;
- lease changed.

Each reason exposes an explicit kebab-case machine name and a separate descriptive
Display message.
The CLI renders semantic failures as:

`claim rejected [<name>]: <description>`

Added the shared `assignment_file_name` helper.
It returns `assignment-{attempt}-{nonce}.md` and is now the one filename contract for
both halves of S-045-01.

Modified `crates/lisa-core/src/lib.rs` to export the new module.

Modified `crates/lisa-plugin/src/assignment.rs` to import the shared filename helper
and remove its private duplicate.
The assignment writer, `AssignmentRef`, atomic publication, and delivery wiring did
not otherwise change.

## Shared contract tests

Added three core unit tests:

1. filename formatting for a normal identity and `u64::MAX`/`u128::MAX`;
2. the complete stable rejection-name table;
3. JSON round-trip of hostile ticket text and a nonce larger than `u64::MAX`.

Ran:

```text
cargo test -p lisa-core claim::tests
cargo test -p lisa-plugin assignment::tests
```

Results:

- 3 core claim tests passed;
- 2 assignment tests passed;
- the predecessor writer continues to publish the exact shared filename.

## Shared identity commit

Committed through Lisa's isolated transaction with only:

- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-plugin/src/assignment.rs`.

Commit:

`69b3f9b8392ecd42ac6e9a3e9156d9da5b5017c8 feat(core): define assignment claim identity`

The three source paths were clean after the transaction.
Unrelated runtime and documentation files remained outside the commit.

## CLI validation and publication module

Created `crates/lisa-cli/src/claim.rs`.

The module accepts a request containing:

- resolved project root;
- raw optional `LISA_PANE_ID`;
- typed `AssignmentClaim`.

Validation now performs these checks in order:

1. `LISA_PANE_ID` parses as a `u32`;
2. `.lisa/signals/pane-{pane}.lease` is a regular file;
3. its body deserializes as `AttemptLease`;
4. the claim ticket equals the lease ticket;
5. the claim attempt equals the lease attempt;
6. a lower attempt is named `stale-attempt`;
7. a higher attempt is named `attempt-mismatch`;
8. the exact shared nonce-bearing assignment path is a regular file;
9. rereading the marker yields the same complete lease.

The exact checked assignment path is:

`.lisa/attempts/{ticket}/{attempt}/work/assignment-{attempt}-{nonce}.md`

The validator never scans an attempt directory or chooses a file by ordering.
Missing/non-file exact identity is rejected as `wrong-nonce`.

After validation, the module serializes `AssignmentClaim` and writes it to a hidden
same-directory temporary containing pane ID, process ID, and a wall-clock nonce.
It renames that complete temporary over:

`.lisa/signals/pane-{pane}.claim`

Rename failure removes the temporary and reports an operational error.
The durable signal is returned only after rename succeeds.

## CLI command registration

Modified `crates/lisa-cli/src/main.rs`.

Added hidden plumbing syntax:

```text
lisa claim \
  --path <project-root> \
  --ticket-id <ticket> \
  --attempt-id <u64> \
  --nonce <u128>
```

`--path` defaults to `.`.
Pane routing intentionally comes only from `LISA_PANE_ID`; there is no explicit pane
argument that could detach the claim from scheduler launch context.

On success the command prints:

`Claim accepted: {ticket} attempt {attempt} nonce {nonce}`

On failure it uses the repository's normal `Error: ...` stderr prefix and exits 1.

Added `claim` to the curated plumbing help footer.
Shifted the two Git transaction commands' display-order numbers without changing
their syntax or behavior.

## Command-level acceptance tests

Created `crates/lisa-cli/tests/claim_cli.rs`.

The tests invoke `CARGO_BIN_EXE_lisa`, set only `LISA_PANE_ID`, and use a temporary
project passed through `--path`.

Accepted-current test:

- creates attempt-2 pane lease;
- creates assignment nonce 100;
- invokes attempt 2 / nonce 100;
- asserts exit 0 and exact stdout;
- deserializes the durable signal as `AssignmentClaim`;
- asserts no claim temporary residue.

Stale-prior test:

- creates attempt-2 pane lease;
- creates real assignment files for attempts 1 and 2;
- invokes attempt 1 with its actual old nonce;
- asserts `[stale-attempt]` and no claim signal.

Keeping the stale assignment file present proves the lease comparison, rather than
file absence, rejects the predecessor.

Wrong-nonce test:

- creates matching attempt-2 lease;
- creates nonce 100;
- invokes nonce 101;
- asserts `[wrong-nonce]` and no claim signal.

All 3 command-level tests passed.

## Help regression

Modified `crates/lisa-cli/tests/help_surface.rs`.

Updated:

- own command count from 12 to 13;
- plumbing command count from 4 to 5;
- exact top-level footer snapshot;
- categorized arrays;
- count-bearing comments and test name.

All 5 help-surface tests passed.

## Focused and package verification

Ran after the CLI implementation:

```text
cargo fmt --all -- --check
cargo test -p lisa-core claim::tests
cargo test -p lisa-plugin assignment::tests
cargo test -p lisa-cli --test claim_cli
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli
```

Results:

- formatting check passed;
- 3 core claim tests passed;
- 2 plugin assignment tests passed;
- 3 CLI claim command tests passed;
- 5 CLI help tests passed;
- all `lisa-cli` package tests passed:
  - 14 library tests;
  - 269 binary unit tests;
  - integration suites passed;
  - the real-Zellij boundary test remained intentionally ignored by its existing
    environment gate;
  - doc tests passed.

No ticket-attributable compiler warning remains.

## Deviations from the plan

The planned `ClaimReceipt` initially carried both claim identity and the emitted
signal path.
The command only needs the identity for stdout, while black-box tests verify the path
directly.
Keeping the unused path produced a compiler dead-code warning.
The receipt was narrowed to the claim identity after successful publication.
This removes unused API surface without changing validation or filesystem behavior.

The help-surface file had already gained a fifth pinned property and operator-help
snapshots from concurrent prerequisite work by the time implementation began.
The edit was rebased onto that current form and changed only claim-related counts,
arrays, and footer text.
No concurrent operator-help content was overwritten.

## Authority boundary retained

The CLI validates durable E-034 pane identity and exact assignment-file evidence.
The pane marker is not a replacement for the plugin's authoritative
`State::current_leases` map.
The signal producer cannot inspect the plugin's retained `assignment_refs` map.

T-045-03-01 must consume `pane-{pane}.claim` and compare all of:

- strict pane routing;
- slot lease;
- current lease;
- retained assignment lease;
- retained assignment nonce;
- claim ticket, attempt, and nonce.

That dependent scheduler admission is outside this ticket, as are ownership promotion,
claim timeout policy, and nonce revocation.

## CLI command commit

Committed through Lisa's isolated transaction with only:

- `crates/lisa-cli/src/claim.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/claim_cli.rs`;
- `crates/lisa-cli/tests/help_surface.rs`.

Commit:

`d02d93b2a25c82a5af4bef08db80bc2c63c32594 feat(cli): add lease-bound claim command`

Lisa returned the commit ID successfully.
The new command module, command tests, and help regression are clean in the worktree.

## Workspace regression

Ran:

```text
cargo test --workspace
```

The first invocation encountered a shared-worktree race with concurrent ticket
T-045-02-01.
That ticket created `crates/lisa-cli/tests/codex_launcher.rs` after Cargo had compiled
the CLI binary but before integration-test discovery reached the new test.
The newly discovered test therefore ran against the previously compiled binary and
reported that its unrelated `launch-codex` command was unrecognized.

Inspection showed:

- this ticket's HEAD remained `d02d93b`;
- T-045-02-01 had concurrently added an untracked launcher module and test;
- its only tracked modification was an additive `main.rs` launcher command diff on
  top of this ticket's committed claim command;
- no claim source or test had changed;
- the failure was not reproducible once Cargo saw one consistent worktree snapshot.

Reran `cargo test --workspace` after the neighboring unit was present consistently.
The rerun passed:

- 19 CLI library tests;
- 269 CLI binary unit tests;
- all CLI integration suites, including 3 claim tests and the neighboring launcher
  test;
- 200 core unit tests;
- 2 core integration tests;
- 387 plugin tests;
- CLI/core doc tests;
- the existing real-Zellij test remained intentionally ignored by its environment
  gate.

No test failed on the stable rerun.

## WASM-inclusive check

Ran:

```text
just check
```

Results:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- the workspace test suite passed again;
- the plugin compiled with its new core filename-helper import on the production
  WASM target;
- no ticket-attributable warning or failure appeared.

## Final source and index audit

Both ticket commits pass `git show --check`.

Clean at HEAD/worktree:

- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-cli/src/claim.rs`;
- `crates/lisa-cli/tests/claim_cli.rs`;
- `crates/lisa-cli/tests/help_surface.rs`.

`crates/lisa-cli/src/main.rs` contains a foreign, uncommitted additive diff from
concurrent T-045-02-01 (`mod codex_launcher`, `LaunchCodex`, and its dispatch arm).
This ticket's claim additions in the same file are already committed in `d02d93b`.
The foreign diff was neither edited nor included in either T-045-01-02 transaction.

`git diff --cached --name-status` is empty.
This ticket used no ordinary-index staging or commit command.
Unrelated runtime ledgers, epic/story/ticket materialization, and T-045-02-01 worktree
files remain outside this ticket's commits.

## Remaining step

Complete Review artifacts and disposition, then remain on this ticket.
