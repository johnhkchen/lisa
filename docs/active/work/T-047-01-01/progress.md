# Progress: benefit-first context and run summary

## Status

Implementation is complete.

All ticket-owned source changes are committed through `lisa commit-ticket`.

Focused tests, the full workspace suite, and formatting pass. Strict Clippy
passed before concurrent work modified `lisa-plugin`; the final rerun reaches
only five warnings in that uncommitted, non-ticket-owned edit.

No ticket-owned source file remains modified, staged, or untracked.

## Completed: canonical purpose copy

Created `crates/lisa-core/src/context.rs`.

It defines the exact T-046-07-02 paragraph once as `PURPOSE_PARAGRAPH`.

Exported the module from `crates/lisa-core/src/lib.rs`.

The shared location is dependency-safe for both CLI and plugin consumers.

## Completed: generated context

Updated `crates/lisa-cli/src/templates.rs`.

`generate_claude_md` now interpolates the shared purpose constant.

Replaced the static `AGENTS_MD` prose constant with `generate_agents_md()`.

The Agents generator also interpolates the shared purpose constant.

Updated `crates/lisa-cli/src/init.rs` to call the generator.

Existing project-owned `AGENTS.md` no-overwrite behavior is unchanged.

The one-line RDSPI agent contract remains immediately after the purpose.

## Completed: workflow preamble

Kept `crates/lisa-cli/data/rdspi-workflow.md` as the mechanics-only body.

`RDSPI_WORKFLOW` renders the shared purpose constant, a blank line, and that
body. The checked-in `docs/knowledge/rdspi-workflow.md` matches the rendered
template byte-for-byte.

No phase rule or workflow mechanic changed.

## Completed: assignment framing

Updated `ticket_prompt` in `crates/lisa-plugin/src/lib.rs`.

Every Claude and Codex ticket assignment now starts with the shared paragraph.

The prior `Read the ticket ...` instruction follows after a blank line.

Provider selection, artifact paths, commit rules, and Review rules are
unchanged.

## Completed: purpose-first tests

Template tests render Claude, Agents, and embedded workflow context.

They require the canonical paragraph exactly once in each output.

They require it before every present case-insensitive occurrence of:

- `DAG`;
- `phase`;
- `scheduling`;
- `Zellij`.

The test reads every context-producing Rust source plus the raw workflow body
and asserts the complete prose literal has exactly one template source.

Plugin tests require every ticket prompt to begin with the canonical constant.

## Completed: latest-run baseline

Created `crates/lisa-cli/src/run_summary.rs`.

Immediately before launching Zellij, the CLI records current byte lengths for:

- `.lisa/provenance.jsonl`;
- `.lisa/run-events.jsonl`.

The baseline is serialized at `.lisa/run-baseline.json`.

It is published through a same-directory temporary and rename.

The summary reads only bytes appended after those offsets.

Earlier-run failures and gates are therefore excluded.

## Completed: tracking capability guard

The baseline also records whether installed Claude hook settings contain both
the question and permission event writers.

If the installed project context predates this feature, the summary reports
manual-approval tracking as unavailable.

It does not infer zero from an absent event file.

Malformed settings likewise disable a zero claim.

## Completed: interactive-gate evidence

Updated the AskUserQuestion command to append:

```json
{"event":"manual-intervention","kind":"question"}
```

Updated the permission/attention command to append:

```json
{"event":"manual-intervention","kind":"permission"}
```

The commands retain no prompt or notification payload.

The question command still writes its `.awaiting` scheduler signal.

The permission command still skips `idle_prompt` events.

The optional `on-notify` hook still runs only when executable.

Both hook commands now exit successfully when that optional hook is absent.

## Completed: generated settings single source

Changed `settings_local_json()` to build its empty settings object through
`merge_hooks`.

Generated and merged settings now consume the same hook command constants.

This removes the prior second hardcoded copy of both interaction commands.

Improved Lisa-hook matching to identify owned commands by known hook path.

Existing settings with the older attention/question commands are upgraded
instead of duplicated.

## Completed: runtime ignore rules

Extended the generated `.lisa/.gitignore` rules with:

- `run-events.jsonl`;
- `run-baseline.json`.

`lisa init` appends these entries while preserving project-owned ignore rules.

Updated exact-output and customization-preservation tests.

## Completed: factual outcome loading

The summary uses current ticket phases for completed and remaining counts.

It parses latest-run provenance lines as generic JSON values.

That supports old and current provenance schema shapes.

It filters records to ticket IDs on the current board.

It counts only explicit `failed` and `timed-out` outcomes.

Malformed or torn JSONL makes outcome evidence unavailable.

It never converts malformed evidence into a clean run.

## Completed: narrative rendering

Both surfaces use the same writer-based renderer.

Every nonempty board reports exact completed/total counts.

Partial boards report exact remaining counts.

Explicit failed and timed-out attempts are reported with real counts.

A tracked event segment with no gate rows prints:

> Manual approvals requested: 0.

A gate event prints the exact aggregate, question, and permission counts.

An untracked or malformed segment prints an unavailable statement.

The full unattended-win sentence requires:

