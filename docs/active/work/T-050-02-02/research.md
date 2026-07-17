# Research: one-source configuration table

## Ticket scope

- Ticket: `T-050-02-02`.
- Story: `S-050-02`.
- Current phase at assignment: Research.
- The ticket concerns operator-facing documentation for `.lisa.toml`.
- The named operator-facing surfaces are generated init stubs and the README.
- T-050-02-01 already connected init stubs to an in-code configuration catalog.
- This ticket starts with that catalog present on the branch.
- This ticket must bind the remaining README surface to the same catalog.
- The acceptance criteria require a build failure on documentation drift.
- Drift includes a missing parsed key, a default mismatch, or a missing description.
- The criteria also require a negative test whose failure identifies the absent description.
- Guards and triage are explicitly named as current keys that must appear.

## Repository and workflow constraints

- The repository is a Rust workspace.
- `crates/lisa-cli` owns `.lisa.toml` parsing and generation.
- Native workspace tests are the normal verification path.
- The README is at repository root.
- The CLI crate declares `../../README.md` as its Cargo package readme.
- Rust source can include repository files at compile time with `include_str!`.
- `templates.rs` already uses this mechanism for a documentation sync test.
- The RDSPI workflow requires artifacts in the private attempt work directory.
- Lisa publishes admitted artifacts later; this attempt must not write phase files to
  `docs/active/work/T-050-02-02`.
- The ticket frontmatter is Lisa-owned during phase transitions.
- Ticket source changes must be committed with `lisa commit-ticket`.
- Exact repository-relative include paths are required for that command.
- The ordinary Git index is shared and already contains unrelated concurrent work.
- `README.md` and `crates/lisa-cli/src/config.rs` are clean at the start of this attempt.

## Configuration catalog

- `crates/lisa-cli/src/config.rs` defines `ConfigKey`.
- `ConfigKey` is crate-visible but not public outside `lisa-cli`.
- Each record contains a dotted `path`.
- Each record contains a TOML `section`.
- Each record contains the local TOML `key`.
- Each record contains a `default` TOML right-hand-side literal.
- Each record contains a one-line plain-English `description`.
- `ConfigKey::commented_stub` renders a description and an inert assignment.
- `CONFIG_KEYS` is a static slice of `ConfigKey` records.
- Its comments define it as every fixed `.lisa.toml` key.
- Its order is explicitly operator-facing render order.
- Map children under `phase_timeouts` are not individual fixed keys.
- Map children under `provider_caps` are not individual fixed keys.
- Those two maps are represented by their parent configuration records.

## Current catalog contents

- `version` has a default derived from `CARGO_PKG_VERSION`.
- `dirs.tickets` defaults to `"docs/active/tickets"`.
- `dirs.stories` defaults to `"docs/active/stories"`.
- `dirs.work` defaults to `"docs/active/work"`.
- `runtime.zellij` defaults to `"managed"`.
- `agent.client` defaults to `"claude"`.
- `guards.completion` defaults to `"auto"`.
- `triage.enabled` defaults to `true`.
- `triage.timeout_secs` defaults to `120`.
- `scheduling.max_threads` defaults to `2`.
- `scheduling.auto_advance` defaults to `false`.
- `scheduling.review_timeout_secs` defaults to `600`.
- `scheduling.session_timeout_secs` defaults to `3600`.
- `scheduling.wind_down_secs` defaults to `300`.
- `scheduling.assignment_ack_timeout_secs` defaults to `30`.
- `scheduling.phase_timeouts` defaults to `{}`.
- `scheduling.provider_caps` defaults to `{}`.
- The catalog therefore contains 17 records.

## Catalog consumers

- `config_key` finds a catalog entry by dotted path.
- Config validation derives known top-level sections from the catalog.
- Config validation derives known keys inside each section from the catalog.
- `default_config_toml` obtains every rendered fixed setting through `config_key`.
- Active defaults and commented defaults both use catalog values.
- Active comments and commented stub comments both use catalog descriptions.
- `init.rs` iterates the catalog for scheduling upserts.
- `init.rs` iterates the catalog for appended agent, guards, and triage stubs.
- Existing tests iterate the catalog to check init output.
- No README consumer currently iterates the catalog.

## Catalog integrity tests

- `COMPLETE_CONFIG_FIXTURE` contains every fixed parsed setting.
- `parsed_fixed_paths` converts that TOML fixture into dotted paths.
- `config_catalog_covers_every_parsed_key_exactly_once` compares those paths with
  the catalog.
