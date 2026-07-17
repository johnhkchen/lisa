# Research: init-history-default

## Ticket frame

- Ticket `T-050-01-01` begins in Research.
- It supersedes the no-flag contract introduced by `T-049-02-01`.
- The product principle is that an unconfigured default makes a decision.
- A fresh-folder default should select the strongest history mode available.
- When project history is usable, init should create and announce it.
- When project history is not usable, init should choose the journal path.
- Explicit `--with-history` remains a hard request rather than a preference.
- Explicit `--no-history` remains a deterministic journal override.
- Interactive users retain the existing plain-language offer.
- The offer must continue to avoid the word `git`.
- Existing repositories, identities, configs, indexes, and history remain safety boundaries.
- Documentation should return to showing bare `lisa init` as the normal path.

## Repository instructions and workflow

- `AGENTS.md` points all agents to `CLAUDE.md` as the project source of truth.
- `CLAUDE.md` identifies a Rust workspace containing core, plugin, and CLI crates.
- The assignment requires all remaining RDSPI phases in one uninterrupted run.
- Phase artifacts belong in the attempt-private `.lisa/attempts/.../work` directory.
- The agent must not change ticket phase or status frontmatter.
- Source and public documentation changes must use `lisa commit-ticket`.
- Each commit command must use exact repository-relative include paths.
- The ordinary Git index must not be used for ticket work.
- Review requires `review.md`, `review-disposition.json`, and disposition validation.

## CLI surface

- `crates/lisa-cli/src/main.rs` declares `Commands::Init` with Clap.
- Init accepts `--dry-run`, `--path`, `--with-history`, and `--no-history`.
- Clap marks the two history flags as mutually exclusive.
- Dispatch converts the two booleans into `HistoryPreference`.
- No flags maps to `HistoryPreference::Ask` today.
- The enum name reflects its original interactive/no-flag semantics.
- Changing the enum or CLI flags is not required to change default resolution.
- `crates/lisa-cli/tests/help_surface.rs` pins the public flag descriptions.
- The flags already read as available choices, not required options.

## Init ownership

- `crates/lisa-cli/src/init.rs` owns planning, history setup, scaffolding, and validation.
- `run_init` detects whether standard input is a terminal.
- It passes terminal state, locked input, and locked output to `run_init_with_io`.
- `run_init_with_io` resolves history before detecting the project type.
- This ordering prevents scaffold writes before history resolution succeeds.
- The entire scaffold action list is planned before those actions are executed.
- History setup happens before scaffold files are written.
- The root commit therefore always has an explicitly empty tree.
- File mutations are tracked for the final changed-files report.
- Existing managed-file ownership and safety-skip rules are independent of history choice.

## Existing history vocabulary

- `HistoryPreference` has `Ask`, `WithHistory`, and `NoHistory` variants.
- `RepositoryState` has `Missing`, `Unborn { root }`, and `Born` variants.
- `HistoryAction` has `None`, `CreateRepository`, `CreateInitialCommit`, and `Decline`.
- `Born` resolves immediately to `None` for every preference.
- `NoHistory` resolves to `Decline` when a choice is relevant.
- `WithHistory` resolves to repository creation or an initial commit.
- `Ask` prompts only for interactive input.
- `Ask` currently errors for non-interactive input.
- That last branch is the first-contact failure this ticket removes.

## Repository probing

- `repository_state` shells out to `git -C <root> rev-parse --show-toplevel`.
- Command-not-found currently returns `RepositoryState::Missing`.
- A normal “not a git repository” diagnostic also returns `Missing`.
- Those states have different capabilities despite sharing one representation.
- In the command-not-found case Lisa cannot later run `git init`.
- In the ordinary missing-repository case Lisa can initialize history.
- Other process launch failures currently return a `Could not inspect` error.
- Unexpected nonzero Git results also become errors.
- Successful repository discovery produces the actual top-level path.
- This avoids creating a nested repository inside a parent repository.
- `rev-parse --verify HEAD` distinguishes born from unresolved HEAD.
- `symbolic-ref --quiet HEAD` confirms a legitimate unborn branch.
- Failure to verify either state is currently surfaced as an inspection error.

## History bootstrap mechanics from T-049-02-01

- `initialize_project_history` runs `git init --quiet <root>`.
- It writes `user.name` and `user.email` with local scope.
- The project-local identity is `Lisa (project history) <lisa@project>`.
- It then delegates to `create_initial_history_commit`.
- Initial-commit creation obtains an empty tree through `git mktree`.
- It creates the commit directly with `git commit-tree`.
- Author and committer identity are supplied through command-scoped variables.
- It advances `HEAD` through compare-and-swap `git update-ref`.
- It never reads, writes, or consumes the ordinary repository index.
- For an existing unborn repository, local config is not changed.
- The command-scoped identity still gives the root commit a stable author.
- These mechanics already satisfy the new default’s successful-history branch.

## Current user-visible copy

