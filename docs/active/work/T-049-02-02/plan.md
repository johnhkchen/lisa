# Plan — T-049-02-02 doctor identity preflight

## Step 1 — Establish source ownership

Confirm current worktree status before editing.

Treat existing changes in `.lisa` metadata, ticket frontmatter, `init.rs`,
`main.rs`, and T-049-02-01 work artifacts as pre-existing and out of scope.

Own only completion-seal, doctor, and seal-visibility test changes.

Do not use the ordinary Git index.

## Step 2 — Centralize identity guidance

In `completion_seal.rs`, replace the existing partial private remedy with one
crate-visible complete block.

Keep the two existing `git config` command spellings.

Add the missing-identity sentence.

Add the `lisa init` history-offer alternative.

Use line breaks that render the two remedies side by side in one diagnosis.

Update `format_preflight_failure` to include the block verbatim.

Verify no duplicate production literals remain with `rg`.

## Step 3 — Expose the retained fallback reason

Add `RunCompletionSeal::commit_unavailable`.

Delegate directly to the wrapped core resolution.

Keep the return borrowed and typed.

Do not expose mutable state or internal probe structures.

Extend the auto fallback unit test to use the accessor.

## Step 4 — Make doctor validate the configured seal

Replace doctor's tier-only inspection call with `resolve_for_run`.

Do not use `?`; retain failures so the whole doctor report still renders.

Pass the result into completion formatting.

Keep all dependency and project checks unchanged.

## Step 5 — Format completion diagnosis

For successful commit resolution, print only the existing commit seal line.

For explicit journal, print only the existing journal seal line.

For auto repository-missing fallback, print only the journal seal line.

For auto identity-missing fallback, print the journal line plus a reason using
the shared guidance block.

For explicit commit failure, print the named hard failure returned by the seal
resolver.

Add completion failure to doctor's final failure condition.

## Step 6 — Update focused unit tests

Adapt the existing doctor completion formatting test to the revised helper
shape or replace redundant coverage with result-aware cases.

Pin the complete identity guidance constant in `completion_seal.rs`.

Assert explicit commit includes the exact block intact.

Preserve existing one-probe and zero-probe assertions.

## Step 7 — Extend CLI fixtures

Generalize `seal_visibility.rs` fixture setup with repository variants.

Use temporary home and disable system Git config for deterministic results.

Initialize a repository without identity for the negative auto case.

Initialize and locally configure identity for the positive auto case.

Leave the directory repo-less for the deferral case.

Add explicit commit missing-identity coverage.

Assert exact seal lines, process statuses, and verbatim guidance.

Assert guidance absence in configured-identity and no-repository cases.

## Step 8 — Focused verification

Run `cargo fmt --all -- --check` after formatting.

Run the completion-seal and doctor library tests through `cargo test -p lisa-cli`.

Run the `seal_visibility` integration test directly for fixture diagnostics.

If a test fails because another ticket's in-flight `main.rs`/`init.rs` API is
temporarily inconsistent, inspect and coordinate without overwriting that work.

## Step 9 — Inspect exact diff

Use `git diff --` only for the three owned source paths.

Confirm no accidental edit to neighboring ticket work.

Use `rg` to confirm each guidance literal appears once in production source.

Confirm the ordinary Git index has no ticket-owned entries.

## Step 10 — Commit meaningful source unit

Commit the cohesive behavior and its fixtures with:

`lisa commit-ticket --ticket-id T-049-02-02 --message "Diagnose missing commit identity in doctor" --include crates/lisa-cli/src/completion_seal.rs --include crates/lisa-cli/src/doctor.rs --include crates/lisa-cli/tests/seal_visibility.rs`

If implementation naturally separates into independent verified units, use two
exact-path Lisa commits instead; never split code from the tests that establish
its contract without a reason.

Do not use `git add`, `git commit`, or a broad include.

## Step 11 — Full verification

Run `cargo test -p lisa-cli`.

Run `cargo test --workspace`.

Run `cargo fmt --all -- --check` once more.

Use `just check` only if it adds WASM-target coverage without disturbing
neighboring work.

Record exact results and any unrelated failures.

## Step 12 — Progress artifact

Write `progress.md` in the private attempt work directory.

Record source files, behavior, tests, commit identifier, and ownership check.

Document any deviation before or alongside the changed implementation course.

Confirm no owned source remains modified, staged, or untracked.

## Step 13 — Review artifact

Review each acceptance criterion against compiled fixture evidence.

Confirm repo without identity fires with both remedies.

Confirm repo with identity is silent apart from its commit seal.

Confirm no repo falls back to only the journal seal line.

Confirm explicit commit produces the shared hard-failure text.

Confirm production guidance literals have one source.

Write `review.md` with changes, coverage, ownership, and open concerns.

Write `review-disposition.json` exactly as pass only if implementation is
committed and verification passes.

Remain on T-049-02-02 after both Review artifacts exist.

