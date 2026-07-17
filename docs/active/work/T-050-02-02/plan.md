# Plan: one-source configuration table

## Implementation strategy

Implement the README edge as one atomic source unit: test helpers and tests in
`config.rs`, plus the synchronized 17-row README table. Verify incrementally before
committing both paths through Lisa's isolated ticket transaction.

## Step 1: establish pre-change ownership and baseline

### Actions

- Confirm `README.md` is not modified or staged.
- Confirm `crates/lisa-cli/src/config.rs` is not modified or staged.
- Note unrelated dirty paths without changing them.
- Run the existing focused config catalog tests if practical.

### Verification

- `git status --short -- README.md crates/lisa-cli/src/config.rs`
- `cargo test -p lisa-cli config::tests::config_catalog`

### Completion condition

- Both intended source paths are clean before editing.
- Existing catalog coverage/default/description tests pass.

## Step 2: add README table parsing support

### Actions

- Import `BTreeMap` alongside the existing `BTreeSet` in the test module.
- Embed root `README.md` with `include_str!`.
- Add `ReadmeConfigRow`.
- Add a helper to remove Markdown code ticks.
- Add a parser anchored on the exact three-column configuration header.
- Make the parser reject malformed and duplicate rows with readable errors.

### Verification

- Compile the test module with `cargo test -p lisa-cli config::tests --no-run`.
- Review parser code for no runtime/non-test exposure.

### Completion condition

- The parser returns a map keyed by README dotted path.
- Missing header, malformed row, empty key, and duplicate key are error paths.

## Step 3: add catalog-to-README verification

### Actions

- Add README display normalization for catalog defaults.
- Add `verify_readme_config_table` over an arbitrary catalog slice and README text.
- Check missing or empty descriptions before field comparisons.
- Compare normalized default exactly.
- Compare description exactly.
- Reject README rows absent from the supplied catalog.

### Verification

- Add or temporarily exercise helper calls through tests.
- Inspect error strings for named dotted paths.

### Completion condition

- Every key/default/description mismatch becomes `Err(String)`.
- A catalog key absent from README reports `missing description` and its path.

## Step 4: add positive and negative tests

### Positive test actions

- Call the verifier with `CONFIG_KEYS` and embedded `README`.
- Panic with the verifier diagnostic on failure.

### Negative test actions

- Clone the real catalog into a vector.
- Append a fake `ConfigKey` named `fake.missing_description`.
- Verify the helper returns an error.
- Assert that error contains `missing description`.
- Assert that error contains `fake.missing_description`.

### Verification

- Run the focused test filter.
- Expect the positive test to fail before README synchronization.
- Confirm the failure points at the first missing catalog row.
- Confirm the negative fixture itself passes by observing the helper error.

### Completion condition

- The negative fixture proves the requested contributor-facing failure mode.
- The only remaining positive failure is actual README drift.

## Step 5: synchronize README

### Actions

- Retain the existing table header and separator.
- Replace the eight data rows with 17 rows.
- Follow `CONFIG_KEYS` order.
- Include `version`.
- Include `runtime.zellij`.
- Include all scheduling settings.
- Include `guards.completion`.
- Include both triage settings.
- Remove TOML quote delimiters from displayed string defaults.
- Copy descriptions exactly, including final periods.

### Verification

- Run `cargo test -p lisa-cli config::tests::readme_config_table`.
- Count README table rows if useful.
- Compare guards and triage rows directly with catalog records.

### Completion condition

- Positive sync test passes for the current full key set.
- Negative fixture passes while proving its simulated sync check fails correctly.

## Step 6: format and inspect the diff

### Actions

- Run `cargo fmt --all` if formatting changes are needed.
- Inspect only ticket-owned diffs.
- Ensure helpers are test-only.
- Ensure no unrelated README prose changed.
- Ensure no production catalog descriptions changed merely to fit docs.

### Verification

- `cargo fmt --all -- --check`
- `git diff --check -- README.md crates/lisa-cli/src/config.rs`
- `git diff -- README.md crates/lisa-cli/src/config.rs`

