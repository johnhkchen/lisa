# Progress: one-source configuration table

## Status

- Research: complete.
- Design: complete.
- Structure: complete.
- Plan: complete.
- Implement: source changes complete and verified.
- Lisa-managed source commit: complete.
- Review: pending at this checkpoint.

## Baseline confirmation

- Read `CLAUDE.md` before changing the repository.
- Read `docs/active/tickets/T-050-02-02.md`.
- Read `docs/knowledge/rdspi-workflow.md`.
- Read the full private assignment.
- Confirmed the ticket began in Research.
- Confirmed `README.md` was clean before editing.
- Confirmed `crates/lisa-cli/src/config.rs` was clean before editing.
- Observed unrelated staged and unstaged changes from concurrent Lisa work.
- Left all unrelated paths untouched.

## Research completed

- Mapped `ConfigKey` and the 17-entry `CONFIG_KEYS` catalog.
- Confirmed catalog fields cover path, section, local key, TOML default, and
  description.
- Confirmed config validation derives known fixed keys from the catalog.
- Confirmed fresh `.lisa.toml` rendering derives defaults and descriptions from
  the catalog.
- Confirmed init upsert logic consumes the catalog.
- Confirmed existing tests bind parsed fixed paths to the catalog.
- Confirmed README had only eight independently written configuration rows.
- Identified missing version, runtime, and scheduling rows.
- Identified divergent README description wording and punctuation.
- Identified `templates.rs` as the compile-time documentation-sync precedent.

## Design completed

- Evaluated a build-time generator.
- Evaluated byte-for-byte rendered-block comparison.
- Evaluated parsed README table validation.
- Evaluated a separate snapshot fixture.
- Selected parsed table validation, an approach explicitly permitted by the ticket.
- Kept the production catalog as canonical metadata.
- Defined README display normalization for quoted TOML string defaults.
- Defined exact description comparison.
- Defined named errors for absent descriptions and mismatched defaults.
- Defined rejection of duplicate and stale extra README rows.
- Defined a fake catalog entry as the negative fixture.

## Structure completed

- Limited source ownership to `crates/lisa-cli/src/config.rs` and `README.md`.
- Kept all new executable logic in the inline `#[cfg(test)]` module.
- Added no new dependency.
- Added no production public interface.
- Added no runtime README reads.
- Planned one atomic Lisa commit for test and synchronized documentation.

## Implementation completed

### `crates/lisa-cli/src/config.rs`

- Embedded repository-root README with `include_str!` under test configuration.
- Added `ReadmeConfigRow` for parsed default and description cells.
- Added `strip_code_ticks` for README key/default presentation.
- Added `parse_readme_config_table`.
- Anchored parsing on the existing exact Key/Default/Description header.
- Required the existing table separator.
- Parsed exactly three cells per row.
- Rejected malformed rows.
- Rejected empty paths.
- Rejected duplicate paths.
- Added `readme_default` to remove outer TOML quotes for human presentation.
- Added `verify_readme_config_table` over an arbitrary catalog slice.
- Missing rows report a missing description with the exact dotted path.
- Empty descriptions report a missing description with the exact dotted path.
- Default mismatches report expected and actual values with the exact dotted path.
- Description mismatches report expected and actual text with the exact dotted path.
- Extra README keys absent from the catalog are rejected.
- Added `readme_config_table_matches_catalog` for the real catalog and README.
- Added `readme_config_table_names_missing_fake_description`.
- The negative test appends `fake.missing_description` to a copy of the catalog.
- The negative test asserts both `missing description` and the fake dotted path.

### `README.md`

- Preserved the existing Configuration section and table header.
- Expanded the table from eight rows to 17 rows.
- Added `version` with current package default `0.4.4-rc.1`.
- Retained all three directory keys.
- Added `runtime.zellij`.
- Retained `agent.client`.
- Retained `guards.completion`.
- Retained `triage.enabled` and `triage.timeout_secs`.
- Added every scheduling key.
- Reordered rows into catalog render order.
- Replaced independent prose with exact catalog descriptions.
- Displayed string defaults without TOML quote delimiters.
- Displayed booleans, integers, and inline table defaults unchanged.

## Acceptance-criteria evidence

### Parsed key coverage

- Existing test `config_catalog_covers_every_parsed_key_exactly_once` proves the
  complete fixed parsed-key fixture and `CONFIG_KEYS` have identical path sets.
- New test `readme_config_table_matches_catalog` proves every catalog entry has a
  README row.
- The combination means a fixed parsed key cannot remain absent from README while
  the suite passes.

### Default mismatch

- The verifier derives expected README values from `ConfigKey::default`.
- String defaults lose only their outer TOML quotes.
- Every other literal is compared unchanged.
- The positive test checks all 17 defaults.

### Missing or drifted description

- Missing and empty descriptions return a path-specific error.
- Present descriptions are compared exactly with `ConfigKey::description`.
- Independent wording cannot silently diverge.

### Negative fixture

- `fake.missing_description` is absent from README by construction.
- Verification returns the simulated build failure.
- The test requires the diagnostic to name both the missing-description problem
  and fake path.

### Guards and triage

- README contains `guards.completion`.
- README contains `triage.enabled`.
- README contains `triage.timeout_secs`.
- Their defaults and exact descriptions pass the positive sync test.

## Verification completed

### Baseline catalog tests

Command:

```text
cargo test -p lisa-cli config::tests::config_catalog
```

Result:

- Passed: 3.
- Failed: 0.

### Formatting and whitespace

Commands:

```text
cargo fmt --all -- --check
git diff --check -- README.md crates/lisa-cli/src/config.rs
```

Result:

- Rust formatting passed.
- Diff whitespace check passed.

### New focused tests

Command:

```text
cargo test -p lisa-cli config::tests::readme_config_table
```

Result:

- Passed: 2.
- Failed: 0.
- Positive real-catalog sync passed.
- Negative fake-key diagnostic passed.

### Config module

Command:

```text
cargo test -p lisa-cli config::tests
```

Result:

- Passed: 67.
- Failed: 0.

### CLI package

Command:

```text
cargo test -p lisa-cli
```

Result:

- All unit, integration, and doc tests passed.
- The real-Zellij delivery test remained ignored by its declared environment gate.
- No failures.

### Workspace

Command:

```text
cargo test --workspace
```

Result:

- All workspace unit, integration, and doc tests passed.
- No failures.

## Diff review

- Ticket-owned diff contains two files.
- README change is limited to configuration table rows.
- Rust changes are limited to the config test module.
- No production behavior changed.
- No Cargo metadata or dependencies changed.
- No unrelated source changes were included.

## Deviations from plan

- None material.
- The test and README remained one atomic source unit as planned.
- The parser uses the exact established table header and separator as planned.
- The negative fixture uses the planned fake dotted path.

## Remaining Implement actions

- Source commit created through `lisa commit-ticket`.
- Commit: `113c934d380a9c2e86d9f4c0fb15d1bf44b73d31`.
- Commit message: `Bind README configuration to the catalog`.
- Exact includes: `crates/lisa-cli/src/config.rs` and `README.md`.
- No ordinary-index staging or ordinary commit command was used.
- Confirm both owned paths are clean afterward.
- Continue immediately into Review.
