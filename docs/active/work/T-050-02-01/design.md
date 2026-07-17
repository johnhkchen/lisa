# Design: single-source inert config stubs

## Goals

- Make every current CLI-parsed configuration key discoverable from one table.
- Append missing `[agent]`, `[guards]`, and `[triage]` sections on re-init.
- Continue adding missing optional `[scheduling]` keys.
- Give every stubbed key one plain-English purpose sentence.
- Show every key's effective default in an inert assignment.
- Preserve all existing bytes in their original order.
- Keep a fully current file byte-identical.
- Leave configuration resolution unchanged.
- Prepare a stable source for T-050-02-02's README sync.

## Option 1: add three more hard-coded blocks in `init.rs`

This is the smallest immediate patch. `upsert_missing_config_keys` could gain
three string constants and append each when `has_section` returns false.

Advantages:

- Very small production diff.
- Matches the current scheduling implementation.
- Does not disturb `config.rs`.

Costs:

- Defaults remain duplicated between the resolver, template, and init strings.
- Descriptions have no reusable API for the dependent README ticket.
- The parsed-key coverage criterion would need a separate inventory.
- A future key could again be parsed without appearing in the upsert table.
- This repeats the exact structural condition behind the ticket.

Decision: reject. It closes the three observed gaps but not the required
single-source contract.

## Option 2: parse and rewrite TOML through a document editor

A format-preserving TOML editor could inspect tables and insert missing items
through syntax-tree operations.

Advantages:

- Strong TOML-aware section and assignment detection.
- Easier support for adding a missing key to any arbitrary existing table.
- Could preserve comments more reliably than line matching in unusual syntax.

Costs:

- Adds a dependency or a new editing abstraction for a narrow append task.
- Exact byte-prefix preservation becomes harder to demonstrate.
- Formatting decisions could move whitespace or comments.
- Existing behavior intentionally uses an auditable append-only line merger.
- The ticket does not require normalizing unusual TOML layouts.

Decision: reject. It increases mutation risk at the user-owned file boundary.

## Option 3: generate the entire file from typed config

The existing config could be parsed, merged with defaults, then emitted from a
canonical template.

Advantages:

- Simple complete output.
- Every known setting could be shown consistently.
- Idempotence would be straightforward after the first rewrite.

Costs:

- Destroys user comments and ordering.
- Re-values omitted settings into active values unless carefully commented.
- Violates the explicit append-only discipline.
- A first re-init would never preserve existing bytes.

Decision: reject. It contradicts the central acceptance requirement.

## Option 4: one metadata catalog plus the existing merger

Define a `ConfigKey` record in `config.rs` with:

- a stable dotted path;
- the containing TOML section;
- the leaf key;
- the TOML spelling of its default; and
- one plain-English description sentence.

Store all 17 parsed CLI configuration keys in a static catalog. Use that
catalog for validation's known-key checks, fresh-template rendering, and init
stub rendering. Keep the current line-oriented presence scanner and insertion
logic.

Advantages:

- One source owns key names, defaults, and descriptions.
- The dependent ticket can render README rows from the same records.
- Validation no longer maintains independent per-section key arrays.
- Existing append-only behavior remains intact.
- The table is testable without filesystem or process setup.
- Default strings are valid TOML and can be reused verbatim.

Costs:

- The production diff spans both `config.rs` and `init.rs`.
- Map-valued settings need a useful inert default spelling.
- The fresh template must be assembled partly from metadata.
- Table inventories in Rust cannot be derived automatically from Serde fields.

Decision: choose this option. It meets both the immediate behavior and the
dependent ticket's source contract while retaining the safest existing merger.

## Catalog shape

Use a public-within-crate immutable type:

```text
ConfigKey {
    path,
    section,
    key,
    default,
    description,
}
```

The dotted `path` is the stable identity for tests and future README rows.
Top-level `version` has an empty section. All other records use their exact TOML
section. `phase_timeouts` and `provider_caps` are represented as parent config
keys, not as every possible map member.

The catalog remains in `config.rs` because that module owns:

- input field names;
- validation;
- effective defaults; and
- fresh `.lisa.toml` generation.

Putting the table in `init.rs` would make config metadata depend on one consumer.
Putting it in `templates.rs` would mix operator configuration with installation
assets and hooks.

## Default spellings

Every `default` value is a TOML right-hand side:

