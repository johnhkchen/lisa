# Plan: init-history-offer

## Implementation objective

Extend `lisa init` so an accepted project-history offer creates a commit-capable local repository with a resolvable root commit, while decline and all existing-repository paths preserve the ticket's safety rules.

## Working-tree discipline

- Inspect `git status --short` before source edits.
- Treat all pre-existing modified/untracked paths as foreign unless this ticket explicitly owns them.
- Do not use ordinary `git add`.
- Do not use ordinary `git commit`.
- Keep phase artifacts only in the attempt-private work directory.
- Commit source with one exact-path `lisa commit-ticket` transaction.
- Re-check the ordinary index and ticket-owned paths before Review.

## Step 1: Extend the CLI syntax

File: `crates/lisa-cli/src/main.rs`

Actions:

1. Add `with_history` and `no_history` booleans to `Commands::Init`.
2. Give each flag plain-language help.
3. Mark each flag as conflicting with the other.
4. Expand the `Commands::Init` match arm.
5. Convert flags into `init::HistoryPreference`.
6. Pass the typed preference to `init::run_init`.

Verification:

- `cargo test -p lisa-cli --test help_surface operator_help_matches_snapshots` will initially identify the expected snapshot delta.
- Running built help must show both flags.
- Supplying both flags must fail in argument parsing before init runs.

Atomicity:

- This step is not committed independently because it temporarily breaks the init function signature and help snapshot.

## Step 2: Add history domain types and copy

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Import buffered input, terminal detection, and process-command types.
2. Add constants for offer, decline consequence, identity, and initial commit message.
3. Add public `HistoryPreference`.
4. Add private `RepositoryState`.
5. Keep all mechanism details out of offer/decline constants.

Verification:

- Add unit assertion that the offer contains undo and agent-record benefits.
- Add unit assertion that offer/decline copy contains no `git` token.
- Compile-check visibility from `main.rs`.

## Step 3: Implement read-only repository detection

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Run `rev-parse --show-toplevel` at the requested project root.
2. Treat command failure as no discoverable repository.
3. Parse the successful top-level path.
4. Probe `rev-parse --verify HEAD` at that top-level path.
5. Return born or unborn state accordingly.

Verification:

- Fixture coverage for bare folders.
- Fixture coverage for nested folders inside a born repository.
- Fixture coverage for existing unborn repositories.

Safety check:

- The probe performs no writes.
- A nested folder never becomes a new repository when discovery succeeds.

## Step 4: Implement checked history commands

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Add one helper that checks launch and exit status.
2. Preserve useful stderr in failure messages.
3. Add empty-root-commit helper with command-scoped author/committer identity.
4. Add new-repository bootstrap helper.
5. Use local scope for required new-repository identity writes.
6. Never call config-writing helpers for an existing repository.

Verification:

- Accepted bare-folder fixture reads exact local name/email.
- Fixture inspects root commit identity.
- Fixture asserts root tree is empty.
- Existing-unborn fixture snapshots config before and after acceptance.

## Step 5: Implement preference resolution and prompting

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Add generic buffered-input prompt loop.
2. Flush after writing the prompt.
3. Treat empty/y/yes as acceptance.
4. Treat n/no as decline.
5. Retry other input.
6. Return an error for non-terminal `Ask` on missing/unborn state.
7. Skip the choice entirely for born repositories.
8. Ensure dry-run never blocks on input.

Verification:

- Unit-test yes, no, default, retry, and EOF behavior.
- Unit-test non-interactive missing-folder behavior.
- Black-box flag fixtures avoid reliance on terminal behavior.

## Step 6: Integrate history with init execution

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Change `run_init` signature to take `HistoryPreference`.
2. Detect stdin/stdout terminal state in the public wrapper.
3. Route to an internal I/O-aware function.
4. Resolve repository state before history decision.
5. Preserve existing project detection and plan output.
6. Apply accepted history bootstrap before scaffold writes.
7. Print the decline consequence only for relevant missing/unborn states.
8. Preserve all existing scaffold action behavior.
9. Preserve hook mode behavior.
10. Preserve changed-file report and next-step ordering.

Verification:

- Update legacy unit calls with explicit `NoHistory`.
- Run all `init.rs` unit tests.
- Ensure decline does not add repository paths to the changed-files report.
- Ensure accepted bootstrap reports project history separately from scaffold file mutations.

## Step 7: Preserve dry-run semantics

File: `crates/lisa-cli/src/init.rs`

Actions:

1. Keep the existing filesystem plan visible.
2. Print a history plan line for explicit acceptance/decline when relevant.
3. Do not initialize metadata.
4. Do not write config.
5. Do not create a root commit.
6. Do not prompt when no explicit choice is supplied.

Verification:

