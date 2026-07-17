# Design: benefit-first context and run summary

## Goals

Put the already-approved purpose paragraph before workflow mechanics in every
managed session surface.

Keep that paragraph byte-identical and sourced once.

Add a compact narrative that a nontechnical operator can repeat.

Base every count and claim on observable board or runtime evidence.

Retain enough run-boundary and interactive-gate evidence to avoid inference.

Do not change scheduling, completion, phase transitions, or ticket state.

## Purpose-copy ownership

### Option A: keep duplicated literals and compare them in tests

This preserves the present static `AGENTS_MD` shape.

It does not meet the new requirement that the prose have one template source.

The copies already drift only by human discipline.

### Option B: define the constant in the CLI template module

This naturally owns `CLAUDE.md`, `AGENTS.md`, and workflow output.

The plugin assignment also needs the paragraph.

The plugin cannot import the CLI crate without creating a dependency cycle.

It would force one remaining duplicate in the most important session surface.

### Option C: define stable session copy in `lisa-core`

Both the CLI and plugin already depend on `lisa-core`.

A small `context` module can own the exact paragraph as a public constant.

CLI templates interpolate it into generated project context.

The plugin assignment interpolates the same value before instructions.

Tests import the constant instead of repeating expected prose.

### Decision

Choose Option C.

The paragraph is product-level shared vocabulary, not CLI-only behavior.

The lower shared module is the only dependency-safe single source.

Change the static `AGENTS_MD` constant into `generate_agents_md() -> String` so
the canonical constant can be interpolated.

`init.rs` already handles owned strings for generated files.

No external library API exposes the binary's private template module.

## Benefit-first surfaces

### Generated CLAUDE.md and AGENTS.md

Keep each Markdown H1 first.

Put the canonical paragraph immediately after the H1.

The heading is structural, not mechanism prose.

Retain the agent contract immediately after the purpose paragraph.

### Ticket assignment

Put the canonical paragraph and a blank paragraph separator before `Read the
ticket at ...`.

This makes the first user-visible prose state the benefit before phase and
workflow instructions.

Both provider adapters inherit the change through `ticket_prompt`.

### Workflow document

Put the purpose paragraph before `## RDSPI Workflow` in both the repository
knowledge file and the embedded init template.

The embedded and checked-in workflow files remain byte-identical.

The source-of-truth Rust constant still defines expectations in tests.

The checked-in Markdown is an installed/generated context artifact, not a
second Rust template literal used to build multiple outputs.

## Ordering tests

Define a helper that locates the canonical paragraph and the mechanism terms
`DAG`, `phase`, `scheduling`, and `Zellij`, case-insensitively.

For each rendered context, require every present mechanism term to occur after
the purpose paragraph.

Test generated Claude context, generated Agents context, and embedded workflow.

Test ticket assignments separately in the plugin module.

Also read `templates.rs` itself in its test and assert the complete canonical
sentence is not retyped there.

The only test expectation is the shared constant.

## Narrative summary placement

Create a focused `crates/lisa-cli/src/run_summary.rs` module.

It owns evidence loading, summary facts, and writer-based rendering.

`status.rs` remains responsible for the existing DAG/wave report.

After the ready-to-schedule line, `status` invokes the narrative renderer.

`loop_cmd.rs` establishes a run baseline immediately before launching Zellij.

After Zellij exits, it invokes the same renderer.

The two surfaces therefore cannot drift in wording or truth rules.

## Keeping the CLI alive

### Option A: ask the Zellij plugin to print the summary

The plugin already knows when a board drains.

It cannot naturally print after the session has ended.

It would duplicate status-side filesystem reporting in WASM.

### Option B: run a detached wrapper process before Unix `exec`

A wrapper could wait for Zellij and print later.

It complicates terminal ownership and error propagation.

The detached output may land after the invoking shell prompt.

### Option C: spawn Zellij as a child and wait

The non-Unix implementation already follows this shape.

The CLI resumes on every platform when the session ends.

