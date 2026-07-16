# Plan — T-046-04-01

## Implementation strategy

Treat the documentation sweep as one coherent source unit with four entry
points and one shared message.

Make small file-local edits, verify the reader order mechanically, then commit
only the four owned files through Lisa's isolated transaction.

## Step 1: Lead README with the released installer

Move the existing install section directly below the project description.

Rename it `Install Lisa` so its generated anchor is explicit.

Add the no-Rust sentence and agent instruction before the first code block.

Keep the shell installer command unchanged.

Keep Homebrew as a short alternative below the blessed command.

Remove the unavailable crates.io path and source-build subsection from the user
installation surface.

Add a direct development link to `CONTRIBUTING.md`.

Preserve all product explanation and quick-start material after installation.

Verification:

- the first README fence is Bash;
- its content is the release installer one-liner;
- `You do not need Rust to use Lisa` appears before it;
- no Cargo or rustup command remains in the README install section;
- a development link remains discoverable.

## Step 2: Clarify the README development boundary

Rename the closing `Contributing` heading to `Develop Lisa`.

Use direct prose that sends source builders to `CONTRIBUTING.md`.

Do not copy contributor commands into README.

Verification:

- the heading clearly names development intent;
- the contributor guide link resolves as a root-relative repository path;
- the README contains no competing source-build recipe.

## Step 3: Add the boundary to CLAUDE.md

Insert `Using Lisa?` immediately after the document title.

Link the one-command path in README.

State that use does not require Rust.

Tell agents not to build from source for install/use requests.

Insert a short `Developing Lisa?` transition before repository context.

Keep all existing repository build/test and layout guidance unchanged.

Verification:

- the warning precedes the first Cargo command;
- the install link uses `README.md#install-lisa`;
- Cargo commands remain available to repository developers.

## Step 4: Add the boundary to AGENTS.md

Insert the same compact use warning immediately after the title.

Put it before the CLAUDE source-of-truth handoff.

Keep the source-of-truth and workflow routing paragraphs.

Verification:

- the no-Rust wording appears before `Read CLAUDE.md first`;
- the file remains concise;
- the README link matches CLAUDE's target.

## Step 5: Tombstone the old setup guide

Replace the old guide body rather than deleting the file.

Keep the title for historical navigation.

Mark it retired and name `lisa init` as the current setup action.

Link to README Install Lisa and Quick Start.

Repeat the no-Rust/agent boundary.

Verification:

- no `wasm32-wasi` string remains;
- no Cargo, git clone, mkdir, layout, hook, or manual context template command
  remains;
- the nested relative links point two directories up to README;
- the tombstone is short enough that it cannot be mistaken for a second guide.

## Step 6: Review voice

Read all newly written copy as one path.

Check for plain words and direct verbs.

Remove abstract phrases that do not help the reader act.

Keep the agent instruction direct but scoped to install/use work so it does not
conflict with repository development.

Verification:

- sentences are short;
- instructions use `Install`, `Use`, `Run`, or `Read`;
- no new unexplained jargon appears;
- user and developer intents cannot be confused.

## Step 7: Run focused documentation checks

Use a small read-only script or shell inspection to find the README's first
fence and compare its body with the expected installer.

Use `rg -n` over the four files to inspect:

- no-Rust warning locations;
- `cargo`, `rustup`, and target-name locations;
- README and contributor links;
- stale manual setup terms.

Inspect the exact four-file diff.

No Cargo test is planned because no executable source or generated template
changes.

If a repository-wide documentation checker exists and runs cheaply, it may be
run, but its absence does not block the focused acceptance checks.

## Step 8: Commit the source unit

Run:

`lisa commit-ticket --ticket-id T-046-04-01 --message "docs: make released install the blessed path" --include README.md --include CLAUDE.md --include AGENTS.md --include docs/knowledge/lisa-loop-setup-guide.md`

Do not use `git add`, `git commit`, or the ordinary index.

If `lisa` on PATH lacks the required command, inspect repository-local CLI
options without staging work. Use the available Lisa implementation that
supports the assignment contract.

Verification:

- the command reports a successful ticket commit;
- all four exact paths are clean afterward;
- unrelated worktree changes remain untouched.

## Step 9: Record implementation progress

Create `progress.md` in the attempt work directory.

Record each completed edit and check.

Document any deviation from this plan before taking the alternative action.

Include the ticket commit identity or command result.

## Step 10: Review and disposition

Review the committed diff and acceptance criteria.

Create `review.md` with:

- summary by file;
- textual verification results;
- test coverage rationale;
- open concerns or limitations;
- worktree ownership result.

Create `review-disposition.json` with exactly the allowed pass shape if all
checks and the isolated commit succeed.

Use a block disposition only if there is a concrete unresolved issue, with an
actionable reason.

After both Review artifacts exist, remain on T-046-04-01 and stop.

## Atomicity

All four documentation changes enforce one reader contract and should land in a
single meaningful unit.

Splitting them would temporarily leave one agent entry point teaching a
different path, which is not independently complete.

The phase artifacts are not part of the source unit; Lisa handles their final
publication and completion commit.

## Expected final state

README's first code block installs the released CLI.

README explicitly says Rust is unnecessary for users and tells agents not to
compile for install/use tasks.

CLAUDE and AGENTS show that rule before repository build guidance.

The stale guide contains only a safe redirect.

Detailed source-build instructions live in `CONTRIBUTING.md`, clearly separated
from the use path.

The four source paths are committed and clean, while unrelated worktree state
is unchanged.
