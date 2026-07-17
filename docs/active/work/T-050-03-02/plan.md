# Plan: T-050-03-02 flag-and-question-audit

## Goal

Land a complete, plain-language inventory of Lisa's flags, direct questions, and
fixed config keys, with CI proof that new Clap flags or config keys cannot ship
without an audit row.

## Preconditions

- Treat `docs/active/tickets/T-050-03-02.md` as Lisa-owned lease state.
- Preserve existing `.lisa` journal and provenance modifications.
- Write phase artifacts only through the attempt work directory.
- Use no ordinary `git add` or `git commit`.
- Commit source paths only through `lisa commit-ticket`.

## Step 1: Add the test-only audit parser

Modify `crates/lisa-cli/src/main.rs`.

Add a `#[cfg(test)]` module with:

- compile-time inclusion of the final knowledge document;
- compile-time inclusion of the negative fixture;
- an `AuditRow` representation;
- a Markdown table-row parser;
- deterministic duplicate diagnostics.

Verification after this substep:

- `cargo test -p lisa-cli --bin lisa flag_audit --no-fail-fast`

Expected interim state:

- compilation may fail until fixture/doc paths exist;
- once paths exist, positive coverage may fail with a complete list of missing
  row IDs;
- parser unit behavior is exercised through the positive and negative tests.

## Step 2: Derive the live flag set

In the same test module, import `clap::CommandFactory` and recursively traverse
`Cli::command()`.

For every command:

1. construct the stable slash-separated command path;
2. inspect every long argument;
3. exclude Clap-generated help/version actions;
4. emit one `flag:<path>:--<long>` ID;
5. recurse into non-generated subcommands.

Add deterministic comparisons between expected IDs and document IDs.
Report both missing and unexpected rows.

Verification:

- temporarily run the positive test against the not-yet-complete doc;
- use its missing-set diagnostic to confirm hidden commands and nested proposal
  commands are present;
- cross-check the resulting count against `main.rs` declarations with
  `rg -n "#\[arg" crates/lisa-cli/src/main.rs`;
- inspect any difference instead of forcing counts to match, because one
  attribute may describe a positional rather than a flag.

## Step 3: Derive the fixed config set

Map `config::CONFIG_KEYS` to `config:<dotted-path>`.
Assert no path collapses due to duplication.
Compare that set to document rows using the same diagnostic as flags.

Verification:

- confirm the expected set has 17 entries;
- run
  `cargo test -p lisa-cli config_catalog_covers_every_parsed_key_exactly_once`;
- ensure the audit test reports a missing config row before the doc is filled.

## Step 4: Enforce row policy and voice

Add structural validation for all `flag:`, `config:`, and `prompt:` rows.

Require:

- exactly one row per ID;
- nonempty one-line surface prose;
- bar value `working default` or `justified ask`;
- a complete default/justification sentence;
- a named fixture for every working default;
- category `—` for a default;
- category `destructive/irreversible` or `expert override` for an ask;
- no banned operator-facing jargon in surface or rule prose.

Require the exact four current prompt IDs.

Verification:

- use focused parser assertions if necessary;
- confirm malformed rows return named errors rather than panicking;
- keep diagnostics deterministic with ordered collections.

## Step 5: Create the negative fixture

