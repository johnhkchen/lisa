# Review: one-source configuration table

## Disposition recommendation

Pass. The implementation satisfies both acceptance criteria, all ticket-owned
source is committed through Lisa's isolated transaction, and targeted through
workspace-wide verification passes.

## Change summary

This ticket closes the remaining documentation edge around the configuration
catalog introduced by T-050-02-01.

Before this change:

- `CONFIG_KEYS` covered all fixed `.lisa.toml` settings;
- config validation used that catalog;
- fresh config generation used catalog defaults and descriptions;
- init upserts used the catalog; but
- README maintained an independent, incomplete eight-row configuration table.

After this change:

- README contains all 17 current catalog entries;
- README defaults are checked against catalog defaults;
- README descriptions are checked exactly against catalog descriptions;
- missing descriptions produce a named build-test diagnostic;
- duplicate and stale extra README keys are rejected; and
- the existing parsed-key coverage test composes with the new README test to cover
  the whole parsed-config-to-documentation chain.

## Files modified

### `crates/lisa-cli/src/config.rs`

All additions are inside the existing inline `#[cfg(test)]` module.

Added a compile-time README input:

- `README` uses `include_str!("../../../README.md")`.
- Tests do not depend on the process working directory.
- README changes invalidate and rebuild the relevant test target.
- No production runtime reads or I/O were added.

Added a parsed-row representation:

- `ReadmeConfigRow` owns the displayed default and description.
- The dotted path remains the key in a `BTreeMap`.

Added Markdown-table parsing:

- `strip_code_ticks` normalizes the existing backtick presentation.
- `parse_readme_config_table` finds the exact Key/Default/Description table.
- It requires the expected separator line.
- It reads consecutive pipe-prefixed data rows.
- It requires exactly three cells.
- It rejects malformed rows.
- It rejects empty keys.
- It rejects duplicate keys.

Added catalog comparison:

- `readme_default` removes outer TOML quotes for README string presentation.
- Boolean, integer, and inline-table defaults remain unchanged.
- `verify_readme_config_table` accepts any `&[ConfigKey]` and README input.
- It checks every catalog path for a matching README row.
- It treats an absent row as a missing description and names the path.
- It treats an empty description cell the same way.
- It checks the displayed default exactly.
- It checks description prose exactly, including punctuation.
- It rejects README rows not present in the supplied catalog.

Added two tests:

- `readme_config_table_matches_catalog` checks the real catalog against the root
  README.
- `readme_config_table_names_missing_fake_description` appends a fake catalog
  entry and checks the failure diagnostic.

### `README.md`

The Configuration table changed from eight rows to the complete 17-row catalog.

Added previously absent rows:

- `version`;
- `runtime.zellij`;
- `scheduling.auto_advance`;
- `scheduling.review_timeout_secs`;
- `scheduling.session_timeout_secs`;
- `scheduling.wind_down_secs`;
- `scheduling.assignment_ack_timeout_secs`;
- `scheduling.phase_timeouts`; and
- `scheduling.provider_caps`.

Retained and synchronized rows:

- `dirs.tickets`;
- `dirs.stories`;
- `dirs.work`;
- `agent.client`;
- `guards.completion`;
- `triage.enabled`;
- `triage.timeout_secs`; and
- `scheduling.max_threads`.

All descriptions now match `ConfigKey::description` exactly. Table order matches
catalog render order. String defaults retain README's established unquoted display
inside Markdown code ticks.

## Commit review

- Commit: `113c934d380a9c2e86d9f4c0fb15d1bf44b73d31`.
- Message: `Bind README configuration to the catalog`.
- Created with `lisa commit-ticket`.
- Exact included source paths:
  - `crates/lisa-cli/src/config.rs`
  - `README.md`
- No ordinary `git add` was used.
- No ordinary `git commit` was used.
- Both owned paths are clean after the transaction.
- The commit contains no phase artifact and no unrelated concurrent file.

## Acceptance criterion 1

> A sync test binds the README configuration table to the description table: any
> parsed config key missing from the README, any default mismatch, or any missing
> description fails the test; the current full key set passes.

### Evidence

Parsed fixed key to catalog:

- Existing `config_catalog_covers_every_parsed_key_exactly_once` parses the complete
  fixed-key fixture and compares its dotted path set with `CONFIG_KEYS`.
- The same test rejects duplicate catalog paths and section/key pairs.

Catalog to README:

- New `readme_config_table_matches_catalog` verifies the complete real catalog.
- Missing rows return a named missing-description error.
- Empty descriptions return a named missing-description error.
- Display defaults are derived from catalog TOML defaults and compared exactly.
- Present descriptions are compared exactly with catalog descriptions.
- Extra README rows are rejected.

Composition:

