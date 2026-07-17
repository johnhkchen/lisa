# Progress — T-049-02-02 doctor identity preflight

## Status

Implementation is complete.

All ticket-owned source is committed through Lisa's isolated transaction.

Focused fixture, package, workspace, formatting, and ownership checks pass.

No ticket-owned source remains staged, modified, or untracked.

## Research through Plan

Read `CLAUDE.md`, the ticket, and the complete RDSPI workflow before editing.

Mapped the existing core seal types, native Git probe, doctor report flow, and
compiled CLI fixture patterns.

Confirmed the existing completion probe already distinguishes repository
missing from identity missing.

Confirmed `ResolvedCompletionSeal` already retains the automatic fallback
reason.

Confirmed doctor was discarding that reason by calling the tier-only inspection
helper.

Confirmed the explicit-commit loop hard failure already lived in
`completion_seal.rs` with a partial identity remedy.

Wrote `research.md`, `design.md`, `structure.md`, and `plan.md` in the private
attempt work directory before implementation.

## Completed source work

### Shared identity guidance

Modified `crates/lisa-cli/src/completion_seal.rs`.

Replaced the private two-command remedy with one crate-visible complete remedy
block.

The block contains the exact local identity commands:

`git config user.name "You"`

`git config user.email you@example.com`

The same block contains the Lisa-native alternative:

`Or rerun lisa init and accept the history offer`, with `lisa init` rendered as
code in actual output.

`format_preflight_failure` now appends that shared block verbatim.

This means loop startup and doctor explicit-commit diagnosis use one remedy
source.

The typed `CommitSealUnavailable::IdentityMissing` display remains the one
source for the missing-identity reason.

### Retained reason access

Added `RunCompletionSeal::commit_unavailable`.

The accessor borrows the already-pinned typed reason from the core resolution.

No new Git probe, boolean, or duplicate resolution path was introduced.

The auto resolver test now checks the public wrapper accessor.

### Doctor completion validation

Modified `crates/lisa-cli/src/doctor.rs`.

Doctor now calls the same read-only `resolve_for_run` preflight used by real
loop startup.

Explicit journal still bypasses the Git probe through the resolver's existing
short path.

Auto with available commit support still reports commit-sealed.

Auto with a missing repository still reports journal-only without identity
guidance.

Auto with missing identity reports journal-only, then names the typed reason and
prints both shared remedies.

Explicit commit with unavailable support prints the existing named completion
seal preflight failure.

Doctor does not early-return on that failure, so dependency, project, cache, and
trust sections remain available in the consolidated report.

Doctor's process result now fails when explicit completion resolution fails in
addition to its existing required-dependency failures.

### Compiled fixture coverage

Modified `crates/lisa-cli/tests/seal_visibility.rs`.

Added repository fixture states for absent, missing identity, and configured
identity.

Fixture Git resolution is isolated with a temporary `HOME` and
`GIT_CONFIG_NOSYSTEM=1`.

The positive fixture configures a local name and email and creates an empty root
commit so the existing transaction-readiness probe is fully satisfied.

The auto missing-identity fixture asserts the journal line, exact typed reason,
and complete shared remedies.

The auto configured-identity fixture asserts the commit line and the absence of
all missing-identity guidance.

The auto no-repository fixture asserts the journal line and the absence of all
identity guidance.

The explicit commit fixture asserts nonzero exit, named preflight, configured
field, exact typed reason, and complete remedy block.

Existing explicit doctor/status visibility coverage remains intact.

## Commit

Ran the required isolated transaction with exact paths:

`lisa commit-ticket --ticket-id T-049-02-02 --message "Diagnose missing commit identity in doctor" --include crates/lisa-cli/src/completion_seal.rs --include crates/lisa-cli/src/doctor.rs --include crates/lisa-cli/tests/seal_visibility.rs`

Lisa created commit:

`ec949c1af5489942aa0f001713dab910c98de9ab`

The commit contains only the three declared paths.

No ordinary `git add`, `git commit`, or broad staging command was used.

## Verification

`cargo fmt --all -- --check` passed in the live worktree.

`git show --format= --check ec949c1` passed.

The initial live integration build exposed an unrelated concurrent edit:
T-049-02-01's `main.rs` called a three-argument `run_init` while its in-flight
`init.rs` still exposed the old two-argument signature.

Those files were pre-existing, outside this ticket, and were not modified to
make this ticket pass.

To verify independently, created a temporary clean archive of the committed
baseline and overlaid only this ticket's three source files.

The temporary copy used the repository's built WASM artifact and shared Cargo
target directory.

`cargo test -p lisa-cli --test seal_visibility` passed: 5 passed, 0 failed.

The five cases include all three acceptance fixtures plus explicit commit and
the pre-existing two-tier visibility contract.

`cargo test -p lisa-cli` passed.

Its main binary unit suite passed 340 tests.

All CLI integration suites passed; the existing real-Zellij boundary remained
ignored by its declared external-environment policy.

`cargo test --workspace` passed in the same clean-baseline overlay.

The workspace run covered lisa-cli, lisa-core, and lisa-plugin.

Key reported suites included 340 CLI binary tests, 226 core tests, and 409
plugin tests, plus all enabled integration and property/regression suites.

No failure was hidden or filtered beyond the repository's pre-existing ignored
real-Zellij test.

## Ownership checks

`git status --short` for all three ticket-owned paths returned empty after the
Lisa commit.

`git diff --cached --name-only` returned empty.

Remaining live worktree changes belong to Lisa metadata/publication state and
the concurrent T-049-02-01 attempt.

The private phase artifacts are managed and published by Lisa and were not
included in the source commit.

## Deviations

The planned live-tree package/workspace test could not be used because of the
concurrent adjacent ticket's transient API mismatch.

Verification moved to a clean committed-baseline archive with only this
ticket's exact files overlaid.

This isolates this ticket more strictly than a dirty shared-tree run and avoids
altering or masking the neighboring work.

No product design or implementation deviation was required.

## Remaining work

Write `review.md` and `review-disposition.json`.

Remain on T-049-02-02 after Review and let Lisa publish completion.

