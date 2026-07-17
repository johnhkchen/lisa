# Research: T-049-08-01 identity remedy that works

## Ticket boundary

- The ticket is a stable-0.4.4 blocker in the completion-seal and project-history path.
- It requires remedies to cure the exact state that caused Lisa to print them.
- It also requires accepted history setup in an existing unborn repository to persist a local identity.
- Existing born repositories remain an explicit no-mutation boundary.
- Phase and status transitions are owned by Lisa and are not part of the source change.

## Completion-seal types

- `crates/lisa-core/src/completion.rs` defines `CommitSealUnavailable`.
- The enum has three variants: `RepositoryMissing`, `IdentityMissing`, and `TransactionUnavailable`.
- `RepositoryMissing` has no payload.
- `IdentityMissing` has no payload.
- `TransactionUnavailable` retains a diagnostic `detail` string.
- The enum's `Display` implementation comes from `thiserror` messages on the variants.
- `CommitSealSupport` is either `Available` or `Unavailable(CommitSealUnavailable)`.
- `ResolvedCompletionSeal` retains the unavailable reason when auto mode falls back to journal.
- Explicit commit mode returns a resolution error carrying the same typed reason.

## Native probe and hard preflight

- `crates/lisa-cli/src/completion_seal.rs` implements the native environment probe.
- `probe_commit_support` first runs `git rev-parse --show-toplevel`.
- Any failure at repository discovery currently becomes `RepositoryMissing`.
- A discovered root is canonicalized before later probes.
- An empty or unresolvable root becomes `TransactionUnavailable`.
- The identity probe is `git config --get user.email`.
- Empty output or command failure becomes `IdentityMissing`.
- Identity is checked before `HEAD`.
- Consequently, an unborn repository with no identity is classified as `IdentityMissing`.
- With identity present, an unborn repository reaches the `HEAD` probe and becomes `TransactionUnavailable`.
- Later Git metadata failures also become `TransactionUnavailable` with their raw prerequisite detail.
- `resolve_for_run_with` feeds the typed support result to the pure core resolver.
- Explicit commit errors are formatted by `format_preflight_failure`.
- That formatter currently appends `COMMIT_IDENTITY_REMEDIES` for every variant.
- The blanket append makes repository and transaction failures advertise identity configuration.

## Existing CLI remedy copy

- `COMMIT_IDENTITY_REMEDIES` lives in `crates/lisa-cli/src/completion_seal.rs`.
- It contains a heading, local-scope `git config` commands, and an init alternative.
- The commands omit `--global`, so they write repository-local configuration when run in a repository.
- The init alternative says to rerun `lisa init` and accept the history offer.
- A born repository receives no offer because init resolves its history action to `None` immediately.
- Therefore that alternative cannot cure identity absence in a born repository.
- An existing unborn repository does receive a history action when explicitly accepted.
- Before this ticket, that action creates a commit with environment-scoped author data only.
- Therefore it also fails to make the later `git config --get user.email` probe succeed.

## Doctor rendering

- `crates/lisa-cli/src/doctor.rs` calls `resolve_for_run` before rendering its report.
- Auto mode can succeed with a journal seal while retaining a commit-unavailable reason.
- `append_completion_seal_report` prints the seal line for successful auto resolution.
- It only adds a reason/remedy block when the retained reason is `IdentityMissing`.
- That block directly appends `COMMIT_IDENTITY_REMEDIES`.
- Other retained unavailable variants receive no remedy in doctor auto mode.
- Explicit commit mode prints the error returned by `format_preflight_failure`.
- Thus doctor currently has two formatting paths with only a partially shared string.

## Init repository states

