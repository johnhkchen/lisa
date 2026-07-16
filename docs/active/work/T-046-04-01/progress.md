# Progress — T-046-04-01

## Status

Implementation, focused verification, and the isolated Lisa commit are
complete.

## Completed work

### 1. README now leads with installation

- Moved the released install path directly below the project description.
- Renamed the section `Install Lisa`.
- Added the explicit statement that users do not need Rust.
- Added a direct instruction that agents must not build from source for an
  install or use request.
- Kept the shell installer command unchanged.
- Kept Homebrew as a short macOS alternative.
- Removed the unavailable crates.io path from user installation guidance.
- Removed the source-build recipe from user installation guidance.
- Linked people changing Lisa to the existing contributor guide.

### 2. README now names the development boundary

- Renamed the final `Contributing` heading to `Develop Lisa`.
- Kept detailed build and test setup in `CONTRIBUTING.md`.
- Added plain copy explaining that source builds are for changing Lisa itself.

### 3. CLAUDE now starts with the use/develop split

- Added `Using Lisa?` immediately after the title.
- Linked the one-command README installation.
- Put the no-Rust and agent warning before every Cargo command.
- Added `Developing Lisa?` to scope the existing repository notes.
- Preserved the repository's build, test, layout, and workflow context.

### 4. AGENTS now starts with the install boundary

- Added `Using Lisa?` immediately after the title.
- Repeated the same install link and warning used by CLAUDE.
- Put that guidance before the handoff to `CLAUDE.md`.
- Preserved the source-of-truth and RDSPI workflow routing.

### 5. The stale guide is a tombstone

- Replaced the 471-line manual guide with a nine-line redirect.
- Kept the historical filename and recognizable title.
- Named `lisa init` as the current setup action.
- Linked README's install and quick-start sections.
- Repeated the no-Rust and agent warning.
- Removed obsolete WASM target, manual directories, manual context template,
  layout, hook, and future-init guidance.

## Verification completed

### First README code block

An `awk` inspection found the first opening fence at README line 14.

The fence language is `bash`.

Its only body line is the expected release installer command.

The explicit no-Rust warning appears at README line 9, before the command.

### Warning order

Focused `rg -n` output showed:

- the warning at README line 9;
- the warning at CLAUDE lines 5–7;
- the first CLAUDE Cargo command at line 21;
- the warning at AGENTS lines 5–7;
- the warning at setup-guide lines 8–9.

The agent-facing boundary therefore precedes repository build instructions and
the CLAUDE handoff.

### Stale guide

A focused search for `wasm32-wasi`, `cargo`, `rustup`, `mkdir`, `git clone`,
`layout`, and `hooks` returned no match in the tombstone.

### Formatting and diff

`git diff --check -- README.md CLAUDE.md AGENTS.md
docs/knowledge/lisa-loop-setup-guide.md` passed with no output.

The scoped diff reports:

- four files changed;
- 45 insertions;
- 502 deletions.

Most deletions are the retired manual guide.

The exact diff was inspected and contains only the intended reader-path edits.

## Test strategy result

No Rust build or test was run.

This ticket changes Markdown copy and ordering only. Application compilation
does not exercise the acceptance criteria, while the focused structural and
search checks exercise them directly.

## Plan deviations

The first post-commit review found one later README sentence that said
`lisa doctor` reports the `wasm32-wasip1` target. Although doctor treats that
developer check as optional, the copy contradicted the new user-facing promise
that Rust is not required.

The CLI reference was tightened to describe the selected agent and Zellij
runtime checks without presenting the Rust target as a user dependency. This is
inside the planned README boundary and will be committed as a second exact-path
documentation unit.

The correction did not expand into CLI behavior or another ticket's source.

## Isolated commit

Ran `lisa commit-ticket` with ticket ID `T-046-04-01`, message
`docs: make released install the blessed path`, and the four exact owned paths.

The command exited successfully and created commit:

`0dc8aaa0b6a60182a83a4e23c819ad7e858de496`

No ordinary `git add`, `git commit`, or broad include was used.

The review-found README clarification was committed with a second
`lisa commit-ticket` call containing only `README.md`.

That command exited successfully and created commit:

`16e3bef67418ee50d6b62416eeb69bb17e5af9fc`

## Remaining work

- None in Implement. Final committed-state assertions passed and all four
  ticket-owned paths are clean.
- Review artifacts are the next and final phase deliverables.

## Ownership guard

The repository contains unrelated modified and untracked files that predate this
implementation.

They were not edited, staged, cleaned, or included in the scoped diff.

The ticket commit will name only:

- `README.md`;
- `CLAUDE.md`;
- `AGENTS.md`;
- `docs/knowledge/lisa-loop-setup-guide.md`.
