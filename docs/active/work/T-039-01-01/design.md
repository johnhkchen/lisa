# Design: T-039-01-01

## Goal

Eliminate the 13 reproduced test-only Clippy warnings while preserving every test's
meaning and leaving non-test product code unchanged.

## Option 1: Apply Clippy's expression-level suggestions

- Replace `collection.contains(&"ID".to_string())` with
  `collection.contains("ID")` at the twelve reported membership assertions.
- Replace `fs::write(path, &format!(...))` with `fs::write(path, format!(...))`.
- Keep surrounding fixtures, APIs, assertion order, and test names unchanged.
- This removes only allocations or borrows that the called APIs do not require.
- It produces the smallest reviewable diff.
- It exactly follows the diagnostic explanations.
- It has no product-code effect because every site is inside a test module.

## Option 2: Refactor test identifiers to shared constants

- Define constants for common ticket IDs.
- Replace repeated literals and conversions across the entire DAG test module.
- This could reduce textual repetition beyond the warning sites.
- It would touch substantially more lines than the diagnostic baseline requires.
- Constants would not improve the behavioral coverage of these tests.
- A broad cleanup could obscure which edits correspond to the 13 warnings.
- It also risks changing unflagged calls whose APIs still expect owned identifiers.

## Option 3: Change DAG APIs to accept `&str`

- Generalize methods such as `can_start` or `get_dependencies` around borrowed IDs.
- This might eliminate additional test-side conversions.
- It changes production interfaces and therefore violates the explicit ticket boundary.
- It expands the work from lint cleanup into API design.
- It would require broader call-site analysis and potentially new tests.
- It is not justified by the findings, which are only on `contains` assertions.

## Option 4: Suppress the lints

- Add local `#[allow(clippy::unnecessary_to_owned)]` and
  `#[allow(clippy::needless_borrows_for_generic_args)]` attributes.
- This would make the command quiet without simplifying the code.
- It would retain unnecessary allocations and borrowing syntax.
- It would weaken the green lint baseline by encoding exceptions.
- It conflicts with the ticket's intent to clear debt rather than hide it.

## Option 5: Run automated `cargo clippy --fix`

- Clippy can mechanically apply these suggestions.
- The current worktree includes Lisa-managed ticket state.
- A broad automated fix can modify more sites or targets than the ticket owns.
- Exact ownership and a minimal diff matter under concurrent scheduling.
- Manual targeted edits are safer and equally straightforward for thirteen sites.

## Decision

Choose Option 1: apply the diagnostic's expression-level changes only.

## Rationale

- It is grounded in the reproduced output rather than a speculative cleanup.
- Each change removes an unnecessary temporary value.
- `String` collection membership supports borrowed string lookup at these sites.
- `fs::write` accepts the formatted `String` directly.
- Test results remain unchanged because compared bytes and string values are identical.
- The source diff remains limited to the two files reported by Clippy.
- The relevant lines are all test-only.
- No product source, public API, or dependency changes.
- The before/after warning count remains easy to audit.

## Detailed decisions

### Membership assertions

- Use string literals directly as the `contains` probe.
- Do not convert the collection to `&str` values.
- Do not alter fixture construction, which legitimately stores owned IDs.
- Do not change unrelated `.to_string()` expressions unless Clippy reported them.
- Preserve assertion ordering so failures retain the same debugging sequence.

### Temporary configuration content

- Pass the `String` returned from `format!` by value to `fs::write`.
- Preserve the exact TOML text.
- Preserve the current-version interpolation.
- Preserve all subsequent assertions on upserted configuration keys.

### Warning policy

- Verify the after state with `-D warnings` even though the reproducer omits it.
- Count warnings in the normal reproducer to record the explicit after value of zero.
- Treat any newly surfaced warning as a failure rather than an acceptable deviation.

### WASM interpretation

- Use the repository's established target-specific command:
  `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.
- The plugin is the WASM artifact; native CLI and test binaries are not WASM targets.
- This check complements rather than replaces the native workspace all-target run.

## Rejected scope

- No production DAG API cleanup.
- No global replacement of `.to_string()` in tests.
- No lint configuration changes.
- No CI or `justfile` changes.
- No new regression tests for syntax-only ownership changes.
- No ticket frontmatter edits.
- No shared work-artifact publication.

## Expected outcome

- Native all-target/all-feature warning count changes from 13 to 0.
- WASM Clippy emits zero warnings and exits successfully.
- Formatting remains unchanged or normalized.
- All native workspace tests pass.
- The meaningful source unit consists of two exact test-bearing source paths.
