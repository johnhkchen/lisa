# Research: verb-forward command help and examples

## Ticket boundary

- The ticket is `T-044-01-02`, titled `verb-forward-command-help-and-examples`.
- It begins in the Research phase and depends on completed ticket `T-044-01-01`.
- The acceptance surface is the command-specific help for five operator commands.
- Those commands are `init`, `validate`, `status`, `doctor`, and `loop`.
- Each command help must state a plain, verb-forward purpose.
- Each command help must contain a concrete usage line introduced by `Example:`.
- The predecessor's help-surface test must cover the examples.
- The test must continue rejecting jargon in operator-facing help.
- Runtime command behavior is outside the ticket.
- The parent story limits source changes to Clap help metadata in `main.rs` and
  the help-surface integration test.

## Repository help architecture

- The CLI binary is the `lisa-cli` crate's `lisa` target.
- Its entry point is `crates/lisa-cli/src/main.rs`.
- Clap 4 derive macros define the parser surface.
- The private `Cli` struct derives `Parser`.
- The private `Commands` enum derives `Subcommand`.
- Rust documentation comments on variants supply their short help text.
- Variant-level `#[command(...)]` attributes supply rendering metadata.
- Fields within each variant become command options.
- Documentation comments on fields become option help.
- `Cli::parse()` renders help before the normal match dispatch runs.
- Help-only metadata therefore has no execution-side dependency.

## Predecessor state

- `T-044-01-01` is complete on the current branch.
- It added an everyday-path line through top-level `before_help`.
- It retained the product description in top-level `about`.
- It hid four plumbing variants from Clap's generated command list.
- It reproduced those plumbing commands in a labeled `after_help` footer.
- It left five operator commands in the primary generated list.
- It added `crates/lisa-cli/tests/help_surface.rs` as a black-box regression
  harness.
- The harness invokes the compiled binary using `CARGO_BIN_EXE_lisa`.
- Its top-level expected output is an inline raw-string snapshot.
- It separately checks parser resolution for all twelve Lisa-owned commands.
- It separately checks the operator/plumbing grouping boundary.
- It already checks operator command help against a banned-jargon list.

## Current operator summaries

- `Init` currently says `Set up a project to run with Lisa.` in source.
- Rendered `lisa init --help` begins `Set up a project to run with Lisa`.
- `Validate` says `Check your tickets and project setup for problems before a
  run.` in source.
- Rendered validate help begins with the corresponding text without the final
  source period.
- `Status` says `Show which tickets are ready to run and which are waiting, and
  why.`
- `Doctor` says `Check that the tools Lisa needs are installed.`
- `Loop` says `Start a run: work through the ready tickets, in parallel where
  they don't collide.`
- All five summaries begin with an imperative verb.
- Their verbs are respectively Set, Check, Show, Check, and Start.
- The text addresses an operator action or observable result.
- None of the five rendered summaries contains a term in the current banned
  jargon set.
- The summaries also appear, without final punctuation, in top-level help.

## Current command-specific rendering

- Each operator help screen begins with its rendered variant summary.
- A blank line separates that summary from `Usage:`.
- `Usage:` contains the command name and `[OPTIONS]`.
- An `Options:` block follows the usage line.
- Clap supplies the built-in `-h, --help` row.
- Each command also displays its own option fields and option documentation.
- No operator command currently has `before_help`, `after_help`, or
  `long_about` metadata.
- No operator command currently renders an `Example:` line.
- Command-specific help exits successfully for all five commands.

## Command option surfaces

- `init` accepts `--dry-run` and `--path <PATH>`.
- `validate` accepts `--path <PATH>` and `--check-tools`.
- `status` accepts `--path <PATH>`, `--ticket <TICKET>`, and
  `--ledger <LEDGER>`.
- `--ledger` requires `--ticket`.
- `doctor` accepts only `--path <PATH>`.
- `loop` accepts `--path <PATH>`, `--max-threads <MAX_THREADS>`,
  `--client <CLIENT>`, and `--dry-run`.
- Every `--path` defaults to the current directory.
- An example may demonstrate defaults or provide actual flag values.
- Help parsing does not execute or validate paths shown in static copy.

## Existing brand guard

- `BANNED_JARGON` contains nine terms or prefixes.
- They include `dag`, `orchestrat`, and `scheduling`.
- The list also includes several marketing-style phrases from prior brand
  guidance.
- Matching is case-insensitive.
- Matching requires a non-alphanumeric boundary around each term.
- The helper intentionally lets a prefix such as `orchestrat` catch multiple
  grammatical forms.
- The existing test scans the complete rendered help of each operator command.
- Consequently, new example text will automatically pass through the same
  jargon guard.
- Plumbing help is intentionally outside this brand gate.

## Existing test gaps

- `top_level_help_matches_snapshot` only invokes `lisa --help`.
- Command-specific examples cannot appear in that top-level output.
- No command-specific help has an expected-output snapshot.
- The jargon test is purely negative.
- If an operator variant lost its purpose summary entirely, an empty or
  arguments-only help body could still contain no banned jargon.
- The resolution test only proves that `lisa <cmd> --help` exits successfully.
- It does not require a purpose line.
- It does not require an `Example:` marker.
- It does not pin concrete example content.

## Clap metadata relevant to the ticket

- Variant `after_help` content is appended after generated command options.
- It affects the command-specific help screen for that variant.
- It does not add content to the parent command's one-line listing.
- Variant `before_help` would place content ahead of the command description.
- Variant `about` controls the short description.
- Variant `long_about` can provide a distinct detailed description.
- Rust doc comments already provide the current `about` values.
- Static string literals fit the five fixed examples named by the story.
- Clap preserves a visible `Example:` label supplied through `after_help`.
- Exact blank lines and trailing newlines are part of captured stdout.

## Source ownership and commit constraints

- The two ticket-owned source files are expected to be
  `crates/lisa-cli/src/main.rs` and `crates/lisa-cli/tests/help_surface.rs`.
- Attempt artifacts belong under
  `.lisa/attempts/T-044-01-02/1/work/`.
- They must not be written directly to `docs/active/work/T-044-01-02/`.
- Lisa controls ticket phase and status transitions.
- Source changes must be committed with `lisa commit-ticket`.
- Each commit must pass exact repository-relative include paths.
- Ordinary `git add`, `git add -A`, and `git commit` are prohibited.
- The current worktree contains unrelated Lisa state and planning files.
- Those files predate this attempt and are outside ticket ownership.
- Ticket commits must not include or disturb them.

## Verification surfaces

- `cargo test -p lisa-cli --test help_surface` is the narrow black-box check.
- `cargo test -p lisa-cli` covers the full CLI crate.
- `cargo test --workspace` covers integration with all workspace crates.
- `cargo fmt --all -- --check` checks Rust formatting.
- The repository's `just check` also includes a WASM target check.
- Help text is deterministic for the locked Clap dependency and noninteractive
  test invocation.
- Direct `cargo run -q -p lisa-cli -- <cmd> --help` permits visual inspection
  of the final five screens.

## Observed acceptance gap

- All five current summaries are already plain and verb-forward.
- None of the five screens contains the required concrete example.
- Existing tests reject jargon but do not positively preserve purpose lines.
- Existing tests do not inspect any example marker or example command.
- The remaining gap is therefore help metadata plus positive command-specific
  regression coverage, with the existing runtime boundary unchanged.
