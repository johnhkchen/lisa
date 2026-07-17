# Progress: T-049-08-01 identity remedy that works

## Status

- Implementation is complete.
- All ticket-owned source and test changes are committed through `lisa commit-ticket`.
- The ordinary Git index is empty.
- No ticket-owned source file remains modified or untracked.
- Unrelated journal, ticket metadata, work artifacts, and neighboring-ticket changes were preserved.

## Completed unit 1: shared identity remedy source

- Added canonical local identity command strings to `lisa-core::completion`.
- Added the canonical combined history/identity operator ask to the same module.
- Removed the plugin-local `HISTORY_IDENTITY_ASK` prose definition.
- Imported the core constant into the plugin.
- Existing plugin classification, retry, park, and field-replay behavior remain unchanged.
- Added CLI assertions proving its command bytes are sourced from the same core strings.
- Commit: `157e8d1 Share completion identity remedy copy`.
- Included paths:
  - `crates/lisa-core/src/completion.rs`
  - `crates/lisa-plugin/src/lib.rs`

## Completed unit 2: accepted unborn identity

- Extracted `configure_project_history_identity` in init.
- Fresh repository initialization still uses the same local name and email values.
- Accepted existing-unborn history now also calls the local identity helper.
- Root commit construction remains an empty-tree `commit-tree` operation.
- The ordinary index is still preserved byte-for-byte across initial commit creation.
- Declined unborn repositories retain config and index byte snapshots.
- Born repository initialization remains a no-action history branch.
- Updated the unborn fixture to begin with an empty effective local email.
- Asserted accepted init writes `Lisa (project history)` and `lisa@project` locally.
- Asserted global Git configuration remains unchanged.
- Asserted a later isolated `commit-ticket` succeeds.
- Asserted doctor no longer prints the missing-identity reason after the cure.
- Commit: `56309fe Persist identity for accepted unborn history`.
- Included paths:
  - `crates/lisa-cli/src/init.rs`
  - `crates/lisa-cli/tests/init_history.rs`

## Completed unit 3: variant remedy table

- Replaced the blanket identity append with `remedy_for`.
- `RepositoryMissing` now prints only the project-history init remedy.
- `IdentityMissing` always prints the two repository-local config commands.
- Missing identity adds the init alternative only when a read-only probe confirms unborn history.
- `TransactionUnavailable` prints a dependency remedy rather than identity commands.
- Doctor auto mode now uses the same table for every retained unavailable reason.
- Explicit commit preflight uses the same table through `format_preflight_failure`.
- Added born identityless, unborn identityless, repository-missing, and transaction fixtures.
- Each fixture asserts required remedy bytes and excludes unrelated remedy sets.
- Commit: `fb2dfff Match completion remedies to failure variants`.
- Included paths:
  - `crates/lisa-cli/src/completion_seal.rs`
  - `crates/lisa-cli/src/doctor.rs`
  - `crates/lisa-cli/tests/seal_visibility.rs`

## Completed unit 4: contextual transaction cure and end-to-end proof

- Generalized the native context flag from identity-only to history-init remedy context.
- An unborn repository with identity but no `HEAD` receives a concrete init remedy.
- Other transaction prerequisite failures retain the generic dependency repair remedy.
- Added unit coverage for both transaction contexts.
- Added a real born-repository cure fixture.
- The fixture starts with a born repository and no effective identity.
- Doctor prints the exact two configuration commands and no init alternative.
- The fixture runs those commands, creates ticket-owned content, and runs `commit-ticket`.
- The completion commit succeeds and a second doctor run is commit-sealed and identity-silent.
- Commit: `86a8386 Prove contextual completion remedies cure stalls`.
- Included paths:
  - `crates/lisa-cli/src/completion_seal.rs`
  - `crates/lisa-cli/tests/seal_visibility.rs`

## Completed unit 5: every variant cured end to end

- Added persistent real-process cure fixtures for all three unavailable variants.
- Repository missing: doctor prints only init, init creates commit-ready history, and `commit-ticket` succeeds.
- Identity missing in a born repository: doctor prints only the two config commands, the fixture runs them, and `commit-ticket` succeeds.
- Transaction unavailable for an unborn `HEAD`: doctor prints the contextual history dependency remedy, accepted init creates `HEAD`, and `commit-ticket` succeeds.
- Every cure fixture reruns doctor and observes a commit-sealed result.
- Commit: `e5722e2 Exercise every printed remedy end to end`.
- Included path:
  - `crates/lisa-cli/tests/seal_visibility.rs`

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo test -p lisa-core completion::tests --lib` passed: 33 tests.
- `cargo test -p lisa-cli --bin lisa completion_seal::tests` passed: 7 tests.
- `cargo test -p lisa-cli --test init_history` passed: 7 tests.
- `cargo test -p lisa-cli --test seal_visibility` passed: 10 tests.
- `cargo test -p lisa-plugin completion_failure_classifier_is_conservative_and_asks_are_plain` passed.
- `cargo test -p lisa-plugin field_journal_replay_bounds_unborn_identityless_completion_and_cleans_lock` passed.
- `cargo test --workspace` passed on the final ticket snapshot.
- The full suite had zero failures.
- The real-Zellij delivery boundary remained intentionally ignored by its existing environment gate.

## Verification isolation

- A neighboring ticket modified separate shared-checkout source files concurrently.
- Focused builds in the shared checkout briefly observed that ticket's incomplete API migration.
- Verification therefore used a clean archive of this ticket's committed snapshot.
- The archive was supplied the repository's already-built valid WASM artifact so doctor integration tests exercised a real embedded plugin instead of the allowed empty development placeholder.
- No neighboring-ticket file was edited, staged, or included by this ticket.

## Plan deviations

- The planned three implementation commits became five.
- The fourth and fifth commits add stronger contextual transaction handling and real cure paths for every unavailable variant.
- This is within ticket scope and strengthens the end-to-end acceptance evidence.
- The shared checkout could not be used for final compilation while the neighboring ticket was mid-edit.
- A clean committed archive replaced it for deterministic verification.

## Remaining work

- Write `review.md`.
- Write the exact review disposition JSON.
- Run `lisa check-disposition T-049-08-01`.
- Stop on this ticket after Review.