Create
`crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.

Make it intentionally incomplete while keeping its included rows structurally
valid.
Include all four prompt IDs so the negative test demonstrates machine-discovered
flag/config gaps without depending on a separate prompt omission.

Add `flag_audit_missing_row_fixture_names_every_gap`.
Assert the returned error names:

- a representative omitted live flag;
- a representative omitted fixed config path.

Verification:

- run only the negative test;
- confirm it passes because the verifier rejects the fixture;
- ensure no branch uses a separate/weaker validation implementation.

## Step 6: Write the CLI flag inventory

Create `docs/knowledge/flag-audit.md` with the governing rule and scope.
Populate one row for every ID emitted by the live-tree diagnostic.

For each row:

- identify whether omission provides a useful default;
- for a working default, name the value/decision and a test that pins it;
- for an expert or machine override, explain why it remains available;
- use the exact ask category;
- describe the effect in direct operator language.

Explicitly cover hidden plumbing flags.
Document framework `--help`/`--version` behavior in scope prose even though the
derived Lisa-owned inventory excludes auto-generated arguments.
Do not add positional operands as fake flag rows.

Verification:

- run the positive audit test;
- correct every missing/unexpected/duplicate ID;
- inspect the table visually for concise consistent copy.

## Step 7: Write the prompt inventory

Add the four prompt rows.

For init project history:

- state the empty-input `yes` default;
- cite `project_history_prompt_accepts_defaults_and_retries_invalid_answers`;
- mention automatic nonterminal decision in prose.

For mark done, reset, and quit:

- record the safety justification;
- use `destructive/irreversible`;
- cite renderer or scheduler fixtures that pin the surface/effect.

Verification:

- run the CLI audit test for row policy;
- run focused CLI init prompt tests;
- run focused plugin modal tests.

## Step 8: Write the config inventory

Add all 17 `CONFIG_KEYS` rows in catalog order.
Use catalog defaults and semantic resolution evidence.

Pay special attention to:

- `agent.client`, whose no-key behavior now detects PATH;
- `runtime.zellij`, whose default is managed selection;
- `guards.completion`, whose `auto` mode selects the strongest supported seal;
- map-valued phase/provider overrides;
- version as setup/protocol metadata rather than an operator choice.

Verification:

- positive audit test passes exact set comparison;
- config catalog/README tests still pass;
- no row claims a default contradicted by `ResolvedConfig` or its focused tests.

## Step 9: Record proposed follow-up disposition

Review every row against the bar.

If a row has neither a useful default nor an allowed one-line justification:

- repeat it under `Proposed for follow-up`;
- add one line explaining the gap;
- do not remove the surface;
- do not mint a ticket in this task.

If all rows clear the bar, write one direct sentence saying there are no current
follow-up proposals.

Verification:

- search for placeholder text and empty cells;
- ensure every justified ask has an allowed category;
- ensure there are zero silent exceptions.

## Step 10: Focused test pass

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli --bin lisa flag_audit --no-fail-fast
cargo test -p lisa-cli config_catalog_covers_every_parsed_key_exactly_once
cargo test -p lisa-cli project_history_prompt_accepts_defaults_and_retries_invalid_answers
cargo test -p lisa-plugin test_modal_title --no-fail-fast
```

If a filter unexpectedly matches zero tests, rerun with the exact test name or
the containing target and record the corrected command in `progress.md`.

## Step 11: Crate and workspace verification

Run:

```text
cargo test -p lisa-cli
cargo test -p lisa-plugin
cargo test --workspace
```

The workspace run may duplicate crate coverage but is the final integration gate.
Do not build the WASM release artifact; this ticket changes no shipped behavior
and the repository guidance reserves building for development needs, while native
tests compile the modified target.

Inspect `git diff --check` for whitespace errors.
Inspect the exact ticket-owned diff.

## Step 12: Write progress and commit the source unit

Write `progress.md` before committing, recording:

- implemented files;
- inventory counts;
- test commands and outcomes;
- deviations from this plan;
- remaining Review work.

Confirm the ordinary index has no ticket-owned paths staged.
Commit the one meaningful implementation unit:

```text
lisa commit-ticket \
  --ticket-id T-050-03-02 \
  --message "Add executable flag and question audit" \
  --include docs/knowledge/flag-audit.md \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md
```

After the command, inspect status and the commit diff.
No ticket-owned source file may remain modified, staged, or untracked.

## Step 13: Review

Write `review.md` in the attempt work directory.
Cover:

- exact file changes;
- inventory counts and scope decisions;
- positive and negative executable evidence;
- focused and workspace test coverage;
- any prompt-discovery limitation;
- open concerns or lack thereof;
- acceptance-criterion mapping.

Write exactly:

```json
{"disposition":"pass","reason":null}
```

only if all implementation checks pass and source changes are committed.
Otherwise write an actionable block disposition matching the workflow schema.

Run:

```text
lisa check-disposition T-050-03-02
```

Correct every issue it reports.
Remain on this ticket after Review; do not update phase/status or begin other work.
