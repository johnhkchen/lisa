# Design: T-050-03-02 flag-and-question-audit

## Outcome

Create one readable knowledge document backed by a narrow unit-test verifier.
The document will use stable machine-readable row identifiers inside ordinary
Markdown tables.
The verifier will derive Lisa-owned long flags from `Cli::command()` and fixed
configuration paths from `config::CONFIG_KEYS`, then require an exact audit row
for every derived identifier.
Prompt rows will be explicit and manually reviewed because the ticket does not
require code discovery for prompts and they span native and WASM crates.

## Decision drivers

The design must satisfy five properties at once:

1. A future flag added anywhere in the nested Clap tree fails CI.
2. A future fixed config key added to the parser-backed catalog fails CI.
3. The document remains useful to an operator rather than becoming test data.
4. Default and ask evidence is visible per row.
5. A missing-row failure is demonstrated without temporarily breaking the real
   audit document.

The current command schema is private to `main.rs`.
The current config catalog is private to `config.rs` but visible to its parent
module and binary-crate unit tests.
The smallest test location with access to both is a `#[cfg(test)]` module in
`main.rs`.

## Option 1: Parse Rust source text

The test could scan `main.rs` for `#[arg(...)]` and scan config structs for fields.

Advantages:

- no production visibility changes;
- can be implemented as an integration test;
- finds declaration spelling directly.

Disadvantages:

- attribute formatting is not a stable grammar;
- derive defaults such as inferred kebab-case long names require reimplementing
  Clap behavior;
- nested/global behavior is easy to misread;
- source scanning does not satisfy the criterion's “enumerates the clap command
  tree” wording;
- config structs contain resolved/internal fields that are not operator keys.

Decision: reject.

## Option 2: Move the entire CLI schema into the library

The command enums could become public library types and an integration test could
call `Cli::command()`.

Advantages:

- integration tests can inspect the exact generated tree;
- command schema becomes reusable;
- the audit logic can live outside the binary target.

Disadvantages:

- expands production API solely for a test;
- requires moving dispatch-adjacent types or widening many visibilities;
- creates a public compatibility promise not required by the ticket;
- risks a large diff in the epic's documentation closer.

Decision: reject.

## Option 3: Add a production audit/catalog module

A new module could expose normalized flag records and validate a document at
runtime or test time.

Advantages:

- central reusable representation;
- clean separation between parsing and policy;
- could support a future `lisa audit` command.

Disadvantages:

- duplicates information already available from Clap;
- risks the second catalog drifting from the command tree;
- adds shipped code for a CI-only standing bar;
- a runtime command is outside ticket scope.

Decision: reject.

## Option 4: Unit-test the generated tree in `main.rs`

Add a test-only module beside `Cli` and `Commands`.
It imports `clap::CommandFactory`, builds the generated command tree, recursively
walks commands and arguments, and compares normalized IDs with the document.

Advantages:

- directly exercises the derived Clap model;
- retains private production types;
- sees hidden commands as well as everyday commands;
- sees nested `notes` and `proposal` commands;
- requires no shipped behavior changes;
- can consume `config::CONFIG_KEYS` directly.

Disadvantages:

- binary unit tests are less discoverable than an integration test;
- parsing Markdown in a test needs a deliberately small convention;
- generated help/version entries need an explicit scope rule.

Decision: choose.

## Flag identity

Each Lisa-owned flag row will have an ID shaped as:

`flag:lisa[/subcommand[/nested-subcommand]]:--long-name`

Examples:

- `flag:lisa/init:--dry-run`;
- `flag:lisa/notes:--path`;
- `flag:lisa/proposal/apply:--path`;
- `flag:lisa/commit-ticket:--include`.

Command names come from Clap, not Rust enum variant names.
Long names come from `Arg::get_long()`.
The full command path prevents repeated flags such as `--path`, `--client`, and
`--dry-run` from collapsing into one row.
Inherited global arguments are cataloged at their declaration command, not copied
onto every descendant invocation.
That makes `notes --path` one control with one row even though Clap permits it
around `notes ack`.

The verifier will ignore positional arguments because the criterion's executable
boundary names flags and keys, not operands.
It will ignore Clap-generated help/version arguments while walking children.
Those framework controls will be documented once in a small framework row or
introductory note, because they are not Lisa-authored choices and cannot silently
proliferate from a Lisa code change.
All explicit long arguments in the derived schema remain in scope, including
hidden machine commands.

## Config identity

Each config row will have an ID shaped as:

`config:<dotted-path>`

Examples:

- `config:agent.client`;
- `config:scheduling.max_threads`;
- `config:version`.