- `crates/lisa-cli/src/init.rs` models `RepositoryState` as unavailable, missing, unborn, or born.
- `repository_state` discovers a containing Git root.
- A resolvable `HEAD` produces `Born`.
- A symbolic branch without a resolvable `HEAD` produces `Unborn { root }`.
- `HistoryAction` separates no action, repository creation, initial commit creation, and decline.
- `resolve_history_action` returns `None` for `Born` before prompting or interpreting flags.
- That early return is the born-repository no-mutation boundary.
- Missing state maps accepted history to `CreateRepository`.
- Unborn state maps accepted history to `CreateInitialCommit { root }`.
- Decline maps both missing and unborn states to no history mutation.

## Init identity mechanics

- `initialize_project_history` runs `git init` for a missing repository.
- It then writes local `user.name` as `Lisa (project history)`.
- It writes local `user.email` as `lisa@project`.
- It finally calls `create_initial_history_commit`.
- `create_initial_history_commit` creates an empty tree and a root commit.
- The root commit uses `GIT_AUTHOR_*` and `GIT_COMMITTER_*` environment variables.
- It advances `HEAD` with `git update-ref`.
- It does not persist either identity key.
- The existing unborn action calls only `create_initial_history_commit`.
- This explains why its commit has the Lisa author while local config remains unchanged.

## Init fixture coverage

- `crates/lisa-cli/tests/init_history.rs` exercises real CLI and Git commands.
- The fixture isolates `HOME`, global Git config, and system Git config.
- Bare-folder acceptance already asserts local Lisa name and email.
- It also asserts an empty initial commit and a later ticket-style commit.
- Decline asserts no repository is created.
- Born-repository coverage snapshots `.git`, config bytes, global config, and `HEAD`.
- The born fixture therefore protects the ticket's hard immutability boundary.
- Existing-unborn coverage creates both accepted and declined repositories.
- It snapshots the ordinary index to prove Lisa's isolated root creation preserves staged work.
- The declined side snapshots config and index bytes.
- The accepted side currently asserts config bytes remain unchanged.
- That accepted assertion conflicts with this ticket's required new local identity behavior.
- The accepted fixture already verifies the root commit author and empty tree.

## Seal visibility fixture coverage

- `crates/lisa-cli/tests/seal_visibility.rs` runs `doctor` and `status` as subprocesses.
- It can create absent, identityless, and commit-ready repository fixtures.
- Its identityless fixture is unborn because it initializes Git without creating a root commit.
- Existing doctor assertions pin the old combined identity/init remedy.
- There is no born identityless fixture today.
- There are no integration assertions for variant-specific hard-preflight remedy sets.
- The fixture already isolates Git configuration and can support new born/unborn cases.

## Plugin copy

- `crates/lisa-plugin/src/lib.rs` declares `HISTORY_IDENTITY_ASK` locally.
- Its wording is a single-line operator ask rather than the CLI's multiline block.
- It includes the same two Git commands and the init alternative.
- Completion failure classification groups unborn history and identity failures together.
- The shared class is `OperatorHistoryOrIdentity`.
- The class retries once and parks on the bounded second failure.
- The parked disposition and rendered rejection use `HISTORY_IDENTITY_ASK`.
- Multiple plugin regression tests compare the ask exactly.
- One field-replay test covers a real unborn, identityless repository.
- The plugin constant currently has no compile-time source relationship with CLI prose.
- Because the plugin already depends on `lisa-core`, core is an available shared-string boundary.

## Constraints and observations

- CLI and plugin crates cannot directly depend on one another without changing architecture.
- Both already depend on `lisa-core`.
- The plugin's combined history-or-identity class needs an ask valid for both raw failure families.
- The two local config commands alone cure identity in a born repository but do not create `HEAD`.
- Accepted init can cure an unborn repository once it persists local identity as required.
- The init alternative is therefore valid for the plugin's unborn side after this ticket.
- It remains invalid for a known born identity-only doctor result.
- Remedy selection in the native CLI has more repository context than the typed enum alone.
- A contextual fact is needed to decide whether the init alternative is valid for `IdentityMissing`.
- Existing user/Lisa worktree changes are limited to Lisa journals and ticket phase metadata.
- Ticket source files are clean before implementation.