- That test also rejects duplicate dotted paths.
- It also rejects duplicate section/key pairs.
- `config_catalog_defaults_are_valid_toml` parses each catalog default in context.
- `config_catalog_descriptions_pass_brand_voice_checks` requires descriptions to
  remain one line.
- The brand-voice test requires descriptions to end in a period.
- The test requires a direct leading verb from a fixed list.
- It rejects a short list of internal or marketing terms.
- `default_config_renders_every_catalog_description_and_default` checks the init
  output for every catalog entry.
- These tests establish coverage from parsing to the catalog and from the catalog
  to `.lisa.toml` generation.
- They do not establish coverage from the catalog to README.

## Current README configuration surface

- README has one `## Configuration` section.
- That section first shows a short `.lisa.toml` example.
- A Markdown table follows the example.
- The table header is `Key`, `Default`, and `Description`.
- The table currently has eight data rows.
- The rows cover the three directory settings.
- The rows cover `scheduling.max_threads`.
- The rows cover `agent.client`.
- The rows cover `guards.completion`.
- The rows cover both triage settings.
- The table does not cover `version`.
- The table does not cover `runtime.zellij`.
- The table does not cover seven scheduling settings after `max_threads`.
- Current README row order differs from catalog order around scheduling and agent.
- Current README string defaults omit TOML quote characters.
- Current README descriptions omit the catalog's final periods.
- Most current README descriptions use different language from the catalog.
- The guards and triage README descriptions are longer than their catalog entries.
- Consequently, an exact catalog-to-README comparison does not currently pass.

## Markdown representation facts

- Table keys are enclosed in backticks.
- Table defaults are enclosed in backticks.
- TOML string defaults are displayed without their TOML quote delimiters.
- Boolean, integer, and inline-table defaults already need no unquoting.
- Descriptions are ordinary Markdown cell text.
- None of the current catalog descriptions contains a pipe character.
- None contains a newline or carriage return because an existing test forbids it.
- The `agent.client` description includes `PATH` and a semicolon but no table delimiter.
- The configuration table ends at the blank line before `## Codex client`.
- The current table has no generated-block markers.

## Existing documentation sync precedent

- `crates/lisa-cli/src/templates.rs` embeds the distributed RDSPI workflow.
- A unit test uses `include_str!("../../../docs/knowledge/rdspi-workflow.md")`.
- It compares documentation bytes with the bundled copy.
- The relative path shows that CLI unit tests can observe root documentation.
- The sync invariant runs under normal Rust tests without a separate script.
- The configuration code's existing tests live in an inline `#[cfg(test)]` module.
- A README sync check placed there would run in the same test target as catalog tests.

## Version behavior

- Workspace package version is currently `0.4.4-rc.1`.
- The `version` catalog default is constructed with surrounding TOML quotes.
- A README row derived during compilation would therefore show `0.4.4-rc.1` after
  presentation-level quote removal.
- A later version bump changes the catalog's effective default automatically.
- Any committed README value would then need a corresponding update for sync to pass.
- This is consistent with the requested build-time drift detection.

## Test and module boundaries

- `ConfigKey` uses only static string fields and is `Copy`.
- Tests in the inline module can construct fake `ConfigKey` values directly.
- Tests can pass a slice containing a fake record to a local verifier.
- Rust assertion messages can include a dotted path from that record.
- A helper returning `Result<(), String>` permits direct assertions on diagnostics.
- Production config loading does not read README at runtime.
- A compile-time README include inside `#[cfg(test)]` adds no runtime I/O.
- No new dependency is necessary for line-oriented Markdown handling.
- The existing standard library is sufficient for rendering or parsing the table.

## Observed boundaries and assumptions

- The ticket calls `CONFIG_KEYS` the one in-code table established by its dependency.
- The catalog record's `default` is canonical TOML syntax, not canonical Markdown.
- README presentation must account for the difference for string values.
- A full-key-set invariant must include top-level `version`, because the parsed-key
  coverage test and catalog both include it.
- Dynamic child names inside the two map values are outside the fixed-key set.
- The short TOML example above the table is explanatory and is not named as a
  complete key inventory.
- README table cells should be derived from catalog fields without maintaining a
  second description vocabulary.
- The working tree contains unrelated ticket artifacts and journal mutations.
- Those unrelated paths must remain untouched by this ticket.

## Research conclusion

- The parser-to-catalog and catalog-to-init edges are already enforced.
- The missing edge is catalog-to-README.
- README is currently incomplete relative to the 17-record catalog.
- README defaults and descriptions are independently maintained today.
- The CLI unit-test target already has the access and precedent needed to enforce a
  root-document sync invariant.
- A fake `ConfigKey` can exercise the requested missing-description diagnostic without
  changing the production catalog.