- every board ticket Done;
- readable latest-run outcomes with no failure or timeout;
- readable latest-run gate tracking with zero events.

## Completed: evidence paths

The renderer tests each path on disk before naming it.

It names `.lisa/provenance.jsonl` only when present.

It names `.lisa/completion-journal.jsonl` only when present.

It names the configured work directory only when a current board ticket has an
actual work-document file under that directory.

If none exist, the `Evidence:` line is omitted.

## Completed: status integration

`crates/lisa-cli/src/status.rs` retains the scanned ticket vector.

It still builds and prints the same DAG/wave scheduling report.

It invokes the narrative after the ready-to-schedule result.

It passes the resolved configured work directory.

The command remains read-only.

## Completed: post-loop integration

`crates/lisa-cli/src/loop_cmd.rs` now retains the CLI as Zellij's foreground
parent instead of replacing it with Unix `exec`.

After Zellij exits, it rescans tickets and invokes the shared renderer.

The narrative is printed before returning a nonzero Zellij exit error.

Dry-run behavior is unchanged and creates no run baseline.

## Fixture coverage

`run_summary` has ten fixture tests.

They cover:

- a clean two-ticket board;
- a failed partially completed board;
- a partially completed board without fabricated failure;
- an interactive question gate;
- missing ledger and work paths;
- malformed/torn evidence;
- exclusion of previous-run failures and gates;
- real baseline offset capture;
- old installed hooks that cannot substantiate zero.
- an initialized but empty work root that is not evidence.

Hook tests execute the POSIX commands in temporary directories.

They verify two payload-free rows, the retained `.awaiting` signal, successful
absence of `on-notify`, and no false permission row for `idle_prompt`.

## Commit record

### Context unit

Commit `4d1384e13b455d3915e87602ac9350e1944c3367`.

Message: `T-047-01-01: lead session context with Lisa's purpose`.

Seven exact include paths; no ticket/journal/artifact paths.

### Reporting unit

Commit `e150f40153bf4daa35b9d221dd00e486b0ca4132`.

Message: `T-047-01-01: report factual run outcomes`.

Five exact include paths.

### Init expectation correction

Commit `931aa02e4a1cdec382dbacf0856fe76aa5ba9d89`.

Message: `T-047-01-01: cover run evidence ignore rules`.

One exact include path.

### Strict lint correction

Commit `cd9c43921d7d78298b883981ce56bee54e479f7f`.

Message: `T-047-01-01: satisfy strict summary lint`.

One exact include path.

### Per-ticket evidence refinement

Commit `c506744fd98236c20d584a0253703a4a01d53765`.

Message: `T-047-01-01: require real per-ticket evidence`.

One exact include path.

### Single-source workflow refinement

Commit `c4675cb3d1bd0805a28d12b636e0ed5e9c20524e`.

Message: `T-047-01-01: single-source workflow purpose copy`.

Three exact include paths.

## Plan deviations

The planned two commits became six.

The full suite found three exact `.gitignore` expectations after the main
reporting commit; their test-only update was committed separately.

Strict Clippy then identified one needless generic-argument borrow; its one-line
cleanup was committed separately.

Final truthfulness review tightened work evidence from “directory exists” to
“a current ticket has a real document”; that focused refinement was also
committed separately.

A final source audit found that the workflow body also contained the canonical
literal. The last refinement composes the rendered workflow from the shared
constant and mechanics-only body, so the exact prose now has one template
source while the checked-in installed output remains unchanged.

This preserved the requirement that no ticket source remain modified while
keeping the corrections independently auditable.

The design originally described an integration-style captured `status` test as
optional.

The shared renderer's filesystem fixtures provide the exact output assertions,
while existing status tests exercise the real status call path.

## Verification

`cargo fmt --all -- --check` passed.

`cargo test --workspace --quiet` passed.

Workspace result: 972 passed, 0 failed, 1 intentionally ignored.

The ignored test is the existing real-Zellij boundary that requires external
Zellij/zsh/script/jq/wasm32 dependencies.

`cargo clippy --workspace --all-targets -- -D warnings` passed after the ticket
implementation and `cargo clippy -p lisa-cli --all-targets -- -D warnings`
passed after the final workflow refinement.

The final workspace Clippy rerun was externally blocked by five warnings in an
uncommitted concurrent edit to `crates/lisa-plugin/src/lib.rs` (parking/review
block symbols). `git blame` marks those lines `Not Committed Yet`; none are
owned by T-047-01-01.

Focused template tests passed: 35.

Focused run-summary tests passed: 10.

Focused status tests passed.

Focused loop command tests passed.

Focused plugin ticket-prompt tests passed.

`git diff --check` passed.

The canonical exact-copy search finds one template literal in
`lisa-core/src/context.rs` plus the intentional checked-in rendered Markdown
output.

## Ownership audit

`git status --short` contains no modified or untracked ticket source.

Remaining changes are Lisa-managed provenance/completion journals, active
ticket frontmatter, concurrent-ticket state, and published phase artifacts.

None were included in ticket source commits.

## Remaining work

Write Review artifacts.

Remain on T-047-01-01 after Review and wait for Lisa completion confirmation.
