# Research: purpose-first CLI strings

## Ticket frame

- Ticket: `T-046-07-01`.
- Story: `S-046-07`.
- Current phase at assignment: Research.
- The requested change is explicitly a wording-layer change.
- The ticket prohibits changes to help grouping, display order, and command separation.
- The desired reader is an agent or newcomer with only the installed CLI surface.
- That reader must encounter purpose before implementation vocabulary.
- The purpose has three parts in the ticket context.
- Lisa runs coding agents.
- Claude Code and Codex are the named agent clients.
- The agents work against a ticket board.
- The operator should not have to approve every step by hand.
- Mechanism terms called out by the acceptance criteria are DAG, WASM, Zellij, and scheduling.

## Repository guidance

- `AGENTS.md` delegates project-wide agent context to `CLAUDE.md`.
- `CLAUDE.md` describes Lisa as a Rust workspace and Zellij WASM plugin with a CLI.
- `docs/knowledge/rdspi-workflow.md` requires all six RDSPI phases.
- Phase artifacts for this attempt belong under `.lisa/attempts/T-046-07-01/1/work/`.
- Shared `docs/active/work/T-046-07-01/` is publication-owned by Lisa.
- Ticket phase and status fields are also publication-owned by Lisa.
- Source commits must use `lisa commit-ticket` with exact include paths.
- The ordinary Git index must not be used for ticket work.

## CLI definition

- `crates/lisa-cli/src/main.rs` owns the Clap command tree.
- `Cli` derives `clap::Parser`.
- The top-level `#[command(...)]` metadata supplies the primary help framing.
- `name = "lisa"` supplies the executable name.
- `about` supplies the prose line rendered near the start of `lisa --help`.
- The current about line is `Runs your coding agents through a project's tickets.`
- That current line already includes the exact phrase `coding agents`.
- It also describes tickets, though as project possessions rather than a board.
- It does not state the reduced-approval purpose from the ticket context.
- `before_help` renders ahead of the about line.
- Its current text is `Everyday path: init → validate → status → loop`.
- That orientation line is procedural but contains none of the four named mechanism terms.
- `after_help` contains the curated plumbing-command footer.
- The footer separates machinery-invoked commands from operator commands.
- Individual `Commands` variants own descriptions and examples.
- `display_order` pins the visible operator sequence.
- Hidden command attributes keep setup-guide, hooks-guide, and version out of the listing.
- Plumbing commands are hidden from Clap's generated listing and described in the footer.

## Help regression boundary

- `crates/lisa-cli/tests/help_surface.rs` is a black-box integration test.
- It invokes `CARGO_BIN_EXE_lisa`, so it tests rendered process output.
- `TOP_LEVEL_HELP_SNAPSHOT` pins the complete top-level help bytes.
- The snapshot currently contains the same about line as `main.rs`.
- `OPERATOR_COMMANDS` pins the five visible operator commands.
- Their canonical order is init, validate, status, doctor, loop.
- `PLUMBING_COMMANDS` pins five machinery-invoked commands.
- `HIDDEN_COMMANDS` pins setup-guide, hooks-guide, and version.
- `OWN_COMMANDS` confirms all thirteen commands remain resolvable.
- `top_level_help_matches_snapshot` detects any byte-level help drift.
- `operator_help_matches_snapshots` separately locks each operator subcommand.
- `plumbing_commands_are_separate_and_internal_hidden` locks separation and visibility.
- `about_line_and_operator_help_are_jargon_free` locates a line containing `coding agents`.
- The same test rejects a configured list of marketing and mechanism jargon.
- `dag` and `scheduling` are already banned in the about line and operator help.
- `WASM` and `Zellij` are not both included in that generic banned list.
- The ticket's ordering requirement is therefore more specific than the current jargon test.
- The integration helper `help_stdout` can exercise hidden guide commands as real output.
- A guide command passed `--help` only tests its Clap description, not the printed guide.
- Invoking `setup-guide` or `hooks-guide` without `--help` tests the guide body.

