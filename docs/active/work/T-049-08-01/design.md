# Design: T-049-08-01 identity remedy that works

## Goals

- Make accepted init in an existing unborn repository persist the project-local Lisa identity.
- Keep every born repository byte-untouched by init.
- Give each `CommitSealUnavailable` variant its own remedy.
- Include the init alternative for missing identity only when init can cure the observed state.
- Keep doctor and explicit preflight formatting on one remedy table.
- Prevent CLI/plugin command prose from drifting again.
- Prove the behavior with unit and subprocess fixture tests.

## Option 1: Keep the blanket string and only fix init

- This would make the init alternative valid for identityless unborn repositories.
- It would remain invalid for born identityless repositories.
- Repository and transaction failures would still receive irrelevant identity commands.
- It would fail the per-variant remedy acceptance criterion.
- Rejected.

## Option 2: Expand `CommitSealUnavailable::IdentityMissing`

- The variant could carry a `repository_state` or `init_can_cure` field.
- The formatter could then choose a remedy using only the enum.
- This changes a public core enum used by many tests and pattern matches.
- It couples the environment-independent completion type to native Git state semantics.
- The plugin operates from raw completion errors and would not benefit from the payload.
- The additional API churn is not necessary to satisfy the behavior.
- Rejected.

## Option 3: Retain typed variants and add native remedy context

- Keep `CommitSealUnavailable` unchanged.
- Extend the native probe outcome with whether accepted init can cure missing identity.
- Determine that fact read-only when identity is missing by probing `HEAD`.
- A missing `HEAD` with a symbolic repository branch means the current repository is unborn.
- A present `HEAD` means the repository is born and init must not be advertised.
- Feed the fact into a single CLI remedy selector.
- This keeps the core error taxonomy stable and places repository-state knowledge in the native CLI.
- Chosen.

## Init implementation choice

- Extract the two local `git config` commands from `initialize_project_history`.
- Name the helper around project-history identity rather than generic Git identity.
- Call it after `git init` on the missing-repository path.
- Call it on the accepted existing-unborn path before creating the initial commit.
- Continue using environment-scoped identity for `commit-tree` so commit attribution is deterministic.
- Do not call the helper from the born `HistoryAction::None` path.
- Do not call it from decline or dry-run paths.

## Remedy table

- `RepositoryMissing` receives a repository setup remedy led by `lisa init`.
- `IdentityMissing` always receives the two local config commands.
- `IdentityMissing` also receives the history-offer alternative only for an unborn repository.
- `TransactionUnavailable` receives a prerequisite/dependency remedy, not identity configuration.
- The typed reason remains printed immediately before the remedy.
- Exact remedy constants are pinned by unit tests.

## Doctor formatting

- Generalize doctor auto-mode rendering from identity-only to every retained unavailable reason.
- Print the retained reason for repository, identity, and transaction variants.
- Select the remedy through the same formatter/table used by hard preflight.
- Use the `RunCompletionSeal` contextual fact for identity remedy selection.
- Explicit commit mode receives a fully formatted hard error from the same selection path.
- Status remains a seal-only surface and does not need remedies.

## Shared CLI/plugin prose

- Move the exact combined history/identity operator ask to `lisa-core::completion`.
- The plugin imports and uses that constant instead of defining a local copy.
- Expose the two command spellings as shared core constants as well.
- Build CLI multiline identity remedies from exact shared text where practical.
- Add a unit assertion that the CLI remedy contains the shared command strings byte-for-byte.
- The plugin's combined ask remains valid: config commands cure identity, while accepted init cures unborn history after the init fix.
- Doctor does not use the combined plugin ask for born repositories, so it never advertises init there.

## Transaction remedy limits

- `TransactionUnavailable` carries arbitrary prerequisite detail.
- A static formatter cannot safely invent a command that repairs every possible detail.
- Its remedy will direct the operator to repair the named transaction prerequisite and rerun doctor.
- This is deliberately distinct from identity and repository setup instructions.
- Tests prove the mapping and exclusion boundaries rather than pretending arbitrary OS failures are auto-fixable.

## Test strategy

- Unit-test every variant through the remedy selector and hard-preflight formatter.
- Assert each variant includes its own remedy and excludes the other remedy sets.
- Unit-test identity remedies with both unborn and born context.
- Assert the CLI identity commands equal the shared core command bytes.
- Update the accepted-unborn init fixture to expect local Lisa identity.
- Preserve the declined-unborn config/index byte snapshots.
- Preserve the born-repository complete `.git` byte snapshot.
- Extend accepted-unborn coverage with a later ticket-style commit.
- Run doctor after accepted init and assert the identity reason is absent.
- Add a born identityless doctor fixture and assert only config commands are printed.
- Keep the bare-folder fixture unchanged to protect existing behavior.
- Keep plugin regression tests exact by importing the shared core ask.

## End-to-end cure evidence

- The unborn fixture starts in the actual stall state: repository, no `HEAD`, no local identity.
- `lisa init --with-history` is the printed valid alternative for that contextual identity failure.
- After it runs, local identity and `HEAD` resolve.
- A `commit-ticket` invocation proves the completion transaction can commit.
- `lisa doctor` then omits the missing-identity reason.
- The born fixture runs the printed local config commands through real Git.
- A completion-style commit then proves those commands cure the born identity gap.
- Repository-missing setup is already exercised by the bare init fixture and will be tied to exact remedy copy.

## Safety rationale

- Accepted history is the only existing-unborn path that gains config writes.
- The operator already authorized a root commit in that path.
- Local config changes are restricted to the discovered existing repository root.
- No global config is written.
- No ordinary Git index mutation is introduced.
- Born state continues returning `HistoryAction::None` before any history prompt/action.
- Decline and dry run retain no-write behavior.
