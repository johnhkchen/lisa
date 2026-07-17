# Review: benefit-first context and run summary

## Disposition

Pass.

The implementation satisfies both acceptance criteria, is covered by focused
fixture tests, and leaves no T-047-01-01 source file uncommitted.

## Outcome

Lisa's generated agent context and per-ticket assignment prompt now lead with
the exact canonical purpose paragraph:

> Lisa runs coding agents like Claude Code and Codex through your ticket board,
> so you don't have to approve every step by hand.

That prose has one template source in `lisa-core`. Claude context, Codex agent
context, the rendered RDSPI workflow, and plugin assignments consume that
source. The checked-in workflow remains an installed/rendered output and
starts with the same paragraph.

`lisa status` and the foreground return from `lisa loop` now use one factual
run-summary renderer. It reports ticket completion, explicit latest-run
failure/timeout outcomes, manual interaction evidence, and only evidence paths
that actually exist.

## Files changed

Eleven unique repository source or knowledge files are owned by this ticket:

- `crates/lisa-core/src/context.rs` (new);
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-cli/data/rdspi-workflow.md`;
- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/src/init.rs`;
- `crates/lisa-cli/src/run_summary.rs` (new);
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/status.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `docs/knowledge/rdspi-workflow.md`.

No source file was deleted.

## Canonical context review

`lisa_core::context::PURPOSE_PARAGRAPH` contains the exact approved prose.

The Claude and Agents generators interpolate that constant before their agent
contract or operational material.

The raw workflow data file is mechanics-only. A `LazyLock<String>` renders the
purpose constant, a blank line, and the raw body. This avoids a second template
literal while retaining the expected installed workflow bytes.

The checked-in knowledge document equals the rendered workflow template.

The plugin's `ticket_prompt` begins with the constant for every provider. The
existing ticket path, attempt directory, commit, phase, and Review instructions
follow it unchanged.

No DAG semantics, phase ordering, scheduling rule, or Zellij routing rule was
changed by the context work.

## Run-boundary review

Immediately before a real loop launch, Lisa stores byte offsets for the
provenance and interaction-event ledgers in `.lisa/run-baseline.json`.

The baseline is written through a same-directory temporary file and rename.

The post-run reader considers only bytes appended after those offsets. Old
failure rows and old interactive gates cannot be attributed to the latest run.

The baseline also records whether the installed hook settings can substantiate
question and permission events. Missing, old, or malformed settings produce an
unavailable result rather than a fabricated zero.

Dry-run loop behavior remains unchanged and does not create a baseline.

## Interaction evidence review

Generated question hooks append a payload-free question row.

Generated permission/attention hooks append a payload-free permission row.

No question text, permission details, notification payload, or user response is
persisted.

Question hooks retain the scheduler `.awaiting` signal.

Permission hooks continue to exclude `idle_prompt` notifications.

The optional `on-notify` hook remains optional. Missing notification scripts
do not prevent event accounting and do not make the hook fail.

Generated and merged Claude settings now share the same hook command constants,
so upgrades do not create duplicate Lisa commands.

The generated `.lisa/.gitignore` covers `run-events.jsonl` and
`run-baseline.json` while preserving project-owned ignore additions.

## Summary truth rules

Completion and remaining counts come from the current scanned ticket board.

Failure and timeout counts require explicit latest-run provenance outcomes and
are filtered to ticket IDs on the current board.

Malformed or torn provenance JSONL makes outcome evidence unavailable. It is
never treated as a clean run.

Manual approval zero is printed only when the baseline proves the installed
hooks tracked both interactive event classes and the latest event segment is
readable and empty.

If a question or permission gate fires, its event is counted. Therefore an
interactive run cannot be summarized as requiring zero approvals.

The unattended-win sentence requires all tickets Done, readable and clean
latest-run outcome evidence, and readable tracked interaction evidence with
zero events.

Evidence paths are checked individually before rendering. Missing provenance
or completion ledgers are omitted.

The work directory is named only when a real document exists beneath the
configured directory for a ticket on the current board. An empty directory is
not represented as evidence.

