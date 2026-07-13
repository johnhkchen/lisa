# Plan: Disposition parse model

## Step 1: establish the public module

Create `crates/lisa-core/src/disposition.rs` with module documentation and the
`ReviewDisposition` enum.
Add `pub mod disposition;` to `crates/lisa-core/src/lib.rs`.

Verification: run a core crate check after the parser skeleton is present so
the module registry and public types compile together.

## Step 2: implement file-to-domain parsing

Add `parse_review_disposition(path: impl AsRef<Path>)`.
Read the file with `fs::read_to_string`.
Convert every read error to `ReviewDisposition::Invalid` with a diagnostic
reason that includes path context.

Parse the contents with `serde_json::from_str::<Value>`.
Convert every syntax/type parse failure to `Invalid`.
Do not introduce a success default or a `Result` wrapper at the public boundary.

Verification: compile the core crate and inspect the exhaustive construction
sites for `Pass`.

## Step 3: implement relationship validation

Require an object root and both `disposition` and `reason` keys.
Require a string disposition.

Implement the validity matrix:

- `pass` plus null -> `Pass`;
- `block` plus a non-blank string -> `Block`, preserving the reason;
- every other value or relationship -> `Invalid`.

Give missing keys, unknown disposition, wrong reason types, and contradictory
relationships descriptive invalid reasons.

Verification: search the module to ensure `Pass` is constructed in exactly the
canonical pass/null branch.

## Step 4: add required positive tests

Add a test helper using a temporary directory and the canonical filename.
Write and parse the canonical pass document; assert exact `Pass` equality.
Write and parse a canonical block document; assert exact block equality and
reason preservation.

Verification:

```text
cargo test -p lisa-core disposition::tests::parses
```

## Step 5: add required invalid-input tests

Parse a nonexistent path and require `Invalid`.
Write malformed JSON and require `Invalid`.

Exercise block contradictions with missing, null, empty, and whitespace-only
reasons; require `Invalid` for every case.
Write a pass with a non-null block reason and require `Invalid`.

Add low-cost coverage for a missing pass reason, unknown disposition, and
non-object root to pin the surrounding strictness.

Verification:

```text
cargo test -p lisa-core disposition
```

The negative-test helper must reject both `Pass` and `Block`, proving all bad
documents resolve specifically to the non-passing variant.

## Step 6: format and verify the source unit

Run the formatter on the workspace.
Run:

```text
cargo fmt --check
cargo test -p lisa-core disposition
cargo test -p lisa-core
cargo test --workspace
```

If workspace tests expose an unrelated concurrent failure, distinguish it from
the focused core result and document it in progress/review rather than changing
another ticket's files.

## Step 7: inspect ownership and diff

Review the exact diff for `disposition.rs` and `lib.rs`.
Check `git status --short` and preserve all pre-existing Lisa-managed and
other-ticket changes.
Confirm no ticket-owned file is staged in the ordinary index.

Verification criteria:

- only the two planned source paths are ticket-owned;
- no manifest was changed;
- every acceptance case has a named test;
- only the canonical branch can return `Pass`.

## Step 8: commit the meaningful source unit

Use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-01-02 \
  --message "Add fail-closed review disposition parser" \
  --include crates/lisa-core/src/disposition.rs \
  --include crates/lisa-core/src/lib.rs
```

Do not use ordinary `git add` or `git commit`.
Afterward, verify the commit contains exactly the intended paths and neither
ticket-owned source file remains modified, staged, or untracked.

## Step 9: record implementation progress

Write `progress.md` in the private attempt work directory.
Record the implemented API, validation rules, tests, verification commands,
commit identity, and any deviations from this plan.

The artifact is not part of the source transaction; Lisa publishes admitted
phase artifacts later.

## Step 10: perform Review

Re-read the ticket acceptance criteria against the committed diff and test
output.
Write `review.md` with the change inventory, behavior, test evidence, ownership
check, gaps, and downstream considerations.

Write `review-disposition.json` as pass only if source ownership is clean and
all relevant tests pass. Otherwise write a block disposition with a non-empty,
actionable reason.

Remain on this ticket after both Review artifacts exist.
