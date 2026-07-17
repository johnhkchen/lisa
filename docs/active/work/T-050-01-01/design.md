# Design: init-history-default

## Decision summary

Keep the existing history preferences and bootstrap mechanics, but add an explicit
repository-probe outcome for unavailable history tooling. Resolve the no-flag
preference as an automatic decision outside terminals and as the existing offer
inside terminals.

The decision table becomes:

| Observed state | No flag, non-interactive | No flag, interactive accept | `--with-history` | `--no-history` |
|---|---|---|---|---|
| Missing repository, history usable | create history | create history | create history | journal |
| Existing unborn repository | create initial commit | create initial commit | create initial commit | journal |
| Existing born repository | no history mutation | no history mutation | no history mutation | no history mutation |
| History tooling unavailable | journal fallback | journal fallback | actionable error | journal |

Successful history creation prints the required sentence:

`Keeping project history — finished work will be undoable.`

Journal choice or fallback retains the existing consequence sentence. Existing
repository creation, identity, empty-tree commit, and compare-and-swap behavior
remain unchanged.

## Goals

- Make bare `lisa init` succeed in non-interactive fresh folders.
- Prefer commit-capable history whenever the environment can establish it.
- Fall back to journal-only operation when the history executable is unavailable.
- Preserve the interactive offer and its empty-input default.
- Prevent an accepted interactive default from failing on a Git-less machine.
- Preserve explicit override semantics.
- Give explicit `--with-history` failures a named remedy.
- Keep existing repository metadata and user state byte-untouched.
- Reuse the proven empty-root bootstrap from `T-049-02-01`.
- Return documentation to bare `lisa init` as the normal first command.

## Non-goals

- Remove either history flag.
- Change the offer wording.
- Change the journal consequence wording.
- Change completion seal probing or transaction behavior.
- Add repository support when Git itself is absent.
- Retry installation or repair of external history tooling.
- Modify born repository identity or configuration.
- Populate the initial root commit with scaffold or operator files.
- Change init’s owned-template and safety-skip policies.
- Address sibling empty-state and pre-init command surfaces.

## Option 1: Treat every missing repository as history-capable

This keeps `RepositoryState::Missing` unchanged and makes no-flag `Ask` resolve to
`CreateRepository`.

Advantages:

- The resolver change is extremely small.
- Normal machines immediately receive the desired default.
- Existing repository state matching changes minimally.

Disadvantages:

- Command-not-found is currently folded into `Missing`.
- A Git-less machine would attempt `git init` and fail after choosing history.
- Interactive empty input would retain the exact release-audit failure.
- Explicit and default requests could not receive different downgrade behavior.
- The implementation would not satisfy the no-Git acceptance fixtures.

Decision: reject. Capability absence is a first-class product outcome.

## Option 2: Probe Git separately before repository discovery

Run a standalone capability command such as `git --version`, then call the existing
repository-state probe only when it succeeds.

Advantages:

- Capability is explicit before any repository inspection.
- Error copy can distinguish an absent executable cleanly.
- The existing state enum could remain unchanged.

Disadvantages:

- Every init run adds a redundant process invocation.
- Version success does not prove repository inspection will work.
- The same executable launch is immediately repeated by `rev-parse`.
- Other inability states still need an error/fallback policy.
- It creates two sources of truth for “history usable.”

Decision: reject. Repository discovery already performs the necessary capability
probe and can preserve its failure reason.

## Option 3: Add an unavailable repository-probe state

Extend `RepositoryState` with an unavailable variant carrying a diagnostic reason.
Map executable absence and other inspection failures into that state, then let the
preference resolver decide whether to downgrade or fail.

Advantages:

- Capability and repository shape remain one exhaustive domain result.
- No extra subprocess is needed.
- Default behavior can downgrade without discarding the diagnostic.
- Explicit `--with-history` can surface the diagnostic and a remedy.
- Interactive fallback is directly unit-testable without modifying global `PATH`.
- Born and unborn paths retain their existing representations and mechanics.

Disadvantages:

- Some failures that previously stopped init will now choose journal by default.
- The enum name covers both repository state and probe availability.
- The resolver must carefully keep explicit requests distinct from defaults.

Decision: choose. This is the smallest model that expresses every required branch.

## Option 4: Attempt history creation and catch failures

Always resolve the default to keep history, then downgrade if `git init`, config, or
commit creation returns an error.

Advantages:

- Avoids modeling availability before execution.
- Handles failures after a successful initial probe too.
- Appears robust against a broad range of machine conditions.

Disadvantages:

- `git init` may partially create repository metadata before a later failure.
- Downgrading after mutation violates a clean journal-only fallback boundary.
- Rollback would be destructive and unsafe around pre-existing metadata.
- Existing unborn repositories could receive partial changes.
- Explicit and default behavior would need mutation-aware recovery logic.

Decision: reject. Fallback must be selected before any history mutation.

