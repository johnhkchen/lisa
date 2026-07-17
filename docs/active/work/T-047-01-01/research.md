# Research: benefit-first context and run summary

## Ticket scope

T-047-01-01 changes two operator-facing areas.

The first is context injected into managed agent sessions.

The second is a factual narrative shown after a loop and by `lisa status`.

The ticket explicitly excludes scheduler, completion, and state-machine changes.

The output must distinguish clean, failed, and partially completed boards.

It may claim zero manual approvals only when an interactive gate did not fire.

Evidence paths may be printed only when they exist.

## Repository and workflow constraints

`CLAUDE.md` is the repository's agent-context source of truth.

`docs/knowledge/rdspi-workflow.md` defines the six mandatory phases.

The current attempt owns only files explicitly committed for T-047-01-01.

Phase artifacts belong under `.lisa/attempts/T-047-01-01/1/work/`.

Lisa publishes admitted phase artifacts later.

The worktree contains unrelated modified and untracked files.

Ticket commits therefore need exact repository-relative include paths.

## Canonical purpose copy

T-046-07-02 landed this paragraph:

> Lisa runs coding agents like Claude Code and Codex through your ticket board,
> so you don't have to approve every step by hand.

The text appears literally in two places in `crates/lisa-cli/src/templates.rs`.

`AGENTS_MD` contains one copy.

`generate_claude_md` contains the other copy.

The T-046-07-02 test defines a third test-local copy.

That test verifies ordering within generated `CLAUDE.md` and `AGENTS.md`.

The public `AGENTS_MD` surface is currently a static `&str` constant.

`generate_claude_md` already returns an owned `String`.

`init.rs` writes `AGENTS_MD` for new projects.

It does not overwrite an existing user `AGENTS.md`.

The setup guide also reads the generated Claude template.

## Workflow preamble

`templates::RDSPI_WORKFLOW` embeds `crates/lisa-cli/data/rdspi-workflow.md`.

The embedded data file currently begins with `## RDSPI Workflow`.

Its first prose explains the six phases and continuous execution.

It does not state Lisa's purpose first.

The embedded data file is byte-identical to
`docs/knowledge/rdspi-workflow.md`.

`lisa init` publishes the embedded workflow to the project knowledge path.

The init planner recognizes exact legacy workflow generations.

Changing the embedded bytes therefore participates in the existing owned-
template upgrade mechanism.

## Assignment framing

The provider-neutral assignment text lives in
`crates/lisa-plugin/src/lib.rs`, in `ticket_prompt`.

The adapter selects `CLAUDE.md` or `AGENTS.md` as the provider context file.

The rest of the assignment body is shared between providers.

The assignment currently begins `Read the ticket at ...`.

It then names current phase, every remaining RDSPI phase, artifact paths,
commit rules, Review requirements, and completion ownership.

The first benefit statement is absent.

`ClaudeCodeAdapter::assignment_text` and `CodexAdapter::assignment_text` both
delegate to `ticket_prompt`.

Tests around the adapter assert provider context selection and assignment
details, but not benefit-first ordering.

The CLI crate depends on `lisa-core` and `lisa-plugin`.

The plugin crate also depends on `lisa-core`.

The plugin cannot depend on the CLI without a dependency cycle.

Shared copy needed by CLI templates and plugin assignments therefore requires
a lower shared boundary or duplicated literals.

## Status command

`crates/lisa-cli/src/status.rs` owns ordinary `lisa status` output.

It loads resolved configuration, falling back to defaults on config failure.

It scans the configured ticket directory.

It builds a `Dag` and rejects missing dependencies or cycles.

It prints DAG size, edge count, critical path, scheduling counts, config,
execution waves, and ready ticket IDs.

It returns early when the ticket directory is empty.

The command currently has no narrative about the operator benefit.

Its tests build temporary project fixtures and invoke `run_status`.

The tests currently verify success or failure, not captured output.

`run_status` writes directly through `println!`, which makes exact output
fixtures harder to assert without redirecting process stdout.

A writer-oriented internal function would match the pattern used by
`preownership_status.rs` and make fixture output deterministic.

## Board counts

`Dag::stats` counts a ticket as done when its phase is `Phase::Done`.