## Surface integration

`lisa status` keeps its existing DAG, wave, and readiness report, then appends
the shared narrative summary.

`lisa loop` remains the foreground parent while Zellij runs, rescans the board
after Zellij exits, and prints the same summary before propagating a nonzero
Zellij exit as an error.

The implementation does not change ticket phase/status frontmatter, scheduler
selection, ticket completion authority, or publication ownership.

## Acceptance criteria

### AC1: benefit-first context

Pass.

String-level tests render Claude, Agents, and RDSPI workflow contexts. They
assert that each starts with the exact canonical paragraph, contains it exactly
once, and places it before every present case-insensitive occurrence of `DAG`,
`phase`, `scheduling`, and `Zellij`.

The source audit includes the core context source, plugin source, CLI template
source, and raw workflow data. It asserts exactly one template source contains
the complete literal.

Plugin tests assert all assignment prompts begin with the same constant.

### AC2: truthful clean, failed, and partial summaries

Pass.

Ten filesystem fixture tests cover:

- a clean completed board;
- an explicitly failed partial board;
- a partial board without a fabricated failure;
- an interactive question gate;
- missing ledgers and work paths;
- malformed or torn evidence;
- exclusion of previous-run failures and gates;
- real baseline offset capture;
- older installed hooks that cannot prove zero;
- an initialized but empty work directory.

Shell-level hook tests execute the generated POSIX commands and verify
payload-free rows, the retained question signal, successful absence of the
optional notification hook, and idle-prompt filtering.

## Verification

Final `cargo fmt --all -- --check`: passed.

Final `cargo test --workspace --quiet`: passed with 972 tests passed, zero
failed, and one intentionally ignored.

The ignored test is the pre-existing real-Zellij delivery boundary requiring
external Zellij, zsh, script, jq, and the wasm32-wasip1 target.

Focused template tests: 35 passed.

Focused run-summary tests: 10 passed.

Focused status, loop-command, and plugin ticket-prompt tests passed.

`git diff --check`: passed.

Strict workspace Clippy passed after the ticket implementation. After the final
workflow single-source refinement, strict `lisa-cli` all-target Clippy also
passed.

The last workspace Clippy rerun encountered five warnings solely in a
concurrent uncommitted edit to `crates/lisa-plugin/src/lib.rs`: unused parking
imports and unused review-block retry declarations. `git blame` identifies
those lines as `Not Committed Yet`; they are not part of any T-047-01-01 commit.
The ticket-owned plugin context change had passed the earlier workspace run.

## Commit ownership

All ticket source was committed with `lisa commit-ticket` and exact include
paths:

- `4d1384e13b455d3915e87602ac9350e1944c3367` — lead session context with Lisa's purpose;
- `e150f40153bf4daa35b9d221dd00e486b0ca4132` — report factual run outcomes;
- `931aa02e4a1cdec382dbacf0856fe76aa5ba9d89` — cover run evidence ignore rules;
- `cd9c43921d7d78298b883981ce56bee54e479f7f` — satisfy strict summary lint;
- `c506744fd98236c20d584a0253703a4a01d53765` — require real per-ticket evidence;
- `c4675cb3d1bd0805a28d12b636e0ed5e9c20524e` — single-source workflow purpose copy.

The remaining modified/untracked paths are Lisa-managed journals, active
ticket state, admitted phase artifacts, and concurrent ticket work. No
T-047-01-01 source remains staged, modified, or untracked.

## Open concerns

The real interactive Zellij boundary was not executed because its existing
integration test is intentionally ignored without the external dependency set.
Loop command construction and post-exit behavior are unit-tested, and all
workspace tests pass.

Projects initialized before interaction tracking was introduced will see
“manual approval tracking is unavailable” until `lisa init` upgrades their
owned hook settings. This is intentional: unknown evidence must not be shown as
zero approvals.

The concurrent `lisa-plugin` lint warnings should be resolved by their owning
ticket before the repository's next clean all-target Clippy run. They do not
alter this ticket's implementation or acceptance result.

No ticket-owned blocker remains.
