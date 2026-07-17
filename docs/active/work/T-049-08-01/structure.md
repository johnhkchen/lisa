# Structure: T-049-08-01 identity remedy that works

## Modified file: `crates/lisa-core/src/completion.rs`

- Add public string constants for the canonical identity configuration commands.
- Add the canonical combined plugin history/identity ask.
- Keep `CommitSealUnavailable` and all existing type shapes unchanged.
- Locate the constants near the completion-seal error taxonomy they explain.
- No new module is needed.

## Modified file: `crates/lisa-cli/src/init.rs`

- Extract local project-history identity writes into one helper.
- The helper accepts a repository root and returns `Result<(), String>`.
- It writes only `user.name` and `user.email` with `--local`.
- `initialize_project_history` calls the helper after `git init`.
- `HistoryAction::CreateInitialCommit` calls the helper before root-commit creation.
- `HistoryAction::None`, `Decline`, and dry-run branches remain free of identity writes.
- Existing commit-tree environment identity remains intact.

## Modified file: `crates/lisa-cli/src/completion_seal.rs`

- Replace the blanket `COMMIT_IDENTITY_REMEDIES` constant with variant-specific constants/functions.
- Import shared identity command bytes from `lisa-core::completion`.
- Extend `CommitProbeOutcome` with contextual remedy information.
- The context records whether accepted init can cure an identity-missing result.
- Explicit journal creates a default false context without probing Git.
- Repository and transaction results set false.
- Identity-missing probe results inspect `HEAD` to distinguish unborn from born for remedy purposes.
- `RunCompletionSeal` retains the contextual fact for doctor auto-mode rendering.
- Add an accessor or rendering method at crate visibility as needed.
- Add a `remedy_for(reason, context)` function returning the exact remedy string.
- `format_preflight_failure` delegates to this remedy selector.
- Repository remedy leads with `lisa init`.
- Identity remedy contains shared command bytes and optional init alternative.
- Transaction remedy tells the operator to repair the named prerequisite and rerun doctor.

## Modified file: `crates/lisa-cli/src/doctor.rs`

- Generalize `append_completion_seal_report` to render every unavailable reason.
- Obtain exact remedy text from `completion_seal` instead of naming a constant.
- Preserve the existing seal visibility lines.
- Preserve explicit-commit error rendering from `resolve_for_run`.
- Do not duplicate remedy prose in doctor.

## Modified file: `crates/lisa-cli/tests/init_history.rs`

- Change accepted-unborn expectations from config-byte identity to local Lisa identity.
- Keep the declined-unborn config and index byte snapshots.
- Keep the accepted ordinary-index snapshot.
- Add a completion-style ticket commit after accepted init.
- Run doctor and assert the identity diagnosis/remedy disappears.
- Continue checking root commit attribution and empty tree.
- Continue checking global Git config bytes.

## Modified file: `crates/lisa-cli/tests/seal_visibility.rs`

- Split identityless fixture state into unborn and born cases.
- Configure environment isolation so born can be created with temporary commit env identity without persisting config.
- Pin exact repository, identity, and transaction remedy blocks where exposed.
- Assert born identityless doctor output contains only the configuration commands.
- Assert it omits the init alternative and unrelated transaction/repository remedies.
- Exercise the printed config commands with real Git and prove a later commit succeeds.
- Keep existing seal visibility assertions.

## Modified file: `crates/lisa-plugin/src/lib.rs`

- Remove the local `HISTORY_IDENTITY_ASK` definition.
- Import or alias `lisa_core::completion::HISTORY_IDENTITY_ASK`.
- Existing exact comparisons and field replay continue using the same identifier.
- No completion classifier or retry behavior changes.

## Internal interfaces

- Shared core constants are immutable `&'static str` values.
- CLI remedy context remains private to `completion_seal.rs`.
- Doctor accesses only a crate-visible remedy rendering boundary.
- Init's identity helper remains private to `init.rs`.
- No public CLI API changes.
- No serialized format changes.
- No ticket schema changes.

## Ordering

1. Add shared core prose constants.
2. Switch plugin to the shared ask so drift is eliminated at compile time.
3. Extract and reuse init local-identity setup.
4. Introduce contextual variant remedy selection in completion seal.
5. Route doctor through the shared CLI selector.
6. Update unit and integration fixtures.
7. Format and run focused tests.
8. Run workspace tests/checks proportional to the change.

## Commit units

- Unit 1: shared completion remedy source and plugin adoption.
- Unit 2: accepted-unborn local identity plus init fixtures.
- Unit 3: variant-specific CLI/doctor remedies plus fixtures.
- Each unit uses `lisa commit-ticket` with exact paths.
- Phase artifacts remain in the private attempt directory for Lisa publication.

## Non-goals

- Do not change completion-seal tier resolution semantics.
- Do not alter the public unavailable enum variants.
- Do not change plugin retry counts or parking classification.
- Do not make init mutate born repositories.
- Do not write global identity.
- Do not update ticket phase/status manually.