It supplies total, done, ready, in-progress, and blocked counts.

The plugin's clean termination check also requires every ticket phase to be
Done and no thread to remain running.

Using phase-based done counts is therefore consistent with existing UI and
termination semantics.

Partially completed state can be expressed without inference as total minus
done.

## Loop process boundary

`crates/lisa-cli/src/loop_cmd.rs` owns `lisa loop` startup.

After preflight it writes the embedded WASM and `.lisa-layout.kdl`.

On Unix, `exec_zellij` uses `CommandExt::exec`.

That replaces the Lisa CLI process with Zellij.

No Lisa CLI code can execute after the Zellij session exits on Unix.

The non-Unix branch already uses `Command::status` and returns after Zellij.

A post-run CLI narrative therefore requires retaining the parent process.

The Zellij command already has a pure constructor tested for path, args, and
working directory.

There are no tests that require Unix `exec` replacement specifically.

## Provenance evidence

`.lisa/provenance.jsonl` is the append-only execution ledger.

Current schema execution records include ticket ID, outcome, authoritative
flag, attempt lease, route, timing, usage, concurrency, and pane.

Outcomes are `done`, `failed`, or `timed-out`.

The repository ledger also contains older schema records.

Older execution records omit current fields such as attempt lease and
authoritative.

A summary reader concerned only with ticket ID and outcome can read JSON values
instead of requiring the newest typed record.

The ledger may contain records from earlier boards and earlier attempts.

Counting every historical row as part of the latest loop would be inaccurate.

## Completion evidence

`.lisa/completion-journal.jsonl` is a durable completion transition journal.

Confirmed records bind a completion/ticket ID to a Git commit ID.

The plugin restores this journal before scheduling.

The path may not exist in a new or never-completed project.

The summary only needs to name the journal when the file exists.

It does not need to reimplement completion reduction.

## Per-ticket work evidence

Resolved configuration includes a `work_dir`.

The default is `docs/active/work`.

Current-attempt artifacts are private until Lisa admits and publishes them.

Published per-ticket work directories may therefore be absent even when a
board has tickets.

The evidence line must check the configured work path before naming it.

## Interactive-gate evidence

Claude's `PreToolUse[AskUserQuestion]` hook writes a transient `.awaiting`
signal.

The plugin consumes and deletes that signal.

The in-memory `awaiting_human` flag clears on the next heartbeat.

The event is not retained in provenance or the completion journal.

Claude's catch-all `Notification` hook can represent a permission/attention
request.

That hook currently does nothing unless the user opted into `on-notify`.

The notification is likewise not retained by Lisa.

Codex is launched in no-approval/full-access mode and does not expose the same
AskUserQuestion hook capability.

Launch flags reduce approval prompts but are not evidence that none fired.

Absence of an `.awaiting` signal after the loop is also not evidence because
the signal is deliberately consumed.

The current files therefore cannot truthfully support a durable zero-approval
claim after a loop.

## Run boundary evidence

The provenance ledger has ticket attempts but no loop/run identifier.

The completion journal also has no loop identifier.

Without a baseline, a failed attempt from an earlier loop can be mistaken for
a failure in the latest loop.

A run summary needs a small boundary record if it is to report latest-run
failures rather than lifetime history.

The loop command is the natural host-side place to establish that boundary
before Zellij starts.

## Testing boundaries

Template string tests belong in `templates.rs` and plugin assignment tests.

They can compare byte-for-byte against one exported canonical paragraph.

They can locate forbidden mechanism words and assert the purpose comes first.

Run-summary tests can create complete temporary project fixtures.

Fixtures can include ticket Markdown, provenance JSONL, completion journal,
work directories, a run baseline, and interactive-gate rows.

Writer-based rendering permits assertions for clean, failed, partial, gate,
and missing-evidence cases without launching Zellij.

Loop command tests can verify the retained child-process helper separately from
the summary renderer.

## Observed constraints

Summary computation must remain read-only in `lisa status`.

Loop-start bookkeeping may write only Lisa runtime state.

No summary result may alter ticket frontmatter or DAG scheduling.

Malformed optional evidence should degrade to omission or explicit unknown,
not fabricate clean state.

Existing unrelated worktree modifications must remain untouched.
