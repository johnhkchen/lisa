# Progress: init-history-default

## Current state

Implementation is complete. Source, fixtures, README, and runbook changes are committed
through Lisa with exact paths. Targeted, CLI-wide, formatting, and workspace gates pass.

## Completed: phase preparation

- Read `AGENTS.md` and the delegated `CLAUDE.md` guidance.
- Read the complete ticket and RDSPI workflow.
- Inspected prior `T-049-02-01` artifacts and implementation boundaries.
- Inspected CLI parsing, init repository probing, history actions, and fixtures.
- Inspected README Quick Start and CLI reference.
- Inspected the Chromebook install/no-Git runbook paths.
- Wrote `research.md` in the attempt-private work directory.
- Wrote `design.md` in the attempt-private work directory.
- Wrote `structure.md` in the attempt-private work directory.
- Wrote `plan.md` in the attempt-private work directory.

## Completed: source implementation

Modified `crates/lisa-cli/src/init.rs`:

- Added the exact positive announcement:
  `Keeping project history — finished work will be undoable.`
- Added `RepositoryState::Unavailable { reason }`.
- Distinguished a missing Git executable from a usable machine with no repository.
- Preserved ordinary missing, unborn, and born repository states.
- Preserved existing empty-tree root-commit mechanics.
- Changed non-interactive no-flag resolution to keep history when available.
- Changed non-interactive no-flag resolution to decline/fallback when unavailable.
- Retained the interactive offer.
- Changed interactive acceptance with unavailable Git to journal fallback.
- Retained explicit `--no-history` as journal choice.
- Made explicit `--with-history` on an unavailable machine return an actionable error.
- The error names Git repair/reinstallation, retry, and the no-history override.
- Removed the obsolete non-interactive flag demand.
- Changed no-flag dry run to preview the automatic decision without prompting.
- Replaced generic positive output with the exact ticket sentence.
- Added a state-injected internal execution seam for deterministic interactive fallback.

Modified `crates/lisa-cli/tests/init_history.rs`:

- Converted the accepted fresh-folder fixture to bare init.
- Retained identity, empty root, transaction, and commit-seal assertions.
- Added the exact positive announcement assertion.
- Added a per-command empty `PATH` helper without global environment mutation.
- Added bare no-Git init success and journal-seal coverage.
- Added explicit with-history/no-Git failure and remedy coverage.
- Retained conflicting flag behavior.
- Updated dry-run coverage for automatic resolution.
- Retained born-repository full metadata/config/HEAD snapshots.
- Retained unborn-repository config/index/staged-entry snapshots.

## Completed: unit verification

Ran:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli init::tests
```

The first format check identified one integration-fixture wrapping difference, which
was corrected with the formatter. The init test command then passed all 75 selected
binary init tests, including:

- exact history copy;
- automatic non-interactive keep-history behavior;
- unavailable-state preference matrix;
- prompt parsing;
- dry-run behavior;
- existing init/validate regressions.

The command also ran zero matching tests in unrelated targets, as expected from the
filter. Selected binary result: `75 passed; 0 failed`.

After adding the full interactive fallback execution test, a subsequent format check
reported one wrapping-only difference in the new call. That difference was corrected.
The new test still needs rerunning once the concurrent compile failure clears.

## Resolved concurrent shared-file event

While the source verification command was running, sibling ticket `T-050-01-02`
modified unrelated validation behavior in the same `init.rs` file. Its in-flight
`print_diagnostics` change currently has an `else if` without a final `else`, producing:

```text
error[E0317]: `if` may be missing an `else` clause
```

The sibling temporarily removed its validation hunks, explicitly confirmed that the
remaining `init.rs` and `init_history.rs` diffs were exclusively owned by this ticket,
and requested this ticket commit them. This ticket committed only those exact paths.
The sibling then reapplied and committed its validation work separately. No foreign
hunk was included in either ticket-owned commit, and no ordinary-index command was used.

## Completed: README

Modified `README.md`:

- Kept bare `lisa init` as the Quick Start command.
- Described automatic keep-history and journal fallback.
- Retained the interactive offer in prose.
- Reframed both history flags as overrides.
- Removed the requirement that scripts and agents pass a flag.
- Updated CLI reference command comments and explanatory prose.

## Completed: Chromebook runbook

Modified `docs/knowledge/chromebook-install-test.md`:

- Changed the no-Git completion leg instruction to bare init.
- Made automatic journal fallback part of the measured instruction.
- Changed the fresh-container command to bare init.
- Removed the designed-error note for bare non-interactive init.
- Documented flags only as deliberate branch-forcing overrides.

Documentation search confirmed that normal paths show bare init and both override
flags remain documented where relevant.

## Documentation commit

Committed through Lisa with exact paths:

```text
lisa commit-ticket --ticket-id T-050-01-01 \
  --message "Teach bare init as the automatic history path" \
  --include README.md \
  --include docs/knowledge/chromebook-install-test.md
