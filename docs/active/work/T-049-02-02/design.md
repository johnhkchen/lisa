# Design — T-049-02-02 doctor identity preflight

## Decision summary

Keep Git capability probing in `completion_seal.rs` and let doctor consume the
same resolved value used by loop startup.

Expose the retained auto-fallback reason through `RunCompletionSeal`.

Create one shared identity-guidance string in `completion_seal.rs` containing
the gap, both local Git commands, and the Lisa-history alternative.

Use that string in both explicit-commit hard failures and doctor diagnosis.

Make doctor call the real run resolver rather than the seal-only inspection
helper, while leaving status inspection behavior unchanged.

## Output contract

The shared guidance will read as one complete block.

It first names that Git commit identity is not configured.

It then introduces the operator-owned identity remedy and prints the existing
two commands on separate indented lines.

It ends with the alternative to rerun `lisa init` and accept the history offer.

The entire block is a single constant so there is only one production literal
source.

Doctor auto fallback prints the journal seal line followed by `Reason:` and the
shared guidance.

Explicit commit prints the existing named hard-failure prefix followed by the
same guidance.

Configured identity prints the commit seal line without guidance.

No repository prints the journal seal line without identity guidance.

Explicit journal prints the journal line without probing or guidance.

## Resolution API

Add `commit_unavailable()` to `RunCompletionSeal`.

The method returns `Option<&CommitSealUnavailable>` from the wrapped core
resolution.

This keeps the core resolution field private while exposing the already-stored
diagnostic fact.

No new probe or derived boolean is introduced.

`resolve_for_inspection` remains the status-oriented tier-only helper.

Doctor calls `resolve_for_run` because a health check must actually validate an
explicit commit request.

The naming remains acceptable because doctor is checking the same preflight a
run will execute; it does not mutate state.

## Doctor control flow

Store the `Result<RunCompletionSeal, String>` returned by `resolve_for_run`.

Continue running and rendering all dependency and project checks even if the
completion check fails.

This preserves doctor's value as a consolidated report.

Replace the seal-only append helper with a result-aware helper.

For a successful resolution, print the stable visibility line.

If its retained reason is `IdentityMissing`, append the shared guidance as the
reason for automatic journal fallback.

For other retained reasons, do not print identity guidance.

For an error, print the resolver's named hard-failure text.

Track whether the completion result failed and combine that bit with existing
required dependency failures for doctor's final `Err`.

The helper remains pure output formatting and can receive synthetic resolved
fixtures in unit tests if useful.

## Why use `resolve_for_run`

The existing run resolver already applies all configured-mode rules.

It already skips Git for explicit journal.

It already probes exactly once for auto and explicit commit.

It already preserves repository-versus-identity distinctions.

It already formats the explicit-commit hard failure used by loop startup.

Reusing it prevents doctor and loop from disagreeing about commit readiness.

It also avoids adding a second `git config` subprocess implementation to
doctor.

## Alternatives considered

### Add a standalone `check_git_identity` in doctor

Rejected because it would duplicate repository discovery and identity
semantics.

It could disagree with loop startup on empty output, canonical root, or
explicit journal behavior.

It would also make “one string source” harder to maintain.

### Extend `resolve_for_inspection` to return diagnostics

Rejected because status intentionally treats explicit configured tiers as
display facts and does not promise a health probe.

Changing its return type would widen the ticket into status behavior and touch
unrelated callers.

### Change `resolve_for_inspection` to probe explicit commit globally

Rejected because status should remain a read-only state presentation command,
not become a hard environment validator as a side effect of doctor work.

### Expose `probe_commit_support` directly

Rejected because doctor would then need to repeat pure resolution rules and
hard-error formatting.

The higher-level run resolver is the established adapter boundary.

### Put copy in doctor and import it into completion seal

Rejected because the seal module owns the pre-existing hard failure and is
usable without doctor.

Keeping guidance beside the hard failure avoids a lower-level module depending
on an operator command module.

### Split each line into several constants

Rejected because production could assemble them differently and tests would
not pin a single shared contract.

A complete block is easier to compare verbatim and guarantees side-by-side
remedies.

## Test design

Add compiled CLI fixtures for auto and explicit commit behavior.

Build an isolated project with valid Lisa config and stub runtime/provider
executables.

Use the real Git binary for repository initialization and local identity.

Set temporary `HOME` and `GIT_CONFIG_NOSYSTEM=1` for deterministic identity
resolution.

The missing-identity auto fixture creates a repository without `user.email`.

It must succeed journal-sealed, name the gap, and contain the complete shared
guidance verbatim.

The configured-identity fixture sets both local name and email.

It must succeed commit-sealed and contain none of the missing guidance.

The no-repository fixture creates no Git repository.

It must succeed journal-sealed without identity guidance.

The explicit-commit missing-identity fixture must fail and include the same
named hard failure and complete guidance.

Keep a module test in `completion_seal.rs` pinning the exact constant and
asserting the hard failure contains it as an intact block.

This test proves production sharing without duplicating production literals.

## Compatibility

No CLI flags or configuration schema change.

No existing seal visibility wording changes.

No repository is created and no Git configuration is written by doctor.

Explicit journal retains its zero-probe behavior.

Auto remains non-failing when commit support is unavailable.

Explicit commit remains fail-closed.

The only new failure is doctor correctly reporting an explicit commit mode that
cannot satisfy its configured contract.

## Ownership

Expected source ownership is limited to `crates/lisa-cli/src/completion_seal.rs`,
`crates/lisa-cli/src/doctor.rs`, and one CLI integration test file.

The in-flight `init.rs` and `main.rs` changes remain untouched.

All owned source will be committed together or in coherent exact-path units via
`lisa commit-ticket`.

