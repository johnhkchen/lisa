# Review — T-050-03-01 client-autodetect

## Disposition

Ready to complete.
The ticket's client-selection behavior is implemented.
All ticket-owned source is committed through Lisa's isolated transaction.
Focused, workspace, and repository aggregate verification pass.
No blocking concern remains.

## Outcome

An unconfigured Lisa client now resolves from executable presence on PATH.
A Codex-only machine selects Codex.
A Claude-only machine selects Claude.
A machine with both agents selects Claude as the stated default.
A machine with neither agent selects Claude so the established doctor remedy remains intact.
Explicit `.lisa.toml` client configuration retains precedence.
The loop `--client` flag retains highest precedence.
Doctor announces the selected client and why.
Real loop startup announces the same resolved decision.
Dry-run behavior remains unchanged.
No detection result is persisted.
No provider adapter behavior changed.

## Files changed

### `crates/lisa-cli/src/detect.rs`

Added `AgentAvailability` with four exhaustive states.
Added production PATH inspection for `claude` and `codex`.
Detection reads filesystem metadata only.
Detection never invokes either provider.
Detection never invokes a version command.
Detection never checks credentials, accounts, or network state.
Unix detection requires a regular file with an executable permission bit.
Windows detection uses PATHEXT candidates.
Other targets use regular-file presence as a conservative fallback.
Added pure classification tests for all four boolean combinations.
Added Unix executable-permission coverage.
Existing project-type detection remains unchanged.

### `crates/lisa-cli/src/config.rs`

Added typed `ClientResolution` provenance.
The variants distinguish CLI, config file, and detected availability.
Added transient provenance to `ResolvedConfig`.
Kept `ResolvedConfig.client` as the concrete provider consumed downstream.
Kept the public `resolve_config` call signature unchanged.
Production resolution detects PATH only for an unconfigured client.
Explicit CLI and file selections do not consult environment detection for their value.
Added a deterministic injected-availability helper for unit tests.
Mapped only `CodexOnly` to Codex.
Mapped Claude-only, both, and neither to Claude.
Centralized all operator announcements on resolved configuration.
Pinned strings for all four detected states.
Pinned strings for both explicit selection sources and both providers.
Updated the config catalog description to explain automatic PATH detection.
Kept the generated client assignment commented and inert.
Added availability-matrix tests.
Strengthened explicit precedence tests with conflicting detected availability.

### `crates/lisa-cli/src/doctor.rs`

Prepended the resolved client announcement to doctor output.
Kept provider dependency checks unchanged.
Kept Claude's install hint unchanged.
Kept Codex's install hint unchanged.
Kept selected-provider version probing unchanged.
Kept completion, project, Zellij, and trust checks unchanged.
The neither-installed case still enters the established Claude check and remedy.

### `crates/lisa-cli/src/loop_cmd.rs`

Printed the resolved announcement on real loop startup.
Placed it after cheap project/protocol validation.
Placed it after the dry-run early return.
Placed it before dependency preflight so a failed explicit choice is still understandable.
Kept configured-client collection unchanged.
Kept mixed per-ticket routing unchanged.
Kept provider preflight unchanged.
Kept generated layout and adapter commands unchanged.

### `crates/lisa-cli/tests/client_autodetect.rs`

Added a 236-line Unix real-binary fixture suite.
Each fixture creates an isolated project, HOME, and bin directory.
PATH contains only fixture binaries.
Git and Zellij are deterministic shell stubs.
Claude and Codex stubs are created only for the requested availability case.
No host agent can leak into fixture detection.
No real provider is invoked.
No network or account state is used.
The suite covers seven operator-visible scenarios.

## Acceptance criterion evidence

### Codex-only

`codex_only_resolves_codex_and_doctor_is_green` supplies only a Codex stub.
The test asserts doctor exits successfully.
The test asserts the exact Codex-only announcement.
The test asserts the Codex version row is present.
The test asserts the Claude remedy is absent.

### Claude-only

`claude_only_resolves_claude_and_doctor_is_green` supplies only a Claude stub.
The test asserts doctor exits successfully.
The test asserts the exact Claude-only announcement.
The test asserts the Claude version row is present.
The test asserts no Codex trust section is entered.

### Both installed

