# Research: init-history-offer

## Ticket frame

- Ticket `T-049-02-01` starts in the Research phase.
- Its implementation scope is the `lisa init` history offer.
- The user-facing goal is to make commit-sealed completion available in a new folder.
- The offer describes project history as undo for finished work and a record of agent work.
- The offer path must not use the word `git`.
- Acceptance creates a repository, a project-local Lisa identity, and an initial commit.
- Decline leaves the folder without a repository and explains the journal-only consequence.
- `--with-history` and `--no-history` are the non-interactive contract.
- Existing repositories and identities form the main safety boundary.
- An existing unborn repository may gain its first commit only after explicit acceptance.

## Repository instructions

- `AGENTS.md` delegates all project guidance to `CLAUDE.md`.
- `CLAUDE.md` identifies Lisa as a Rust workspace with a CLI, core crate, and WASM plugin.
- The assignment requires all six RDSPI phases in one continuous pass.
- Phase artifacts belong under the current attempt-private work directory.
- Ticket phase and status frontmatter must not be edited by the agent.
- Ticket-owned source changes must be committed with `lisa commit-ticket`.
- Exact repository-relative include paths are mandatory.
- Ordinary `git add` and `git commit` are prohibited for ticket work.
- Review requires both `review.md` and `review-disposition.json`.

## CLI command surface

- `crates/lisa-cli/src/main.rs` defines the Clap command tree.
- `Commands::Init` currently contains `dry_run: bool` and `path: PathBuf`.
- The command dispatch resolves the path and calls `init::run_init(&path, dry_run)`.
- Mutually exclusive history flags do not exist yet.
- The init help snapshot is pinned in `crates/lisa-cli/tests/help_surface.rs`.
- Any new flags will intentionally change that snapshot.
- Operator help is also checked for a fixed list of banned jargon.
- `git` is not in that global banned-jargon list.
- The ticket has a narrower copy rule specifically for the offer path.

## Init planning and execution

- `crates/lisa-cli/src/init.rs` contains both initialization and validation behavior.
- `plan_init_actions` creates a complete filesystem plan before mutations occur.
- `InitAction` distinguishes directory creation, file creation, owned updates, no-ops, and safety skips.
- Existing unknown content is preserved by the owned-template helpers.
- `.lisa/.gitignore` is maintained with append-only behavior.
- `run_init` locks stdout and delegates to `run_init_with_writer`.
- `run_init_with_writer` accepts an injectable output writer for tests.
- It does not accept injectable input or an explicit history decision.
- It validates that the requested root exists.
- It detects and prints the project type.
- It plans and prints all scaffold actions.
- Dry-run returns after printing the plan and before filesystem mutation.
- Non-dry execution applies the action list and tracks changed files.
- Hook executable bits are changed only for hook files written during that run.
- Completion output reports initialization, changed files, and four next steps.
- Existing unit tests exercise dry-run, file creation, preservation, upgrades, and output fidelity.
- Tests call both `run_init` and the private writer-injected function.

## Current input behavior

- `lisa init` is currently non-interactive.
- There is no prompt dependency in `lisa-cli`.
- The crate already depends on standard-library I/O and does not require a prompt crate.
- `std::io::IsTerminal` can distinguish an interactive stdin/stdout session on supported Rust versions.
- Existing test calls must not block waiting for input.
- Black-box integration tests launch the built binary with piped output.
- Those processes therefore have non-terminal standard streams by default.

## Completion-seal resolution

- `crates/lisa-cli/src/completion_seal.rs` owns native environment probing.
- Configuration expresses `auto`, `commit`, or `journal` intent.
- The default generated `.lisa.toml` comments out `completion = "auto"`.
- Config resolution therefore uses the automatic mode for a fresh project.
- `resolve_for_run` probes the environment once and pins the result.
- `resolve_for_inspection` probes only for automatic mode.
- A missing repository makes automatic resolution choose journal.
- A repository without an effective `user.email` also makes commit support unavailable.
- A repository without a resolvable `HEAD` is transaction-unavailable.
- A repository with identity, `HEAD`, and usable metadata resolves to commit.
- The stable journal visibility line says finished work is recorded but not undoable.
- That line contains no occurrence of `git`.

## Commit transaction dependency on HEAD

