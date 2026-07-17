# Structure: T-050-03-02 flag-and-question-audit

## Change set

The implementation changes three ticket-owned repository paths:

1. Create `docs/knowledge/flag-audit.md`.
2. Modify `crates/lisa-cli/src/main.rs` with a test-only audit verifier.
3. Create
   `crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.

No production command, parser, config default, or dashboard behavior changes.
No existing file is deleted.
The RDSPI artifacts remain in the attempt work directory and are published by
Lisa, not included in source commits by the agent.

## `docs/knowledge/flag-audit.md`

### Purpose and audience

The document is the human inventory and the machine fixture.
It addresses operators deciding whether Lisa asks too much and maintainers adding
new flags or keys.
The opening section states the rule in plain language:

- Lisa chooses when the expected answer is clear;
- an override stays available for expert control;
- a question remains only when it protects work or represents a real expert
  choice;
- CI rejects uncataloged flags and fixed config keys.

### Scope section

Define what is counted:

- every explicit long flag in the derived Lisa Clap tree, including hidden
  plumbing commands;
- every fixed `.lisa.toml` path in `CONFIG_KEYS`;
- direct native questions and dashboard confirmation/selection modals;
- top-level framework `--help`/`--version` as documented framework behavior, not
  Lisa-owned proliferation;
- no positional command operands;
- no arbitrary children of map-valued config keys.

State the two accepted ask categories exactly:

- `destructive/irreversible`;
- `expert override`.

### CLI flag table

One row per explicit long flag declaration.
Rows are grouped by everyday commands, nested operator commands, and hidden
plumbing commands for readability.
Each row uses six columns:

1. `ID`;
2. `Surface`;
3. `Bar`;
4. `Default / justification`;
5. `Fixture`;
6. `Category`.

The ID is an exact backticked token such as
`flag:lisa/loop:--max-threads`.
The surface cell is operator-facing prose, not a code parser input.
The bar is exactly `working default` or `justified ask`.
The fixture cell names a Rust test function without requiring a source link.
The category cell is `—` for defaults and one allowed category for asks.

Repeated spellings receive distinct rows at their declaration paths.
For example, `loop --client` and `triage-agent --client` differ because the first
is an optional operator override and the second is a required internal routing
value.

### Prompt table

Four rows:

- `prompt:init-project-history`;
- `prompt:dashboard-mark-done`;
- `prompt:dashboard-reset-ticket`;
- `prompt:dashboard-quit-pending`.

The init row is a working-default row and cites the prompt default test.
Mark-done is a destructive/irreversible confirmation because it requests durable
completion.
Reset is a destructive/irreversible confirmation because it rewrites progress
state and can discard the current run.
Quit is a destructive/irreversible confirmation because it can stop work that is
still pending.
All three dashboard rows cite UI or scheduler tests.

### Config table

Seventeen rows, one for every current `CONFIG_KEYS` path.
Each ID is `config:<path>`.
Defaults use the catalog's existing TOML values or the actual semantic resolution
where the catalog value is a documented override stub.
Environment-derived choices such as `agent.client` and `runtime.zellij` explain
the decision path and cite their focused detection/runtime fixtures.
Expert controls remain labeled as working defaults when omission already produces
the expected safe behavior; the config key itself is an override but does not
force a question.

### Proposed follow-up

Always present.
The initial content is either a list of failing rows with one-line rationales or a
single direct sentence saying no current row fails.
No new ticket IDs are minted here.

### Maintainer note

Briefly explain the executable convention:

- keep the ID cell exact;
- add a row in the same change as a new flag/key;
- name a fixture for a claimed default;
- use an allowed category for a justified ask;
- place anything that clears neither bar in proposed follow-up.

## `crates/lisa-cli/src/main.rs`

### Test module boundary

Append `#[cfg(test)] mod flag_audit_tests` after existing functions/tests.
The module imports:

- `super::*`;
- `clap::CommandFactory`;
- ordered set/map collections;
- `std::path::Path` as needed.

The module embeds the real audit document with an absolute-at-compile-time path:

```text
include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/knowledge/flag-audit.md"
))
```

It embeds the negative fixture from the crate-local `tests/fixtures` directory.

### `AuditRow`

A private test-only struct owns parsed fields:

- `id: String`;
- `surface: String`;
- `bar: String`;
- `rule: String`;
- `fixture: String`;
- `category: String`.

