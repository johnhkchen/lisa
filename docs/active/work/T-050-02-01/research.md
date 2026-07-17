# Research: upsert new configuration sections

## Ticket boundary

- Ticket: `T-050-02-01`, `upsert-new-sections`.
- The ticket starts in Research and requires the complete RDSPI sequence.
- Production scope is the native CLI configuration and initialization path.
- The named implementation file is `crates/lisa-cli/src/init.rs`.
- The configuration model lives in `crates/lisa-cli/src/config.rs`.
- No new configuration behavior or new configuration key is requested.
- Existing `.lisa.toml` values must remain authoritative.
- Existing comments and ordering must remain in place.
- Re-initialization may append discoverability text only.
- Appended stubs must not activate a setting.
- A current file must remain byte-identical.

## Initialization flow

- `plan_init_actions` builds the complete `lisa init` action list.
- It handles directories, context files, workflow text, configuration, hooks,
  settings, and ignore rules.
- `.lisa.toml` receives its own planning branch.
- A missing `.lisa.toml` becomes a `CreateFile` action.
- Fresh content comes from `config::default_config_toml()`.
- An unreadable existing file becomes a `SafetySkip` action.
- A readable existing file is parsed as `config::LisaConfig`.
- The parsed `version` decides whether the version line is updated.
- `update_version_in_toml` preserves the existing newline convention when it
  replaces a version line.
- It inserts a missing version after leading comments.
- `upsert_missing_config_keys` runs even when the version is current.
- Equality with the original bytes selects `NoOp`.
- Any changed output selects `UpdateFile`.
- Execution happens later from the planned action list.

## Existing upsert behavior

- `upsert_missing_config_keys` starts from an exact copy of the input.
- Its `has_key` scanner recognizes active and commented assignments.
- Key matching is scoped to a named TOML section.
- Its section scanner recognizes active headers such as `[scheduling]`.
- It also recognizes commented headers such as `# [scheduling.phase_timeouts]`.
- `find_section_end` locates an insertion point without parsing and rewriting
  the TOML document.
- `insert_after` reconstructs lines around that insertion point.
- The current upsert list is hard-coded inside `init.rs`.
- It contains five scalar scheduling keys.
- Those are `auto_advance`, `review_timeout_secs`,
  `session_timeout_secs`, `wind_down_secs`, and
  `assignment_ack_timeout_secs`.
- Missing scheduling scalars are inserted as commented assignments.
- A missing `scheduling.phase_timeouts` block is inserted separately.
- The block contains commented research, design, and implement examples.
- No other top-level section is currently appended.
- Therefore an older file cannot discover agent, guard, or triage settings by
  re-running init.

## Parsed configuration surface

- `LisaConfig` is the typed top-level input structure.
- Its scalar top-level input is `version`.
- Its table fields are `dirs`, `scheduling`, `agent`, `runtime`, `guards`, and
  `triage`.
- Every table field has `#[serde(default)]`.
- Missing tables therefore deserialize successfully.
- `DirsConfig` parses `tickets`, `stories`, and `work`.
- `AgentConfig` parses `client` as a raw optional string.
- `RuntimeConfig` parses `zellij` as a raw optional string.
- `GuardsConfig` parses `completion` as a raw optional string.
- `TriageConfig` parses `enabled` and `timeout_secs`.
- `SchedulingConfig` parses `max_threads`, `auto_advance`,
  `review_timeout_secs`, `session_timeout_secs`, `wind_down_secs`,
  `assignment_ack_timeout_secs`, `phase_timeouts`, and `provider_caps`.
- This is a 17-key operator-facing surface when map-valued settings count as
  their parent keys.
- Entries inside `phase_timeouts` and `provider_caps` are user-selected map
  members rather than additional fixed config fields.

## Defaults and resolution

