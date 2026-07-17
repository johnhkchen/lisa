# Structure: block triage proposal

## Source change set

Create `crates/lisa-core/src/triage.rs`.

Modify `crates/lisa-core/src/lib.rs`.

Modify `crates/lisa-core/src/parking.rs`.

Modify `crates/lisa-core/src/provenance.rs`.

Create `crates/lisa-cli/src/triage_agent.rs`.

Create `crates/lisa-cli/src/proposal.rs`.

Modify `crates/lisa-cli/src/main.rs`.

Modify `crates/lisa-cli/src/config.rs`.

Modify `crates/lisa-cli/src/loop_cmd.rs`.

Modify `crates/lisa-cli/src/status.rs`.

Modify `crates/lisa-plugin/src/lib.rs`.

Modify `crates/lisa-plugin/src/ui.rs`.

No dependency or Cargo manifest change is expected.

## Core triage module

`triage.rs` owns the durable proposal vocabulary.

Public constant `TRIAGE_PROPOSAL_FILE` names the canonical sidecar.

`TriageProposal` carries summary, recommendation, and prepared steps.

`PreparedStep` is a tagged enum.

`Command` carries description and command.

`FileEdit` carries description, path, old text, and new text.

`ProposalState` is Pending, Applied, or Dismissed.

`StoredTriageProposal` carries ticket ID, source lease, state, and proposal.

Constructors/parsers validate visible strings and safe relative edit paths.

`read_stored_proposal` returns absence for missing files and an error for
malformed files where callers need diagnostics.

`write_stored_proposal` uses a same-directory temporary file and rename.

Unit tests cover schema round trip, validation, path rejection, and atomic
state replacement.

## Parking projection

Add `proposal: Option<TriageProposal>` to `ParkedRemedy`.

`collect_parked_remedies` reads `<work>/<ticket>/triage-proposal.json`.

It accepts only matching ticket ID and Pending state.

Applied, Dismissed, malformed, mismatched, or missing sidecars project None.

Disposition remains required and authoritative.

Update projection fixtures for absent and present proposal behavior.

## Provenance extension

Bump `SCHEMA_VERSION` from six to seven.

Add `TriageTransitionType::TriageTransition` discriminator.

Add `TriageState` with Started, Proposed, Failed, TimedOut, Invalid.

Add `TriageTransitionRecord`.

It carries seal, ticket/source lease, route, timeout, state, optional reason,
timestamps, and wall-clock seconds.

Add `ProposalActionType::ProposalAction` discriminator.

Add `ProposalAction` with Proposed, Applied, Dismissed.

Add `ProposalActionRecord` carrying ticket/source lease, action, actor, optional
proposal, and timestamp.

Extend `ProvenanceLedgerRecord` with both shapes before legacy execution.

Add append functions for both records.

Update exhaustive readers to ignore the new rows where unrelated.

Add compact-line, round-trip, append-order, and legacy-mix tests.

## CLI configuration

Add `TriageConfig` under top-level `LisaConfig.triage`.

Recognize `enabled` and `timeout_secs`.

Add resolved fields to `ResolvedConfig`.

Use core plugin defaults as the single default source.

Validate a positive timeout even if a malformed runtime could degrade safely.

Teach unknown-key validation the new section.

Add config parse, resolve, invalid timeout, and unknown-key tests.

## Plugin configuration/layout

Add `triage_enabled` and `triage_timeout_secs` to `PluginConfig`.

Add constants for defaults.

Parse KDL keys leniently.

Emit both values from `generate_layout`.

Update layout tests to pin the values.

Legacy layouts inherit safe defaults.

## Native triage agent runner

`triage_agent.rs` owns a hidden plumbing command.

`TriageAgentArgs` contains root, client, model, ticket path, disposition path,
timeout, and optional executable override for tests.

`build_prompt` creates the read-only evidence-triage instruction and exact JSON
contract.

`build_command` selects Claude print mode or Codex exec JSON mode.

`run_child_bounded` redirects output to temporary files, polls the child, and
kills its process group at deadline.

Exit 124 identifies timeout to the plugin.

Provider output extractors return the final candidate JSON.

Successful command stdout is exactly one compact proposal JSON document.

Tests use fake shell executables for success, failure, and timeout.

## Operator proposal command

`proposal.rs` owns `ProposalActionRequest::{Apply,Dismiss}`.

`run_proposal_action` resolves configured ticket/work directories.

It requires blocked status, Operator disposition, and Pending stored proposal.

Apply first validates every file edit against its current exact unique old
text and validates command strings.

It executes steps in displayed order only after explicit invocation.

File writes use temporary replacement through core helpers local to the CLI.

After all steps pass, it writes Applied state and appends action provenance.

It then writes ticket status Open.

Dismiss writes Dismissed state and appends action provenance only.

Tests cover both actions, command failure, edit mismatch, status behavior, and
provenance rows.

## CLI command surface

Add nested `Proposal` command with `Apply` and `Dismiss` subcommands.

Add hidden `TriageAgent` plumbing command.

Dispatch operator commands with plain success/failure messages.

Dispatch triage runner with exit 124 preserved for timeout.

Keep existing everyday command ordering stable.

## Plugin triage state

Add `TriageInFlight` with source lease, route, started time, and timeout.

Add `triage_in_flight: HashMap<TicketId, TriageInFlight>` to `State`.

Add helpers to read latest triage transitions per source generation.

Add `triage_capacity_available(route)` combining ordinary threads and triage
jobs for global/provider limits.

Add `build_triage_command` using absolute Lisa binary and host paths.

Add `request_operator_triage`.

It returns immediately when disabled or host boundaries are unavailable.

It scans canonical remedies in order and selects Operator parks only.

It resolves the latest Park source lease.

It skips Pending proposal or any existing same-generation triage transition.

It appends Started before placing the job in memory and launching.

Add `handle_triage_result`.

It removes in-flight accounting first.

It classifies timeout, failure, invalid output, or valid proposal.

Valid output writes the sidecar and Proposal/Proposed provenance.

Every outcome appends a terminal triage transition.

It never edits ticket or disposition state.

## Plugin integration order

Call `request_operator_triage` after live/orphan block reconciliation.

Call it before ordinary ready-ticket scheduling at the end of each poll.

Call it at permission grant and plugin load boundaries when safe.

Count in-flight triage in ordinary scheduling cap checks.

Handle `lisa_triage` RunCommandResult before notify fallback.

After result handling, request the next triage and schedule tickets.

## Status rendering

Extend `waiting_on_you_lines`.

For Pending proposal emit summary, suggested action, and each prepared step.

Then emit the original ask and raw reviewer note.

Keep no-proposal output exactly unchanged.

Add T-046 deterministic proposal ordering fixture.

## Dashboard rendering

Extend `ui::WaitingItem` with optional proposal.

Map the shared core projection without re-reading files.

Update `render_waiting_on_you` with the same field order and labels as status.

Keep color and section boundaries unchanged.

Update all direct `WaitingItem` fixtures.

## Artifact and commit boundary

Research, Design, Structure, Plan, Progress, and Review remain in the private
attempt work directory.

Source units are committed through exact-path `lisa commit-ticket` commands.

Core schema/projection is one meaningful unit.

CLI runner/config/actions is a second meaningful unit if cleanly separable.

Plugin scheduling/rendering is a third meaningful unit.

No ordinary index command is used.
