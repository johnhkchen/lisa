# Structure: configuration catalog and append-only consumers

## Change summary

Two ticket-owned source files change:

- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/init.rs`

Five private attempt artifacts are created during the remaining workflow:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`

Review adds:

- `review.md`
- `review-disposition.json`

No production file is created or deleted.
No dependency or Cargo manifest changes.
No README change belongs to this ticket; T-050-02-02 owns that integration.

## `crates/lisa-cli/src/config.rs`

### New metadata type

Add `ConfigKey` near the input structures.

Fields:

- `path: &'static str`
- `section: &'static str`
- `key: &'static str`
- `default: &'static str`
- `description: &'static str`

Visibility is `pub(crate)` so `init.rs` and the dependent README work can read
the records without exposing a public library API.

The type derives traits useful for tests and lookup diagnostics:

- `Debug`
- `Clone`
- `Copy`
- `PartialEq`
- `Eq`

### New catalog constant

Add `CONFIG_KEYS: &[ConfigKey]` beside the type.

Catalog order is operator-facing render order:

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

Map child names do not become catalog rows. The fixed parsed setting is the map
itself; allowed child names continue to be validated by their specialized code.

### Lookup helpers

Add small private or crate-visible functions:

- lookup by dotted path for template assembly and tests;
- test whether a section/key pair is cataloged;
- test whether a top-level section is cataloged;
- render a description plus commented assignment for one record.

The renderer returns:

```text
# <description>
# <key> = <default>
```

It does not include a section header because callers own placement.

### Validation changes

Remove these duplicated fixed-key arrays from `validate_config`:

- `known_top`
- `known_dirs`
- `known_agent`
- `known_runtime`
- `known_guards`
- `known_triage`
- `known_scheduling`

Replace their membership checks with catalog membership helpers.

Keep the following specialized arrays and logic:

- known RDSPI phase names;
- valid agent-client names for provider caps;
- semantic nonzero bounds;
- agent-client parsing;
- runtime spelling/path validation;
- completion-mode parsing.

Unknown warning strings remain unchanged.

### Fresh template changes

Refactor `default_config_toml` to interpolate catalog-rendered descriptions and
defaults.

Keep the same overall section order:

- title/version;
- dirs;
- runtime;
- agent;
- guards;
- triage;
- scheduling.

Keep active values where the current template activates them:

- version;
- all directory values;
- max threads.

Render all optional keys as commented assignments. Render empty maps as inline
empty table defaults. Eliminate hard-coded duplicate descriptions for cataloged
keys.

### Config test additions

Add helpers inside the existing test module to build and enumerate the complete
parsed fixture.

Add tests for:

- exact catalog path coverage;
- catalog path uniqueness;
- section/key uniqueness;
- default assignment parseability;
- default-value agreement for representative resolved types;
- description one-line shape;
- sentence termination;
- approved verb prefixes;
- banned jargon absence;
- fresh template containing each catalog description/default;
- fresh template parsing with unchanged effective defaults.

Existing tests continue to exercise semantic validation.

## `crates/lisa-cli/src/init.rs`

### Existing scanners retained

Retain:

- `has_key`
- `has_section`
- `find_section_end`
- `insert_after`

These functions preserve the current append-only editing boundary.

### New append helper

Add a local helper that appends a complete block while handling:

- empty input;
- input ending in one newline;
- input with no trailing newline;
- a blank-line separator between old bytes and new block.

This helper must never trim or normalize the existing string.

### Scheduling catalog consumption

Replace the hard-coded `scheduling_keys` tuple array with an iterator over
`config::CONFIG_KEYS` filtered to section `scheduling`.

Skip `max_threads` when present, just as all active/commented assignments are
skipped. For any other absent record:

- locate the current section end;
- render its catalog stub;
- insert the two lines at the end;
- continue in catalog order.

The former special `scheduling.phase_timeouts` block is removed. Both map-valued
settings use the same catalog rendering as scalar settings.

### Missing-section catalog consumption

Iterate the explicit ticket section order:

1. `agent`
2. `guards`
3. `triage`

For an absent section:

- select all catalog rows for the section;
- render `# [section]`;
- render every row's description/default pair;
- append the complete block.

For an existing active or commented header:

- do nothing to that section;
- do not add missing leaf keys;
- do not modify descriptions;
- do not reorder it.

### Init test fixture helpers

Add a helper to extract the planned `.lisa.toml` action or invoke the pure
upsert directly where filesystem behavior is irrelevant.

Add a legacy fixture with:

- current version;
- `[dirs]` and customized directory values;
- a user comment;
- `[scheduling]` and customized max threads;
- at least one existing optional scheduling value.

### Init test assertions

Assert:

- output begins with the exact legacy fixture bytes;
- agent, guards, and triage commented headers are present;
- each new row contains its catalog description and default;
- the customized scheduling value occurs once;
- the user comment occurs once and at the same prefix location;
- every other missing scheduling key appears once;
- typed resolved values before and after match;
- a second pure upsert is byte-identical;
- `plan_init_actions` selects one update for the legacy fixture;
- `plan_init_actions` selects a no-op for the current template;
- a customized current file remains byte-identical when all relevant sections
  and scheduling keys are already present.

## Interface boundaries

`config.rs` owns meaning:

- which keys exist;
- their stable paths;
- their defaults;
- their plain descriptions;
- whether validation recognizes them.

`init.rs` owns placement:

- whether a setting is present;
- where missing scheduling lines are inserted;
- whether a whole section needs appending;
- exact preservation of pre-existing bytes.

`default_config_toml` owns fresh-file activation choices. The catalog does not
decide whether a key is active or commented, only how its inert stub reads.

T-050-02-02 may consume `CONFIG_KEYS` and rendering helpers but must not move the
catalog or duplicate descriptions.

## Error behavior

- No new runtime error path is introduced.
- Unreadable `.lisa.toml` files remain safety-skipped.
- Invalid TOML continues through the current version/upsert planning behavior.
- Invalid values continue to be rejected by validation, not init rendering.
- Unknown map children keep their current warnings.
- Missing catalog metadata makes a fixed key unknown to validation.

## Ordering constraints

1. Introduce the catalog without changing consumers.
2. Wire validation and template generation to it.
3. Verify config-focused tests.
4. Wire init upsert behavior to it.
5. Add preservation and effective-config fixtures.
6. Format and run focused tests.
7. Commit both ticket-owned source files together if the catalog and consumer
   cannot compile independently.
8. Run workspace-level verification.
9. Complete review artifacts without touching ticket phase/status.

## Non-goals

- No README table in this ticket.
- No new configuration setting.
- No new CLI flag.
- No changes to resolution precedence.
- No changes to plugin layout keys.
- No general-purpose TOML formatter.
- No repair of unknown or malformed user content.
- No automatic uncommenting.
- No replacement of customized values.
- No ticket-frontmatter changes by the agent.
