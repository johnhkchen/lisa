# Research: T-050-03-02 flag-and-question-audit

## Ticket boundary

The ticket is the closing task in E-050, `common-sense-defaults`.
It inventories the state left by all earlier epic tickets.
Its dependencies cover history selection, empty/setup output, configuration
cataloging, README synchronization, and agent-client detection.
The ticket creates no new product choice and removes no existing control.
Its durable product artifact is `docs/knowledge/flag-audit.md`.
Its executable boundary is a test that reads the live Clap command tree and the
parsed configuration catalog.
The acceptance criteria distinguish three kinds of surface:

- CLI flags;
- interactive prompts;
- `.lisa.toml` configuration keys.

Every audited row must state either a working default or why Lisa asks.
A default row must name the fixture or test that pins the behavior.
An ask row must name one of the epic's allowed categories.
Rows that do not clear either bar must be visible under proposed follow-up.

## CLI command definition

`crates/lisa-cli/src/main.rs` owns the entire Clap tree.
`Cli` derives `Parser` and has a single `Commands` subcommand field.
`Commands` derives `Subcommand` and contains both operator and plumbing commands.
Nested command enums are `NotesCommands` and `ProposalCommands`.
The command structs are private to the binary crate.
Because `Cli` is private, an integration test cannot call `Cli::command()` without
moving the command schema into the library.
A unit test in `main.rs` can access `Cli`, every nested command, and private
`config::CONFIG_KEYS` without changing production visibility.
Clap's `CommandFactory` trait exposes the derived tree as `clap::Command`.
Each command exposes arguments through `get_arguments()`.
Each nested command exposes its children through `get_subcommands()`.
An argument's `get_long()` identifies a long flag.
The generated tree also contains framework help/version arguments and generated
`help` subcommands.

The operator command list currently includes:

- `init`;
- `validate`;
- `status`;
- `notes` and `notes ack`;
- `unblock`;
- `doctor`;
- `proposal apply` and `proposal dismiss`;
- `loop`.

The hidden or machine-facing list also includes:

- `recheck-world`;
- `triage-agent`;
- `setup-guide`;
- `hooks-guide`;
- `version`;
- `agent-exec`;
- `capture-usage`;
- `launch-codex`;
- `claim`;
- `check-disposition`;
- `commit-ticket`;
- `complete-ticket`.

`crates/lisa-cli/tests/help_surface.rs` snapshots the public help presentation.
That fixture separates eight everyday commands, five advertised plumbing
commands, and four hidden commands.
The snapshot predates some newer hidden commands, so it is not a complete source
for the audit inventory.
It is useful evidence for operator-facing defaults and descriptions, but the new
test must use the actual command tree to catch later additions.

## Current flag patterns

Most operator commands accept `--path` with Clap default `.`.
`notes --path` is global within the `notes` subtree.
`agent-exec` and `capture-usage` use `--cwd`, also defaulting to `.`.
Boolean switches resolve to `false` when absent.
Examples include `init --dry-run`, `validate --check-tools`, and
`loop --dry-run`.
`init --with-history` and `init --no-history` are mutually exclusive overrides.
With neither flag, `main` maps the request to `HistoryPreference::Ask`.
The resolver then distinguishes terminal and nonterminal input.
`loop --max-threads` and `loop --client` are explicit overrides over config or
environment-derived behavior.
`status --ledger` requires `--ticket` and otherwise has no role.
Many hidden commands carry required correlation, lease, path, or transaction
arguments. They are machine protocol rather than everyday operator questions.
The audit still needs their flags because the acceptance criterion says every
current flag, not only visible help flags.

Clap adds `--help` throughout the tree and top-level `--version`.
Those framework flags have stable behavior but do not represent product choices.
The tree can identify them by their argument action and identifiers.
The generated `help` subcommand is also framework-owned and contains positional
command names rather than Lisa-defined flags.

Positional arguments are present on commands such as `unblock`, `notes ack`,
`proposal apply`, `agent-exec`, and `launch-codex`.
The ticket specifically scopes executable enumeration to flags and keys.
Positionals are command operands rather than flags and are outside that stated
enumeration gate.

## Configuration catalog and parser

`crates/lisa-cli/src/config.rs` owns `.lisa.toml` deserialization.
`LisaConfig` contains `version` plus six sections:

- `dirs`;
- `scheduling`;
- `agent`;
- `runtime`;
- `guards`;
- `triage`.

The fixed parsed keys are represented by `ConfigKey` records in `CONFIG_KEYS`.
Each record contains dotted path, section, key, TOML default, and description.
The catalog currently has 17 entries.
It was introduced by T-050-02-01 as the shared source for validation, init stubs,
and operator documentation.
T-050-02-02 bound the README configuration table to the same catalog.
`config_key`, `is_known_top_level`, and `is_known_section_key` query it.
`init.rs` iterates it when generating and upserting configuration stubs.