- A parsed fixed key missing from the catalog fails the existing coverage test.
- A cataloged fixed key missing from README fails the new sync test.
- A default mismatch fails the new sync test.
- A missing or independently changed description fails the new sync test.
- The current 17-record set passes both tests.

Assessment: satisfied.

## Acceptance criterion 2

> Adding a fake key in a test (or an equivalent negative fixture) demonstrates the
> failure mode with an error message naming the missing description; README table
> regenerated/verified for all current keys including guards/triage.

### Evidence

- `readme_config_table_names_missing_fake_description` copies the real catalog.
- It appends `fake.missing_description` as a valid-shaped `ConfigKey`.
- README intentionally has no matching row.
- The verifier returns an error rather than accepting the incomplete rendering.
- The test asserts that the error contains `missing description`.
- The test asserts that the error contains `fake.missing_description`.
- The real README positive test checks all catalog entries.
- The table includes `guards.completion`.
- The table includes `triage.enabled`.
- The table includes `triage.timeout_secs`.
- Those three entries' defaults and descriptions are catalog-exact.

Assessment: satisfied.

## Test coverage

### Baseline catalog coverage

Command:

```text
cargo test -p lisa-cli config::tests::config_catalog
```

Result:

- 3 passed.
- 0 failed.

This proves the pre-existing parser-to-catalog checks remained green.

### New focused coverage

Command:

```text
cargo test -p lisa-cli config::tests::readme_config_table
```

Result:

- 2 passed.
- 0 failed.

This directly covers the real README sync and fake missing-description path.

### Config module coverage

Command:

```text
cargo test -p lisa-cli config::tests
```

Result:

- 67 passed.
- 0 failed.

This covers catalog integrity, config parsing, validation, resolution, generation,
and README synchronization together.

### CLI package coverage

Command:

```text
cargo test -p lisa-cli
```

Result:

- All package unit, integration, and doc tests passed.
- The declared real-Zellij environment-gated test remained ignored.
- No failures occurred.

### Workspace coverage

Command:

```text
cargo test --workspace
```

Result:

- All workspace unit, integration, and doc tests passed.
- No failures occurred.

### Formatting and diff hygiene

Commands:

```text
cargo fmt --all -- --check
git diff --check -- README.md crates/lisa-cli/src/config.rs
```

Result:

- Both checks passed.

## Review of failure diagnostics

The most important diagnostic is intentionally evaluated before count or default
comparisons. A new catalog key absent from README produces:

```text
README configuration table is missing description for `fake.missing_description`
```

Default drift names the path and both actual and expected values. Description drift
names the path and both actual and expected strings. Duplicate and unknown table
keys also name the affected path.

These messages make the CI failure actionable without requiring a contributor to
reverse-engineer a full Markdown snapshot diff.

## Non-blocking limitations

### Simple Markdown parser

The test parser deliberately supports the repository's current three-column table,
not the full Markdown grammar. A literal unescaped pipe in a future description
would make the row malformed. Current catalog descriptions contain no pipes, and
the error is explicit. Extending escaping support can happen with the first use
case; it is not needed for current acceptance.

### Simple string-default presentation

`readme_default` removes one ordinary surrounding double-quote pair. Current fixed
string defaults are simple TOML strings, and the existing default-validity test
checks their TOML syntax. If a future catalog default uses multiline strings or
special presentation requirements, README normalization will need an explicit
extension. Current values are correctly covered.

### Version row maintenance

The `version` default is derived from `CARGO_PKG_VERSION`. A package-version bump
will therefore require updating README before tests pass. This is intentional: the
version setting is part of the fixed parsed-key catalog and the ticket requires
default drift to fail the build.

### Row order

The verifier keys rows by dotted path and does not independently reject reordering.
Current README follows catalog order. The acceptance criteria bind keys, defaults,
and descriptions rather than ordering, so this is not a gap.

## Open concerns

- No correctness blockers.
- No uncommitted ticket-owned source changes.
- No test gaps against the stated acceptance criteria.
- No migration or runtime compatibility concern because behavior is test-only and
  documentation-only.
- No new dependency or public API burden.

## Human review focus

A reviewer can concentrate on three questions:

1. Is treating TOML outer quotes as presentation-only correct for README defaults?
2. Are exact catalog descriptions the desired README table voice?
3. Is the simple known-shape Markdown parser preferable to a byte snapshot?

The implementation and ticket support “yes” for all three: it preserves existing
README default presentation, makes the catalog truly authoritative for prose, and
provides substantially clearer field-specific errors than a raw byte mismatch.

## Final assessment

The ticket's proliferation concern is now structurally enforced. A fixed config key
must appear in the catalog, the catalog metadata must render in init stubs, and the
same default and description must appear in README. A fake key demonstrates the
missing-description failure explicitly. Guards and triage are included. All tests
pass, the source commit is isolated and exact, and there are no open blockers.