- `ResolvedConfig::default` centralizes effective default behavior.
- Directory and scheduler defaults are primarily constants on `PluginConfig`.
- Default ticket, story, and work directories are under `docs/active`.
- Default `max_threads` is 2.
- Default `auto_advance` is false.
- Default review timeout is 600 seconds.
- Default session timeout is 3600 seconds.
- Default wind-down time is 300 seconds.
- Default assignment acknowledgment timeout is 30 seconds.
- Default agent client is Claude through `AgentClient::default`.
- Default runtime comes from the platform-aware runtime resolver.
- Its operator spelling in the template is `managed`.
- Default completion intent is `auto`.
- Default provider caps and phase timeouts are empty maps.
- Default triage is enabled with a 120-second bound.
- `resolve_config` applies file values over those defaults.
- CLI max-thread and client arguments retain higher precedence.
- A file containing only comments for a setting resolves like a missing field.
- Commented table headers are ignored by TOML parsing as well.

## Validation boundary

- `validate_config` first parses a generic `toml::Value`.
- It owns manual arrays of known top-level sections and known keys per table.
- Those arrays drive unknown-section and unknown-key warnings.
- It then deserializes the same input into `LisaConfig`.
- Semantic checks reject invalid bounds and enumerated strings.
- The known-key arrays currently duplicate the struct field inventory.
- They do not provide descriptions or default spellings.
- There is no current single source tying validation, init stubs, and docs.
- T-050-02-02 depends on this ticket and will bind README rendering to the
  description source introduced here.

## Fresh template

- `default_config_toml` returns generated text containing the package version.
- Directory values and `scheduling.max_threads` are active.
- Runtime examples are commented under an active `[runtime]` header.
- Agent client is commented under an active `[agent]` header.
- Completion is commented under an active `[guards]` header.
- Triage settings are commented under an active `[triage]` header.
- Scheduling optional values are commented under an active `[scheduling]`
  header.
- Phase timeouts and provider caps use commented subsection examples.
- Existing comments vary in style and sometimes describe multiple keys with
  one comment.
- The ticket instead requires one plain-English comment per key.
- The ticket supplies the intended completion description as a voice example.

## Existing tests

- Unit tests for initialization are colocated in `init.rs`.
- `test_plan_init_skips_current_version` uses the fresh default template and
  expects a `.lisa.toml` no-op.
- `test_plan_init_upserts_missing_config_keys` starts with scheduling only.
- It currently checks only scheduling additions.
- Active and commented scheduling values have duplicate-prevention tests.
- `test_upsert_noop_when_complete` requires byte identity for the fresh
  template.
- Other init tests establish the repository's append-only and ownership-aware
  update discipline.
- Config parsing, defaults, validation, and fresh-template tests are colocated
  in `config.rs`.
- Existing tests prove agent, guards, runtime, triage, provider caps, and phase
  timeout parsing independently.
- `help_surface.rs` contains the broad operator-language jargon list.
- Its banned terms include DAG, orchestration, scheduling, leverage, solutions,
  deployment, case study, build log, and research release.
- It also pins Lisa's purpose-before-mechanism voice on help surfaces.
- There is not yet a config-description brand-voice test.

## Preservation constraints

- The working tree already contains Lisa-owned journal and ticket changes.
- Another ticket also owns files under `docs/active/work/T-050-01-02`.
- Those paths are outside this ticket and must remain untouched.
- Ticket artifacts belong only in the private attempt work directory.
- Source commits must use `lisa commit-ticket` with exact include paths.
- Ordinary Git staging and commits are prohibited for ticket work.
- The final ticket transition and artifact publication belong to Lisa.

## Observed edge conditions

- A commented assignment counts as present and must not be duplicated.
- An active customized assignment counts as present and must not be replaced.
- An active or commented section header counts as present.
- Appending to a file without a trailing newline needs an explicit separator.
- Repeated init must be idempotent.
- Generic TOML equality is stricter than effective-config equality because an
  active empty table appears in the generic value.
- Commented headers guarantee the appended block is inert at both generic and
  typed parse boundaries.
- Version updating is a separate behavior from stub insertion.
- Effective-config tests need a current version to isolate the stub invariant.

## Research conclusion

- The defect is a missing connection between the configuration inventory and
  the append-only init merger.
- The relevant production code is contained in `config.rs` and `init.rs`.
- The relevant regression coverage can remain in their existing unit-test
  modules.
- No plugin, scheduler, provider adapter, ticket parser, or documentation file
  needs production behavior changes in this ticket.
