# Research — T-046-04-01

## Ticket boundary

This ticket changes the documentation path a person or coding agent encounters
when learning how to install Lisa.

The acceptance criteria name four documentation surfaces:

- `README.md`;
- `CLAUDE.md`;
- `AGENTS.md`;
- `docs/knowledge/lisa-loop-setup-guide.md`.

The ticket does not ask for CLI behavior, installer behavior, release changes,
or generated template changes.

The ticket frontmatter is in `phase: research` and must remain under Lisa's
control.

Phase artifacts belong in the current attempt's private work directory, not in
`docs/active/work/T-046-04-01/`.

## Repository guidance

`AGENTS.md` points all agent clients to `CLAUDE.md` as the source of truth.

`CLAUDE.md` describes the repository, its build commands, its source tree, and
the RDSPI workflow location.

The project workflow requires all six phases and exact-path commits through
`lisa commit-ticket`.

The ordinary worktree already contains unrelated modified and untracked files.
Those files are outside this ticket and must be preserved.

## README entry path

`README.md` opens with the project name, release badge, and a one-line product
description.

Its first substantive section is `What It Does`.

That section explains ticket dependency scheduling, Zellij, agent sessions, the
dashboard, and all six RDSPI phases.

The next section is `Prerequisites`.

It lists Claude Code and Zellij before the install command appears.

It also explains the optional Codex client and `lisa doctor`.

The `Install` section follows.

The first fenced code block in the README is currently the release installer:

`curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh`

That satisfies the narrow ordering of the first code block at the current
revision, but the command is not the first action-oriented content.

The README does not say that using Lisa requires no Rust toolchain.

The README does not tell coding agents not to build from source when their goal
is to use Lisa.

The install section presents four paths together:

- shell installer, marked recommended;
- Homebrew;
- crates.io, marked coming soon;
- source build.

The crates.io subsection contains a non-working future command.

Its note discusses the Rust WASM target and source compilation.

The source subsection contains `git clone`, `rustup target add`, and
`just install`.

That subsection says Rust and `just` are required.

Those details are accurate for repository development, but they share the same
decision surface as installing Lisa for use.

The README already has a `Contributing` section near the end.

That section links to `CONTRIBUTING.md` for build instructions, tests, and
submissions.

`CONTRIBUTING.md` contains the source-build prerequisites and commands that a
developer needs.

The README therefore already has a natural boundary for development guidance.

## README use path

`Quick Start` tells a user to change into a project and run `lisa init`.

It explains what init creates.

It then shows a ticket and tells the user to run `lisa loop`.

Later sections document configuration, client selection, workflow behavior,
project layout, and individual CLI commands.

The CLI reference says `lisa init` scaffolds the Lisa project files.

That makes the manual directory and context-file setup in the old guide
unnecessary for ordinary users.

## CLAUDE.md entry path

`CLAUDE.md` begins with the file title and then `Project`.

The opening paragraph describes Lisa as a Rust Zellij WASM plugin.

The next subsection is `Build and Test`.

Its first fenced block contains two Cargo build commands, a Cargo test command,
and `just check`.

There is no install command before those build commands.

There is no distinction between using Lisa and developing this repository.

There is no warning aimed at agents whose task is merely to install Lisa.

Because Claude Code reads this file directly, its earliest executable guidance
is source compilation.

The rest of the file is repository-specific and useful to contributors: source
layout, directory conventions, and the workflow pointer.

## AGENTS.md entry path

`AGENTS.md` is intentionally short.

It tells Codex to read `CLAUDE.md` first.

It then points at the injected RDSPI workflow.

It contains no Cargo command itself.

However, its first instruction sends an agent to a file whose earliest command
block is the source build block.

It does not independently state the install boundary before that handoff.

Because Codex reads `AGENTS.md` as repository context, the ticket explicitly
requires the warning in both files rather than relying only on `CLAUDE.md`.

## Stale setup guide

`docs/knowledge/lisa-loop-setup-guide.md` is 471 lines long.

It presents itself as the setup guide for Lisa Loop.

Its prerequisites tell users to install the Lisa plugin by downloading a WASM
file or building it.

Its source-build block uses the obsolete target name `wasm32-wasi`.

It tells users to copy the WASM file and reference it from a Zellij layout.

It instructs users to create `docs/active` directories manually.

It provides a complete manual `CLAUDE.md` template.

It teaches manual tickets, stories, hooks, layouts, and launch steps that now
overlap with CLI-managed setup.

The README documents `lisa init` as the current scaffold command.

The guide's model predates that current path and can redirect an agent away from
the supported setup flow.

Current live documentation does not link to the guide by name.

Historical tickets and archived artifacts mention it, so deleting it would
leave those historical references pointing at a missing file.

The current epic and ticket identify it as intentionally stale.

## Voice and consistency constraints

The ticket calls for plain kitchen-table English, verbs first, and no jargon.

The installer command itself must remain exact and copyable.

The warning must make two audiences clear:

- people using Lisa do not need Rust;
- coding agents should not choose a source build for an install task.

The repository still needs build commands for contributors, so the warning
cannot imply that Rust is never used in Lisa development.

The clean boundary is contextual: install the released tool to use Lisa; open
the contributor guide when changing Lisa itself.

## Verification surface

This ticket changes prose only, so Rust compilation does not exercise the
changed behavior.

Useful checks are textual and structural:

- inspect the first fenced block in `README.md`;
- search the live target files for obsolete `wasm32-wasi` guidance;
- search the README install area for source-build instructions;
- confirm both agent context files lead with the no-Rust warning;
- confirm the old guide points to the current README path;
- inspect the diff for plain, direct wording;
- confirm only exact ticket-owned paths enter the ticket commit.

## Research conclusion

The repository already contains a working released installer and a current
`lisa init` path.

The failure is information ordering and competing documentation, not missing
installation machinery.

The README mixes use and development guidance, both agent entry files omit the
boundary warning, and the old setup guide preserves a superseded manual flow.

Those four files form the complete implementation surface named by the ticket.
