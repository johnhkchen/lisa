# Structure: T-035-03-01 Codex trust path canonicalization

## Change summary

The implementation is one source-unit change in the Lisa CLI's Codex trust
module.

No new production module, dependency, command-line option, configuration field,
or public API is introduced.

The existing trust pregrant call graph remains intact:

`loop_cmd::run_loop` or `doctor::run_doctor`

→ `doctor::pregrant_codex_trust`

→ `doctor::pregrant_codex_trust_in`

→ `$CODEX_HOME/config.toml`

Only the project identity used by the final helper changes.

## File modified

### `crates/lisa-cli/src/doctor.rs`

This file continues to own:

- Codex home discovery;
- Codex project-trust table generation;
- preservation and idempotence behavior;
- doctor reporting;
- trust-pregrant unit tests.

The file gains no new external imports or crate dependencies.

`Path` and `PathBuf` are already imported at module scope.

## Production function boundary

The existing signature stays unchanged:

```rust
pub(crate) fn pregrant_codex_trust_in(
    codex_home: &Path,
    work_tree: &Path,
) -> bool
```

The function will derive a local normalized project path before constructing
the table header.

Conceptual organization:

```text
canonical_work_tree = canonicalize(work_tree)
    or fallback to work_tree

header = projects table for canonical_work_tree
existing = read config or empty

if exact header exists:
    return true

append trusted entry
ensure Codex home exists
write config
return write success
```

The canonical path is used for both existing-header lookup and appended-header
construction.

This prevents a second corrected invocation from appending another canonical
entry.

## Path ownership and lifetimes

Canonicalization returns an owned `PathBuf` on success.

Fallback also produces an owned `PathBuf` from the borrowed input.

Using one owned local value keeps header formatting simple and avoids a
conditional borrowed/owned abstraction for a preflight-only code path.

No canonical path is persisted outside the generated TOML string.

The wrapper `pregrant_codex_trust` continues returning only the config path on
success.

## Documentation update

The Rust documentation immediately above `pregrant_codex_trust_in` will state:

- existing work trees are canonicalized before table generation;
- this aligns the trust key with Codex's resolved cwd;
- the observed macOS `/var` to `/private/var` case is covered;
- canonicalization failure falls back to the input under the existing
  best-effort policy.

The existing notes about user-level configuration, idempotence, issue #14345,
and bypass behavior remain.

## Test added

The `#[cfg(test)] mod tests` block in `doctor.rs` gains one Unix-gated test.

Proposed name:

`test_pregrant_codex_trust_matches_canonicalized_cwd`

Fixture layout:

```text
temp root/
  project/             real existing directory
  project-link -> project
  codex-home/          created by pregrant
```

The temporary root and Codex configuration storage may use distinct
`tempfile::tempdir` instances or siblings under one root; either shape remains
fully isolated.

The test imports `std::os::unix::fs::symlink` inside the test or under a
Unix-only import so non-Unix compilation does not reference the API.

## Test assertions

The regression establishes that it exercises a true alias:

```text
alias path != canonicalized alias path
```

It calls the existing low-level pregrant helper using the alias.

It computes the expected Codex cwd identity with filesystem canonicalization.

It constructs the exact expected TOML entry using that canonical value.

It asserts the config contains:

```toml
[projects."<canonical cwd>"]
trust_level = "trusted"
```

It also asserts the alias-form project table is absent when the alias and
canonical strings differ.

This negative assertion ensures the implementation did not merely add both
spellings.

## Existing tests retained

The following tests remain structurally unchanged:

- `test_pregrant_codex_trust_writes_block`;
- `test_pregrant_codex_trust_is_idempotent`;
- `test_pregrant_codex_trust_preserves_existing`;
- `test_codex_home_honors_env`.

Their nonexistent `/work/tree` input exercises canonicalization fallback.

Together, old and new tests cover both branches:

- unresolvable path → preserve supplied identity;
- existing symlink path → use canonical identity.

## Files not modified

### `crates/lisa-cli/src/main.rs`

`resolve_path` continues to preserve absolute path spellings and join relative
paths to the current directory.

This avoids affecting init and unrelated subcommands.

### `crates/lisa-cli/src/loop_cmd.rs`

The loop continues to call `pregrant_codex_trust(root)` at the same preflight
point.

It receives corrected behavior through the shared helper.

### `docs/knowledge/codex-day-runbook.md`

No edit in this ticket.

The next ticket owns the committed harness/runbook extension and its live
provider evidence.

### Scheduler and provider modules

No scheduler, adapter, acknowledgement, lease, pane, or process-start source is
within scope.

## Artifact structure

Attempt-private workflow documents are written to:

```text
.lisa/attempts/T-035-03-01/1/work/
  research.md
  design.md
  structure.md
  plan.md
  progress.md
  review.md
```

Lisa, not this implementation, publishes admitted artifacts to the shared work
directory.

## Commit structure

The source change is atomic because the implementation and its directly coupled
regression live in the same file.

After focused and full CLI verification, commit exactly:

```text
crates/lisa-cli/src/doctor.rs
```

using `lisa commit-ticket` for ticket `T-035-03-01`.

No artifact path is included in that source transaction; Lisa owns final
workflow-artifact publication.

## Verification boundaries

Focused verification runs the new test by exact filter.

Module-level verification runs all `lisa-cli` tests.

Workspace hygiene verification includes formatting and diff whitespace checks.

No live Codex invocation is permitted or required.

The acceptance proof is the exact equality asserted between the generated
project header and the fixture's canonicalized cwd.

## Ordering constraints

1. Add the regression and production normalization together.
2. Format the repository.
3. Run the focused canonical-cwd test.
4. Run the complete CLI test suite.
5. Inspect the source diff and ordinary index.
6. Commit the one source path through Lisa's isolated transaction.
7. Recheck ticket-owned source cleanliness.
8. Complete progress and review artifacts.

The test must pass before the source transaction is created.
