# Review: upsert new configuration sections

## Disposition

Pass.

The implementation satisfies both ticket acceptance criteria, is committed
through Lisa's isolated transaction, passes focused and workspace-wide tests,
and leaves no ticket-owned source path staged, modified, or untracked.

## Source commit

`363e82d4c962317b743f0388027c0fdb6dfaca71`

Commit message:

`Make init discover every config section`

Exact ticket source includes:

- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/init.rs`

The ordinary Git index was not used.

## Files modified

### `crates/lisa-cli/src/config.rs`

Added a crate-visible `ConfigKey` metadata record with:

- stable dotted path;
- TOML section;
- leaf key;
- valid TOML default spelling; and
- one plain-English description.

Added `CONFIG_KEYS`, containing all 17 fixed keys parsed by the CLI:

- `version`;
- three directory keys;
- runtime selection;
- agent client selection;
- completion guard;
- two triage keys;
- eight scheduling keys, including the two map parents.

Map children remain specialized values rather than fixed catalog rows. Phase
names and provider names retain their existing validation.

Replaced the fixed per-section known-key arrays in `validate_config` with
catalog membership checks. This makes the description catalog part of the
accepted configuration contract: a fixed key without a catalog entry is now
reported as unknown.

Refactored `default_config_toml` so every key's description and default come
from the catalog. Fresh files still activate only the established values:

- current Lisa version;
- default directories; and
- `max_threads`.

Every optional setting remains commented. Empty phase-timeout and provider-cap
maps now render their actual `{}` default rather than non-default examples.

Derived equality for typed input config structures so inertness fixtures can
compare parsed config directly instead of comparing a hand-selected subset.

Added tests for:

- complete parsed-key/catalog equality;
- dotted-path uniqueness;
- section/key uniqueness;
- TOML-valid defaults;
- direct-verb descriptions;
- one-line complete sentences;
- banned operator-jargon absence; and
- fresh-template coverage of every catalog description/default.

### `crates/lisa-cli/src/init.rs`

Replaced the local hard-coded scheduling tuple list with an iterator over the
shared catalog.

Missing scheduling settings now receive:

- the catalog purpose sentence; and
- the catalog commented default assignment.

Active and commented existing assignments still count as present. Existing
values are never replaced or duplicated.

Added inert whole-section appends for:

- `# [agent]`;
- `# [guards]`; and
- `# [triage]`.

Each absent section is rendered from its catalog rows. Both the header and
assignments are commented, so even generic TOML parsing sees no new value.

An active or commented existing section header remains ownership evidence. Init
does not rewrite the section, insert prose into it, reorder it, or append a
second copy.

The append helper retains every original byte, supplies a separator only after
the original content, and handles files without a final newline.

Expanded the legacy init fixture to include:

- a current version;
- dirs and scheduling only;
- customized directory paths;
- customized max threads;
- customized session timeout;
- a file-level user comment;
- a scheduling user comment; and
- no final newline.

The fixture proves:

- original bytes remain an exact prefix;
- the three missing sections appear exactly once;
- each appended key uses catalog text;
- every scheduling setting is discoverable;
- customized content is not duplicated;
- parsed `LisaConfig` is equal before and after; and
- a second upsert is byte-identical.

A separate current-file fixture activates customized agent, guard, triage, and
thread values, adds a user comment, and proves the entire document remains
byte-identical.

## Acceptance criterion 1

> Fixture tests: a dirs+scheduling-only `.lisa.toml` re-inits to gain
> [agent]/[guards]/[triage] as commented stubs with prior bytes preserved in
> place; a current file no-ops (byte-identical); customized values and user
> comments survive; stubs never change effective config (parse before == parse
> after).

Evidence:

- `test_plan_init_upserts_missing_config_keys` uses the required legacy shape.
- `content.starts_with(&existing)` pins exact preservation of every old byte.
- Catalog loops assert each missing section and key is present once.
- Comment occurrence assertions pin both user comments.
- Existing session timeout occurrence count pins non-duplication.
- Direct `LisaConfig` equality pins parse-before/parse-after inertness.
- Reapplying `upsert_missing_config_keys` pins idempotent byte equality.
- `test_upsert_noop_when_complete` pins the canonical current file.
- `test_upsert_preserves_custom_values_comments_and_current_order` pins a
  customized current document byte-for-byte.

Result: satisfied.

## Acceptance criterion 2

> The stub text lives in one table keyed by config key; a test enumerates
> parsed config keys and fails if any lacks a table entry; stub comments pass
> the brand-voice string checks.

Evidence:

- `CONFIG_KEYS` is the only source for paths, defaults, and descriptions.
- `config_catalog_covers_every_parsed_key_exactly_once` parses a complete
  fixture and compares its fixed dotted paths with catalog paths.
- The same test rejects duplicate dotted paths and section/key pairs.
- `config_catalog_defaults_are_valid_toml` validates every rendered default.
- `config_catalog_descriptions_pass_brand_voice_checks` requires direct verbs,
  complete one-line sentences, and the shared no-jargon bar.
- `default_config_renders_every_catalog_description_and_default` prevents the
  fresh template from drifting away from the table.
- Init renders both scheduling additions and missing sections from the table.
- Validation recognizes fixed fields through the table, closing the silent
  addition path.

Result: satisfied.

## Verification

Focused checks:

- `cargo test -p lisa-cli config::tests`: 65 passed.
- `cargo test -p lisa-cli init::tests`: 77 passed.
- `git diff --check` on both source paths: passed before commit.

Package check:

- `cargo test -p lisa-cli`: passed before source commit.
- 16 library tests passed in that run.
- 365 binary tests passed in that run.
- All enabled integration tests passed.

Final integrated workspace check:

- `cargo test --workspace`: passed.
- CLI library: 21 passed.
- CLI binary: 365 passed.
- CLI integrations: all enabled suites passed.
- Core: 248 passed.
- Core integration regressions: 2 passed.
- Plugin: 437 passed.
- Doc tests: passed.
- The real-Zellij boundary fixture remained intentionally ignored because it
  requires external executables and the WASM target.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `git show --check 363e82d`: passed.

## Concurrent-file review

`T-050-03-01` independently needed `config.rs` for client auto-detection. Its
uncommitted resolver initially overlapped this ticket's working file.

The agents coordinated through their Lisa panes:

1. `T-050-03-01` committed its independent detection unit.
2. It removed its uncommitted `config.rs` resolver changes.
3. This ticket re-audited the remaining two-file diff.
4. This ticket committed only its catalog/upsert unit as `363e82d`.
5. `T-050-03-01` reapplied and committed its resolver and announcement units as
   `47e7336` and `d88cd13`.
6. Final workspace verification ran on the integrated result.

No commit absorbed another ticket's uncommitted source. The later resolver
extends the agent description to explain PATH detection while retaining the
catalog's direct-verb and no-jargon contract.

## Open concerns

None blocking.

The README does not yet render or verify the catalog. That is deliberately
owned by dependent ticket `T-050-02-02`, whose context names this table as its
input. This ticket leaves the required crate-visible metadata seam ready.

The upsert merger remains line-oriented. It recognizes ordinary active and
commented section/assignment forms established by existing Lisa templates. A
general TOML concrete-syntax editor remains out of scope and is unnecessary for
the accepted fixtures and preservation discipline.

## Repository state

- Ticket-owned source is committed.
- Ticket-owned source paths are clean.
- No ticket-owned source is staged.
- Other modified journal, ticket, and work paths are Lisa-managed or belong to
  concurrent tickets and were not included in this ticket's source commit.
- Ticket phase/status was not edited manually.
- Completion publication remains Lisa's responsibility.