### Completion condition

- Rust formatting passes.
- No whitespace errors exist.
- Diff is limited to the sync invariant and complete table.

## Step 7: run targeted and crate-level tests

### Actions

- Run the two new README tests.
- Run all config module tests.
- Run the entire `lisa-cli` package tests if the focused tests pass.

### Verification commands

```text
cargo test -p lisa-cli config::tests::readme_config_table
cargo test -p lisa-cli config::tests
cargo test -p lisa-cli
```

### Completion condition

- New tests pass.
- Existing config tests pass.
- CLI unit and integration tests pass.

## Step 8: run workspace verification

### Actions

- Run the workspace test suite.
- Record any failure and distinguish ticket-caused failures from environmental or
  concurrent failures.
- Do not modify unrelated ticket files to repair unrelated failures.

### Verification command

```text
cargo test --workspace
```

### Completion condition

- Workspace tests pass, or any non-ticket failure is documented precisely before
  Review disposition is chosen.

## Step 9: update progress artifact before commit

### Actions

- Record completed code and README changes.
- Record test results.
- Record deviations from this plan, if any.
- Record remaining commit/review tasks.

### Completion condition

- `progress.md` accurately reflects the source tree and verification state.

## Step 10: commit the meaningful source unit

### Actions

- Invoke Lisa's isolated commit transaction.
- Include exactly the two ticket-owned source paths.
- Do not use `git add`.
- Do not use ordinary `git commit`.

### Command

```text
lisa commit-ticket \
  --ticket-id T-050-02-02 \
  --message "Bind README configuration to the catalog" \
  --include crates/lisa-cli/src/config.rs \
  --include README.md
```

### Verification

- Inspect command output for the created commit.
- Inspect `git status --short -- README.md crates/lisa-cli/src/config.rs`.
- Inspect the committed diff if necessary.

### Completion condition

- Ticket source is durable in a Lisa-managed commit.
- Neither owned path remains staged, modified, or untracked.

## Step 11: Review phase

### Actions

- Write `review.md` in the private attempt work directory.
- Summarize modified files and behavioral invariant.
- List targeted, package, workspace, format, and diff checks.
- Assess parser limitations and open concerns.
- Write the exact pass disposition when ready:
  `{"disposition":"pass","reason":null}`.
- If an actual blocker remains, write a compliant actionable block disposition
  instead of passing.
- Run `lisa check-disposition T-050-02-02`.
- Correct every disposition issue.

### Completion condition

- Both Review artifacts exist.
- Disposition validation passes.
- No ticket-owned source changes remain outside the Lisa commit.
- The ticket frontmatter has not been manually edited.

## Atomicity rationale

The Rust test and README update form one meaningful unit. Committing the test alone
would intentionally fail against the old incomplete table. Committing README alone
would improve documentation but leave no enforced invariant. One Lisa transaction
with both exact paths gives the repository a green, self-enforcing state.

## Test coverage matrix

| Requirement | Evidence |
|---|---|
| Parsed fixed key missing from catalog fails | Existing `config_catalog_covers_every_parsed_key_exactly_once` |
| Catalog key missing from README fails | New positive verifier test |
| README default mismatch fails | New verifier default comparison |
| README description missing fails | New verifier existence/empty check |
| README description drift fails | New exact description comparison |
| Fake key names missing description | New negative fixture test |
| Current guards key passes | Updated README plus positive test |
| Current triage keys pass | Updated README plus positive test |
| Init stubs remain catalog-driven | Existing default/init rendering tests |

## Risk controls

- Keep all new helper code under `#[cfg(test)]`.
- Use compile-time README inclusion to avoid working-directory sensitivity.
- Avoid broad Markdown parsing beyond the known table shape.
- Reject duplicate and extra rows so stale documentation cannot hide.
- Preserve unrelated dirty work in the shared tree.
- Use exact `--include` paths in the isolated commit.
- Do not update ticket phase or status.
