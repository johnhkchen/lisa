# Review: T-049-08-01 identity remedy that works

## Disposition

- Ready to complete.
- All acceptance criteria have implementation and executable test evidence.
- No ticket-owned source changes remain outside Lisa-managed commits.
- No critical issue, TODO, or operator action remains.

## Outcome

Lisa now prints a remedy that matches the unavailable completion-seal state.
Accepted project history in an existing unborn repository also persists the
same project-local Lisa identity used for a fresh repository. The change closes
the loop in which doctor could send an operator back through init and still
observe the same missing identity.

## Source changes

### `crates/lisa-cli/src/init.rs`

- Extracted the local project-history identity setup into one helper.
- Fresh repository setup still writes `Lisa (project history) <lisa@project>`.
- Accepted existing-unborn setup now writes the same local identity.
- Existing-unborn setup still creates an empty-tree initial commit.
- Born repositories still resolve directly to no history action.
- Decline and dry-run paths still avoid history identity writes.

### `crates/lisa-cli/src/completion_seal.rs`

- Replaced the blanket identity remedy append with a typed remedy table.
- Added read-only context for whether accepted init can repair the observed state.
- `RepositoryMissing` now leads with the init remedy.
- `IdentityMissing` always provides the two local config commands.
- The identity init alternative appears only for an unborn repository.
- An unborn `HEAD` transaction failure receives a concrete project-history dependency remedy.
- Other transaction failures receive only the generic named-dependency repair guidance.
- Explicit commit preflight and doctor use the same selector.

### `crates/lisa-cli/src/doctor.rs`

- Doctor now renders the retained reason and selected remedy for all unavailable variants.
- It no longer has a separate identity-only string append.
- Auto journal fallback stays successful while still explaining the unavailable commit tier.

### `crates/lisa-core/src/completion.rs`

- Added canonical bytes for the local name and email commands.
- Added the canonical combined history/identity plugin ask.
- Kept the public `CommitSealUnavailable` enum shape unchanged.
- Kept seal resolution semantics unchanged.

### `crates/lisa-plugin/src/lib.rs`

- Removed the locally defined `HISTORY_IDENTITY_ASK` copy.
- Imports the core constant instead.
- Retry, classification, parking, and journal behavior are unchanged.

## Test changes

### `crates/lisa-cli/tests/init_history.rs`

- Existing-unborn acceptance begins with an empty effective identity.
- Accepted init must resolve local `user.name` and `user.email` to Lisa values.
- The fixture proves global configuration remains byte-identical.
- It proves the existing ordinary index remains byte-identical.
- It proves the initial root commit has an empty tree and correct author.
- It proves a later isolated completion-style commit succeeds.
- It proves doctor is silent about the cured identity gap.
- Declined unborn config/index snapshots remain intact.
- Born repository metadata/config/HEAD snapshots remain intact.
- Bare-folder history behavior remains covered without weakening assertions.

### `crates/lisa-cli/tests/seal_visibility.rs`

- Added distinct unborn identityless, born identityless, and unborn-with-identity fixtures.
- Pins each remedy block verbatim.
- Asserts unrelated remedy blocks are absent for every variant.
- Pins born missing identity to only the two config commands.
- Pins unborn missing identity to config commands plus the valid init alternative.
- Pins unborn transaction failure to the concrete history dependency remedy.
- Adds real doctor-to-cure-to-completion flows for all three enum variants.
- Every cure flow ends with a successful `commit-ticket` and commit-sealed doctor result.

### Unit and plugin regressions

- The CLI unit table covers all unavailable variants and both contextual branches.
- CLI assertions prove the identity command bytes come from the core constants.
- The plugin imports its whole ask from core, making drift a compile-time source issue.
- Existing plugin classifier and preserved field-journal replay tests pass unchanged.

## Acceptance criteria evidence

### Unborn accepted offer

- `existing_unborn_repository_acceptance_adds_commit_ready_local_identity` proves local identity resolution.
- The same fixture proves the initial commit and a subsequent ticket commit succeed.
- Its final doctor assertion proves the original diagnosis is absent.

### Born identityless repository

- `doctor_auto_born_missing_identity_prints_only_config_commands_verbatim` pins exact output.
- It excludes both init alternatives and both transaction/repository remedies.
- `born_identity_commands_printed_by_doctor_cure_a_completion_commit` runs the commands and completes.

### Bare and born safety boundaries

- `bare_folder_default_creates_commit_ready_project_history` continues to pass.
- `folder_inside_born_repository_leaves_repository_metadata_and_config_unchanged` continues to pass.
- Declined unborn config and ordinary-index snapshots continue to pass.
- No born init source branch gained a write.

### Per-variant remedy mapping

- `every_unavailable_variant_maps_to_its_own_remedy_set` covers the formatter directly.
- Repository, identity, unborn transaction, and generic transaction cases exclude blanket identity text.
- Doctor subprocess fixtures independently cover the same mapping.

### Single-source plugin/CLI identity copy

- `IDENTITY_NAME_COMMAND` and `IDENTITY_EMAIL_COMMAND` live in core.
- CLI remedies interpolate those constants.
- The plugin imports the complete core `HISTORY_IDENTITY_ASK` constant.
- `cli_and_plugin_identity_commands_are_byte_sourced_from_core` pins their relationship.

### End-to-end stall-to-cure

- `repository_remedy_printed_by_doctor_cures_a_completion_commit` covers `RepositoryMissing`.
- `born_identity_commands_printed_by_doctor_cure_a_completion_commit` covers `IdentityMissing`.
- `unborn_transaction_remedy_printed_by_doctor_cures_a_completion_commit` covers `TransactionUnavailable`.
- Each test starts from the named doctor reason, runs the printed remedy, commits, and rechecks doctor.

## Verification results

- Formatting check passed.
- Core completion tests: 33 passed.
- CLI completion-seal unit tests: 7 passed.
- Init-history integration tests: 7 passed.
- Seal-visibility integration tests: 10 passed.
- Plugin classifier focused regression: passed.
- Plugin preserved field-journal replay regression: passed.
- Final `cargo test --workspace`: passed with zero failures.
- The existing real-Zellij environment-gated test remained ignored as designed.

## Commit review

- `157e8d1` shares identity remedy copy through core.
- `56309fe` persists identity for accepted unborn history.
- `fb2dfff` maps CLI and doctor remedies by failure variant.
- `86a8386` adds contextual transaction cure behavior and a real identity cure.
- `e5722e2` exercises every printed remedy end to end.
- Every commit used exact repository-relative include paths through `lisa commit-ticket`.
- The ordinary index is empty.

## Open concerns

- Arbitrary transaction prerequisite failures cannot have one invented shell command; their retained detail and dependency guidance remain the honest remedy.
- The concrete field-relevant unborn `HEAD` transaction state does receive and pass a first-attempt init cure.
- No release-blocking concern remains.
