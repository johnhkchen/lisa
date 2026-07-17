# Review: init-history-offer

## Outcome

The ticket is ready to complete.

`lisa init` now offers project history in plain language when a history choice
is relevant. Acceptance gives a bare project a repository, a dedicated local
Lisa identity, and an empty root commit, so `HEAD` is immediately resolvable and
the existing completion transaction has a parent. Decline creates no repository
and states that finished work remains recorded in Lisa's journal but cannot be
undone.

Non-interactive callers can choose deterministically with `--with-history` or
`--no-history`. When a missing or unborn repository needs a choice and neither
flag is supplied through non-interactive input, init returns an actionable error
before writing scaffold files.

## Files changed

### `crates/lisa-cli/src/main.rs`

- Added `--with-history`.
- Added `--no-history`.
- Made the flags mutually exclusive through Clap.
- Converted the syntax booleans into `HistoryPreference`.
- Passed the typed preference into init execution.

### `crates/lisa-cli/src/init.rs`

- Added the project-history copy and identity constants.
- Added `HistoryPreference`.
- Added repository-state and resolved-action types.
- Added read-only discovery for missing, unborn, and born repositories.
- Added checked command helpers with stderr-preserving errors.
- Added accepted bare-folder repository initialization.
- Added project-local identity configuration only for newly created repositories.
- Added empty-tree root commit construction.
- Added command-scoped author and committer identity.
- Added compare-and-swap branch creation for an unborn repository.
- Added injectable prompt handling for testability.
- Added interactive terminal detection in the public init entry point.
- Added explicit non-interactive choice enforcement.
- Integrated history setup before scaffold writes.
- Preserved dry-run as non-mutating and non-blocking.
- Updated legacy unit callers to choose journal-only explicitly.
- Added focused prompt, copy, dry-run, and non-interactive unit tests.

### `crates/lisa-cli/tests/help_surface.rs`

- Updated the exact init help snapshot with both new options.

### `crates/lisa-cli/tests/init_history.rs`

- Added five black-box acceptance fixtures.
- Isolated home, global config, and system config inputs.
- Exercised the compiled Lisa binary and real repository commands.
- Exercised a real subsequent `commit-ticket` transaction.
- Exercised real seal resolution through `lisa status`.

### Scripted fixture callers

The following deterministic, non-interactive fixtures now pass
`--no-history` explicitly:

- `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`
- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`
- `docs/active/work/T-031-03/harness/run.sh`

## Behavior matrix

| Repository state | Interactive, no flag | Non-interactive, no flag | `--with-history` | `--no-history` |
|---|---|---|---|---|
| Missing | Offer | Error before mutation | Create repo, local Lisa identity, root commit | No repo; print consequence |
| Existing unborn | Offer | Error before mutation | Create root commit only | Preserve unborn state; print consequence |
| Existing born | No history action | No history action | No history action | No history action |

Dry-run never reads input or performs history mutation. With no explicit flag,
it shows the offer and tells the operator which flags make the eventual choice.

## Safety assessment

### Existing repository preservation

Repository discovery happens before any history mutation. A folder inside an
existing born repository is recognized through its top-level root, so init does
not create nested metadata. The fixture snapshots the complete metadata tree,
local config bytes, global config bytes, and `HEAD`; they remain unchanged.

### Identity preservation

The local Lisa identity is written only after this invocation creates a new
repository. Existing repositories never receive config writes. Existing unborn
repositories use command-scoped author/committer values for their explicit root
commit, preserving configured local and global identities.

### Ordinary index preservation

Root commits are assembled with `mktree` and `commit-tree`; they do not stage
work. The unborn fixture begins with operator content already staged and proves
that the initial commit tree is empty while the exact staged entry remains in
the ordinary index.

### Concurrent branch creation

The final `update-ref` supplies an all-zero expected old ID sized to the current
object format. If another process births the branch after discovery, the update
fails instead of overwriting the concurrent commit.

### Existing content preservation

The root commit deliberately uses an empty tree. Pre-existing project files and
the scaffold files written by init remain unclaimed. Later ticket completion
commits can include only their owned paths through the existing isolated
transaction.

## Acceptance-criteria coverage

- Bare acceptance creates repository metadata: covered.
- Bare acceptance sets exact local identity: covered.
- Bare acceptance resolves `HEAD`: covered.
- Bare acceptance supports a later completion-style commit: covered with real `commit-ticket`.
- Bare acceptance resolves commit seal: covered with real `status`.
- Bare decline creates no repository: covered.
- Bare decline prints the required consequence: covered exactly.
- Bare decline resolves journal seal: covered with real `status`.
- Existing repository metadata remains byte-identical: covered recursively.
- Existing local and global config remains byte-identical: covered.
- Existing unborn repository changes `HEAD` only after acceptance: covered.
- Existing unborn ordinary index remains intact: covered.
- Both non-interactive flags work: covered.
- Conflicting flags fail: covered.
- Offer copy is asserted and avoids the forbidden mechanism word: covered.

## Test results

- Formatting check passed.
- Diff whitespace check passed.
- Init-focused suite: 74 passed.
- Help-surface suite: 6 passed.
- New history integration suite: 5 passed.
- Complete isolated workspace: 1,049 passed, 0 failed.
- One real-Zellij integration test remained ignored by its existing environment gate.
- Workspace doc tests passed.

One unrelated managed-runtime cache test produced a transient local-response
write error on the first isolated run. It passed immediately when rerun alone,
and passed again as part of the final full workspace run. This is not connected
to init or repository history behavior.

## Commits

- `10f4d03a12a7b098a43b811da81e57732c5722a3` contains the CLI, init behavior, help snapshot, and acceptance fixtures.
- `86a4cb5c00195827320aecd102544e1ada448b6c` contains explicit history choices for non-interactive scripted fixtures.

Both commits were created with `lisa commit-ticket` and exact include paths.
The ordinary index was not used.

## Open concerns and limitations

- The interactive prompt is tested through its injected input/output boundary rather than a PTY integration fixture.
- Accepted history setup requires the repository command-line tool to be available; failures are surfaced with checked, actionable errors.
- The empty root commit intentionally does not make the scaffolded working tree clean.
- Existing born repositories receive no new initial commit even when a history flag is supplied; their existing history is authoritative.
- No remote, publishing, journal-seal implementation, or doctor behavior is changed by this ticket.

No blocking issue, source TODO, or acceptance gap remains.

## Final disposition rationale

The implemented state satisfies every acceptance criterion, preserves existing
repository identities and index state, leaves accepted new projects with a
resolvable `HEAD`, passes focused and workspace verification, and has no
ticket-owned source changes outside the two isolated commits.
