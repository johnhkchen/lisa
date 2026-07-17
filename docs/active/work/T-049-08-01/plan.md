# Plan: T-049-08-01 identity remedy that works

## 1. Establish shared remedy bytes

- Add canonical name and email command constants to `lisa-core::completion`.
- Add the canonical combined history/identity ask there.
- Replace the plugin-local constant with the core import.
- Run the plugin's focused completion failure tests.
- Commit the core and plugin files as one meaningful unit.

## 2. Persist identity on accepted unborn history

- Extract the existing `git config --local` operations into a helper.
- Reuse it from missing-repository initialization.
- Call it for `CreateInitialCommit` in an existing unborn repository.
- Keep root creation via the existing empty-tree/commit-tree path.
- Do not touch born, decline, or dry-run branches.
- Update accepted-unborn fixture assertions.
- Preserve ordinary-index and declined snapshots.
- Add local identity, completion commit, and doctor-silence assertions.
- Run init-history integration tests.
- Commit init source and test together.

## 3. Implement the per-variant remedy table

- Define exact repository, identity, optional-init, and transaction remedy text.
- Extend native probe outcome with the identity init-cure context.
- When identity is absent, probe whether `HEAD` is absent in the discovered repository.
- Treat an absent `HEAD` as the contextual condition where accepted init is valid.
- Keep the typed unavailable reason unchanged.
- Route explicit commit errors through the table.
- Expose the same selected remedy to doctor.
- Generalize doctor auto-mode report rendering to all retained reasons.

## 4. Pin remedy behavior

- Add unit cases for `RepositoryMissing`.
- Add unit cases for born-context `IdentityMissing`.
- Add unit cases for unborn-context `IdentityMissing`.
- Add unit cases for `TransactionUnavailable`.
- For each, assert required text and exclusion of unrelated remedies.
- Assert CLI identity command bytes come from core constants.
- Update seal visibility fixture constants and expectations.
- Add a born identityless doctor subprocess fixture.
- Assert the born output omits the init alternative verbatim.

## 5. Prove cure paths

- Use existing bare-folder init coverage as repository-missing cure evidence.
- Use accepted existing-unborn init as the contextual init cure.
- Use real local `git config` commands for a born identityless repository.
- After each concrete cure, make a completion-style commit.
- Assert doctor no longer reports the original identity gap.
- Retain byte snapshots proving born init does not write anything.

## 6. Verify source quality

- Run `cargo fmt --all -- --check` after formatting.
- Run focused `lisa-core` completion tests.
- Run focused CLI unit tests for completion seal and init.
- Run `cargo test -p lisa-cli --test init_history`.
- Run `cargo test -p lisa-cli --test seal_visibility`.
- Run focused plugin tests that pin the shared ask and field replay.
- Run `cargo test --workspace` if focused tests pass within the environment.
- Run clippy/check only if required by failures or feasible after the full test run.

## 7. Commit final meaningful unit

- Commit completion-seal, doctor, and seal-visibility changes with exact include paths.
- Use only `lisa commit-ticket`.
- Inspect `git status --short` after every commit.
- Ensure no ticket-owned source is staged, modified, or untracked at Review.
- Preserve unrelated Lisa journal/ticket changes.

## 8. Record implementation

- Create `progress.md` in the private attempt work directory.
- Record completed units, tests, and any plan deviations.
- Record exact commit identifiers returned by Lisa where available.
- Continue immediately into Review.

## 9. Review

- Inspect the committed diff and status.
- Summarize files and behavior in `review.md`.
- Map each acceptance criterion to test evidence.
- Identify any remaining limitation honestly.
- Write exact pass/block disposition JSON.
- Run `lisa check-disposition T-049-08-01`.
- Correct every reported disposition issue.
- Remain on this ticket after Review.
