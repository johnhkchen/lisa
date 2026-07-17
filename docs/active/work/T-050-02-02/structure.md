# Structure: one-source configuration table

## Change inventory

### Modified source files

1. `crates/lisa-cli/src/config.rs`
   - Extend the existing inline test module.
   - Add a compile-time reference to the root README.
   - Add a small internal representation for parsed README rows.
   - Add Markdown table parsing and catalog verification helpers.
   - Add positive and negative tests.

2. `README.md`
   - Replace the incomplete configuration table body.
   - List every current `CONFIG_KEYS` record.
   - Use catalog order.
   - Use catalog-derived display defaults.
   - Use catalog descriptions verbatim.

### Private phase artifacts

- `.lisa/attempts/T-050-02-02/1/work/research.md`
- `.lisa/attempts/T-050-02-02/1/work/design.md`
- `.lisa/attempts/T-050-02-02/1/work/structure.md`
- `.lisa/attempts/T-050-02-02/1/work/plan.md`
- `.lisa/attempts/T-050-02-02/1/work/progress.md`
- `.lisa/attempts/T-050-02-02/1/work/review.md`
- `.lisa/attempts/T-050-02-02/1/work/review-disposition.json`

### Created production files

- None.

### Deleted files

- None.

## Module boundary

All executable verification logic stays inside `config.rs` under `#[cfg(test)]`.
The production `ConfigKey`, `CONFIG_KEYS`, parsing, validation, and init generation
interfaces remain unchanged.

This boundary is appropriate because:

- README synchronization is a repository invariant, not CLI runtime behavior;
- the test module has direct private access to `ConfigKey` and `CONFIG_KEYS`;
- no cross-module API is needed; and
- normal `cargo test` execution compiles the invariant.

## Test-module additions

### Embedded document constant

Add:

```rust
const README: &str = include_str!("../../../README.md");
```

Location: near `COMPLETE_CONFIG_FIXTURE` at the top of the inline test module.

Responsibilities:

- make the root README test input compile-time visible;
- ensure Cargo rebuilds the test target when README changes; and
- avoid runtime current-directory assumptions.

### `ReadmeConfigRow`

Add a test-local record:

```rust
#[derive(Debug, PartialEq, Eq)]
struct ReadmeConfigRow {
    default: String,
    description: String,
}
```

The key is held by the surrounding map rather than duplicated in the row.

Responsibilities:

- separate parsed Markdown representation from `ConfigKey`;
- preserve owned strings after line parsing; and
- support clear debug output if assertions are later expanded.

### Table-header constant or inline anchor

The parser uses the exact adjacent header and delimiter lines:

```markdown
| Key | Default | Description |
|-----|---------|-------------|
```

The anchor is intentionally scoped to the configuration schema used by README.
It avoids accidentally parsing unrelated tables.

### `strip_code_ticks`

Signature:

```rust
fn strip_code_ticks(cell: &str) -> &str
```

Responsibilities:

- remove a single matching leading/trailing backtick pair;
- return an unwrapped cell unchanged; and
- support the current README convention without a Markdown dependency.

This helper is used only for key and default cells. Descriptions remain prose.

### `parse_readme_config_table`

Signature:

```rust
fn parse_readme_config_table(
    readme: &str,
) -> Result<BTreeMap<String, ReadmeConfigRow>, String>
```

Responsibilities:

1. Find the table header line.
2. Require the expected separator on the next line.
3. Iterate subsequent pipe-prefixed rows.
4. Remove the outer pipe characters.
5. Split each row into exactly three cells.
6. Trim whitespace.
7. Normalize code ticks on key and default.
8. Reject an empty key.
9. Reject duplicate dotted keys.
10. Return rows keyed by dotted path.

Error messages identify:

- a missing table header;
- a missing or incorrect separator;
- a malformed row;
- an empty key; or
- a duplicate row.

The parser does not know about `CONFIG_KEYS`. That separation makes parser output
usable by the verifier and keeps catalog policy out of Markdown mechanics.

### `readme_default`

Signature:

```rust
fn readme_default(default: &str) -> &str
```

Responsibilities:

- convert canonical TOML string-literal syntax to README presentation;
- remove ordinary surrounding double quotes when both are present; and
- leave non-string TOML literals unchanged.

The function does not parse arbitrary TOML. The catalog default-validity test
already validates syntax, and all fixed string defaults use simple quoted values.

### `verify_readme_config_table`

Signature:

```rust
fn verify_readme_config_table(
    catalog: &[ConfigKey],
    readme: &str,
) -> Result<(), String>
```

Dependencies:

- calls `parse_readme_config_table`;
- calls `readme_default`; and
- uses `BTreeSet` or direct catalog lookup for extra-row detection.

Responsibilities for each catalog entry:

1. Find a row by `entry.path`.
2. Return `README configuration table is missing description for <path>` if the
   row is absent.
3. Return the same named class of error if its description cell is blank.
4. Compare `row.default` with `readme_default(entry.default)`.
5. Return a named default-mismatch error with expected and actual values.
6. Compare `row.description` with `entry.description` exactly.
7. Return a named description-mismatch error with expected and actual values.

Responsibilities after catalog traversal:

- find the first README row absent from the catalog; and
- return a named undocumented/stale-row error.

An empty catalog is structurally supported, though the real catalog is non-empty.

## Test cases

### `readme_config_table_matches_catalog`

Inputs:

- `CONFIG_KEYS`
- `README`

Expected behavior:

- `verify_readme_config_table` returns `Ok(())`.

Failure behavior:

- panic text is the helper's actionable error.

Coverage:

- every catalog path exists in README;
- each displayed default matches;
- each exact description matches;
- no duplicate README paths exist; and
- no stale extra README path exists.

### `readme_config_table_names_missing_fake_description`

Setup:

- copy `CONFIG_KEYS` into a mutable vector;
- append one fake `ConfigKey` with path `fake.missing_description`;
- give it a valid section, key, default, and description.

Expected behavior:

- verification returns `Err`;
- error contains `missing description`; and
- error contains `fake.missing_description`.

This is a negative fixture around the verifier, not a `#[should_panic]` test. The
test suite stays green while checking the exact failed-build diagnostic a new key
would produce.

## Existing test interaction

The new positive test depends on existing catalog guarantees rather than repeating
them:

- parsed-key coverage remains in
  `config_catalog_covers_every_parsed_key_exactly_once`;
- default TOML validity remains in
  `config_catalog_defaults_are_valid_toml`;
- description shape remains in
  `config_catalog_descriptions_pass_brand_voice_checks`;
- init rendering remains in
  `default_config_renders_every_catalog_description_and_default`.

The combined tests establish this dependency graph:

```text
parsed fixed keys -> CONFIG_KEYS -> init stubs
                                -> README rows
```

## README table structure

Retain the existing header and separator. Replace only data rows.

Row order:

1. `version`
2. `dirs.tickets`
3. `dirs.stories`
4. `dirs.work`
5. `runtime.zellij`
6. `agent.client`
7. `guards.completion`
8. `triage.enabled`
9. `triage.timeout_secs`
10. `scheduling.max_threads`
11. `scheduling.auto_advance`
12. `scheduling.review_timeout_secs`
13. `scheduling.session_timeout_secs`
14. `scheduling.wind_down_secs`
15. `scheduling.assignment_ack_timeout_secs`
16. `scheduling.phase_timeouts`
17. `scheduling.provider_caps`

Default presentation:

- `version`: current package version without TOML quotes;
- directory paths: values without TOML quotes;
- runtime, agent, and guard choices: values without TOML quotes;
- booleans, numeric values, and `{}`: unchanged.

Description presentation:

- exact `ConfigKey::description` contents;
- complete sentence punctuation retained;
- no independent elaboration in table cells.

Long-form explanations for complex knobs can live elsewhere in README, but the
schema table remains the catalog rendering.

## Commit boundaries

One meaningful ticket-owned source unit spans both modified files because the test
and synchronized README must land together to keep the commit green.

Commit command shape:

```text
lisa commit-ticket --ticket-id T-050-02-02 \
  --message "Bind README configuration to the catalog" \
  --include crates/lisa-cli/src/config.rs \
  --include README.md
```

No phase artifact is included in this source commit. Lisa handles final artifact
publication and the completion commit.

## Verification boundary

Targeted verification:

```text
cargo test -p lisa-cli config::tests::readme_config_table
```

Module verification:

```text
cargo test -p lisa-cli config::tests
```

Workspace verification:

```text
cargo test --workspace
```

Formatting verification:

```text
cargo fmt --all -- --check
```

After commit, Git status must show no modifications or untracked files for
`crates/lisa-cli/src/config.rs` or `README.md`. Unrelated concurrent changes remain
outside ticket ownership and are not altered.