- `crates/lisa-cli/src/commit_transaction.rs` implements isolated ticket commits.
- It resolves `HEAD` before it initializes an alternate index.
- It reads the current tree into the alternate index.
- It creates the commit with `commit-tree` and an unconditional `-p <old_head>`.
- It advances `HEAD` with a compare-and-swap `update-ref`.
- An unborn branch cannot satisfy the initial `rev-parse HEAD` operation.
- An empty initial commit is sufficient to provide the required parent.
- The transaction itself does not need changes for this ticket.
- Transaction tests already cover ordinary success, staged-entry isolation, and identity failure.

## Repository discovery facts

- `git -C <root> rev-parse --show-toplevel` distinguishes a folder inside a repository.
- It succeeds both at a repository root and within a descendant folder.
- A folder inside a parent repository must not receive a nested `.git` directory.
- `git init` creates repository metadata without adding worktree content to a commit.
- `git config --local` writes only the new repository's local config.
- `git config --get user.name` and `user.email` may resolve local or inherited global values.
- `git config --local --get ...` inspects only local values.
- The test process environment can isolate global configuration with a temporary `HOME`.
- `GIT_CONFIG_NOSYSTEM=1` prevents system configuration from affecting fixtures.
- `GIT_CONFIG_GLOBAL=<fixture path>` provides a deterministic global config source.

## Identity safety boundary

- The required bootstrap identity is `Lisa (project history) <lisa@project>`.
- In a newly created repository, writing those two values locally does not mutate global config.
- In an already existing repository, local or global identity values must remain byte-identical.
- An initial commit can use command-scoped identity variables without writing configuration.
- Git recognizes `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, and `GIT_COMMITTER_EMAIL`.
- Command-scoped values allow an explicit first commit in an identity-less unborn existing repository.
- The resulting commit identifies Lisa without changing repository config.

## Initial commit contents

- The ticket requires an initial commit but does not require user files in that commit.
- `git commit --allow-empty` creates a root commit with no staged user content.
- This avoids claiming authorship of files that predated `lisa init`.
- It also avoids consuming or changing an existing ordinary index in an unborn repository.
- A root commit makes `HEAD` resolvable for later completion transactions.
- Scaffold files remain ordinary working-tree changes after the empty root commit.
- A subsequent exact-path completion-style commit can then parent the root commit.

## Existing tests and fixture conventions

- `crates/lisa-cli/tests/seal_visibility.rs` is a black-box CLI fixture.
- It uses `CARGO_BIN_EXE_lisa`, temporary project roots, and controlled `HOME`/`PATH` values.
- `crates/lisa-cli/tests/help_surface.rs` pins exact help strings.
- Unit tests in `init.rs` use `tempfile` and direct `Command` calls.
- `tempfile` is already a regular dependency of `lisa-cli`.
- No new third-party dependency is necessary for history fixtures.
- A black-box test can invoke `init`, `status`, and `commit-ticket` against one fixture.
- `status` exposes the resolved completion-seal line without making private modules public.
- `commit-ticket` exercises the real completion-style transaction through the CLI.

## Observable acceptance states

- Accepted bare folder: `<root>/.git` exists.
- Accepted bare folder: local `user.name` equals `Lisa (project history)`.
- Accepted bare folder: local `user.email` equals `lisa@project`.
- Accepted bare folder: `git rev-parse --verify HEAD` succeeds.
- Accepted bare folder: a subsequent `commit-ticket` command succeeds.
- Accepted bare folder: `lisa status` reports commit-sealed.
- Declined bare folder: `<root>/.git` does not exist.
- Declined bare folder: output contains the exact journal/undo consequence.
- Declined bare folder: `lisa status` reports journal-only.
- Existing parent repository: no nested repository is created.
- Existing parent repository: repository metadata and config remain unchanged.
- Existing born repository: history flags do not rewrite identity or history.
- Existing unborn repository: absence of explicit acceptance leaves `HEAD` unresolved.
- Existing unborn repository: explicit acceptance creates only the first commit.
- Offer copy: the prompt and surrounding decision copy contains no `git` token.

## Constraints and exclusions

- Journal-seal mechanics belong to story S-049-03, not this ticket.
- Doctor identity messaging belongs to sibling ticket T-049-02-02.
- No remote, push, or global identity behavior is in scope.
- No completion-transaction redesign is required.
- No change to ticket phase/status is permitted.
- Existing unrelated working-tree changes belong to other active Lisa work.
- The source implementation must avoid including those paths in its ticket commit.
