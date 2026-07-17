# Review: init-history-default

## Outcome

`lisa init` now makes the history decision when no flag is provided.

- On a machine that can establish project history, bare init keeps it.
- On a machine that cannot use Git, bare init uses Lisa’s journal and exits zero.
- Interactive init retains the existing plain-language offer.
- Accepting that offer without Git now completes through the journal fallback.
- Explicit `--with-history` without Git remains a named actionable error.
- Explicit `--no-history` remains a deterministic journal override.
- Born repositories remain history no-ops.
- Existing unborn repository bootstrap mechanics and safety remain unchanged.

The positive result prints the exact required sentence:

`Keeping project history — finished work will be undoable.`

The journal result retains the existing exact consequence sentence:

`Continuing without project history: finished work will be recorded in Lisa's journal but won't be undoable.`

The ticket is ready to complete.

## Files changed

### `crates/lisa-cli/src/init.rs`

- Added the exact positive decision announcement constant.
- Extended private repository discovery with `Unavailable { reason }`.
- Distinguished missing Git from a usable machine with no repository.
- Preserved a stable unavailable reason for explicit-request diagnostics.
- Converted other pre-mutation repository inspection failures into unavailable state.
- Left ordinary not-a-repository detection mapped to `Missing`.
- Left born and unborn discovery semantics intact.
- Removed the non-interactive no-flag error.
- Made non-interactive no flag select history for missing/unborn usable states.
- Made non-interactive no flag select journal for unavailable state.
- Retained the interactive prompt for no-flag terminal use.
- Made interactive acceptance select journal when history is unavailable.
- Retained interactive rejection as journal choice.
- Made explicit with-history fail on unavailable state.
- Added install/repair, retry, and no-history override guidance to that error.
- Kept explicit no-history as journal choice without invoking history setup.
- Made no-flag dry runs preview the automatic decision without prompting.
- Replaced `Project history is ready.` with the exact required positive line.
- Added an internal history-state execution seam for deterministic full-path tests.
- Preserved existing local identity and empty-root commit functions unchanged.
- Preserved all scaffold planning, ownership, safety-skip, and output reporting behavior.

### `crates/lisa-cli/tests/init_history.rs`

- Changed the successful fresh-folder fixture from explicit acceptance to bare init.
- Pinned the exact positive announcement.
- Retained repository creation assertions.
- Retained project-local name and email assertions.
- Retained the global-config byte preservation assertion.
- Retained root commit identity/message assertions.
- Retained the explicitly empty root-tree assertion.
- Retained the real exact-path `commit-ticket` follow-up.
- Retained commit-seal visibility through `status`.
- Added a per-fixture empty `PATH` without mutating process-global state.
- Added bare no-Git init success coverage.
- Added exact consequence output coverage for automatic fallback.
- Added no-Git journal-seal status coverage.
- Added explicit with-history/no-Git failure coverage.
- Pinned Git availability, repair, retry, and override remedy fragments.
- Asserted explicit failure occurs before scaffold writes.
- Retained Clap conflict behavior for both flags.
- Updated dry-run coverage for automatic decision preview.
- Retained offer-copy no-`git` assertion.
- Retained all born-repository metadata and config snapshots.
- Retained all unborn-repository config and index byte snapshots.
- Retained staged operator-work preservation assertions.

### `README.md`

- Kept bare `lisa init` as the Quick Start command.
- Described automatic keep-history behavior.
- Described automatic journal fallback.
- Preserved the interactive offer in the user model.
- Reframed both history flags as overrides.
- Removed the claim that scripts and agents must supply a flag.
- Updated CLI reference comments and prose to match runtime behavior.
- Preserved existing-repository safety guidance.

### `docs/knowledge/chromebook-install-test.md`

- Changed the no-Git completion instruction to bare init.
- Made automatic journal fallback part of that measurement.
- Changed the ordinary scripted init command to bare init.
- Removed the designed-error note for non-interactive bare init.
- Documented flags only as deliberate branch-forcing overrides.
- Left unrelated grader and completion-leg mechanics unchanged.

## Behavior matrix

| State | No flag, non-interactive | No flag, interactive | `--with-history` | `--no-history` |
|---|---|---|---|---|
| No repository, Git usable | create history | offer; yes creates | create history | journal |
| Existing unborn repository | create root commit | offer; yes creates | create root commit | journal |
| Existing born repository | no history mutation | no offer | no history mutation | no history mutation |
| Git/history unavailable | journal fallback | offer; yes/no journal | actionable error | journal |

This matrix is exhaustive at the private repository-state boundary.

## Safety assessment

The successful default reuses the exact mechanics from `T-049-02-01`:

