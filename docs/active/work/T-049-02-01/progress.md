# Progress: init-history-offer

## Phase completion

- [x] Research artifact written.
- [x] Design artifact written.
- [x] Structure artifact written.
- [x] Plan artifact written.
- [x] Implementation complete.
- [x] Review prepared.

## Implementation completed

- [x] Added mutually exclusive `--with-history` and `--no-history` flags.
- [x] Added the typed `HistoryPreference` boundary between CLI parsing and init behavior.
- [x] Added plain-language offer and decline copy.
- [x] Asserted that the offer path does not name the underlying mechanism.
- [x] Added repository discovery for missing, unborn, and born states.
- [x] Prevented nested repository creation inside an existing repository.
- [x] Added accepted bare-folder repository initialization.
- [x] Added the required project-local identity.
- [x] Added an empty initial commit with the dedicated Lisa identity.
- [x] Left pre-existing and scaffolded files out of the initial commit.
- [x] Added compare-and-swap protection when birthing an unborn branch.
- [x] Preserved existing local and global identity configuration.
- [x] Preserved an existing unborn repository's ordinary index.
- [x] Added interactive yes/no/default/retry handling.
- [x] Required explicit flags for relevant non-interactive calls.
- [x] Preserved no-op history behavior for repositories with a resolved `HEAD`.
- [x] Preserved dry-run behavior without reading input or mutating history.
- [x] Printed the required journal/undo consequence after decline.
- [x] Updated the operator help snapshot.
- [x] Added black-box history fixtures.
- [x] Updated scripted test callers to select `--no-history` explicitly.

## Acceptance fixture coverage

### Bare folder, acceptance

- Repository metadata is created.
- Local `user.name` is exactly `Lisa (project history)`.
- Local `user.email` is exactly `lisa@project`.
- Global config bytes remain unchanged.
- `HEAD` resolves to a root commit.
- Root commit author and subject are asserted.
- Root commit tree is empty.
- Existing operator content is not claimed by the root commit.
- A real subsequent `commit-ticket` transaction succeeds.
- The subsequent transaction includes only its exact requested file.
- `lisa status` resolves the project as commit-sealed.

### Bare folder, decline

- No `.git` path is created.
- The exact journal/undo consequence is asserted.
- `lisa status` resolves the project as journal-only.

### Non-interactive contract and copy

- Omission of both flags fails before filesystem mutation when a choice matters.
- Supplying both flags is rejected by Clap.
- Dry-run exposes the offer without prompting.
- The offer copy is asserted to contain no `git` token.

### Existing born repository

- No nested repository is created.
- Repository metadata is snapshotted recursively before and after init.
- Repository config bytes remain unchanged.
- Global config bytes remain unchanged.
- `HEAD` remains unchanged.

### Existing unborn repository

- Decline leaves `HEAD` unresolved.
- Decline preserves config and index bytes.
- Acceptance creates the first commit.
- Acceptance preserves config and index bytes.
- The initial tree excludes already-staged operator work.
- The ordinary index still contains the operator's staged entry.
- The initial commit carries the dedicated Lisa identity.

## Verification log

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo test -p lisa-cli init::tests`: 74 passed.
- `cargo test -p lisa-cli --test help_surface`: 6 passed.
- `cargo test -p lisa-cli --test init_history -- --nocapture`: 5 passed.
- Focused acceptance fixtures passed after compare-and-swap hardening.
- `runtime::tests::second_managed_resolution_performs_zero_network_calls`: passed on focused rerun.
- Clean isolated `cargo test --workspace`: 1,049 passed, 0 failed, 1 environment-gated test ignored.
- Workspace doc tests passed.

## Verification deviations

The first workspace run reused a Cargo integration-test artifact whose embedded
`CARGO_MANIFEST_DIR` pointed at an older temporary checkout. That stale harness
did not contain the already-present `--no-history` update and failed before the
ticket behavior ran. A direct current-tree harness run passed. The workspace was
then rebuilt under a fresh isolated target directory, removing the stale path.

During the first isolated workspace run, the unrelated managed-runtime cache
test received a transient macOS `EINVAL` while storing a local HTTP response.
The exact test passed on immediate focused rerun, and the subsequent complete
isolated workspace run passed.

During final review, unborn-branch creation was tightened from an unconditional
`update-ref` to a missing-ref compare-and-swap. The expected zero object ID is
derived from the generated commit ID length, retaining compatibility with the
repository's object format. Existing acceptance fixtures cover the successful
path.

The implementation plan originally named four feature files. Full-suite audit
showed four scripted fixtures that intentionally invoke init non-interactively.
Those callers were updated to choose journal-only explicitly, matching the new
contract, and committed as a separate compatibility unit.

## Isolated commits

- `10f4d03a12a7b098a43b811da81e57732c5722a3` — Add project history offer to init.
- `86a4cb5c00195827320aecd102544e1ada448b6c` — Make scripted init history choice explicit.

## Final ownership check

- The four feature paths are clean.
- The four scripted-caller paths are clean.
- The ordinary index is empty.
- No ticket-owned source path remains modified or untracked.
- Unrelated Lisa ledger, ticket-frontmatter, and other-attempt publication state remains untouched.

## Remaining work

- Review artifacts only; source implementation and verification are complete.
