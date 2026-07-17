# Structure — T-049-02-02 doctor identity preflight

## Modified file: `crates/lisa-cli/src/completion_seal.rs`

This module remains the native completion capability boundary.

Replace the private two-command-only remedy constant with one `pub(crate)`
complete guidance block.

The block owns every exact operator-facing identity and remedy literal.

It includes:

- the missing Git identity sentence;
- a label for operators supplying their own identity;
- the local `user.name` command;
- the local `user.email` command;
- the alternative `lisa init` history-offer sentence.

Add an accessor on `RunCompletionSeal`:

`commit_unavailable(&self) -> Option<&CommitSealUnavailable>`.

The accessor delegates to `ResolvedCompletionSeal::commit_unavailable`.

Do not expose the internal resolution field or the probe outcome type.

Update `format_preflight_failure` to append the shared guidance block.

Preserve the named prefix and configured commit field.

Update resolver unit tests to pin the exact guidance block.

Assert that explicit-commit identity failure contains the complete block.

Keep existing probe cardinality and tier tests unchanged.

## Modified file: `crates/lisa-cli/src/doctor.rs`

Change `run_doctor` to call `completion_seal::resolve_for_run`.

Retain the returned result until completion output and final status are built.

Do not early-return on a completion error.

Dependencies, project version, cache cleanup, and Codex trust checks should all
still render in one invocation.

Replace `append_completion_seal_report(output, seal)` with a result-aware
formatting function.

Its input is a borrowed `Result<RunCompletionSeal, String>`.

Its successful branch prints the existing visibility line.

It pattern-matches only `CommitSealUnavailable::IdentityMissing` for additional
guidance.

The identity branch prints `Reason:` followed by the shared guidance constant.

Repository missing and transaction unavailable do not print identity remedies.

Its error branch prints the existing hard-failure string.

Return a boolean or derive one at the call site to add completion failure to the
final doctor status.

The final result is failure when required dependencies fail or completion
resolution failed.

Update module tests for the revised helper signature.

Synthetic success values are awkward because the wrapper has private fields;
integration fixtures will carry the main behavioral coverage.

Existing plain-tier unit coverage may instead continue through a small
seal-formatting helper or test the unchanged visibility function in its owner.

Avoid changing dependency `CheckResult` because auto identity fallback is a
valid journal resolution, not a failed dependency.

## Modified file: `crates/lisa-cli/tests/seal_visibility.rs`

Extend the existing compiled-binary seal fixture rather than create another
nearly identical test harness.

Generalize fixture setup to accept completion mode and repository state.

Define repository fixture variants:

- absent;
- present without identity;
- present with identity.

Initialize repositories through the real `git` executable.

Configure both local `user.name` and `user.email` only for the positive case.

Set `HOME` to the temporary fixture directory.

Set `GIT_CONFIG_NOSYSTEM=1` on the Lisa child process.

Keep host `PATH` after the fixture bin directory so Git remains available.

Preserve current explicit commit and explicit journal seal-line assertions.

Add an auto missing-identity test.

Assert process success, exact journal seal line, and complete guidance block.

Add an auto configured-identity test.

Assert process success, exact commit seal line, and absence of the missing gap
and both remedies.

Add an auto no-repository test.

Assert process success, exact journal seal line, and absence of all identity
guidance.

Add an explicit commit missing-identity test.

Assert process failure, named completion preflight, configured field, and the
same complete guidance block.

Tests may duplicate expected strings as assertions; the one-source requirement
applies to production sources and string tests are explicitly required to pin
the output.

## Unchanged files

`crates/lisa-core/src/completion.rs` already has the necessary typed reason.

`crates/lisa-cli/src/status.rs` continues using tier-only inspection.

`crates/lisa-cli/src/loop_cmd.rs` continues using the same run resolver and
automatically receives the expanded guidance for explicit commit failures.

`crates/lisa-cli/src/init.rs` and `main.rs` are owned by adjacent in-flight work.

No configuration, plugin, README, or ticket frontmatter change is needed.

## Dependency direction

Core domain types remain independent of native process logic.

`completion_seal.rs` depends on core types and Git subprocesses.

`doctor.rs` depends on `completion_seal.rs` for resolution and copy.

Integration tests depend only on the public CLI binary behavior.

There is no dependency from completion seal back to doctor.

## Implementation ordering

First centralize guidance and expose the unavailable reason.

Second adapt doctor formatting and final status.

Third extend integration fixtures.

Fourth format and run focused tests.

Fifth run the full CLI/workspace suite.

Sixth commit exact owned source paths through Lisa.

## Invariants

One doctor resolution performs at most one probe.

Explicit journal performs zero probes.

No-repository and missing-identity are never conflated in output.

Auto fallback never becomes a doctor hard failure.

Explicit commit unavailability is a doctor hard failure.

The stable seal line remains visible in every successful completion resolution.

Identity/remedy production copy has exactly one literal definition.

Doctor performs no repository or identity mutation.

No ticket-owned source remains modified, staged, or untracked after commit.

