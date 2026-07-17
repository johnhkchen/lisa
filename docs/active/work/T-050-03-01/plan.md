# Plan — T-050-03-01 client-autodetect

## Execution rules

Work only in the five ticket-owned source/test paths named by Structure.
Write implementation progress to the private attempt `progress.md`.
Do not edit ticket frontmatter.
Do not write phase artifacts into `docs/active/work`.
Do not use the ordinary Git index.
Commit each meaningful unit through `lisa commit-ticket` with exact paths.
Run native tests before each commit and broader verification after both commits.
Inspect the final diff and worktree before Review.

## Step 1: add typed PATH availability

Open `crates/lisa-cli/src/detect.rs` at its imports and type declarations.
Add `AgentAvailability` without disturbing `ProjectType`.
Add the production detector near the project detector entry point.
Keep environment detection conceptually separate from project marker detection.
Implement a pure boolean classifier.
Implement platform-aware executable candidate discovery.
On Unix, read metadata and check execute bits.
On Windows, derive candidate suffixes from PATHEXT.
Do not spawn commands.
Do not access provider versions.
Do not cache the result.

Verification for Step 1:
Run the new `detect` module classifier test.
Run executable-file helper tests on Unix.
Confirm a non-executable regular file is rejected.
Confirm an executable regular file is accepted.
Confirm no existing project-detection test regresses.

## Step 2: integrate detection into config resolution

Open `crates/lisa-cli/src/config.rs` around `ResolvedConfig`.
Import the new detector and availability enum.
Add the typed `ClientResolution` enum.
Add transient provenance to `ResolvedConfig`.
Set a coherent default provenance alongside the existing Claude default.
Add the announcement-rendering method with exact pinned strings.

Preserve the public `resolve_config` call shape.
Make it acquire production PATH availability once.
Delegate to `resolve_config_with_availability`.
Move the existing resolution body into the helper.
Resolve client and reason as a pair.
Apply CLI precedence first.
Apply validated file config second.
Apply detected availability only when neither explicit source exists.
Leave all unrelated config fields unchanged.

Update `[agent]` template comments.
Describe automatic detection without adding an `auto` value.
State the both-installed Claude default.
Keep the explicit example commented.

Verification for Step 2:
Replace the unconditional default-client test with a four-case table.
Assert selected client for each availability state.
Assert exact announcement for each state.
Test explicit Codex config against Claude-only availability.
Test CLI Claude against explicit Codex config and Codex-only availability.
Assert explicit source announcements exactly.
Run all `config` module tests.
Run all `detect` module tests.
Run `cargo fmt --check` after formatting.

## Step 3: document first implementation checkpoint

Create private-attempt `progress.md`.
Record completed detection and resolution work.
Record tests run and their results.
Record that operator surfaces and integration fixture remain.
Record any deviation from Design or Structure before proceeding.
If no deviation occurred, say so explicitly.

## Step 4: commit detection and resolution unit

Inspect `git diff -- crates/lisa-cli/src/detect.rs crates/lisa-cli/src/config.rs`.
Confirm only ticket-owned changes are present.
Run `lisa commit-ticket` for `T-050-03-01`.
Use a message describing PATH-aware client resolution.
Include exactly `crates/lisa-cli/src/detect.rs`.
Include exactly `crates/lisa-cli/src/config.rs`.
Do not include progress or phase artifacts; Lisa owns their later publication.
Confirm the command succeeds.
Confirm those two source files are no longer modified.

## Step 5: add doctor announcement

Open `crates/lisa-cli/src/doctor.rs::run_doctor`.
Keep the existing resolution call.
Build the output buffer from the resolved announcement plus a blank line.
Append the preexisting dependency report.
Do not alter provider checks or install hints.
Do not change failure determination.
Do not change Codex trust behavior.

Local verification for Step 5:
Run doctor unit tests.
Inspect the formatted missing-Claude text in existing tests.
Confirm the new line precedes `Checking dependencies...`.

## Step 6: add real-loop startup announcement

Open `crates/lisa-cli/src/loop_cmd.rs::run_loop`.
Keep initial project and protocol checks first.
Keep dry-run early return unchanged.
Print the resolved announcement immediately after the dry-run branch.
Do not add it to dry-run output.
Do not add it to layout KDL.
Do not print it a second time in the runtime summary.
Do not change dependency preflight ordering after the line.

Local verification for Step 6:
Run loop command unit tests.
Confirm dry-run tests still pass.
Confirm layout snapshot-style assertions remain unchanged.

## Step 7: build controlled-PATH fixture

