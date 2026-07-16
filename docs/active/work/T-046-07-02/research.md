# Research: purpose-first docs and templates

## Ticket boundary

Ticket `T-046-07-02` is a copy and template test change.

It owns three user-facing areas:

- the opening paragraph in `README.md`;
- the `CLAUDE.md` and `AGENTS.md` content scaffolded by `lisa init`;
- one provenance sentence in the output of `lisa setup-guide`.

The ticket does not add commands, files written at runtime, or provenance behavior.

The sibling ticket `T-046-07-01` owns CLI-facing strings and guide preambles.

That sibling change is already present in the current branch.

In particular, `build_guide` already opens with a purpose sentence.

The sentence says Lisa runs coding agents through a ticket board so the user does
not have to approve every step by hand.

The hooks guide and Clap help use the same sentence.

This ticket must not undo those changes or restructure the help surface.

## README today

`README.md` begins with the `# Lisa` heading and a release badge.

Its first prose paragraph is currently:

> DAG-driven concurrent task scheduling for AI-assisted development.

That sentence leads with mechanism vocabulary.

It does not name either supported coding agent.

It also does not state the operator benefit described by this ticket.

The README explains the behavior later under `What It Does`.

That later section says Lisa schedules interdependent tasks and runs Claude Code
sessions.

It then explains Zellij, dependency graphs, concurrent sessions, the dashboard,
and the six RDSPI phases.

The later explanation is accurate but does not repair the first-impression gap.

The prerequisites section calls Claude Code the default client.

It also links to the experimental Codex client section.

The Codex section says `lisa init` scaffolds both client context files.

The README's workflow section already describes per-ticket work artifacts as
review checkpoints and crash-recovery inputs.

The atomic-completion section explains isolated ticket commits and the final
completion transaction.

It says provenance is published after Lisa verifies the completion commit.

No single discoverable sentence names all three pieces requested by the ticket:

- the append-only attempt ledger;
- the completion journal that ties tickets to commits;
- the work documents kept for each ticket.

The most natural existing context for that sentence is the workflow discussion,
where readers already learn what the per-ticket documents are for.

## Generated CLAUDE.md today

`crates/lisa-cli/src/templates.rs` owns `generate_claude_md`.

The function receives a `DetectedProject`.

It derives a readable project type label.

It conditionally renders a Build and Test section.

It conditionally renders a Source Layout section.

It always renders directory conventions and the RDSPI workflow reference.

The generated document currently begins with `# CLAUDE.md` and `## Project`.

Its first body line has the detected name and type followed by a TODO asking for
a one-line project description.

Nothing before that TODO says what Lisa does.

Nothing states the contract followed by an agent working under Lisa.

The generated file is written by `plan_init` only when `CLAUDE.md` is absent.

Existing user-authored `CLAUDE.md` files are preserved.

Therefore template changes affect new initialization, not existing project files.

The setup guide embeds the output of `generate_claude_md`, so a template change
also appears in the setup guide's generated-template code block.

## Generated AGENTS.md today

`templates::AGENTS_MD` is a static string.

`lisa init` writes it only when `AGENTS.md` is absent.

Codex loads this file natively.

The file points to `CLAUDE.md` as the shared source of project context.

It repeats the RDSPI workflow location.

Its comments explicitly avoid duplicating the project body, build commands, and
source layout so Claude and Codex instructions do not drift.

The file currently starts with the source-of-truth pointer.

It contains neither Lisa's purpose nor an agent's one-line operating contract.

Adding two stable orientation paragraphs does not duplicate project-specific
context and does not weaken the pointer model.

## Template tests today

Unit tests live in the `templates.rs` `tests` module.

`test_generate_claude_md_rust` checks the heading, detected project metadata,
commands, source layout, ticket directory, and RDSPI reference.

Node and unknown-project tests cover their conditional branches.

`test_agents_md_points_to_claude` checks the heading, `CLAUDE.md` pointer, RDSPI
reference, and absence of duplicated Build and Test or Source Layout sections.

No test currently asserts purpose-first copy.

No test currently asserts an operating contract in either generated file.

The acceptance criteria explicitly require template tests, so assertions belong
beside these existing template unit tests.

The sibling ticket added string-order tests for CLI and guide surfaces in
`crates/lisa-cli/tests/help_surface.rs`.

This ticket need not change help grouping or snapshots.

## Setup guide today

`crates/lisa-cli/src/setup_guide.rs` builds a Markdown guide from seven sections.

`render_guide` emits the header and numbered sections.

`build_guide` detects the project and creates the header.

The current header already carries the sibling ticket's purpose sentence before
the `lisa-loop` mechanism wording.

`section_init` lists generated paths for an uninitialized project.

That table includes `docs/active/work/` and describes it as work artifacts, one
subdirectory per ticket.

The guide does not mention `.lisa/provenance.jsonl`.

It does not mention `.lisa/completion-journal.jsonl`.

It therefore does not expose the complete review trail requested by this ticket.

The setup-guide tests construct temporary Rust, Node, unknown, and initialized
projects and assert notable substrings in `build_guide` output.

There is no provenance-specific assertion.

## Existing provenance behavior

The plugin stores the append-only provenance ledger at
`.lisa/provenance.jsonl`.

Source comments describe it as one record per ticket run at teardown.

The ledger carries execution and assignment-attempt records.

The plugin stores completion transitions separately at
`.lisa/completion-journal.jsonl`.

`completion_journal.rs` calls it a durable append-only completion aggregate
journal.

Its confirmed transition includes a ticket ID and a commit ID.

That makes “ties completed tickets to commits” an accurate plain-language
description.

RDSPI work documents live under `docs/active/work/{ticket-id}/` while active and
move through Lisa's established archival/publication lifecycle.

The ticket asks for discoverability, not a schema or storage explanation.

The sentence should name the paths or concepts without promising new behavior.

## Constraints and assumptions

Copy must use plain, spoken English and lead with verbs.

The purpose paragraph must precede `DAG`, scheduling, WASM, and Zellij wording.

The paragraph must name coding agents and both Claude Code and Codex.

The README can retain detailed mechanism explanations after the new opening.

The two generated context files should carry the same purpose paragraph.

They should also carry the same one-line agent contract.

Project-specific data generation and conditional sections must remain unchanged.

The pointer relationship between `AGENTS.md` and `CLAUDE.md` must remain intact.

No runtime paths, journal formats, or initialization overwrite rules need change.

The repository contains unrelated Lisa-managed and other-ticket changes.

Ticket commits must include only exact paths owned here.

Phase artifacts stay in this attempt directory until Lisa admits them.
