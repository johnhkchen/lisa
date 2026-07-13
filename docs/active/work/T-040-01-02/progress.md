# Progress: Disposition parse model

## Status

Implementation is complete.
The ticket-owned source unit was committed through Lisa's isolated transaction,
all required tests pass, and no ticket-owned source path remains staged,
modified, or untracked.

## Completed work

Created `crates/lisa-core/src/disposition.rs`.
The module owns the complete file-to-domain interpretation for the
machine-readable Review disposition introduced by T-040-01-01.

Added the public domain enum:

```rust
ReviewDisposition::Pass
ReviewDisposition::Block { reason }
ReviewDisposition::Invalid { reason }
```

The enum has no default implementation.
Only explicit validation can construct a passing result.

Added the public parser:

```rust
parse_review_disposition(path: impl AsRef<Path>) -> ReviewDisposition
```

The parser reads UTF-8 file contents, parses JSON, validates required fields,
and returns one exhaustive domain outcome.

Added `pub mod disposition;` to `crates/lisa-core/src/lib.rs` so downstream
scheduler code can consume the model in T-040-01-03.

## Implemented validity rules

The parser returns `Pass` only when all of these conditions hold:

- the file is readable UTF-8;
- the root is a JSON object;
- `disposition` is present and equals the string `"pass"`;
- `reason` is present and is JSON null.

The parser returns `Block { reason }` only when:

- the file is readable UTF-8;
- the root is a JSON object;
- `disposition` is present and equals the string `"block"`;
- `reason` is present and is a JSON string;
- the reason contains non-whitespace content.

The original block reason is preserved in the typed outcome.
Whitespace is used only to decide whether the reason is actionable.

Every other observation returns `Invalid { reason }`.
This includes missing/unreadable files, malformed JSON, wrong root types,
missing required fields, wrong field types, unknown disposition strings, and
contradictory disposition/reason relationships.

## Test coverage added

Nine module-local unit tests now cover:

1. canonical pass;
2. canonical block with reason preservation;
3. missing file;
4. malformed JSON;
5. block with absent reason;
6. block with null reason;
7. block with an empty reason;
8. block with a whitespace-only reason;
9. pass with a non-null block reason;
10. pass with a missing reason;
11. unknown disposition;
12. non-object document.

The block contradiction representations share one named test because they are
the same domain invariant.

Every negative case calls a helper that specifically requires
`ReviewDisposition::Invalid { .. }`.
The assertions therefore fail for both positive variants and pin the ticket's
“never Pass” requirement directly.

## Verification performed

Focused disposition suite:

```text
cargo test -p lisa-core disposition
```

Result: 9 passed, 0 failed.

Complete core suite:

```text
cargo test -p lisa-core
```

Result: 169 passed, 0 failed; doc tests passed.

Formatting:

```text
cargo fmt --all --check
```

Result: passed.

Workspace suite:

```text
cargo test --workspace
```

Result: passed across `lisa-cli`, `lisa-core`, and `lisa-plugin`, including 276
CLI tests, 169 core tests, and 333 plugin tests; doc tests passed.

Diff hygiene:

```text
git diff --check -- crates/lisa-core/src/lib.rs crates/lisa-core/src/disposition.rs
```

Result: passed before commit.

## Commit transaction

Executed:

```text
lisa commit-ticket \
  --ticket-id T-040-01-02 \
  --message "Add fail-closed review disposition parser" \
  --include crates/lisa-core/src/disposition.rs \
  --include crates/lisa-core/src/lib.rs
```

Created commit:

```text
150b2e12e4dd9040bc3782bd0dc524b71109aa25
```

Inspection with `git show` confirmed the commit contains exactly the two listed
source paths: 174 insertions, with no unrelated path.

Post-commit scoped cached and worktree diffs for both owned paths are empty.
The remaining repository status entries are Lisa-managed lifecycle/provenance
changes and concurrent ticket work; they were preserved and excluded.

## Deviations from plan

No architectural or ownership deviation was required.

The plan described a possible private validation helper; implementation used
that helper as planned.
Extra fields are tolerated because the predecessor contract requires the two
canonical fields and relationships but does not declare a closed schema.

## Remaining work

Only Review artifacts remain for this attempt.
Scheduler gating is intentionally not part of this ticket and remains assigned
to dependent ticket T-040-01-03.

