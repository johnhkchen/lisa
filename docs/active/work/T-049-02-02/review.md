# Review — T-049-02-02 doctor identity preflight

## Disposition

Pass.

The implementation satisfies both acceptance criteria.

The ticket-owned source is committed, formatted, verified, and clean.

No critical issue or ticket-owned follow-up remains.

## Change summary

### `crates/lisa-cli/src/completion_seal.rs`

Added a single crate-visible identity remedy contract.

It contains the two local Git configuration commands and the Lisa history-offer
alternative.

The existing explicit-commit preflight now uses that complete shared contract.

Added a read-only accessor for the automatic fallback reason retained inside
`RunCompletionSeal`.

The accessor preserves the typed `CommitSealUnavailable` boundary.

No Git behavior or core resolution matrix changed.

### `crates/lisa-cli/src/doctor.rs`

Doctor now validates completion capability with the same resolver used at loop
startup.

The resolver remains read-only.

Doctor retains a consolidated report even when explicit commit resolution
fails.

Successful completion checks continue to print the stable plain-language seal
line.

Automatic journal fallback caused by missing identity now includes the retained
typed reason.

Only identity fallback adds identity guidance.

Repository absence does not demand an identity.

An explicit commit request with missing identity is a hard doctor failure and
uses the same named preflight text as loop startup.

### `crates/lisa-cli/tests/seal_visibility.rs`

Expanded the compiled CLI fixture to cover repository capability states.

The fixture isolates global/system identity to make negative behavior
deterministic.

It covers missing identity, configured identity, no repository, and explicit
commit failure in addition to the existing seal visibility matrix.

## Acceptance criterion 1

Criterion: repo without identity fires with both remedies asserted verbatim;
repo with identity is silent; no repo defers to the journal seal line.

Pass.

`doctor_auto_names_missing_identity_and_both_remedies_verbatim` creates a real
temporary Git repository without local identity.

The fixture asserts the exact journal-only visibility line.

It asserts the typed reason:

`no commit identity is configured (git config user.email did not resolve)`

It asserts the complete remedy block as one verbatim substring.

That block includes both exact Git commands on separate lines.

It also includes the exact `lisa init` history-offer alternative.

The process succeeds because `auto` legitimately selected journal instead of
silently weakening or hard-failing.

`doctor_auto_is_silent_about_identity_when_repository_can_commit` creates a
repository with local name, email, and `HEAD`.

It asserts commit-sealed output.

It asserts the missing reason and the complete remedies are both absent.

`doctor_auto_without_repository_defers_to_journal_seal_line` creates no Git
repository.

It asserts journal-only output.

It asserts the identity reason and remedy block are absent.

Git executable availability remains a separate doctor dependency check; the
no-repository case does not confuse repository absence with missing Git.

## Acceptance criterion 2

Criterion: failure/remedy strings have one source shared with seal-resolution
preflight, and string tests pin them.

Pass.

The missing-identity failure reason has one production source in
`CommitSealUnavailable::IdentityMissing`.

Both doctor and explicit-commit preflight render that typed reason.

The remedy text has one production constant,
`COMMIT_IDENTITY_REMEDIES`, in `completion_seal.rs`.

Doctor references the constant directly.

`format_preflight_failure`, which is used by loop startup and doctor explicit
commit resolution, references the same constant directly.

There is no doctor-local production copy of either Git command or the
history-offer sentence.

The completion-seal unit test pins the complete constant and proves the hard
failure contains it intact.

The compiled CLI integration test independently pins the emitted contract.

`doctor_explicit_commit_uses_shared_missing_identity_hard_failure` asserts a
nonzero exit, `Completion seal preflight failed`, the exact configured guard
field, typed reason, and verbatim remedy block.

## Behavioral review

Explicit journal still invokes zero Git probes because doctor delegates to the
existing `resolve_for_run` journal short path.

Auto performs one probe and retains its result.

Explicit commit performs one probe and fails closed when support is absent.

Status retains its existing tier-only inspection behavior.

Loop startup retains its one-shot pinned resolution behavior.

The plugin receives no new state and performs no new probe.

Doctor writes no Git configuration and creates no repository.

The output gives operators both actionable paths without choosing an identity
for them.

## Test coverage

Focused compiled fixture suite: 5 passed, 0 failed.

Full lisa-cli package suite passed.

The main CLI unit suite reported 340 passed.

All enabled CLI integration suites passed.

Full workspace suite passed against a clean committed baseline plus only this
ticket's three files.

The workspace run covered 226 core tests and 409 plugin tests in addition to
CLI tests and enabled regression/property suites.

The existing real-Zellij integration remained ignored because it requires its
declared live external environment.

Formatting and commit whitespace checks passed.

## Verification environment note

The shared live worktree contained concurrent T-049-02-01 edits before this
ticket started.

At verification time those adjacent edits temporarily made `main.rs` call a
three-argument initializer while `init.rs` still exposed a two-argument API.

This ticket does not own either file.

Rather than alter or mask that neighboring attempt, package and workspace tests
ran in a clean archive of the committed baseline with only this ticket's exact
files overlaid.

This clean-baseline verification passed completely.

The ticket's three source paths are clean in the live worktree after commit.

The unrelated transient mismatch is not a defect or blocker in this commit.

## Commit and ownership

Commit `ec949c1af5489942aa0f001713dab910c98de9ab` contains the implementation and
fixtures.

It was created by `lisa commit-ticket` with three exact repository-relative
include paths.

No ordinary index or ordinary commit command was used.

The ordinary index is empty.

No ticket-owned file remains modified, staged, or untracked.

Remaining worktree entries are Lisa-managed state or concurrent adjacent-ticket
work.

## Open concerns

No blocking concern remains.

Doctor's explicit commit error continues to show identity remedies for other
commit-support failures, matching the pre-existing T-049-01-01 behavior. This
ticket only strengthens the missing-identity path and does not redesign remedies
for repository or transaction failures.

The identity probe intentionally checks `user.email`, matching the existing
transaction preflight and ticket wording. Git may accept identities from local,
global, system, or command-provided configuration; fixture isolation ensures
the negative case is stable.

The adjacent T-049-02-01 history initialization work may later change how the
offer is implemented, but this ticket depends only on its operator-facing
remedy wording and does not couple to that implementation.

## Handoff

Lisa can admit and publish the phase artifacts and prepare the Done commit.

Remain on T-049-02-02 until Lisa confirms completion and releases the seat.