## Setup guide

- `crates/lisa-cli/src/setup_guide.rs` builds the setup guide as a `String`.
- `GuideSection` holds a section title and body.
- `render_guide` writes the header, then numbered sections in vector order.
- `build_guide` performs a path-existence check before project detection.
- Project detection contributes a project name and project-type label.
- The generated header begins with a Markdown H1 containing that project identity.
- Its prose currently begins `Follow these steps to set up this project for lisa-loop.`
- That prose explains process before explaining what Lisa is for.
- The header is emitted before all numbered sections.
- Later setup sections contain mechanism vocabulary, including scheduling and DAG.
- `section_config` discusses `[scheduling]` configuration.
- `section_dependencies` explicitly describes DAG computation.
- Other sections mention Claude Code, Codex, tickets, hooks, and Zellij-related runtime setup.
- Existing unit tests call private `build_guide` directly.
- They cover project detection, initialized state, RDSPI references, ticket format, numbering, and bad paths.
- There is no current assertion about the semantic order of the preamble.
- `run_setup_guide` prints exactly the `build_guide` result.

## Hooks guide

- `crates/lisa-cli/src/hooks_guide.rs` is a thin output adapter.
- `run_hooks_guide` prints `templates::HOOKS_GUIDE` unchanged.
- `crates/lisa-cli/src/templates.rs` defines `HOOKS_GUIDE` with `include_str!`.
- The underlying file is `crates/lisa-cli/data/hooks-guide.md`.
- The embedded guide begins with `# Lisa Hooks Guide`.
- Its first prose describes setting up or repairing hooks.
- It names Claude Code and Codex immediately.
- It does not first explain why Lisa runs those agents.
- The next lines direct the reader to `lisa init` and manual setup.
- The `How hooks work` section soon introduces sessions, signals, the plugin, and Zellij.
- The guide intentionally contains extensive mechanism detail after its preamble.
- Existing `hooks_guide.rs` tests check non-empty output and contract markers.
- Existing `templates.rs` tests check that the embedded guide includes notification markers.
- No current test locks the opening or purpose-before-mechanism ordering.

## Build and test topology

- `lisa-cli` is a workspace crate with unit and integration tests.
- The focused black-box help test target is `help_surface`.
- Unit tests for `setup_guide` and `hooks_guide` run through the CLI crate test binary.
- A full workspace test covers all crates and cross-crate regressions.
- `just check` additionally checks the WASM target according to `CLAUDE.md`.
- The ticket changes ordinary Rust strings and embedded Markdown only.
- No public Rust API is implicated.
- No serialization format is implicated.
- No runtime scheduler behavior is implicated.
- No generated file is the canonical source for these strings.

## Worktree and ownership observations

- The worktree contains existing modified ticket files unrelated to this ticket.
- It also contains work directories for other ticket IDs.
- Those changes are not owned by `T-046-07-01`.
- The relevant source files were clean at the start of this attempt.
- Exact include paths can isolate this ticket's eventual commit.
- Attempt artifacts are intentionally not committed through the source-unit transaction.
- Lisa will publish admitted phase artifacts after lease verification.

## Constraints surfaced by the map

- Updating the about string necessarily requires updating its exact help snapshot.
- Moving command attributes or variants would exceed the wording-only boundary.
- Changing operator command descriptions would alter five separately pinned snapshots.
- Editing the generated setup guide must preserve its seven-section order.
- Editing the embedded hooks guide changes compile-time content through `include_str!`.
- An ordering assertion must compare case-insensitively because mechanism terms appear with varied case.
- Ordering should use the earliest occurrence of any named mechanism term.
- A positive assertion must ensure the purpose sentence actually exists.
- An ordering-only assertion without a positive purpose anchor could pass vacuously.
- The setup guide title may precede the purpose prose without introducing mechanism vocabulary.
- The hooks guide title likewise precedes prose without introducing mechanism vocabulary.
- Exact shared wording across surfaces makes drift observable and keeps the concept consistent.