## Option 5: Remove interactive prompting entirely

Treat no flag identically in terminals and scripts: automatically keep when possible
and fall back otherwise.

Advantages:

- The product principle is maximally uniform.
- Input plumbing and prompt tests could eventually be deleted.
- Empty-input behavior could not cause a failure.

Disadvantages:

- The ticket explicitly keeps the interactive offer.
- It removes a deliberate human choice introduced in the prior ticket.
- It expands scope beyond the observed non-interactive friction.

Decision: reject. Interactive prompting is a stated compatibility requirement.

## Unavailable-state policy

The unavailable variant carries a human-readable lower-level reason. Command-not-found
uses a stable reason naming Git’s absence. Other command-launch or inspection failures
retain the existing contextual diagnostic.

For `HistoryPreference::Ask`:

- interactive mode still prompts;
- rejection produces `Decline`;
- acceptance produces `Decline` when the state is unavailable;
- non-interactive mode produces `Decline` immediately when unavailable;
- non-interactive mode chooses history immediately for missing or unborn states.

For `HistoryPreference::WithHistory`:

- unavailable state returns a stable, actionable error;
- the error says project history was explicitly requested;
- it includes the observed reason;
- it directs the operator to install or repair Git and retry;
- it also names `--no-history` as the deliberate journal override.

For `HistoryPreference::NoHistory`:

- unavailable state resolves to `Decline` with no error;
- no history command is executed.

## Inspection-failure tradeoff

Turning unexpected probe failures into the unavailable state broadens automatic
fallback beyond command-not-found. This is intentional for the no-flag principle:
an unconfigured default should not demand a capability its environment cannot use.
The diagnostic is not lost, because explicit `--with-history` still reports it.

This policy does not mutate an ambiguous repository. It only permits ordinary Lisa
scaffolding to continue in journal mode. Repository metadata, identity, index, and
history remain untouched because no history action runs.

## Positive announcement

Replace the generic post-bootstrap `Project history is ready.` output with the exact
ticket sentence. Use one constant so unit and integration tests can pin it verbatim.
Print it after successful repository initialization or successful unborn-root commit,
never before. This ensures output cannot claim history was kept when a command failed.

Explicit `--with-history` may use the same successful sentence. The line accurately
states the resulting state, and a single success surface avoids mode-dependent copy.
Born repositories print no new line because Lisa did not decide or establish their
history.

## Dry-run behavior

Dry run remains non-mutating and non-prompting. With no flag, it resolves the same
capability decision but describes the prospective outcome:

- usable missing/unborn state: project history would be kept;
- unavailable state: print the existing journal consequence;
- born state: no history-specific output;
- explicit overrides retain their corresponding prospective output or error.

The obsolete instruction to choose a flag is removed. This aligns preview behavior
with the new default while avoiding the inaccurate present-tense keep announcement.

## Test design

Update the compiled-CLI integration fixture rather than replacing its strong checks.
The accepted fresh-folder test should remove `--with-history`, assert the exact new
announcement, and retain repository identity, empty root, transaction, and seal checks.

Add a controlled no-Git command environment. A bare init in that environment must:

- exit zero;
- print the exact journal consequence;
- leave `.git` absent;
- scaffold the project;
- allow `status` to report journal-only.

The same environment with `--with-history` must fail before scaffolding and assert
the named repair/override remedy. Keep Clap’s conflicting-flags assertion and the
offer’s no-jargon assertion.

At unit level, inject `RepositoryState::Unavailable` into resolver tests. Simulate
interactive `yes` input and assert `Decline`; simulate non-interactive no flag and
assert `Decline`; simulate explicit history and pin the actionable error. Existing
prompt parsing tests remain unchanged.

Retain born and unborn snapshot fixtures without weakening any config, index, tree,
global-config, or `HEAD` comparison. Default behavior may be added to those fixtures
only where it preserves the exact byte boundary.

## Documentation design

README Quick Start should teach one command: bare `lisa init`. The surrounding prose
should explain that Lisa keeps history when available and uses its journal otherwise.
Show both flags as optional overrides for operators and scripts that want to force a
specific outcome.

The CLI reference should use the same hierarchy: bare command first, flags explicitly
labeled overrides, interactive offer noted, and automatic non-interactive decision
described.

The Chromebook runbook should use bare init in both ordinary and no-Git measured legs.
Delete the designed-error comment. Explain that flags are available only when a leg
intentionally needs to force one branch; the normal runbook should measure the default.

## Chosen boundary

Modify only:

- `crates/lisa-cli/src/init.rs` for state, resolution, copy, and unit tests;
- `crates/lisa-cli/tests/init_history.rs` for black-box acceptance fixtures;
- `README.md` for Quick Start and CLI reference;
- `docs/knowledge/chromebook-install-test.md` for bare-init runbook guidance.

No CLI parser, completion seal, transaction, config, or dependency changes are needed.