```

Commit returned:

```text
81d1127a022dd8c2921857027902860b0be48d29
```

No ordinary-index command was used.

## Source commit

After explicit shared-file coordination, committed through Lisa with exact paths:

```text
lisa commit-ticket --ticket-id T-050-01-01 \
  --message "Make init choose the strongest history default" \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-cli/tests/init_history.rs
```

Commit returned:

```text
0dd3b68106889bb87622b25a494a02dcf265843f
```

## Final verification

Targeted commands and results:

```text
cargo test -p lisa-cli interactive_accept_without_git_completes_with_journal_fallback
1 passed; 0 failed

cargo test -p lisa-cli --test init_history
7 passed; 0 failed

cargo test -p lisa-cli --test help_surface
6 passed; 0 failed
```

The seven compiled init-history fixtures cover:

- bare fresh folder with Git and exact positive announcement;
- explicit decline and journal consequence;
- bare Git-less fallback and journal seal;
- explicit with-history Git-less error and remedy;
- history flag conflict/override and dry-run behavior;
- born repository byte snapshots;
- unborn repository config/index preservation.

The full interactive acceptance fallback test executes init with injected unavailable
history state and empty input, then proves exit success, exact journal consequence,
completed scaffolding, and absence of `.git`.

Broad commands and results:

```text
cargo fmt --all -- --check
passed

cargo test -p lisa-cli
passed after one unrelated runtime-test transient was rerun successfully

cargo test --workspace
passed
```

The first broad CLI run overlapped the sibling’s temporary hunk-removal handoff and
therefore observed its new regression test without its implementation. A later stable
HEAD run included both sibling commits. During that sequence the managed-runtime
checksum fixture also failed once with a missing expected diagnostic, then passed in
isolation and on both subsequent broad runs. No ticket code touches runtime download or
checksum behavior.

Final workspace results include:

- `lisa-cli` library: 21 passed;
- `lisa-cli` binary: 358 passed;
- init-history integration: 7 passed;
- sibling never-dead-end integration: 5 passed;
- `lisa-core`: 248 passed;
- completion state-machine and recorded regression integrations: passed;
- `lisa-plugin`: 437 passed;
- CLI/core doc tests: passed;
- real-Zellij delivery test: intentionally ignored by its environment gate.

## Acceptance audit

- Fresh + Git + no flag: proven by `bare_folder_default_creates_commit_ready_project_history`.
- Repo, identity, initial commit: asserted in the same compiled fixture.
- Commit seal: fixture runs a real `commit-ticket` then checks `status`.
- Positive line verbatim: exact constant and stdout assertions.
- Fresh + no Git + no flag: compiled empty-`PATH` fixture exits successfully.
- Journal seal and consequence: exact output and status assertions.
- Explicit with-history without Git: compiled failure fixture pins named remedy.
- Interactive accept without Git: full state-injected init unit execution exits successfully.
- Born repository safety: existing full `.git`, config, global config, and `HEAD` snapshots retained.
- Unborn safety: existing config/index and staged-work assertions retained unweakened.
- README: bare init is primary; flags are overrides.
- Chromebook runbook: both measured paths use bare init; designed-error note removed.

## Remaining work

- Write `review.md`.
- Write exact passing `review-disposition.json`.
- Run `lisa check-disposition T-050-01-01` and correct any reported issue.
- Remain on this ticket for Lisa’s completion commit.

## Deviations from plan

- Documentation was committed before the source unit because it was complete and
  path-independent while a sibling ticket temporarily occupied `init.rs`.
- The shared-file collision was resolved through explicit hunk ownership coordination.
- One additional internal state-injection seam was added so interactive no-Git fallback
  could be tested as a full successful init rather than only as an action resolver.
- No scope, acceptance criterion, or safety boundary changed.