It derives `Debug` and equality traits only if useful to assertions.

### `parse_audit_rows`

Input: `&str` Markdown.
Output: `Result<BTreeMap<String, AuditRow>, String>`.

Behavior:

1. Iterate lines.
2. Consider only pipe-table lines.
3. Split on `|` and trim cells.
4. Require six cells for candidate executable rows.
5. Remove one surrounding backtick pair from the ID cell.
6. Select IDs beginning with `flag:`, `config:`, or `prompt:`.
7. Reject duplicate IDs with the ID in the error.
8. Return rows keyed by ID.

Markdown cells must not contain literal `|` characters.
Descriptions can use code spans, punctuation, and slashes safely.

### `collect_flag_ids`

Input: `clap::Command` from `Cli::command()`.
Output: `BTreeSet<String>`.

Recursive behavior:

1. Begin with path `lisa`.
2. Inspect `get_arguments()` on the current command.
3. Skip arguments whose action is Clap help/help-long/version.
4. For each remaining argument with `get_long()`, emit
   `flag:<path>:--<long>`.
5. Recurse through `get_subcommands()` except Clap's generated `help`
   subcommand.
6. Extend path with the child command name separated by `/`.

The command should be fully built before traversal so derived long names and
nested schemas are stable.
Global arguments are recorded at their defining path.
If Clap propagation creates a duplicate identical ID, the set naturally folds it;
duplicates across different paths remain distinct.

### `collect_config_ids`

Map `config::CONFIG_KEYS` to `config:<path>` in a `BTreeSet`.
Also assert the set length equals `CONFIG_KEYS.len()` so a duplicated catalog path
does not become invisible here, even though the config module already has its own
duplicate check.

### `validate_row_policy`

For each parsed row:

- require a one-line nonempty surface;
- require `bar` to be one of the two values;
- require nonempty direct rule prose ending in punctuation;
- require a nonempty fixture for every working default;
- require category `—` for a working default;
- require an allowed category for every justified ask;
- reject the known operator-facing banned jargon in `surface` and `rule` cells.

Fixture cells may contain multiple comma-separated test names.
The validator checks presence, not whether Rust can reflect all workspace test
names.

### `verify_audit`

Input: Markdown text.
Output: `Result<(), String>`.

Compose the other helpers:

1. Parse rows.
2. Validate row policy for all three namespaces.
3. Compare the parsed `flag:` ID set to the live Clap set.
4. Compare the parsed `config:` ID set to the catalog set.
5. Require the four known prompt IDs exactly.
6. Format missing and unexpected sets into one deterministic diagnostic.

Do not stop at the first coverage mismatch.
The negative fixture must be able to show both a missing flag and config key in
one returned error.

### Positive test

Name: `flag_audit_covers_live_cli_config_and_prompts`.

Run `verify_audit(FLAG_AUDIT)` and panic with its full diagnostic.
This is the standing CI gate and a citeable fixture for the inventory itself.

### Negative test

Name: `flag_audit_missing_row_fixture_names_every_gap`.

Run the same verifier against the negative fixture.
Require failure.
Assert the message contains representative missing IDs from both namespaces,
including at least one current flag and one current config key.
This proves the exact verifier used by the positive test fails closed.

## Negative fixture structure

The fixture contains:

- a heading saying it is intentionally incomplete;
- the exact six-column header;
- one valid flag row;
- one valid config row;
- all four valid prompt rows, or enough prompt rows that prompt errors do not
  obscure the flag/config assertions.

It must not duplicate the whole production audit.
It must remain obviously invalid to a human reader.

## Dependency direction

```text
Cli derive tree --------------------+
                                     +--> main.rs test verifier --> flag-audit.md
config::CONFIG_KEYS ----------------+

negative fixture ----------------------> same verifier --> expected failure
```

Production code never depends on the knowledge document.
The document is compile-time input only for tests.
The config parser continues to depend on `CONFIG_KEYS` exactly as before.

## Commit boundary

The doc, verifier, and negative fixture form one meaningful source unit because
the test cannot pass until the document is complete and the document alone is not
CI-enforced.
Commit them together with exact include paths:

- `docs/knowledge/flag-audit.md`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.

No RDSPI artifact is included in this ticket-owned source commit; Lisa owns their
publication and final completion transaction.
