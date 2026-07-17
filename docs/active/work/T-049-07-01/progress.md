# Progress: block triage proposal

## Completed phases

Read `CLAUDE.md`, `AGENTS.md`, the complete RDSPI workflow, ticket, story,
T-049-05-01 implementation trail, and T-046-06-03 field evidence.

Wrote Research, Design, Structure, and Plan in the private attempt directory.

Kept all phase artifacts out of the shared publication path.

## Core proposal model

Added `crates/lisa-core/src/triage.rs`.

The typed proposal requires:

- one plain summary sentence;
- a visible recommendation;
- at least one prepared action;
- safe project-relative file edit paths; and
- exact differing old/new text for file replacements.

Prepared actions support explicit shell commands and exact file edits.

Stored proposals bind ticket ID to the source attempt lease.

Their state is Pending, Applied, or Dismissed.

Sidecar publication uses a same-directory temporary and rename.

Only a matching Pending sidecar enters the canonical parked projection.

Malformed, mismatched, applied, dismissed, or missing sidecars preserve the
ordinary park projection.

## Provenance

Advanced the additive provenance schema to version seven.

Added triage transition records with Started, Proposed, Failed, TimedOut, and
Invalid states.

Each record carries the source park generation, route, timeout, timestamps, and
bounded failure detail where applicable.

Added proposal action records for Proposed, Applied, and Dismissed.

Creation records carry the proposal payload and Agent actor.

Operator actions carry Operator actor and exact source generation.

Mixed-ledger parsing recognizes both new disjoint record shapes.

Successful visible proposal publication is gated by its proposal provenance
append.

Provider launch is gated by Started provenance, giving restarts a durable
single-spend fence.

## Configuration

Added `[triage] enabled` and `timeout_secs` to native TOML parsing.

Defaults are enabled and 120 seconds.

Zero timeouts fail native validation.

Unknown triage keys produce the established warning shape.

Resolved values flow through generated KDL into `PluginConfig`.

Legacy/bad runtime maps retain bounded defaults.

The generated default config documents both optional values.

## Bounded native agent runner

Added hidden `lisa triage-agent` plumbing.

The prompt names the exact ticket and disposition inputs.

It instructs the provider to inspect cited evidence, remain read-only, and
return the exact typed proposal JSON.

Codex runs unattended with a read-only sandbox.

Claude runs in print mode with read-oriented tools.

Provider stdout/stderr go to temporary files, avoiding pipe back-pressure.

The runner polls a hard deadline and kills the provider process group.

Timeout exits through code 124 for deterministic plugin classification.

Claude and Codex output envelopes reduce to a final candidate.

The candidate is core-validated before compact JSON reaches the plugin.

Fake-provider tests cover success, failure, and a real one-second timeout.

## Plugin scheduling

Added a separate in-flight triage map without adding synthetic DAG tickets.

Normal Running ticket threads plus triage jobs share `max_threads`.

Provider-specific ticket threads plus triage jobs share provider caps.

The blocked ticket's resolved route and model select the first responder.

Only Operator-owned latest Park records are eligible.

An existing Pending proposal, in-flight job, or same-generation triage record
suppresses repeat spend.

Triage Disabled, missing host boundaries, no capacity, and provenance failure
all fail open.

The live/orphan park logic remains unchanged and completes before triage is
requested.

First responders receive scheduling priority after a cord pull but cannot
exceed configured capacity.

RunCommand results classify timeout, failure, invalid output, or proposal.

No result path edits ticket status or the Review disposition.

## Operator actions

Added `lisa proposal apply <ticket>`.

It requires a blocked Operator remedy and matching Pending proposal.

It validates every prepared edit before executing the action list.

Commands run only after explicit operator invocation.

File edits require exact unique old text and publish atomically.

Successful apply records operator provenance, marks the sidecar Applied, and
reopens the ticket for ordinary Review.

Added `lisa proposal dismiss <ticket>`.

Dismiss records operator provenance, marks the sidecar Dismissed, and leaves
the ticket blocked on its original ask.

Both action fixtures verify durable provenance.

## Status and dashboard

Extended the shared `ParkedRemedy` with optional proposal advice.

CLI and dashboard render the same order:

1. first-responder summary;
2. suggested action;
3. prepared actions;
4. original ask; and
5. raw reviewer reason.

No-proposal rendering retains the previous ask-first two-line output.

The T-046 fixture names the criteria-versus-evidence gap and recommends the
calibrated-bound amendment.

## Regression coverage

Core tests cover schema validation, hostile paths, state replacement, parked
projection, and mixed provenance replay.

CLI tests cover provider envelope parsing, prompt contract, provider failure,
hard timeout, apply, dismiss, edit execution, action provenance, reopened
status, and unchanged dismissed status.

Status/UI tests pin proposal ordering ahead of the raw reason.

Plugin tests cover:

- disabled triage with byte-identical ticket, disposition, and ledger;
- failure with the existing park unchanged;
- timeout with the existing park unchanged;
- terminal attempt provenance for both cases; and
- the T-046 criteria/evidence amendment proposal and creation provenance.

## Commit

Committed the complete cross-crate source unit through Lisa's isolated
transaction.

Commit: `49ed4b2553faebd8cf42495805d8c210e7d976bd`.

Subject: `feat: triage operator-owned parks`.

The exact include list contained fourteen ticket-owned source paths.

No ordinary `git add`, `git commit`, or ordinary index was used.

## Verification

- `cargo fmt --all` — passed.
- `cargo check --workspace` — passed before the scoped commit.
- `cargo test -p lisa-core --no-fail-fast` — passed before later focused test
  additions; the new provenance test also passed independently.
- `cargo test -p lisa-cli --no-fail-fast` — 350 unit tests and all applicable
  integration tests passed before the concurrent Notes ticket added its private
  unexported fixtures; one expected real-Zellij test ignored.
- bounded runner focused tests — 4 passed.
- proposal action focused tests — 2 passed.
- `cargo test -p lisa-plugin --no-fail-fast` — 433 passed.
- plugin triage focused tests after provenance-order tightening — 2 passed.
- `git diff --check` — passed before commit.
- Exported commit `49ed4b2553faebd8cf42495805d8c210e7d976bd` to an isolated
  directory and ran `cargo test --workspace --no-fail-fast` — passed in full;
  the one environment-dependent real-Zellij test was ignored as designed.
- Ran `cargo clippy --workspace --all-targets -- -D warnings` against that same
  exact exported commit — passed with no warnings.
- Ran the repository `git diff --check` after the neighboring Notes ticket
  resumed its integration — passed.

## Concurrent-ticket deviation

T-049-06-02 began editing Notes queue files while this ticket was implementing.

Both tickets nominally touch core exports/provenance, CLI main/status/help, and
plugin projection/UI despite lacking a dependency edge.

The T-049-06-02 attempt detected the overlap and removed its shared-file edits,
retaining only its private untracked `notes.rs` and integration fixture files.

This ticket then formatted, tested, and committed its exact source set.

No T-049-06-02 file was included or adopted.

The Notes attempt resumed integration on top of commit `49ed4b2`. Its current
diffs are limited to Notes exports, command/status integration, help coverage,
and private Notes files. They were not included in this ticket's commit.

## Review readiness

Both acceptance criteria are covered by committed fixtures. The exact source
snapshot passes workspace tests and warning-denying Clippy. Review finds no
blocking concern, so the disposition is pass.
