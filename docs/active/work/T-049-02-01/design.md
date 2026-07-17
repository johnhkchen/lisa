# Design: init-history-offer

## Decision summary

Add a typed history preference to `lisa init`, resolve it against repository state, and bootstrap history with an empty root commit.

The CLI exposes two mutually exclusive flags:

- `--with-history` explicitly accepts project history.
- `--no-history` explicitly declines project history.

With neither flag, a terminal invocation presents a plain-language yes/no offer. A non-terminal invocation requires one of the flags whenever a history choice is relevant. A dry run never prompts or mutates.

For a folder with no repository, acceptance initializes repository metadata, writes the required project-local Lisa identity, and creates an empty initial commit. For an existing unborn repository, explicit acceptance creates only the empty initial commit with command-scoped Lisa identity and does not write configuration. A repository with an existing `HEAD` receives no history mutation.

## Goals

- Make a fresh initialized Lisa project satisfy automatic commit-seal probing.
- Leave a resolvable `HEAD` after accepted history setup.
- Preserve pre-existing user files outside the initial commit.
- Preserve every existing local, global, and system identity value.
- Avoid creating nested repository metadata inside a parent repository.
- Give scripts a deterministic, non-interactive contract.
- Keep offer and decline copy in household language rather than mechanism language.
- Exercise the real CLI, seal resolver, and isolated transaction in fixtures.

## Non-goals

- Change the completion transaction's unconditional parent behavior.
- Implement journal-sealed completion.
- Change doctor identity diagnostics.
- Configure any remote or publish history anywhere.
- Stage or commit the user's pre-existing work.
- Change completion-mode configuration during init.
- Modify an existing repository's configured identity.

## Option 1: Commit all initialized files as the root commit

This approach would run scaffold creation first, stage the generated Lisa files, and commit them as the project-history root.

Advantages:

- The initial commit contains visible project setup.
- A freshly initialized folder might appear clean except for pre-existing files.
- The first commit has meaningful content.

Disadvantages:

- It couples filesystem scaffolding and repository mutation into one large operation.
- It risks consuming entries from the user's ordinary index in an existing unborn repository.
- It must distinguish generated files from pre-existing files and safety-skipped files.
- It complicates rollback if a later scaffold write fails.
- It makes the identity appear to author setup files that may have been updated from prior templates.
- It is unnecessary for satisfying the completion transaction's need for a parent.

Decision: reject. The ticket requires a root commit, not committed scaffold content.

## Option 2: Create an empty root commit

This approach initializes metadata where needed, establishes identity safely, and creates an `--allow-empty` root commit.

Advantages:

- It gives `HEAD` a parent target with the smallest possible repository mutation.
- It never stages user content.
- It never consumes the ordinary index.
- It works before or after scaffolding without needing a file ownership map.
- It gives later completion commits exactly the parent they require.
- It is straightforward to assert in fixtures.

Disadvantages:

- Generated Lisa files remain uncommitted working-tree content.
- The first commit communicates its purpose only through its message and identity.
- An empty root can surprise users who inspect low-level history.

Decision: choose. The safety properties dominate the cosmetic value of a populated root.

## Option 3: Change commit transactions to support unborn branches

This approach would make `commit-tree -p` conditional and let the first ticket completion create the root commit.

Advantages:

- `lisa init` would not need to create a commit.
- The first completed ticket could become the history root.

Disadvantages:

- It contradicts the ticket's explicit initial-commit requirement.
- It expands scope into the shared completion transaction.
- It leaves automatic seal probing unable to classify the repository as commit-capable before work starts.
- It delays failure until completion if identity or transaction setup is invalid.
- It reintroduces the field-stall shape the ticket is intended to remove.

Decision: reject.

## Option 4: Always default to acceptance

This approach would create repository history without a prompt or required flag.

Advantages:

- Fresh projects reliably become commit-sealed.
- Existing tests and scripts need no new command arguments.

Disadvantages:

- Repository creation is a meaningful persistent choice.
- It violates the explicit offer-and-acceptance requirement.
- It gives non-interactive callers no safe opportunity to decline.
- It weakens the hard safety contract.

Decision: reject.

## Option 5: Default non-interactive calls to decline

This approach would preserve old scripts by choosing journal-only when stdin is not a terminal.

Advantages:

- Existing automation does not fail.
- It never creates repository metadata unexpectedly.

Disadvantages:

- Silence can look like acceptance of the stronger default when it is not.
- Scripts may unknowingly remain journal-only.
- It undercuts the purpose of explicit non-interactive flags.
- It cannot distinguish deliberate decline from omitted configuration.

Decision: reject for relevant repository states. Require an explicit flag instead.

## History preference model

Use an enum with three states:

