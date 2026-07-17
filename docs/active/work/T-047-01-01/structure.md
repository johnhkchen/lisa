# Structure: benefit-first context and run summary

## Change summary

The implementation has one shared-copy boundary and one summary boundary.

`lisa-core` owns stable benefit copy used across crates.

`lisa-cli::run_summary` owns runtime evidence and narrative rendering.

Existing CLI and plugin entry points call those boundaries.

## New file: `crates/lisa-core/src/context.rs`

This module defines `PURPOSE_PARAGRAPH: &str`.

The constant contains the exact paragraph landed by T-046-07-02.

It contains no rendering or filesystem behavior.

The module documentation explains why cross-crate session copy belongs here.

## Modified file: `crates/lisa-core/src/lib.rs`

Export the new `context` module.

No existing exports change.

## Modified file: `crates/lisa-cli/src/templates.rs`

Import `lisa_core::context::PURPOSE_PARAGRAPH`.

Replace literal purpose prose in generated Claude context with interpolation.

Replace static `AGENTS_MD` with `generate_agents_md() -> String`.

Interpolate the shared purpose into generated Agents context.

Update tests to consume rendered Agents output.

Remove test-local copies of the paragraph.

Add ordering assertions for generated session context and workflow output.

Add a source-level regression assertion against retyping the full canonical
paragraph in this template file.

Extend the question and permission hook commands with append-only run-event
rows.

Preserve `.awaiting` signals and optional `on-notify` dispatch.

Update hook tests to require both event kinds.

## Modified file: `crates/lisa-cli/src/init.rs`

Call `templates::generate_agents_md()` when planning `AGENTS.md` creation.

Update tests and fixture writes that currently use `AGENTS_MD` directly.

Existing no-overwrite behavior remains unchanged.

Existing hook merge logic detects Lisa-owned commands and upgrades them to the
new exact command.

## Modified file: `crates/lisa-plugin/src/lib.rs`

Import `PURPOSE_PARAGRAPH` from `lisa-core`.

Prefix `ticket_prompt` output with the paragraph and a blank line.

Add a focused test that verifies purpose order before all present mechanism
terms.

All adapter and scheduler call sites remain unchanged.

## Modified embedded workflow files

`crates/lisa-cli/data/rdspi-workflow.md` gains the purpose paragraph before its
RDSPI heading.

`docs/knowledge/rdspi-workflow.md` receives the same change.

The files remain byte-identical for init ownership tests.

No workflow rules change.

## New file: `crates/lisa-cli/src/run_summary.rs`

This is a private CLI module.

It defines runtime paths as internal constants:

- `.lisa/provenance.jsonl`;
- `.lisa/completion-journal.jsonl`;
- `.lisa/run-events.jsonl`;
- `.lisa/run-baseline.json`.

It defines a serialized `RunBaseline` with schema version and two byte offsets.

It defines a small internal evidence enum for known counts versus unavailable
tracking.

It defines summary facts separate from rendering.

### `record_run_baseline(root)`

Create `.lisa/` if needed.

Ensure the event ledger exists.

Read current provenance and event file lengths.

Serialize the baseline.

Write to a process-specific temporary in `.lisa/`.

Rename the temporary over the destination atomically.

Return actionable string errors.

### `write_run_summary(root, tickets, work_dir, writer)`

Accept already-scanned tickets so status does not scan twice.

Count total and `Phase::Done` tickets.

Load the latest baseline.

Read provenance and event segments beginning at their offsets.

Parse JSON lines without requiring one provenance schema version.

Filter provenance outcomes to ticket IDs on the current board.

Build existing evidence-path labels.

Render a blank separator plus `Run summary:` and the narrative.

### `print_run_summary(root, tickets, work_dir)`

Thin stdout wrapper around the writer form.

Used by status and loop.

### Test-only fixture helpers

Write minimal ticket facts directly or build `Ticket` values.

Write baseline-prefixed provenance and event data.

Render into memory.

Assertions cover clean, failed, partial, gated, missing-path, and malformed-
evidence behavior.

## Modified file: `crates/lisa-cli/src/main.rs`

Declare the private `run_summary` module.

Command dispatch remains unchanged.

## Modified file: `crates/lisa-cli/src/status.rs`

Keep the scanned ticket vector long enough to pass a slice to the summary.

Build the DAG from a clone because `Dag::from_tickets` consumes its vector.

After the ready-to-schedule output, call the shared summary renderer.

Pass the configured work directory.

Propagate summary read/write errors through the existing `Result`.

The command remains read-only.

## Modified file: `crates/lisa-cli/src/loop_cmd.rs`

Replace platform-specific `exec_zellij` functions with one child-status helper.

Immediately before launching Zellij, call `record_run_baseline`.

Wait for the foreground Zellij child.

After it exits, rescan the configured ticket directory.

Call the shared summary renderer with current ticket facts.

Return an error after rendering when Zellij exits unsuccessfully.

Dry-run flow does not create a baseline and does not print a post-run narrative.

## Runtime data shapes

### Baseline JSON

The baseline is one JSON object:

```json
{"schema_version":1,"provenance_bytes":123,"event_bytes":45}
```

Offsets are filesystem byte lengths measured immediately before Zellij starts.

### Event JSONL

Question hook row:

```json
{"event":"manual-intervention","kind":"question"}
```

Permission hook row:

```json
{"event":"manual-intervention","kind":"permission"}
```

Rows contain no prompt payload and no ticket inference.

## Narrative layout

Clean verified board:

```text
Run summary:
Lisa completed the board without asking you to approve steps by hand.
Completed: 2 of 2 tickets.
Manual approvals requested: 0.
Evidence: .lisa/provenance.jsonl; .lisa/completion-journal.jsonl; docs/active/work/
```

Partial or failed boards omit the unattended-win sentence.

They print exact completed/remaining and explicit recorded issue counts.

Interactive gates replace the zero line with an exact gate count.

Unknown tracking prints an availability statement, not zero.

## Ownership boundaries

The summary reads ticket state but never writes tickets.

Only `record_run_baseline` and hooks write runtime data under `.lisa/`.

The plugin state machine never reads the new baseline or event ledger.

Provenance and completion schemas remain unchanged.

The user notification hook remains optional.

## Commit units

Commit the shared purpose and benefit-first context as one unit.

Commit runtime evidence and summary rendering as a second unit.

Each commit uses only exact paths touched by that unit.

Phase artifacts are not included in source commits.