Create `crates/lisa-cli/tests/client_autodetect.rs`.
Gate the file to Unix.
Implement the executable-writer helper using fixture files and mode `0o755`.
Implement a project fixture with current protocol version.
Write a journal completion guard.
Pin an absolute fixture Zellij path.
Always stub Git and Zellij.
Conditionally stub Claude and Codex.
Keep PATH limited to the fixture bin directory.
Use absolute `/bin/sh` shebangs so stubs do not need host PATH.
Set HOME to an isolated directory.
Write Codex hooks for loop eligibility.

Avoid shell utilities inside stubs.
Git only prints a version and exits zero.
Zellij prints a supported version for `--version` and exits zero for launch.
Provider stubs print versions and exit zero.
No fixture invokes real agents, account state, or network.

## Step 8: cover availability matrix

Add `codex_only_doctor_is_green_and_announces_detection`.
Run doctor with only Codex present.
Assert success.
Assert exact Codex-only sentence.
Assert selected provider row succeeds.

Add `claude_only_doctor_is_green_and_announces_detection`.
Run doctor with only Claude present.
Assert success.
Assert exact Claude-only sentence.

Add `both_agents_choose_claude_and_announce_default`.
Run doctor with both present.
Assert success.
Assert exact both-installed sentence.
Assert doctor checks Claude as the selected provider.
Assert it does not add the Codex trust section.

Add `neither_agent_preserves_claude_install_remedy`.
Run doctor with neither present.
Assert failure.
Assert exact neither-installed sentence.
Assert the complete historical Claude row and install URL fragment.

## Step 9: cover precedence and loop surface

Add an explicit file configuration test.
Put only Claude on PATH.
Configure Codex.
Run doctor.
Assert config-source Codex announcement.
Assert Codex missing remedy appears.
Assert Claude availability did not replace explicit config.

Add a CLI flag test.
Put only Codex on PATH.
Run real loop with `--client claude`.
Assert CLI-source Claude announcement.
Assert Claude dependency preflight fails.
Assert Codex availability did not replace the flag.

Add an unconfigured loop test.
Put only Codex on PATH.
Run real loop.
Assert exact Codex-only announcement.
If embedded WASM is present, assert the fixture Zellij launch can exit cleanly.
If embedded WASM is a development placeholder, permit that later established failure.
Always assert the announcement occurred before any later failure.

## Step 10: verify second unit

Run the dedicated `client_autodetect` integration test target.
Run all `doctor` unit tests.
Run all `loop_cmd` unit tests.
Run the full `lisa-cli` test suite.
Run `cargo fmt --all` and then `cargo fmt --all -- --check`.
Inspect output for exact-string failures.
Fix every failure within ticket scope.
Update `progress.md` with results and deviations.

## Step 11: commit operator surfaces and fixtures

Inspect the three-path diff.
Confirm doctor changes only presentation assembly.
Confirm loop changes only real-startup presentation.
Confirm fixture PATH excludes host agents.
Run `lisa commit-ticket` for `T-050-03-01`.
Use a message describing doctor/loop announcements and fixture coverage.
Include exactly `crates/lisa-cli/src/doctor.rs`.
Include exactly `crates/lisa-cli/src/loop_cmd.rs`.
Include exactly `crates/lisa-cli/tests/client_autodetect.rs`.
Confirm those paths are clean afterward.

## Step 12: broad verification

Run `cargo test --workspace`.
Run `just check` if the environment has the configured WASM target and command prerequisites.
If `just check` fails for an environment-only reason, record the exact reason in progress and Review.
Do not mark pass while a ticket-owned test or compile failure remains.
Run `git status --short`.
Confirm only preexisting scheduler-owned metadata/ticket changes remain.
Confirm no ticket-owned source path is staged, modified, or untracked.
Inspect recent commits and the ticket-owned diffs.
Confirm provider adapters are absent from both commits.

## Step 13: Review preparation

Update `progress.md` to mark all implementation steps complete.
List both isolated commit identifiers.
List all verification commands and outcomes.
List any known platform boundary, especially Unix-only shell integration fixtures.
Distinguish coverage gaps from blockers.

Write `review.md` in the private attempt directory.
Summarize detection, resolution provenance, announcements, and fixture coverage.
Name every source/test file changed.
State that adapters and persistent config schema were untouched.
Evaluate each acceptance criterion against concrete tests.
Record open concerns or state that none are known.

Write `review-disposition.json` exactly.
Use pass only if source paths are clean and all material verification succeeds.
Use `{"disposition":"pass","reason":null}` for a ready result.
Otherwise use the assignment's required block shape with an actionable reason.
Run `lisa check-disposition T-050-03-01`.
Correct every reported issue.
Remain on this ticket after Review.
