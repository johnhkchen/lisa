# Progress: proposal apply records first

## Completed

- Read the canonical repository and RDSPI instructions.
- Mapped proposal sidecars, proposal action provenance, CLI application, Park provenance,
  parked remedy projection, dashboard rendering, and operator triage scheduling.
- Wrote Research, Design, Structure, and Plan artifacts in the private attempt directory.

## Core evidence model

- Added `ProposalAction::Attempted` for the pre-mutation operator row.
- Added `ProposalAction::Failed` for terminal apply failures.
- Added optional/defaulted step evidence to `ProposalActionRecord`:
  - total step count;
  - applied step descriptions;
  - failed step description;
  - failure reason.
- Kept historical Proposed, Applied, and Dismissed JSON spellings unchanged.
- Added serialization and round-trip coverage for Attempted and Failed evidence.
- Added `ProposalState::Failed` as a terminal sidecar state.
- Added write/read coverage for Failed sidecars.

## Park correlation

- Added observational latest-current-Park lease extraction from the mixed provenance ledger.
- A latest Unpark removes the current Park lease for projection purposes.
- `collect_parked_remedies` now receives the ledger path explicitly.
- Pending proposals project only when their source lease exactly matches the latest Park lease.
- Missing/malformed ledger evidence hides only the optional proposal, not the underlying remedy.
- Updated all CLI and plugin callers with their canonical ledger path.
- Added matching, mismatching, and latest-Unpark core tests.

## Records-first apply

- Added independent action-time correlation against the latest Park lease.
- Added a writer-taking internal entrypoint while preserving the public CLI function.
- The CLI appends Attempted, including actor/proposal/step count, before executing steps.
- Each step prints `Applying proposal step: <description>` before it executes.
- The step executor tracks the exact successful prefix.
- File edits recheck exact old-text cardinality at execution time.
- An earlier command can no longer silently invalidate a later prepared edit.
- Successful execution publishes Applied sidecar and Applied outcome evidence.
- Failed execution publishes Failed sidecar and Failed outcome evidence.
- Failed evidence names both the landed prefix and first failed step.
- Sidecar and outcome publication are both attempted after execution.
- Clean apply reopens the ticket only after terminal evidence publication.
- Failed apply leaves the ticket blocked.
- Dismiss behavior remains terminal and correlated to the current Park.

## Tests added or strengthened

- Provenance attempt/failure row JSON round-trip and pinned field names.
- Failed sidecar serialization round-trip.
- Matching Pending proposal projects for matching Park lease.
- Mismatched Pending proposal does not project.
- Latest Unpark invalidates the current Park correlation.
- Clean apply leaves Attempted then Applied rows.
- Clean apply pins the exact operator-visible announcement.
- Mid-list file-edit invalidation leaves the first command's mutation on disk.
- That failure leaves Attempted then Failed rows with applied/failed step names.
- That failure leaves the sidecar Failed and the ticket blocked.
- Stale proposal action is rejected before provenance or mutation.
- Matching proposal suppresses duplicate first-responder triage.
- Stale proposal does not suppress triage for the newer Park attempt.
- A failed apply clears the prior same-Park triage guard in ledger order.
- The next Triage Started row consumes that reset so polling cannot relaunch repeatedly.

## Verification completed

- `cargo test -p lisa-core` — passed (251 unit tests plus integration/property regressions).
- `cargo test -p lisa-cli proposal::tests` — passed (4 focused proposal tests).
- `cargo test -p lisa-plugin proposal_suppresses_triage_only_for_the_current_park_lease` — passed.
- `cargo test -p lisa-plugin failed_apply_resets_same_park_triage_guard_exactly_once` — passed.
- `cargo test --workspace` — passed:
  - lisa-cli library: 21 passed;
  - lisa-cli binary: 373 passed;
  - lisa-core: 251 passed;
  - lisa-plugin: 438 passed;
  - all enabled integration and doc tests passed;
  - one existing real-Zellij environment test remained ignored by its own prerequisites.
- `git diff --check` — passed.

## Deviations from plan

- The core and CLI/plugin edits were developed together before commit because the public
  projection signature and additive record fields require all first-party callers to compile.
- Focused tests were still run at each boundary before the full workspace suite.
- No new integration-test file was needed; existing module tests provide deterministic writer
  capture and scheduler-state access.

## Commits

- `30b55af` — core apply evidence, Failed state, and Park correlation.
- `c9f3443` — CLI records-first apply and ledger-aware callers.
- `3defb16` — plugin projection and stale-Park triage regression.
- `bd95482` — same-Park re-triage reset after a failed apply.

All four commits were created with `lisa commit-ticket` and exact repository-relative includes.

## Remaining

- `just check` passed after concurrent T-049-08-01 source changes settled:
  - WASM target check passed;
  - all workspace tests passed;
  - lisa-plugin now reports 439 passing tests with the final same-Park retry regression.
- Committed diff and worktree ownership were inspected.
- No ticket-owned source file is staged, modified, or untracked.
- Write Review and disposition artifacts.