- Extend dry-run unit test with `.git` absence assertion.
- Add explicit accepted-history dry-run assertion.

## Step 8: Update help contract

File: `crates/lisa-cli/tests/help_surface.rs`

Actions:

1. Generate or inspect exact `lisa init --help` output.
2. Update the init snapshot with both options in Clap's emitted order.
3. Keep descriptions action-first and jargon-free.
4. Add conflict assertion if useful.

Verification:

- Run full `help_surface` test binary.
- Confirm all fourteen commands still resolve.
- Confirm operator jargon checks pass.

## Step 9: Build acceptance/decline fixtures

File: `crates/lisa-cli/tests/init_history.rs`

Actions:

1. Create isolated CLI environment helper.
2. Disable system config and pin global config path.
3. Add repository command assertion helpers.
4. Implement bare acceptance fixture.
5. Implement bare decline fixture.
6. Assert offer/decision copy requirements.
7. Assert local/global identity safety.
8. Assert root commit and empty tree.
9. Exercise real `commit-ticket` after bootstrap.
10. Assert status seal output in both cases.

Verification:

- `cargo test -p lisa-cli --test init_history -- --nocapture`.

## Step 10: Build existing-repository fixtures

File: `crates/lisa-cli/tests/init_history.rs`

Actions:

1. Create a born repository with local and global identity fixtures.
2. Snapshot `.git` recursively and record `HEAD`.
3. Initialize a nested Lisa project with explicit acceptance.
4. Assert no nested metadata and byte-identical repository snapshot/config.
5. Create an existing unborn repository for decline.
6. Assert decline preserves unresolved `HEAD` and config.
7. Create an existing unborn repository for acceptance.
8. Assert acceptance births `HEAD` without config changes.
9. Assert command-scoped Lisa commit identity.

Verification:

- Run the whole `init_history` binary repeatedly to catch fixture leakage.

## Step 11: Format and targeted verification

Commands:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli init::tests
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli --test init_history
```

If formatting fails:

- run `cargo fmt --all` as a mechanical rewrite;
- inspect ticket-owned diffs afterward;
- ensure unrelated files were not reformatted.

If a test fails:

- record the deviation in `progress.md` before changing plan-level behavior;
- fix only ticket-owned paths;
- rerun the narrowest failing test first.

## Step 12: Full verification

Command:

```text
cargo test --workspace
```

Acceptance:

- every workspace test passes;
- no fixture changes real user/global config;
- no test leaves processes or repositories outside temporary directories.

Optional proportional verification:

```text
just check
```

Use if workspace tests reveal target-specific concerns or if time permits. The ticket changes native CLI behavior only, so workspace tests are the primary required gate.

## Step 13: Review the diff before commit

Checks:

```text
git diff -- crates/lisa-cli/src/main.rs crates/lisa-cli/src/init.rs crates/lisa-cli/tests/help_surface.rs crates/lisa-cli/tests/init_history.rs
git diff --cached --name-only
git status --short
```

Review questions:

- Does accepted bare init always leave `HEAD` resolvable?
- Does the root commit avoid all user files?
- Are local identity writes limited to newly created repositories?
- Does existing-repository acceptance avoid config writes?
- Does decline preserve no-repository state?
- Does every offer-path sentence avoid `git`?
- Are flags mutually exclusive and deterministic?
- Are all legacy init tests explicitly non-interactive?

## Step 14: Commit the source unit

Use exactly:

```text
lisa commit-ticket \
  --ticket-id T-049-02-01 \
  --message "Add project history offer to init" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-cli/tests/help_surface.rs \
  --include crates/lisa-cli/tests/init_history.rs
```

If `lisa` is not available, locate the already-installed or already-built CLI and invoke its identical `commit-ticket` subcommand. Do not build Lisa merely to use it; building is authorized here only as normal development/test verification of source changes.

Commit acceptance:

- command prints a commit id;
- `HEAD` includes only the four exact paths;
- ticket-owned source paths are clean afterward;
- ordinary staged state is unchanged;
- unrelated worktree changes remain untouched.

## Step 15: Progress artifact

Create/update attempt-private `progress.md` throughout implementation.

Record:

- each completed implementation step;
- exact tests and results;
- deviations and rationale;
- commit id;
- final ownership/status checks.

Do not include progress.md in the source commit. Lisa publishes admitted artifacts separately.

## Step 16: Review artifacts

Write attempt-private `review.md` with:

- change summary by file;
- behavior decision table;
- test coverage and command results;
- safety analysis;
- known limitations/open concerns;
- source commit id.

Write `review-disposition.json` exactly as pass when all gates succeed:

```json
{"disposition":"pass","reason":null}
```

If a required gate cannot pass, write a block disposition with an actionable reason instead.

After both Review artifacts exist, remain on this ticket and stop.
