# Design: purpose-first docs and templates

## Goals

Make the first README prose answer what Lisa is for.

Give newly initialized Claude Code and Codex agents the same orientation.

State the small operating contract an agent follows under Lisa.

Expose Lisa's existing review trail in the README and setup guide.

Keep the change at the wording and assertion layer.

## Purpose paragraph options

### Option A: reuse the sibling CLI sentence exactly

The landed CLI sentence is:

> Lisa runs coding agents through your ticket board, so you don't have to
> approve every step by hand.

This is short, verb-forward, and purpose-first.

It already anchors CLI help, the setup-guide header, and the hooks guide.

Using it would keep the product surfaces tightly consistent.

Its limitation is that it does not name Claude Code or Codex.

This ticket's README and template acceptance criteria specifically call for the
supported agent names.

Adding the names after a second sentence would technically satisfy naming but
would make the “same purpose paragraph” less direct.

### Option B: extend the landed sentence with the client names

The paragraph can say:

> Lisa runs coding agents like Claude Code and Codex through your ticket board,
> so you don't have to approve every step by hand.

This keeps the structure and voice of the landed sentence.

It names both clients before introducing any mechanism vocabulary.

“Like” is accurate because the sentence gives examples rather than defining a
closed protocol promise.

The README later clarifies that Claude Code is the default and Codex is an
alternative.

The sentence is easy to repeat exactly in both generated context files.

The setup-guide header can retain the shorter sibling-owned wording because this
ticket only needs the same paragraph in the two generated templates.

### Option C: mention agents “against” a board

The story uses the phrase “against a ticket board.”

That phrase is precise in planning language but slightly less natural aloud.

“Through your ticket board” is already established on the landed surfaces and
sounds more conversational.

### Decision

Use Option B for README, generated `CLAUDE.md`, and generated `AGENTS.md`.

It satisfies the client-name requirement while preserving the established voice.

Do not change the sibling-owned CLI/setup-guide preamble merely to force exact
byte-for-byte uniformity across every surface.

## Agent contract options

### Option A: enumerate all six phases and commit mechanics

A detailed line could name Research, Design, Structure, Plan, Implement, Review,
`lisa commit-ticket`, exact include paths, and final waiting behavior.

That would be operationally complete.

It would also duplicate the injected RDSPI workflow and turn the opening into a
dense procedure rather than orientation.

Procedural details evolve more often than the stable high-level contract.

### Option B: one sentence with ticket ownership and lifecycle

The line can say:

> Under Lisa, you take one ticket through every RDSPI phase, leave a reviewable
> record, and wait for Lisa to confirm completion.

This explains the division of responsibility.

The agent owns one ticket and its phase work.

Lisa owns completion confirmation.

“Every RDSPI phase” points to the workflow definition directly below or later in
the file without duplicating it.

“Reviewable record” connects purpose to artifacts without adding storage detail.

### Option C: imperative phrasing

The line could command the agent to work a ticket, write artifacts, commit, and
stop.

Imperative instructions are clear, but the generated file is project context,
not the assignment prompt.

The actual assignment supplies exact attempt paths and transaction commands.

A stable declarative contract avoids contradicting future prompt specifics.

### Decision

Use Option B exactly in both templates.

It is one line in source copy, plain English, and correctly separates agent work
from Lisa's completion gate.

## Sharing template copy

### Option A: duplicate literals

Place the same paragraph and contract directly in `AGENTS_MD` and the
`generate_claude_md` format string.

This is the smallest textual edit.

It also lets the two copies drift later.

Tests could catch drift only if they compare exact full strings.

### Option B: define stable copy constants

Add module constants for the purpose paragraph and agent contract.

Build `AGENTS_MD` with `concat!` only if the macro can consume those constants.

Rust's built-in `concat!` accepts literals rather than arbitrary const values,
so a static `&str` cannot interpolate neighboring constants at compile time.

Changing `AGENTS_MD` into a function would ripple into initialization and tests
for little gain.

### Option C: keep `AGENTS_MD` static and assert shared required sentences

Retain the public constant shape.

Duplicate the two short stable sentences in its literal and in the generated
format string.

Define test-only expected strings or direct assertions that both outputs contain
the same purpose paragraph and contract.

This preserves the existing interface while making drift visible in tests.

### Decision

Use Option C.

The public template interface remains unchanged and acceptance is protected by
explicit tests.

The duplicated content is two stable product sentences, not project-specific
context.

## README placement

### Opening purpose

Replace the mechanism-first lede immediately after the release badge.

Do not add a second mechanism sentence before `Install Lisa`.

The existing `What It Does` section provides the detail after installation.

This guarantees the first prose paragraph contains the purpose and client names
before `DAG`, scheduling, WASM, or Zellij appears in prose.

The release badge URL contains no relevant mechanism language.

### Provenance sentence

Place the provenance sentence after the paragraph that describes six phase
artifacts in `What It Does`.

Readers encounter it while already thinking about review and recovery.

The proposed sentence is:

> Lisa keeps the trail reviewable: an append-only attempt ledger records each
> run, the completion journal ties finished tickets to commits, and each ticket
> keeps its work documents.

This names all requested concepts in ordinary language.

It avoids suggesting that provenance is a new feature.

It does not overload the opening purpose paragraph.

## Setup-guide placement

The guide's uninitialized-project table already lists work artifacts.

However, initialized projects follow a shorter branch that omits the table.

Putting provenance only in `section_init` would make it disappear depending on
repository state.

Place a standalone sentence in the guide header after the purpose paragraph.

The sentence will be discoverable for every project state.

It will appear before numbered setup steps and will not alter step ordering.

Use the same provenance sentence as the README for consistent wording.

The header still begins with the already-landed purpose paragraph.

## Test design

Add a focused template test that constructs a project and obtains both template
outputs.

Assert both contain the exact purpose paragraph.

Assert both contain the exact contract sentence.

Assert each required sentence appears after its H1.

The existing generated-template tests retain coverage of project metadata and
conditional sections.

Extend setup-guide tests with exact concepts from the provenance sentence.

Checking `append-only attempt ledger`, `completion journal`, `commits`, and
`work documents` establishes all three parts without snapshot brittleness.

README is static Markdown and has no existing dedicated test harness.

Verify its opening order with a small script or text inspection during Review.

Run formatting and the `lisa-cli` unit tests after implementation.

Run the broader workspace suite if time and repository state allow.

## Rejected scope

Do not change provenance schemas or journal code.

Do not add a provenance command.

Do not rewrite the detailed `What It Does` explanation.

Do not change `lisa init` overwrite behavior.

Do not duplicate build commands or source layout in `AGENTS.md`.

Do not alter help command grouping, ordering, or hidden commands.

Do not update the active ticket's phase or status fields.
