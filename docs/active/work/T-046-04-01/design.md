# Design — T-046-04-01

## Goal

Make the released installer the unmistakable path for using Lisa while keeping
source-build guidance available to people who are changing Lisa itself.

The design must work for both human readers and coding agents that act on the
first plausible command they see.

## Decision 1: Put installation before explanation

### Option A: Keep the current section order and add a warning

The README could retain `What It Does` and `Prerequisites` before `Install`, then
add a no-Rust sentence beside the existing installer.

This is a small diff and preserves the current narrative.

It still makes a reader process product detail and prerequisites before seeing
the supported action. It also leaves room for agents to infer that Zellij or
Rust preparation comes first.

### Option B: Move the install section to the top

The README can place `Install Lisa` immediately after its one-line description.

The installer becomes the first code block and the earliest executable action.
The no-Rust warning sits directly beside it.

Product detail and prerequisites remain available after installation.

### Decision

Choose Option B.

The ticket is about first contact and explicitly says users must meet the single
command before build instructions. Ordering is part of correctness, not merely
presentation.

## Decision 2: Separate using Lisa from developing Lisa

### Option A: Keep source builds under `Install`

The current source-build section is factually useful and could be relabeled as
advanced.

However, its presence under user installation invites agents to choose it when
the release path encounters any uncertainty. It also makes Rust look like a
normal user prerequisite.

### Option B: Move all development commands into the README

The README could add a full `Develop Lisa` section containing the clone,
toolchain, build, and test commands.

This creates a visible split, but duplicates `CONTRIBUTING.md` and risks the two
copies drifting.

### Option C: Keep the README user-focused and link developers to CONTRIBUTING

Remove the unavailable crates.io and source-build subsections from the user
install section. Keep Homebrew as a supported alternative. Add a short sentence
that directs contributors to `CONTRIBUTING.md`.

This preserves a clean use/develop boundary without hiding development setup.
The existing contributor guide remains the single detailed source-build page.

### Decision

Choose Option C.

The shell installer is the blessed path. Homebrew remains a concise supported
alternative rather than an invitation to compile. Future crates.io copy does
not help anyone install today and should not compete for attention.

The existing `Contributing` section will be renamed or worded as `Develop Lisa`
so the two intents are explicit in the README's heading structure.

## Decision 3: Phrase the warning as a direct boundary

### Option A: A Markdown blockquote note

A note is visually distinct and easy for readers and agents to recognize.

### Option B: Put the warning in the heading

A heading such as `Install Lisa — no Rust needed` is compact, but it cannot
carry the separate instruction for agents gracefully.

### Option C: Plain body prose

Plain prose is simple, though it is easier to skim past.

### Decision

Use a short bold-led note immediately before the installer command:

`You do not need Rust to use Lisa. Agents: do not build Lisa from source when
the goal is to install or use it.`

Follow it with a verb-forward introduction to the command.

This is explicit without relying on jargon such as "binary distribution" or
"toolchain-free consumer path."

## Decision 4: Lead both agent context files with the same rule

### Option A: Put the warning only in CLAUDE.md

`AGENTS.md` already points to `CLAUDE.md`, so one copy would minimize
duplication.

That still makes a Codex agent follow a link before it encounters the crucial
boundary. It also fails the ticket's explicit requirement that both files lead
with the note.

### Option B: Repeat a compact shared paragraph

Place the same `Using Lisa?` paragraph directly after each file's title. Link to
the README install section and distinguish use from repository development.

### Decision

Choose Option B.

Small deliberate duplication is appropriate at two independent agent entry
points. The words should match closely so neither client receives weaker
guidance.

`CLAUDE.md` will retain its Cargo commands because those commands are correct
for work inside the Lisa repository. A `Developing Lisa?` transition will make
that context explicit before `Project` and `Build and Test`.

`AGENTS.md` will state the install rule first, then preserve its pointer to the
shared repository context.

## Decision 5: Tombstone or delete the stale guide

### Option A: Delete the guide

Deletion removes every stale command and makes searches clean.

Historical work artifacts still refer to the path. A reader following one of
those references would get a missing file rather than a current instruction.

### Option B: Rewrite the guide fully

A new setup guide could duplicate README installation and quick start content.

That recreates two authoritative setup surfaces and future drift risk. It also
exceeds the need now that `lisa init` owns setup.

### Option C: Replace it with a tombstone

Keep the path and title, say the manual guide is retired, and link directly to
the README's install and quick-start sections.

Include the no-Rust/agent boundary so a reader arriving from history is safely
redirected before taking action.

### Decision

Choose Option C.

The tombstone preserves navigation while eliminating all obsolete commands,
target names, layout instructions, and future-tense init guidance.

## Voice rules

Use short sentences and familiar words.

Start instructions with verbs: `Install`, `Run`, `Use`, `Read`.

Avoid distribution jargon in new copy.

Do not over-explain why the old guide was wrong; tell the reader where to go
now.

Keep `DAG`, `WASM`, and Rust terminology only in existing developer-focused
repository context where those terms describe the implementation.

## Link choices

Use relative links so the files work in local clones and on GitHub.

From root files, link to `README.md#install-lisa` and `CONTRIBUTING.md`.

From `docs/knowledge/lisa-loop-setup-guide.md`, link two levels up to:

- `../../README.md#install-lisa`;
- `../../README.md#quick-start`.

## Verification design

No application code changes, so no build or runtime tests are required.

Use focused assertions over the four files:

- parse the README text order and ensure its first fence contains the installer;
- verify the no-Rust phrase appears in README, CLAUDE, AGENTS, and tombstone;
- verify `wasm32-wasi`, manual `mkdir`, manual layout, and source clone commands
  no longer appear in the live setup guide;
- verify README sends source developers to the contributor guide;
- review headings and sentences for the requested voice;
- review the exact diff before the isolated commit.

## Rejected scope expansion

Do not edit CLI error messages or doctor checks; ticket T-046-04-02 owns them.

Do not change installer behavior, runtime acquisition, or release targets; other
T-046 tickets own those areas.

Do not rewrite generated project templates. This ticket names the repository
context files that a clone exposes, and a template sweep could overlap other
work.

Do not update the ticket phase or status. Lisa performs those transitions.

## Chosen design summary

Lead the README with the one-command release installer and an explicit no-Rust
agent warning. Keep Homebrew as a secondary install choice, remove unavailable
and source-build paths from the user section, and point development work at
`CONTRIBUTING.md`.

Repeat the same boundary at the top of `CLAUDE.md` and `AGENTS.md`.

Reduce the obsolete setup guide to a short redirect that contains no setup
commands of its own.