- strings include quotes;
- booleans use `true` or `false`;
- numeric values use decimal text;
- empty optional maps use `{}`;
- version uses the compile-time package version in quotes.

This makes `# key = default` both human-readable and directly uncommentable.
It also avoids examples that accidentally imply non-default phase timeouts or
provider caps.

## Description voice

Each description is a sentence that:

- starts with a direct verb such as `Chooses`, `Limits`, `Lets`, `Tracks`, or
  `Controls`;
- ends with a period;
- names the operator-visible outcome;
- avoids the repository's banned marketing and internal mechanism terms;
- stays on one line; and
- does not restate TOML syntax.

The completion description follows the ticket's supplied meaning: finished
work is sealed, and `auto` chooses the strongest supported method.

## Appended section policy

For `[agent]`, `[guards]`, and `[triage]`, append a block only when the section
header itself is absent in both active and commented form.

Each block is:

```text
# [section]
# Description sentence.
# key = default
```

Commenting the header provides the strongest inertness guarantee: generic TOML
and typed config both parse to the same values as before. It also preserves the
operator's ability to enable a block by removing comment markers.

If a section already exists, init does not append a second copy. Customized
values and user prose inside it remain untouched. This ticket does not attempt
to fill a partially populated agent, guards, or triage section because the
requirement is to append genuinely absent sections.

## Scheduling policy

Scheduling retains its established per-key behavior because older files often
already contain `[scheduling]` but lack later optional keys.

- Missing scalar or map parent keys are inserted at the section end.
- Each insertion contains its catalog description followed by its commented
  default assignment.
- Active and commented assignments both count as present.
- A missing scheduling section is not newly synthesized by this path; fresh
  files already receive it from the template.
- The old special phase-timeout example block is replaced by the catalog's
  inert empty-map spelling for consistency.

Repeated insertion changes the located section end, so each following record is
added after the preceding record. Catalog order therefore determines the new
stub order without moving any original line.

## Fresh template policy

The fresh template keeps active values for:

- version;
- directory locations; and
- `scheduling.max_threads`.

All optional settings render their description and commented default from the
catalog. Section headers in a fresh file may remain active because empty typed
tables do not change defaults and the template already owns the whole document.
The key property is that every optional assignment stays commented.

No alternative runtime, provider-cap, or phase-timeout examples are needed for
the acceptance criteria. The catalog's default form is the authoritative stub.

## Validation integration

Replace the manual per-section known-key arrays with catalog lookups. Keep the
existing semantic validation and warning wording.

- A top-level scalar is known when its catalog section is empty.
- A top-level table is known when any catalog record belongs to it.
- A table leaf is known when its section and key match a catalog record.
- Nested phase/provider members retain their existing specialized checks.

This makes a missing metadata record externally visible as an unknown-key
warning even if a future typed field is added. Tests then pin the complete
accepted fixture against the catalog.

## Test design

Add catalog tests that:

- parse one full fixture containing every fixed input field;
- recursively enumerate its fixed parent paths;
- compare that inventory to catalog paths;
- assert path uniqueness;
- validate every default assignment as TOML;
- assert every description is one line, sentence-terminated, verb-forward, and
  free of banned voice terms.

Add init tests that:

- start from a dirs-and-scheduling legacy fixture;
- assert the original fixture is an exact output prefix;
- assert all three missing sections appear as commented stubs;
- assert every appended key uses catalog text;
- compare effective config before and after;
- assert a catalog-current fixture is byte-identical;
- preserve customized values and user comments; and
- prove a second upsert is a no-op.

## Risks and mitigations

- Risk: default strings drift from runtime defaults.
  Mitigation: focused assertions compare catalog defaults with resolved values.
- Risk: a description contains hidden newlines.
  Mitigation: line-count checks.
- Risk: substring key matching confuses prefix keys.
  Mitigation: existing scanner requires `=` immediately after optional spaces.
- Risk: appended blocks concatenate onto a non-newline-terminated file.
  Mitigation: a dedicated append helper adds exactly the needed separators.
- Risk: broad template changes surprise snapshots.
  Mitigation: run all CLI tests and inspect focused diffs.

## Final decision

Implement the immutable 17-row catalog in `config.rs`, consume it from
validation, fresh template generation, and init's append-only merger, and cover
the preservation, inertness, completeness, and voice contracts in colocated
unit tests.