`both_agents_resolve_claude_and_announce_the_default` supplies both stubs.
The test asserts doctor exits successfully.
The test pins `Driving Claude — both agents are installed; claude is the default.`
The test asserts Claude is the checked provider.
The test asserts Codex-specific trust behavior is not activated.

### Neither installed

`neither_agent_keeps_the_existing_claude_install_remedy` supplies neither stub.
The test asserts doctor fails as expected.
The test pins the neither-installed explanatory sentence.
The test pins the complete existing missing-Claude row and install URL as one exact substring.
No doctor remedy function or formatter was changed.

### Explicit config precedence

`explicit_config_beats_claude_only_detection` supplies only Claude on PATH.
The project explicitly configures Codex.
The test asserts the Codex config announcement.
The test asserts doctor reports Codex missing rather than silently selecting Claude.

### CLI flag precedence

`client_flag_beats_codex_only_detection` supplies only Codex on PATH.
The real loop command receives `--client claude`.
The test asserts the CLI-specific Claude announcement.
The test asserts Claude preflight fails.
This demonstrates the flag still beats both file config and environment detection.

### Loop-start visibility

`loop_start_announces_the_detected_client` runs a real non-dry loop command.
It supplies only Codex.
It asserts the exact Codex-only announcement on stdout.
The assertion succeeds before any optional development-placeholder WASM failure.
With embedded WASM present, the fixture Zellij exits successfully.

### No persistent detection state

The detector returns an in-memory enum.
Resolution stores its source only in the in-memory `ResolvedConfig`.
No new `.lisa.toml` key was introduced.
No `.lisa/` state file was introduced.
No layout KDL key was introduced.
No plugin configuration key was introduced.
The only template change is explanatory commented copy.

### Provider adapters untouched

Commit `5e91e5c` includes only `crates/lisa-cli/src/detect.rs`.
Commit `47e7336` includes only `crates/lisa-cli/src/config.rs`.
Commit `d88cd13` includes only doctor, loop startup, and the integration fixture.
No `lisa-core` provider file changed.
No `lisa-plugin` adapter file changed.
No Claude or Codex launcher file changed.
N1 remains intact: the result selects a provider and does not alter how it is driven.

## Test coverage

Focused config tests: 65 passed.
Focused detection tests: 9 passed during implementation.
Focused doctor tests: 50 passed.
Focused loop tests: 24 passed.
Client-autodetect integration fixtures: 7 passed.
`cargo test --workspace` passed all non-ignored workspace tests.
`just check` passed the WASM target check and the full workspace suite.
The preexisting real-Zellij delivery test remains ignored by default because it requires external tools.
This ticket does not require a live agent or live Zellij run because selection is defined solely by PATH presence.

## Commit and ownership audit

All ticket source commits used `lisa commit-ticket`.
No ordinary `git add` was used.
No ordinary `git commit` was used.
No broad include was used.
The ordinary index is empty.
All five ticket-owned source/test paths are clean.
Remaining modified/untracked paths belong to Lisa phase publication and the concurrently completed ticket.
Ticket phase and status frontmatter were not edited by this agent.

## Concurrency handling

`T-050-02-01` began config-catalog work in `config.rs` during this implementation.
This attempt detected the same-file ownership collision before committing.
It removed only its own config edits and committed detection independently.
`T-050-02-01` then committed its catalog and init changes as `363e82d`.
This attempt reapplied client resolution on top of that clean baseline.
Focused and aggregate tests passed after the rebase.
No cross-ticket source was captured by this ticket's commits.

## Open concerns and limitations

The shell-stub integration fixture is Unix-gated.
The production detector includes a Windows PATHEXT branch, but this macOS run does not execute that branch.
This is not blocking because the acceptance fixture pattern and current repository integration suite are Unix-based.
Provider usability beyond PATH presence is intentionally not part of selection.
Doctor and loop preflight continue to diagnose a present but unusable explicit or detected binary.
No known functional issue remains.

## Reviewer summary

The previously unconditional Claude fallback is now environment-aware only when unconfigured.
The resolver remains the single selection authority.
Doctor and loop report one shared, pinned explanation of that authority's decision.
Existing explicit behavior, remedies, routing, adapters, and persistent schemas remain intact.
The implementation satisfies both acceptance criteria and is ready for Lisa's completion transaction.
