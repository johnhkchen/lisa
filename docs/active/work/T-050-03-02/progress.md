# Progress: T-050-03-02 flag-and-question-audit

## Status

Implementation is complete, verified, and committed through Lisa's isolated
transaction.
Review remains.

## Completed: executable inventory verifier

Modified `crates/lisa-cli/src/main.rs` with a test-only `flag_audit_tests` module.
No production dispatch or CLI behavior changed.

The module now:

- embeds `docs/knowledge/flag-audit.md` at compile time;
- embeds the intentionally incomplete negative fixture;
- parses machine-readable rows from ordinary Markdown tables;
- rejects duplicate IDs;
- recursively builds and walks `Cli::command()`;
- includes hidden commands and nested command trees;
- ignores generated help/version arguments and positional operands;
- records inherited `notes --path` at the effective nested path where Clap exposes
  it;
- derives exact IDs for every Lisa-authored long flag;
- records whether Clap requires each flag;
- requires required flags to be `justified ask` rows;
- requires optional flags to be `working default` rows;
- maps every `config::CONFIG_KEYS` path to an exact config ID;
- verifies the config catalog has no collapsed duplicate path;
- requires the four manually identified prompt IDs exactly;
- rejects missing and unexpected rows with one ordered diagnostic;
- validates default fixtures and allowed ask categories;
- validates complete one-line operator-facing rule text;
- rejects the established banned brand-voice vocabulary.

The positive test is:

`flag_audit_covers_live_cli_config_and_prompts`

The negative test is:

`flag_audit_missing_row_fixture_names_every_gap`

## Completed: negative fixture

Created
`crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.

The fixture is deliberately small rather than a second copy of the full
inventory.
It contains:

- one valid live flag row;
- one valid live config row;
- all four structurally valid prompt rows.

It omits the rest of the live flag and config sets.
The negative test sends this fixture through the exact same `verify_audit`
function as the real document.
It asserts that the diagnostic names both:

- `flag:lisa/loop:--client`;
- `config:agent.client`.

This demonstrates that additions or omissions fail CI without weakening the
production audit check or temporarily editing the real document.

## Completed: knowledge inventory

Created `docs/knowledge/flag-audit.md`.

The document begins with the common-sense-default rule and defines the expected
user.
It distinguishes:

- working defaults;
- destructive/irreversible asks;
- expert overrides.

It explains scope for generated Clap help/version controls, positional operands,
and dynamic children of map-valued config keys.

The executable inventory contains:

- 53 live Clap flag rows;
- 17 fixed config-key rows;
- 4 interactive prompt rows;
- 74 total governed rows.

The flag inventory includes both the everyday command surface and every current
hidden/machine-facing long flag.
Repeated long names remain distinct by command path.
Examples include the separately governed `loop --client` and internal
`triage-agent --client` controls.

Every optional flag states what omission does and cites a named pinning fixture.
Every required flag states why the exact value must be carried and uses `expert
override`.
The derived Clap requirement check prevents required and optional controls from
being silently relabeled.

The prompt inventory covers:

- init project history;
- dashboard mark done;
- dashboard reset ticket;
- dashboard quit with pending work.

The init prompt records its Enter-means-yes default and nonterminal automatic
behavior.
The dashboard questions are categorized destructive/irreversible because they
finalize, replace, or stop work.

The config inventory follows `CONFIG_KEYS` order.
It includes PATH-derived client selection, managed runtime selection, automatic
completion sealing, triage defaults, all time limits, and both map-valued
overrides.
Every working default names a focused existing test.

The proposed-follow-up section is explicit.
No current row fails the bar, so it contains one direct zero-proposal statement
rather than a silent empty section.

## Evidence citation check

Extracted every fixture token from the real audit and searched the workspace for
an exact Rust test function definition.
All named fixtures exist.
Ask rows may use `—` because the acceptance criterion requires their category,
not a default-pinning fixture.

## Initial executable failure

The first positive run intentionally used an empty document skeleton.
Its failure reported:

- every 53 missing live flag ID;
- every 17 missing config ID;
- all 4 missing prompt IDs.

That output became the authoritative population list.
After the document was filled, both positive and negative tests passed.

## Focused verification

Passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli --bin lisa flag_audit -- --nocapture
```

Result: 2 audit tests passed.

Passed full CLI coverage:

```text
cargo test -p lisa-cli
```

Results included:

- 369 binary unit tests passed;
- 16 library unit tests passed in this feature selection;
- all CLI integration targets passed;
- one real-Zellij boundary test remained intentionally ignored by its existing
  environment gate;
- no failures.

Passed focused dashboard evidence:

```text
cargo test -p lisa-plugin test_modal_title --no-fail-fast
cargo test -p lisa-plugin test_reset_modal --no-fail-fast
```

Result: 2 modal-title and 2 reset-modal tests passed.

Passed workspace verification:

```text
cargo test --workspace
```

Results included:

- 369 CLI binary tests passed;
- 21 CLI library tests passed under workspace features;
- all CLI integration tests passed;
- 248 core tests passed;
- 2 core integration tests passed;
- 437 plugin tests passed;
- doc tests passed;
- the existing real-Zellij test remained ignored;
- zero failures.

Passed repository hygiene:

```text
git diff --check
```

## Plan deviations

The planned structure was retained: one doc, one main.rs test module, and one
negative fixture.

One implementation refinement strengthened the design.
The verifier records Clap's required bit and enforces the corresponding audit bar,
instead of only checking ID coverage.
This makes the test reject a required value documented as an automatic default or
an optional override documented as a forced question.

The plan listed separate focused config and init test commands.
The full `cargo test -p lisa-cli` run exercised those exact tests plus the rest of
the crate, so redundant filtered invocations were not needed after the full pass.

No product behavior was altered.
No plugin source change was needed for prompt enumeration.
Prompt discovery remains an explicit four-row closed inventory; executable
enumeration is required and implemented for Clap flags and parsed config keys, as
the ticket specifies.

## Commit unit

The meaningful source unit consists of exactly:

- `docs/knowledge/flag-audit.md`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.

These paths were committed together through `lisa commit-ticket` because the
document, positive verifier, and negative evidence jointly satisfy the ticket.
No ordinary-index command was used.

Command:

```text
lisa commit-ticket \
  --ticket-id T-050-03-02 \
  --message "Add executable flag and question audit" \
  --include docs/knowledge/flag-audit.md \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md
```

Commit: `8ff372cda5f71b1bd0576f7326c5298f0f927941`.

## Remaining

1. Confirm no ticket-owned source path remains staged, modified, or untracked.
2. Write `review.md`.
3. Write the Review disposition.
4. Run `lisa check-disposition T-050-03-02`.
5. Remain on this ticket for Lisa's completion commit.
