# Review — T-046-04-01

## Disposition

Pass.

The documentation now gives people and coding agents the released installer
before any source-build instruction, clearly separates using Lisa from changing
Lisa, and removes the obsolete manual setup path.

Both ticket-owned source units were committed through Lisa's isolated
transaction, and all four owned paths are clean.

## What changed

### `README.md`

The README now puts `Install Lisa` directly below the project description.

Its first fenced code block is exactly the release installer one-liner:

`curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh`

Before that command, the README says users do not need Rust and tells agents not
to build Lisa from source when the task is to install or use it.

Homebrew remains a short macOS alternative.

The unavailable crates.io command and source-build recipe were removed from the
user install section.

The README now sends contributors to `CONTRIBUTING.md` instead of duplicating
development setup.

The closing `Contributing` heading became `Develop Lisa`, making the user and
developer paths visible in the heading structure.

During review, the CLI reference was also corrected. Its `lisa doctor` copy no
longer presents the optional Rust WASM target report as a user runtime
dependency. It describes the selected agent and the Zellij runtime Lisa uses.

### `CLAUDE.md`

The repository context now opens with `Using Lisa?`.

That section links to the one-command README install, says Rust is not required
for use, and gives agents the no-source-build instruction.

A `Developing Lisa?` transition scopes the existing repository build and test
commands to work on Lisa itself.

The useful Cargo, layout, and workflow information remains intact for
contributors.

### `AGENTS.md`

The Codex entry file now repeats the same `Using Lisa?` boundary immediately
after its title.

That message appears before its instruction to read `CLAUDE.md`, so a Codex
agent encounters the supported install path before any repository build
command.

The file remains a concise pointer to the shared context and injected workflow.

### `docs/knowledge/lisa-loop-setup-guide.md`

The 471-line stale manual guide is now a nine-line tombstone.

It says `lisa init` handles project setup, links to README Install Lisa and
Quick Start, and repeats the no-Rust agent boundary.

The tombstone preserves the historical path without retaining a second setup
guide.

It contains no obsolete `wasm32-wasi` target, source build, manual directory
tree, context template, hook setup, layout, or future-tense init plan.

## Acceptance criteria

### Stale guide gone or tombstoned

Met.

The original body was removed and replaced with a short redirect to current
README sections.

### README first code block is the installer

Met.

A committed-state assertion extracted the first fenced block and compared its
body exactly with the release installer command.

### Explicit no-Rust and agent warning

Met.

The warning appears in README, CLAUDE, AGENTS, and the old-guide tombstone.

In README it precedes the first fence. In CLAUDE it precedes all Cargo commands.
In AGENTS it precedes the CLAUDE handoff.

### Clean use/develop split

Met.

README owns released installation and ordinary use. `CONTRIBUTING.md` owns the
source build. CLAUDE explicitly scopes its repository commands to development.

### Brand voice

Met.

New instructions use short, direct sentences and ordinary verbs: install,
use, read, follow, and check.

The tombstone directs the reader to the current action rather than explaining
the old architecture.

## Verification

The final shell assertions checked:

- exact contents of README's first fenced block;
- warning-before-command order in README;
- warning-before-Cargo order in CLAUDE;
- warning-before-handoff order in AGENTS;
- absence of stale setup terms from the tombstone;
- absence of the conflicting `wasm32-wasip1 target` phrase from README;
- existence of linked root documents;
- whitespace with `git diff --check`;
- clean status for all four ticket-owned source paths.

All assertions passed.

No Rust build or application test was run. The ticket changes Markdown copy and
ordering only; compilation would not exercise the acceptance criteria. Focused
textual assertions directly cover the changed contract.

## Commits

The main four-file unit was committed as:

`0dc8aaa0b6a60182a83a4e23c819ad7e858de496 docs: make released install the blessed path`

The review-found README clarification was committed as:

`16e3bef67418ee50d6b62416eeb69bb17e5af9fc docs: keep doctor copy on the no-Rust path`

Both used `lisa commit-ticket` with exact repository-relative include paths.

No ordinary index command or ordinary commit was used.

## Scope and worktree safety

The combined committed diff changes four files, with 48 insertions and 506
deletions. Most deletions are the retired guide.

Unrelated modified and untracked files remain in the shared worktree. They were
not cleaned, staged, edited, or included in either ticket commit.

The four ticket-owned source paths report no staged, modified, or untracked
state after both commits.

## Open concerns

No blocking concern remains.

The doctor implementation still performs an optional WASM-target report when a
Rust toolchain happens to exist. This ticket removes that developer detail from
the user install story; it does not change doctor behavior. Ticket T-046-04-02
owns doctor and error-string behavior.

Homebrew remains documented as a supported macOS alternative. The release shell
installer is still unmistakably the blessed path because it comes first, is
introduced as the one-command release install, and is the README's first code
block.

No automated Markdown link checker was available or necessary for these simple
relative targets. The linked files exist and the heading-derived anchors match
the current headings.

## Handoff

The work is ready for Lisa's completion publication and final ticket commit.

Do not manually change the ticket phase or status. Remain on T-046-04-01 until
Lisa confirms completion.