- `HISTORY_OFFER` names undo and a record of agent work.
- Its exact text ends with `[Y/n]` and contains no mechanism jargon.
- Empty interactive input accepts the offer.
- `HISTORY_DECLINED` explains that journal-recorded work is not undoable.
- The declined line is already the required fallback consequence line.
- Accepted setup currently prints `Project history is ready.`.
- The ticket requires a new verbatim line for the positive default decision.
- The required line is `Keeping project history — finished work will be undoable.`.
- Dry-run currently asks no question but tells no-flag callers to choose a flag later.
- That dry-run instruction reflects the contract being removed.

## Default and override distinctions

- A default may safely downgrade when the stronger capability is absent.
- An explicit `--with-history` request must not downgrade silently.
- The explicit failure needs to name the missing capability and a remedy.
- `--no-history` never needs the history executable.
- A non-interactive no-flag run should not consult standard input.
- An interactive no-flag run still presents the offer.
- Interactive rejection follows the existing journal path.
- Interactive acceptance with usable history follows existing bootstrap mechanics.
- Interactive acceptance without usable history must become journal fallback.
- This means capability and repository state must both reach choice resolution.

## Safety behavior

- A born repository is returned as `Born` before any prompt or mutation.
- Both history flags are ignored for history mutation in that state.
- The init scaffold may still perform its normal owned-file actions.
- Repository metadata is snapshot-tested independently of scaffold content.
- A nested project within a born parent does not receive nested metadata.
- An unborn repository may contain pre-staged operator work.
- Existing integration coverage snapshots its config and index bytes.
- Decline leaves unborn `HEAD` unresolved and preserves those snapshots.
- Acceptance changes only `HEAD`, preserving config and index snapshots.
- Default acceptance should reuse that exact action, not a new commit path.
- A missing history executable cannot mutate repository metadata.
- Falling back to journal therefore preserves any undiscoverable metadata too.

## Existing integration fixtures

- `crates/lisa-cli/tests/init_history.rs` runs the compiled CLI binary.
- Its fixture isolates `HOME`, system config, and global Git config.
- It provides helpers for Lisa commands, Git commands, and tree snapshots.
- The accepted bare-folder test passes `--with-history` today.
- It asserts repository creation, local identity, root commit, and empty tree.
- It then proves `commit-ticket` can advance history.
- It writes a fixture ticket and proves `status` selects the commit seal.
- The decline test passes `--no-history` and proves journal-only status.
- The flag-contract test currently expects bare non-interactive init to fail.
- That assertion is directly obsolete under this ticket.
- The born-repository test snapshots `.git`, config, global config, and `HEAD`.
- The unborn test snapshots config and ordinary index for decline and acceptance.
- Those byte-level assertions are the unweakened regression boundary.

## No-history-tool fixture needs

- The binary locates `git` through `PATH` because all history commands use its bare name.
- A fixture can provide a temporary executable directory containing Lisa dependencies but no Git.
- Init scaffolding itself does not require Git.
- Status inspection can resolve journal-only without Git when no repository is present.
- The existing fixture already controls command environments centrally.
- A no-Git test should invoke Lisa with a path that cannot resolve `git`.
- It should assert exit zero, absence of `.git`, the consequence line, and journal seal.
- Explicit `--with-history` should use the same environment and assert nonzero.
- Its stderr should pin a named remedy rather than only a low-level spawn error.
- Interactive acceptance is more directly unit-testable through injectable I/O.
- Choice resolution can be tested with an unavailable capability and `interactive = true`.
- This avoids global environment mutation or a pseudo-terminal dependency.

## Unit coverage

- Unit tests already pin offer wording and allowed answers.
- End-of-input currently produces an error instead of silently accepting.
- A unit test pins the obsolete non-interactive flag requirement.
- Dry-run tests pin explicit inclusion and the obsolete no-flag instruction.
- Resolver-level unit tests can pin default acceptance and unavailable fallback cheaply.
- Interactive fallback should assert one offer and the exact decline consequence downstream.
- Positive announcement copy should be a constant and be asserted verbatim.
- The offer-copy test should retain its no-`git` assertion unchanged.

## Documentation surfaces

- README Quick Start already starts with bare `lisa init`.
- Its next paragraph tells scripts and agents to pass a flag.
- That qualification makes flags look required for non-interactive use.
- The CLI reference repeats that scripts and agents must pass a flag.
- Both sections need to describe flags as overrides instead.
- `docs/knowledge/chromebook-install-test.md` uses `--no-history` in leg N.
- The fresh-container scripted leg also uses `--no-history`.
- A comment calls bare non-interactive init’s failure designed behavior.
- That designed-error note is explicitly obsolete.
- The no-Git leg can now exercise bare init and its automatic fallback.
- The ordinary leg can exercise bare init and its automatic keep-history decision.
- Runbook prose should reserve flags for deliberate test overrides, not the happy path.

## Boundaries and exclusions

- Completion seal resolution itself does not change.
- The automatic seal already selects commit when repository prerequisites exist.
- It already selects journal when they do not.
- `commit_transaction.rs` does not need modification.
- CLI flag declarations and help text do not need semantic changes.
- No dependency is required for implementation or tests.
- No prompt-copy rewrite is requested.
- No existing config upsert or client detection work belongs to this ticket.
- No pre-init empty-surface work from sibling `T-050-01-02` belongs here.
- Ticket frontmatter and shared admitted work artifacts remain Lisa-owned.