The expected set comes from `CONFIG_KEYS`.
That catalog is already proven equal to the complete fixed parsed-key fixture by
`config_catalog_covers_every_parsed_key_exactly_once`.
Using it composes existing evidence instead of reflecting serde fields twice.
Dynamic children of `phase_timeouts` and `provider_caps` stay represented by
their fixed parent-map rows, matching `CONFIG_KEYS` semantics.

## Prompt identity

Prompt rows will use:

`prompt:<stable-name>`

The current set is:

- `prompt:init-project-history`;
- `prompt:dashboard-mark-done`;
- `prompt:dashboard-reset-ticket`;
- `prompt:dashboard-quit-pending`.

The first is a terminal yes/no prompt with a working yes default.
The dashboard rows are explicit action confirmations or selectors.
They remain manual inventory because no shared prompt registry exists across the
native CLI and WASM UI.
Their evidence column cites named tests that render or exercise each surface.

## Row schema

The document will contain separate tables for flags, prompts, and config keys.
Every machine-covered table uses the same leading columns:

| Column | Meaning |
| --- | --- |
| ID | Stable machine-readable identity in backticks |
| Surface | Human command, question, or dotted key |
| Bar | `working default` or `justified ask` |
| Default / justification | Plain operator-facing sentence |
| Fixture | Named test that pins a default or behavior |
| Category | Empty for defaults; allowed ask category otherwise |

The verifier only treats rows whose first cell begins with backticked `flag:` or
`config:` as executable inventory rows.
This avoids coupling to headings or explanatory tables.
Duplicate IDs are errors.
Missing expected IDs are errors.
Unexpected flag/config IDs are errors because stale rows are as misleading as
missing rows.

Every default row must have a nonempty fixture cell.
Every ask row must carry either `destructive/irreversible` or `expert override`.
The verifier will validate these structural rules for all machine-covered rows.
Prompt rows will use the same schema, allowing the same structural validator to
check their evidence/category shape even though discovery is manual.

## Defaults versus asks

A row is a working default when omission has useful, deterministic behavior for
the expected user.
Examples include `--path` choosing `.`, boolean switches staying off, bare init
choosing the strongest available history path, and unconfigured `agent.client`
following PATH detection.

A row is a justified ask when no general default can supply the required command
operand or when the control intentionally overrides a safe default.
Expert and machine-facing overrides use `expert override`.
Questions that can discard or finalize work use `destructive/irreversible`.
Required transaction and lease values are recorded as expert/machine protocol,
not presented as everyday defaults.

The operator-facing sentence will describe effect in direct language.
It will avoid DAG, orchestration, scheduling, leverage, solutions, deployment,
case study, build log, and research release jargon.
The test will check the relevant cell for one line and banned vocabulary.

## Default fixture strategy

The audit can cite existing focused tests wherever they already pin semantics:

- help snapshots for common Clap defaults;
- init history tests for the terminal/nonterminal choice;
- config resolution tests for config defaults;
- runtime and client autodetection tests for environment-derived defaults;
- UI renderer/scheduler tests for dashboard prompts.

For repetitive declared defaults, the new live-tree audit test itself may be the
pinning fixture only if it compares the documented value against Clap metadata.
The verifier will therefore normalize explicit Clap defaults and boolean implicit
defaults, then compare them with a `Default:` prefix in the row.
Rows whose true default is resolved below Clap cite the focused semantic test and
are not forced to mirror empty Clap metadata.

## Negative fixture

Add `crates/lisa-cli/tests/fixtures/flag-audit-missing-row.md`.
It will be a minimal syntactically valid audit fragment that intentionally omits
one known flag and one known config key.
The negative test will run the same verifier against it and assert that the error
names both missing IDs.
This demonstrates the future failure mode without editing the real audit doc.

The fixture should not be a stale copy of the full audit because that would add a
second inventory to maintain.
It can contain one representative valid row and rely on the verifier's missing-set
diagnostic for everything else, then assert representative omissions.

## Proposed follow-up section

The document will always include a `Proposed for follow-up` section.
If every current row passes, it will say that no current surface fails the bar.
If research during implementation exposes a row with neither a defensible default
nor an allowed justification, that row will be repeated there with one direct
rationale.
The ticket itself will not remove the surface or mint another ticket.

## Verification

The focused verifier tests must prove:

- the real document exactly covers all explicit live-tree long flags;
- the real document exactly covers `CONFIG_KEYS`;
- no executable row is duplicated;
- every row has a valid bar/evidence/category shape;
- operator-facing cells pass local voice checks;
- the negative fixture reports missing live rows.

Then run the relevant CLI unit target, the full CLI crate tests, formatting, and
the workspace suite if time permits.
The knowledge doc and the test/fixture form one meaningful implementation unit
because neither independently satisfies the ticket.
