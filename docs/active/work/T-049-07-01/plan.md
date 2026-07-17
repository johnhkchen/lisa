# Plan: block triage proposal

## Step 1: add the core proposal contract

Create the typed proposal and stored-sidecar model.

Implement visible-string, sentence, step, and safe-path validation.

Implement durable sidecar read/write helpers.

Export the module from `lisa-core`.

Add focused core tests.

Verification: `cargo test -p lisa-core triage`.

## Step 2: extend the parked projection

Add optional active proposal data to `ParkedRemedy`.

Read only a matching Pending sidecar.

Preserve legacy ask behavior and raw reason.

Update every direct `ParkedRemedy` fixture.

Add absent, invalid, applied, dismissed, mismatched, and pending cases.

Verification: `cargo test -p lisa-core parking`.

## Step 3: add provenance records

Add triage transition and proposal action record shapes.

Bump the additive schema version.

Add append helpers and extend mixed ledger parsing.

Update existing schema-version assertions.

Fix all exhaustive consumers to ignore unrelated new variants.

Add round-trip and ordered replay tests.

Verification: `cargo test -p lisa-core provenance`.

## Step 4: add triage configuration

Extend native TOML parsing and resolved configuration.

Validate enabled and timeout fields.

Extend core PluginConfig defaults and KDL parsing.

Emit values in generated layouts.

Update configuration and layout tests.

Verification: targeted config and layout test filters.

## Step 5: implement the bounded native runner

Add the hidden `triage-agent` command.

Build deterministic evidence-triage prompts.

Build provider-specific read-only argv.

Implement bounded process-group execution without pipe deadlock.

Extract Claude and Codex final output envelopes.

Print only validated candidate JSON for plugin consumption.

Use fake executables for deterministic unit tests.

Verify success, provider failure, invalid output, and hard timeout.

Verification: `cargo test -p lisa-cli triage_agent`.

## Step 6: implement operator proposal actions

Add nested `lisa proposal apply|dismiss` commands.

Resolve and validate the current blocked Operator park.

Validate all prepared steps before applying.

Execute explicit commands and exact file replacements.

Mark state and append operator provenance.

Reopen only after a successful Apply.

Keep Dismiss blocked while suppressing the proposal projection.

Test exact T-046-style amendments through file-edit steps.

Test failed command and stale edit safety.

Verification: `cargo test -p lisa-cli proposal`.

## Step 7: render proposals in CLI status

Add a shared line formatter for prepared steps if useful.

Render summary, recommendation, and steps before ask/reason.

Preserve existing output exactly when proposal is absent.

Use the historical T-046 reason and amendment recommendation fixture.

Verification: `cargo test -p lisa-cli status`.

## Step 8: add plugin in-flight scheduling

Add durable source-generation selection from Park provenance.

Add same-generation Started/outcome replay suppression.

Add global and provider capacity accounting for triage jobs.

Build the native Lisa command with exact paths and route.

Append Started before RunCommands launch.

Integrate after parking and ahead of ordinary scheduling.

Ensure disabled config performs no launch or provenance write.

Verification: native plugin tests for eligibility and command construction.

## Step 9: handle triage results

Route `RunCommandResult` by dedicated context.

Classify exit 124 as timeout and nonzero as failure.

Parse success output through core validation.

Publish only valid pending sidecars.

Append terminal attempt and Proposed action provenance.

Remove in-flight accounting and resume scheduling in all outcomes.

Test park content/status before result handling.

Test failure, timeout, and invalid result leave the park unmodified.

Verification: focused plugin triage-result tests.

## Step 10: render proposals in dashboard

Extend WaitingItem projection and renderer.

Keep ordering identical to CLI status.

Update direct UI fixtures.

Test proposal ordering and no-proposal compatibility.

Verification: focused `ui` waiting filters.

## Step 11: T-046 field regression

Construct the historical legacy block fixture.

Use the preserved raw reason and cited operator note paths.

Inject a deterministic valid agent result.

Assert summary names criteria versus evidence.

Assert recommendation chooses the two-sentence amendment.

Assert status and dashboard place it before raw reason.

Apply the file-edit proposal in a disposable project.

Assert both amendments, action provenance, and reopened status.

Create a second fixture and dismiss it.

Assert dismissal provenance, blocked status, and raw fallback rendering.

## Step 12: fail-open timing matrix

Disabled: park completes and no triage attempt starts.

Failure: Park row/status exist before handling, no sidecar appears.

Timeout: bounded runner exits at configured deadline; park remains unchanged.

Invalid: output cannot replace or weaken existing parked data.

For each case assert ask and raw reason contents.

Use short fixture timeouts and generous test tolerances.

## Step 13: format and focused verification

Run `cargo fmt --all`.

Run core, CLI, plugin focused tests.

Run `git diff --check`.

Resolve compiler exhaustiveness and fixture construction failures.

Document deviations in Progress before changing the plan materially.

## Step 14: commit meaningful units

Inspect `git diff --name-only` and ordinary index status.

Commit core files with exact repository-relative includes.

Commit CLI files with exact repository-relative includes.

Commit plugin files with exact repository-relative includes.

If implementation interdependence makes partial builds impossible, use one
scoped commit containing the exact ticket-owned paths and document why.

Never include Lisa-managed ticket, journal, provenance, or shared work paths.

## Step 15: full verification

Run `cargo test --workspace --no-fail-fast`.

Run `cargo clippy --workspace --all-targets -- -D warnings`.

Run `git diff --check`.

Confirm no ticket-owned source remains modified, staged, or untracked.

Record test counts and any ignored environment fixture.

## Step 16: Review

Write private `progress.md` with completed work and deviations.

Write private `review.md` with source, behavior, coverage, and concerns.

Write exact passing disposition only if every acceptance path is ready.

Otherwise write an actionable block disposition.

Remain on T-049-07-01 after both Review artifacts exist.