It can render the summary and preserve Zellij's exit-status handling.

### Decision

Choose Option C.

Replace the Unix-only `CommandExt::exec` branch with one shared child-status
function.

Zellij remains the foreground child attached to inherited terminal streams.

This is process-lifecycle plumbing, not scheduler behavior.

## Summary facts

The summary has four independent fact groups.

Board facts come from scanned tickets and `Dag::stats`.

Run outcome facts come from provenance rows written after the latest baseline.

Manual-intervention facts come from an append-only runtime event ledger after
the latest baseline.

Evidence paths come from `Path::exists` checks.

## Board wording

Always report `Completed: N of M tickets` when tickets exist.

When `N == M`, say every ticket is complete.

When tickets remain, include the exact remaining count.

Do not use “finished the run” merely because some tickets are done.

Do not derive completion counts from provenance because the board is the
current scheduling authority.

## Failure wording

Parse only provenance bytes appended after the recorded baseline.

Read each line as `serde_json::Value` for old/new schema tolerance.

Count explicit `failed` and `timed-out` outcomes.

Do not treat malformed rows as successful or clean.

If the baseline or provenance segment cannot be trusted, mark outcomes unknown
and omit any clean-run claim.

If explicit failures exist, print their real counts even when the ticket later
completed.

That accurately says the run encountered failures without saying the board is
still failed.

## Manual-intervention evidence

### Why new evidence is required

The `.awaiting` file is deleted after ingestion.

Notification hooks are transient.

Launch flags are intent, not proof.

Zero cannot be inferred from the absence of these transient files.

### Event ledger

Use `.lisa/run-events.jsonl` as an append-only host runtime ledger.

The question hook appends a static JSON row with kind `question`.

The permission/attention hook appends a static JSON row with kind `permission`.

The append happens whether or not the optional user notification hook exists.

No payload or potentially sensitive question text is persisted.

The hook's existing scheduling signal and notification behavior remain intact.

### Run baseline

Use `.lisa/run-baseline.json` with schema version and byte offsets for the
provenance and event ledgers.

Write it atomically immediately before starting Zellij.

The offsets define exactly which facts belong to the latest loop.

An empty event segment after a valid baseline is positive evidence of zero
interactive gates.

Without a valid baseline, report tracking as unavailable and never claim zero.

If one or more gate rows exist, report the count and never print the zero-
approval sentence.

## Benefit sentence

Only when all of these are true:

- every board ticket is complete;
- the latest provenance segment is readable and has no failures/timeouts;
- the event segment is readable and contains zero interactive gates;

print a plain win sentence:

> Lisa completed the board without asking you to approve steps by hand.

Then print the exact ticket count and zero-approval fact.

If any condition is unknown, the output remains factual but does not claim the
full unattended win.

## Evidence rendering

Check `.lisa/provenance.jsonl` before naming it.

Check `.lisa/completion-journal.jsonl` before naming it.

Check the configured work directory before naming per-ticket work docs.

Render one `Evidence:` line containing only the existing paths.

If none exist, omit the line entirely.

Do not create missing evidence merely so it can be named.

## Fixture strategy

Use one fixture builder to write ticket Markdown and optional runtime files.

Capture output through `write_summary(..., &mut Vec<u8>)`.

Clean fixture: all tickets done, valid baseline, authoritative done provenance,
empty event segment, all three evidence paths present.

Failed fixture: partial board plus a failed provenance row after baseline.

Partial fixture: some tickets done, no fabricated failure count.

Interactive-gate fixture: all tickets done plus a question event after baseline.

Missing-evidence fixture: valid board with absent ledger and work paths.

Assert exact counts and the presence or absence of benefit claims and paths.

## Rejected scope

Do not change `Dag` statistics.

Do not add run IDs to provenance or completion schemas.

Do not alter plugin termination detection.

Do not retain question payloads.

Do not claim the optional `on-notify` hook was invoked successfully.

Do not update ticket phase/status fields.