- `Ask`
- `WithHistory`
- `NoHistory`

Clap owns flag exclusivity. Dispatch converts the two booleans into one enum, so the init module never handles an invalid combination.

This keeps command syntax concerns in `main.rs` and history behavior in `init.rs`.

## Repository-state model

Probe the requested path with `rev-parse --show-toplevel`.

Represent the result as:

- no repository;
- existing repository with unborn `HEAD`;
- existing repository with resolved `HEAD`.

The resolved repository root is retained for existing-repository operations. This prevents accidental nested initialization when the requested project path is below a repository root.

Repository state is resolved before the choice because a born repository needs no offer or history mutation.

## Choice resolution

The decision table is:

| Repository state | Ask in terminal | Ask non-terminal | With history | No history |
|---|---|---|---|---|
| Missing | prompt | actionable error | bootstrap new repository | print consequence |
| Existing unborn | prompt | actionable error | create root commit only | print consequence |
| Existing born | no prompt/no-op | no-op | no-op | no-op |

Dry-run adds an outer rule: print the offer/plan as applicable, perform no probe-dependent mutation, and never read input.

## Interactive offer

Use standard-library terminal detection and line input rather than adding a prompt dependency.

Offer copy is held in constants so tests can inspect the exact surface. The proposed copy is:

`Bring project history along? Finished work can be undone, and you'll have a record of what the agents did. [Y/n]`

Properties:

- The action appears first.
- The benefits are concrete.
- The word `git` does not occur.
- The default is visible.
- The question applies equally to a missing repository and an unborn one.

Accept `y`, `yes`, or an empty response. Accept `n` or `no` as decline. Invalid input prints a short retry line and asks again.

## Decline copy

Use the ticket's exact consequence sentence:

`Finished work will be recorded in Lisa's journal but won't be undoable.`

The output may lead with `Continuing without project history.` but must retain the exact consequence sentence as its own observable substring.

This copy does not mention the underlying mechanism.

## New-repository bootstrap

For acceptance in a folder with no repository:

1. Run repository initialization at the requested root.
2. Write `user.name = Lisa (project history)` with local scope.
3. Write `user.email = lisa@project` with local scope.
4. Create an empty root commit named `Start project history`.
5. Run the normal scaffold action execution.

The commit process also receives command-scoped author and committer identity. That makes the commit deterministic even if environment precedence differs while retaining required local config for later ticket commits.

## Existing-unborn bootstrap

For explicit acceptance in an existing unborn repository:

1. Use the discovered repository root.
2. Do not run repository initialization.
3. Do not run any config command.
4. Create an empty root commit with command-scoped Lisa author/committer identity.
5. Continue normal init scaffolding at the requested project root.

This preserves local and global config bytes, does not stage existing files, and births the branch only after acceptance.

## Existing-born behavior

When `HEAD` resolves, history already exists. Init skips the offer and all repository/config mutations. Normal scaffold planning and execution continue unchanged.

An explicit history flag is treated as an idempotent statement, not an instruction to add another commit.

## Failure behavior

Every external command checks process launch and exit status. Failures are reported as project-history setup failures with the underlying stderr retained for diagnosis.

Decline does not require successful history tooling. Acceptance does.

The implementation will not delete a pre-existing repository or change its config during cleanup. A repository created by this invocation remains observable if a later setup command fails; the error prevents a false success report.

## Test strategy

Add a black-box integration fixture dedicated to history initialization.

Fixture environment:

- Temporary project root.
- Temporary home/global config.
- System config disabled.
- Real built `lisa` binary.
- Real repository commands.

Acceptance fixture asserts:

- repository metadata exists;
- local identity is exact;
- `HEAD` resolves;
- root commit is empty;
- exact-path `commit-ticket` succeeds afterward;
- automatic status resolves commit-sealed;
- accepted output copy does not introduce mechanism jargon.

Decline fixture asserts:

- no repository metadata exists;
- exact consequence copy appears;
- automatic status resolves journal-only.

Existing repository fixture asserts:

- repository metadata snapshot is byte-identical;
- local and global config bytes are unchanged;
- `HEAD` remains the same;
- no nested metadata appears.

Unborn fixture asserts:

- decline leaves `HEAD` unresolved;
- explicit acceptance resolves `HEAD`;
- config snapshots remain unchanged.

Unit tests retain coverage of writer-injected init behavior and add exact offer-copy checks. Help snapshots lock the new flags and their mutual-exclusion behavior is exercised through Clap.

## Commit boundary

The implementation changes form one coherent source unit across:

- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/tests/help_surface.rs`
- `crates/lisa-cli/tests/init_history.rs`

They should be committed together with one exact-path `lisa commit-ticket` transaction because the command signature, implementation, and black-box contract must stay synchronized.
