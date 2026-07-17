# Review: T-050-03-02 flag-and-question-audit

## Disposition

Pass.

The implementation satisfies both acceptance criteria.
It inventories the complete current surface, enforces future flag/config
additions through CI, demonstrates the missing-row failure with a negative
fixture, checks the required policy/evidence shape, and leaves no silent
follow-up disposition.

Ticket-owned source is committed in:

`8ff372cda5f71b1bd0576f7326c5298f0f927941`

Commit subject:

`Add executable flag and question audit`

## Files changed

### `docs/knowledge/flag-audit.md`

New 142-line knowledge document.

It defines:

- the expected user;
- the choose-by-default rule;
- `working default` versus `justified ask`;
- the two allowed ask categories;
- the scope of framework flags, positional operands, hidden commands, and map
  children;
- the executable row-ID convention;
- the maintainer workflow for future additions.

It inventories:

- 53 Lisa-authored long flags from the current Clap tree;
- 17 fixed `.lisa.toml` keys from `CONFIG_KEYS`;
- 4 direct interactive prompt/modal surfaces;
- 74 total governed rows.

Each working-default row has a named Rust test fixture.
Each ask row has one plain justification and an allowed category.
The proposed-follow-up section explicitly says no current row fails the bar.

### `crates/lisa-cli/src/main.rs`

Added a 282-line `#[cfg(test)]` module only.
No production command behavior or visibility changed.

The module:

- derives the real Clap command tree with `Cli::command()`;
- fully builds it before inspection;
- recursively visits visible, hidden, and nested commands;
- extracts every Lisa-authored long flag;
- excludes generated help/version arguments and positional operands by explicit
  scope;
- uses command-qualified IDs so repeated flag names do not collapse;
- records whether Clap requires each flag;
- requires optional flags to be working defaults;
- requires required flags to be justified asks;
- reads all fixed config paths from `config::CONFIG_KEYS`;
- checks catalog uniqueness;
- requires the exact four current prompt IDs;
- parses the Markdown tables without a new runtime dependency;
- rejects duplicate, missing, and unexpected rows;
- validates evidence/category shape;
- checks the operator-facing columns against the established banned voice terms.

The two standing tests are:

- `flag_audit_covers_live_cli_config_and_prompts`;
- `flag_audit_missing_row_fixture_names_every_gap`.

### `crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`

New 12-line intentionally incomplete fixture.

It contains one valid flag row, one valid config row, and all four prompt rows.
It does not clone the full inventory.
The negative test runs it through the same verifier as the real document and
asserts that both a missing live flag and missing live config key are named.

## Acceptance criterion 1

> `docs/knowledge/flag-audit.md` covers every current flag, prompt, and config key;
> the enumeration test fails on any missing row (demonstrated with a negative
> fixture) and passes on the current set; each blessed default cites its pinning
> fixture by test name; each ask names its justification category.

Evidence:

- The positive verifier derives 53 flags and compares exact IDs.
- It derives 17 config keys from the same catalog already bound to the parsed-key
  fixture.
- It requires the 4 researched prompt IDs.
- The real document passes with 74 exact rows.
- The negative fixture is rejected and the test confirms its error names
  `flag:lisa/loop:--client` and `config:agent.client`.
- Duplicate and unexpected rows also fail, preventing stale inventory.
- Working-default rows without fixture names fail policy validation.
- Required flag rows not labeled `justified ask` fail.
- Ask rows outside `destructive/irreversible` and `expert override` fail.
- A workspace search confirmed every fixture named by the real audit corresponds
  to an actual Rust test function.

Assessment: satisfied.

## Acceptance criterion 2

> Any rows failing the bar appear under "proposed for follow-up" with a one-line
> rationale each — zero silent keeps; the doc passes the brand-voice check for its
> operator-facing column.

Evidence:

- Every machine-covered row must declare one of the two accepted bars.
- Every default must name evidence.
- Every ask must name an allowed category.
- Clap requiredness is cross-checked against the bar.
- The document contains an explicit `Proposed for follow-up` section.
- The section states that no current row fails, rather than remaining blank.
- The verifier checks both Surface and Default/justification columns for the
  established banned jargon set.
- It also requires one-line, punctuated rule copy.
- The positive document test passes.

Assessment: satisfied.

## Test coverage

### Audit-focused

Command:

```text
cargo test -p lisa-cli --bin lisa flag_audit -- --nocapture
```

Result: 2 passed, 0 failed.

This covers:

- positive exact inventory match;
- negative missing-row failure;
- flag and config diagnostics;
- prompt-set match;
- duplicate detection through the common parser path;
- row policy;
- required/optional flag classification;
- voice vocabulary.

### Full CLI crate

Command:

```text
cargo test -p lisa-cli
```

Result: all unit and integration targets passed.
The binary target reported 369 passed.
The library target reported 16 passed under that feature selection.
Every CLI integration test passed.
The existing real-Zellij delivery boundary remained ignored by its explicit
environment gate.

This run also executed every existing default fixture named by the audit that
lives in the CLI crate.

### Prompt/UI evidence

Commands:

```text
cargo test -p lisa-plugin test_modal_title --no-fail-fast
cargo test -p lisa-plugin test_reset_modal --no-fail-fast
```

Result: 4 focused tests passed.

These pin the mark-done/reset modal titles and reset selection behavior.
The init prompt/default fixtures ran in the CLI suite.

### Workspace

Command:

```text
cargo test --workspace
```

Result: zero failures.

Notable totals:

- 369 CLI binary tests;
- 21 CLI library tests with workspace features;
- 248 core unit tests;
- 2 core integration tests;
- 437 plugin tests;
- all CLI integration tests and doc tests;
- one pre-existing environment-gated real-Zellij test ignored.

### Formatting and hygiene

Passed:

```text
cargo fmt --all -- --check
git diff --check
```

The isolated commit contains exactly the three intended paths.
After commit, none of those paths is staged, modified, or untracked.
Unrelated Lisa journal, provenance, ticket phase, and published work-artifact
state remains outside the ticket-owned source commit.

## Review observations

The test lives in `main.rs` because `Cli` is deliberately private to the binary.
That avoids widening a production API or creating a second flag catalog.

Config enumeration composes with the pre-existing test that proves
`CONFIG_KEYS` equals the complete parsed fixed-key fixture.
The new test does not reflect serde structs independently, so there is only one
config catalog to maintain.

Prompt discovery is manual rather than reflected from code.
This is consistent with the ticket's explicit executable requirement, which
names the Clap tree and parsed config keys.
The four prompt IDs are still exact and policy-checked, and their code boundaries
were researched across both native CLI and WASM UI.

Generated `--help` and top-level `--version` are documented once and remain pinned
by the existing top-level help snapshot.
They are not repeated for every subcommand because Clap synthesizes them and they
cannot proliferate independently through a Lisa flag declaration.

Required machine protocol values use `expert override` because they are exact
caller-supplied lease, transaction, routing, or evidence boundaries, not everyday
operator questions.
This keeps them visible without pretending there is a safe inferred default.

## Open concerns

No blocking concerns.

One bounded limitation remains visible: prompts do not share a cross-crate
registry, so future prompt additions rely on a maintainer updating the explicit
four-ID set. The acceptance criterion requires executable enumeration only for
flags and config keys, and those standing gates are complete.

No current row was proposed for follow-up.
No surface was removed, no new ticket was minted, and no product default changed.

## Handoff

The work is ready for Lisa's completion transaction.
Lisa should publish the admitted RDSPI artifacts, update Done state, and release
the seat only after the completion commit is confirmed.
