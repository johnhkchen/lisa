# Research — T-049-02-02 doctor identity preflight

## Ticket boundary

T-049-02-02 begins in Research and owns the operator-facing identity diagnosis
for `lisa doctor`.

The motivating failure is a repository that Git can discover but where Git
cannot resolve `user.email`.

The required output serves two audiences.

An operator who already has an identity needs the two local `git config`
commands.

An operator who does not have an identity needs the Lisa-native alternative:
rerun initialization and accept the history offer.

The ticket also requires the wording to be shared with the existing completion
seal startup preflight.

Attempt artifacts belong under the private `.lisa/attempts/.../work` directory.

Ticket phase and status frontmatter are managed by Lisa and must not be edited.

Ticket-owned source must be committed through `lisa commit-ticket` with exact
repository-relative paths.

## Existing completion-seal model

`lisa-core/src/completion.rs` defines configured and resolved seal vocabulary.

`CompletionSealMode` represents `auto`, `commit`, or `journal` configuration.

`CompletionSeal` represents only a pinned runtime tier: commit or journal.

`CommitSealSupport` is either available or unavailable with a typed reason.

`CommitSealUnavailable` distinguishes repository missing, identity missing, and
transaction unavailable.

`ResolvedCompletionSeal` retains the unavailable reason only when `auto`
chooses journal.

Explicit journal has no fallback reason because it is a chosen tier.

Explicit commit with unavailable support returns a typed resolution error.

This type boundary already distinguishes the exact condition needed by doctor.

## Native Git probe

`crates/lisa-cli/src/completion_seal.rs` owns environment probing.

`resolve_for_run` calls an internal `resolve_for_run_with` seam.

The seam accepts `FnOnce`, enforcing at most one support probe per resolution.

Explicit journal bypasses Git probing entirely.

Auto and explicit commit call `probe_commit_support`.

The probe starts with `git -C <root> rev-parse --show-toplevel`.

Failure at repository discovery becomes `RepositoryMissing` with no Git root.

The returned root is canonicalized and retained in `RunCompletionSeal`.

The identity check is `git config --get user.email` at the discovered root.

A nonzero command, empty stdout, or whitespace-only stdout becomes
`IdentityMissing`.

The probe then verifies `HEAD` and the absolute Git directory for current
isolated-transaction compatibility.

Those later failures become `TransactionUnavailable` with diagnostic detail.

The probe is read-only and is already the single source of environmental truth
used at real loop startup.

## Current hard-failure text

`completion_seal.rs` currently has a private `IDENTITY_REMEDY` constant.

It contains the two command lines:

`git config user.name "You"`

`git config user.email you@example.com`

`format_preflight_failure` embeds that constant in the explicit-commit error.

The error names `Completion seal preflight failed` and
`[guards].completion = "commit"`.

The existing resolver test asserts both command substrings.

No Lisa-history alternative currently appears in this hard failure.

The constant is private and doctor cannot currently reuse its text.

## Current inspection behavior

`resolve_for_inspection` is used by both doctor and status.

Explicit modes return their configured seal without probing.

Auto delegates to `resolve_for_run`, extracts only the seal, and discards the
typed unavailable reason.

Unexpected auto errors fail closed to displaying commit.

This behavior is sufficient for status, which only promises seal visibility.

It is insufficient for doctor because doctor must explain identity fallback.

In particular, explicit commit currently displays commit in doctor without
checking whether commit support exists.

## Current doctor flow

`crates/lisa-cli/src/doctor.rs` loads and resolves `.lisa.toml` first.

It selects the configured agent client and obtains the inspection seal.

It builds Zellij, Git, provider, embedded-WASM, and optional wasm-target checks.

Doctor always checks the Git executable as a required dependency.

That dependency check establishes executable availability, not repository or
identity readiness.

Project presence is currently inferred from whether `.lisa.toml` exists.

Doctor formats dependency reports before its project and completion sections.

`append_completion_seal_report` prints only `visibility_line(seal)`.

The final exit status depends on required `CheckReport` failures.

There is no repository-identity report or completion-resolution failure bit.

## Existing output contracts

`visibility_line` is shared by doctor and status.

Commit renders `completion seal: commit-sealed — finished work lands as history`.

Journal renders `completion seal: journal-only — finished work is recorded but not undoable`.

Unit tests in `doctor.rs` pin both lines.

`tests/seal_visibility.rs` executes the compiled binary for explicit commit and
journal fixtures and pins the same output.

Those fixtures deliberately do not create a repository.

Explicit modes currently avoid probing, so both cases succeed.

The status contract should remain unaffected by doctor-specific diagnosis.

## Existing fixture patterns

`tests/seal_visibility.rs` creates an isolated project, bin directory, and home.

It writes stub `zellij` and `claude` executables and prepends their directory to
`PATH`.

It uses the real Git executable inherited from the host when needed.

`tests/zellij_version_preflight.rs` initializes temporary Git repositories and
executes the compiled Lisa binary.

Unix permission helpers make fixture scripts executable.

Temporary `HOME` isolates normal global Git configuration, but system Git
configuration can still be disabled explicitly with `GIT_CONFIG_NOSYSTEM=1`.

Repository-local `user.email` is the positive fixture control.

## Relevant adjacent work

T-049-02-01 currently has uncommitted changes in `init.rs` and `main.rs`.

Those changes introduce the history offer and project-local Lisa identity.

They belong to another ticket and must be preserved and excluded from this
ticket's commits.

T-049-01-01 introduced completion-seal resolution and owns the existing hard
preflight in `completion_seal.rs`.

T-049-01-02 introduced seal visibility in doctor and status.

This ticket extends those existing boundaries rather than creating a second Git
probe implementation.

## Constraints and assumptions

The missing-identity diagnosis applies only after repository discovery.

No repository should not produce identity guidance.

Auto with no repository should simply display journal tier.

Auto with missing identity should display journal tier and explain that exact
fallback reason.

Explicit commit with missing identity must be a hard doctor failure.

Explicit journal must remain free of Git repository probing.

The Git executable check remains a separate dependency concern.

The acceptance phrase “silent” for a configured identity means no missing
identity or remedy copy, not no completion seal line.

The exact guidance literals must exist in one source location.

Tests should compare the complete shared guidance, not reconstruct duplicate
string fragments in production code.

No changes are required to core seal types, plugin code, or transaction code.