- `git init --quiet` only for an actually missing repository;
- local Lisa identity only for the newly created repository;
- empty tree from `git mktree`;
- root commit from `git commit-tree`;
- command-scoped author and committer identity;
- compare-and-swap `git update-ref`;
- no ordinary-index read or write.

Existing unborn repositories still receive only the root commit. Their local config
and ordinary index remain untouched. Pre-staged operator work remains staged and does
not enter the empty root.

Born repositories return `HistoryAction::None` before prompting or considering an
override. No nested repository is created, identity is not rewritten, and `HEAD` is
not advanced.

Unavailable default fallback runs no mutating history command. Explicit history fails
before project detection, planning, or scaffold writes, so it cannot claim success or
leave partial Lisa scaffolding.

## Test evidence

### Focused behavior

`cargo test -p lisa-cli interactive_accept_without_git_completes_with_journal_fallback`

- Result: 1 passed, 0 failed.
- Executes the full init path with interactive empty input and unavailable history.
- Proves the offer appears.
- Proves the exact journal consequence appears.
- Proves initialization completes and scaffolding exists.
- Proves `.git` is absent.

`cargo test -p lisa-cli --test init_history`

- Result: 7 passed, 0 failed.
- Proves bare fresh-folder keep-history behavior.
- Proves explicit journal override.
- Proves bare no-Git fallback and journal seal.
- Proves explicit history/no-Git actionable failure.
- Proves flag conflict and automatic dry run.
- Proves born repository snapshots.
- Proves unborn repository config/index safety.

`cargo test -p lisa-cli --test help_surface`

- Result: 6 passed, 0 failed.
- Confirms no accidental parser or help snapshot drift.

### Formatting and broad coverage

`cargo fmt --all -- --check`

- Result: passed on final stable HEAD.

`cargo test -p lisa-cli`

- Result: passed on rerun.
- Binary unit suite: 358 passed.
- All CLI integration targets passed after sibling shared-file commits stabilized.

`cargo test --workspace`

- Result: passed on final stable HEAD.
- `lisa-cli` library: 21 passed.
- `lisa-cli` binary: 358 passed.
- `lisa-core`: 248 passed.
- `lisa-plugin`: 437 passed.
- Completion ordering and recorded-livelock integrations passed.
- CLI and core doc tests passed.
- The environment-gated real-Zellij fixture remained intentionally ignored.

## Transient verification observations

One early compiled-CLI run overlapped the sibling ticket’s intentional temporary
removal of its validation hunk during exact-path commit coordination. Its new test was
present without its implementation and failed as expected. The sibling reapplied and
committed that hunk, and final workspace verification passed.

The managed-runtime checksum mismatch unit test also failed once because its expected
diagnostic was absent. This ticket does not touch runtime download/checksum code. The
test passed immediately in isolation, passed in the next full CLI run, and passed in
the final workspace run. This is recorded as an existing transient, not an open ticket
defect.

The sibling’s initial pre-init guard also exposed a preownership ledger-status
regression during broad verification. The sibling corrected and committed it as
`3c858bf`; final workspace verification includes that fix and passes.

## Commit evidence

Documentation unit:

```text
81d1127a022dd8c2921857027902860b0be48d29
Teach bare init as the automatic history path
```

Exact paths:

- `README.md`
- `docs/knowledge/chromebook-install-test.md`

Source and fixture unit:

```text
0dd3b68106889bb87622b25a494a02dcf265843f
Make init choose the strongest history default
```

Exact paths:

- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/tests/init_history.rs`

Both commits were created through `lisa commit-ticket`. No ordinary `git add` or
ordinary `git commit` was used for ticket work.

## Working-tree audit

All four ticket-owned repository paths are committed and clean.

Remaining status entries are Lisa-owned ticket frontmatter and admitted/shared phase
artifact paths for `T-050-01-01` and sibling `T-050-01-02`. This ticket did not edit
frontmatter phase/status or directly publish shared work artifacts.

The ordinary index is empty for ticket-owned paths.

## Open concerns and limitations

- No known ticket-blocking concern remains.
- No dependency or public configuration shape changed.
- No live pseudo-terminal test was added; interactive fallback is covered through the
  same injectable I/O execution path used by init and completes full scaffolding.
- Generic pre-mutation repository inspection failures now follow default journal
  fallback, while explicit with-history preserves their diagnostic. This is the chosen
  default-decision policy, not silent loss of an explicit request.
- Dry run now previews the automatic choice and does not show the interactive offer.
  This matches its existing non-prompting contract and removes obsolete flag guidance.

## Acceptance conclusion

Every ticket criterion has direct fixture, output, documentation, and broad-suite
evidence. Existing repository snapshot assertions remain present and unweakened. The
implementation is safe to pass Review.
