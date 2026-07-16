# Structure — T-046-04-01

## Change set

The implementation modifies four tracked Markdown files and creates no product
code, test code, configuration, or generated assets.

Phase artifacts remain private under the current `.lisa/attempts` directory
until Lisa publishes them.

## `README.md`

### Opening order

Keep the existing title, release badge, and one-line description.

Move the user installation surface directly below that introduction.

The new first major section is `## Install Lisa`.

Its opening content has this order:

1. explicit no-Rust statement;
2. direct instruction to agents not to build from source for use/install work;
3. one sentence introducing the release installer;
4. the shell installer as the README's first fenced code block;
5. a brief Homebrew alternative;
6. a link to `CONTRIBUTING.md` for people changing Lisa itself.

### Removed installation branches

Remove `### From crates.io (coming soon)`.

Remove its unavailable `cargo install lisa-cli` command.

Remove its Rust/WASM target note.

Remove `### From source` from the user install section.

Remove its clone, rustup, and `just install` block from the README.

Remove the Rust and `just` prerequisite sentence attached to that user path.

The source-build information remains available in `CONTRIBUTING.md`.

### Existing sections

Place `What It Does` after installation.

Place `Prerequisites` after `What It Does`.

Keep Quick Start, Configuration, client guidance, architecture explanation, CLI
reference, and License in their current relative order.

Rename `## Contributing` to `## Develop Lisa`.

Rewrite its sentence as a direct link to the contributor guide for source
builds, tests, and changes.

Do not alter the installer URL or command flags.

Do not rewrite unrelated product and workflow documentation.

## `CLAUDE.md`

Keep `# CLAUDE.md` as the first line.

Insert a compact two-part context boundary immediately after the title.

### `Using Lisa?`

State that the released tool installs with one command from the README.

State that Rust is not required to use Lisa.

Tell agents not to build from source when asked to install or use Lisa.

Link to `README.md#install-lisa`.

### `Developing Lisa?`

State that the remaining file describes work inside this repository.

This sentence scopes the Cargo commands that follow.

Retain the existing `Project`, `Build and Test`, `Source Layout`, `Directory
Conventions`, and workflow pointer.

No Cargo command appears before the inserted use/develop boundary.

## `AGENTS.md`

Keep `# AGENTS.md` as the first line.

Insert the same `Using Lisa?` message immediately after the title.

The warning appears before the instruction to read `CLAUDE.md`.

Then retain the source-of-truth explanation and workflow pointer.

The file stays a short routing document; do not duplicate repository layout or
build commands.

## `docs/knowledge/lisa-loop-setup-guide.md`

Replace the entire 471-line manual guide with a tombstone.

Keep `# Lisa Loop Setup Guide` so historical links retain a recognizable page.

Add a clear retired marker.

State that `lisa init` now handles project setup.

Direct users to the README install and quick-start anchors.

Repeat the no-Rust and agent no-source-build warning at this old entry point.

Do not retain:

- any shell command;
- the obsolete `wasm32-wasi` target;
- manual directory creation;
- a manual CLAUDE template;
- a Zellij layout;
- hook configuration;
- ticket templates;
- future-roadmap claims.

## Boundaries and ownership

Ticket-owned source paths are exactly:

- `README.md`;
- `CLAUDE.md`;
- `AGENTS.md`;
- `docs/knowledge/lisa-loop-setup-guide.md`.

The meaningful implementation is one cohesive documentation unit because all
four entry points enforce the same reader contract.

Commit that unit with one `lisa commit-ticket` invocation and four exact
`--include` paths.

Do not include `.lisa/provenance.jsonl`, completion journals, planning files,
runtime manifests, stories, tickets, or any pre-existing worktree changes.

Do not commit attempt artifacts manually; the completion system publishes them.

## Textual interfaces

The shell installer command is a user-facing interface and remains byte-for-byte
unchanged.

The README anchor becomes `#install-lisa`, derived from its heading.

Root agent files use `README.md#install-lisa`.

The nested tombstone uses `../../README.md#install-lisa` and
`../../README.md#quick-start`.

`CONTRIBUTING.md` remains the detailed development interface.

No Rust public interfaces or CLI flags change.

## Ordering constraints

First, update README ordering because its heading anchors are targets for the
other files.

Second, update both agent context files to point at that install section.

Third, replace the stale guide with links to the finished README structure.

Fourth, run textual verification across all four files.

Fifth, inspect the exact diff and commit the four paths through Lisa.

## Verification structure

### README first-fence check

Extract or inspect the first triple-backtick block.

Verify it is a Bash block containing only the shell installer command.

Verify no earlier fenced block exists.

### Warning-order check

Find line numbers for `You do not need Rust` and Cargo build commands.

In `CLAUDE.md`, the warning must precede every Cargo occurrence.

In `AGENTS.md`, the warning must precede the CLAUDE handoff.

### Stale-guide check

Search the tombstone for `wasm32-wasi`, `cargo`, `mkdir`, `layout`, and manual
template content.

Expected result: no setup instruction using those terms.

### Diff scope check

Run `git diff --` with only the four owned paths.

Confirm changes match this structure and preserve unrelated content.

### Worktree ownership check

After the ticket commit, query status for the four owned paths only.

Expected result: none is staged, modified, or untracked.

Unrelated repository status may remain dirty and must not be cleaned.

## No new files in the public documentation tree

The tombstone retains the old filename rather than creating a redirect file.

No new install guide is necessary because README owns the current path.

No tests or snapshots are added because behavior is solely Markdown ordering
and copy.

## Resulting reader flow

A README reader sees the released install command first.

A Claude agent sees the same no-build boundary before Cargo commands.

A Codex agent sees the boundary before being routed to CLAUDE.

A reader following an old guide link is redirected to README without seeing a
stale command.

A contributor can still find source-build details through a clearly named
development link.