The current fixed paths are:

- `version`;
- `dirs.tickets`;
- `dirs.stories`;
- `dirs.work`;
- `runtime.zellij`;
- `agent.client`;
- `guards.completion`;
- `triage.enabled`;
- `triage.timeout_secs`;
- `scheduling.max_threads`;
- `scheduling.auto_advance`;
- `scheduling.review_timeout_secs`;
- `scheduling.session_timeout_secs`;
- `scheduling.wind_down_secs`;
- `scheduling.assignment_ack_timeout_secs`;
- `scheduling.phase_timeouts`;
- `scheduling.provider_caps`.

`phase_timeouts` and `provider_caps` are fixed parent keys with dynamic children.
Their allowed child vocabularies are validated separately.
The catalog comment explicitly says those map children are not fixed config keys.
The audit gate can therefore compare against `CONFIG_KEYS` without attempting to
inventory arbitrary map entries.

Existing config tests already verify that the deserialized fixed-key fixture and
`CONFIG_KEYS` have the same dotted-path set.
They also verify catalog descriptions against a small brand-voice vocabulary and
the README table against the catalog.
The new audit test can consume `CONFIG_KEYS` as the already-enforced parsed-key
boundary rather than independently reflecting Rust fields.

## Interactive terminal prompt

`crates/lisa-cli/src/init.rs` contains the only direct standard-input question in
the native CLI.
`HISTORY_OFFER` is:
“Bring project history along? Finished work can be undone, and you'll have a
record of what the agents did. [Y/n]”.
`prompt_for_history` writes and flushes the offer, reads one line, and accepts an
empty line, `y`, or `yes` as true.
It accepts `n` or `no` as false.
Other input prints “Please answer yes or no.” and repeats.
End of input returns a named error.
`run_init` asks only when standard input is a terminal, no history override was
given, the repository is not already born, and the run is not dry.
Noninteractive bare init decides automatically from repository availability.

The unit test
`project_history_prompt_accepts_defaults_and_retries_invalid_answers` pins the
empty-input yes default and the accepted vocabulary.
`noninteractive_init_keeps_history_by_default_when_available` pins the automatic
nonterminal behavior.
`unavailable_history_falls_back_unless_explicitly_required` pins journal fallback.
T-050-01-01 deliberately retained the terminal offer while demoting the flags to
overrides.

## Dashboard questions

The WASM dashboard does not read text answers, but it presents three modal
decision surfaces.
`crates/lisa-plugin/src/ui.rs` defines `ModalKind` with `MarkDone`, `ResetTicket`,
and `QuitConfirm`.
`ModalState` carries the visible tickets, cursor, kind, and outcome.
`render_modal` renders mark-done and reset selection dialogs.
Both use arrow selection, Enter to confirm, and Escape to cancel.
`render_quit_confirm_modal` appears only when quitting would leave current or new
work and offers Enter to keep working or `q` to quit.
With no pending/new work, quitting is immediate and no question is displayed.

`test_modal_title_mark_done` pins the mark-done prompt title.
`test_modal_title_reset` pins the reset prompt title.
Existing scheduler tests pin which tickets enter each selection list and the
completion/reset effects.
The quit modal rendering has implementation-level copy but less direct named test
coverage than the other two modal titles.
These prompts are manually inventoryable because their set is a closed enum, but
the ticket only mandates executable discovery for Clap flags and config keys.

## Brand-voice constraints

`help_surface.rs` bans mechanism and category jargon from everyday help.
`config_catalog_descriptions_pass_brand_voice_checks` requires direct verbs,
punctuation, and the absence of a small banned vocabulary.
The flag audit's operator-facing column needs the same plain, direct style.
The existing tests do not provide a reusable public brand-voice validator.
A local check in the audit test can enforce sentence punctuation and reject the
known banned terms in the doc's operator-facing wording.

## Artifact and repository constraints

Phase artifacts belong only in
`.lisa/attempts/T-050-03-02/1/work/` during this attempt.
Lisa publishes admitted artifacts later.
The ticket frontmatter is lease-managed and must not be edited by the agent.
Source and knowledge-doc changes must be committed through `lisa commit-ticket`
with exact repository-relative include paths.
The ordinary index already coexists with Lisa-owned journal/provenance/ticket
changes, so isolated commits are required to preserve concurrent state.
Review requires both `review.md` and the exact pass/block JSON disposition, then
`lisa check-disposition T-050-03-02`.
