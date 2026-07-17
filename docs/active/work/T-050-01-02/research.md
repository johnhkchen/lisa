# Research — T-050-01-02 never-a-dead-end surfaces

## Ticket boundary

- The ticket starts in Research and requires all six RDSPI phases.
- The product scope is the native `lisa` CLI.
- The ticket covers four kinds of empty or pre-setup output.
- It does not add flags, commands, configuration, or scheduler behavior.
- It does not change the Chromebook grader in `docker/chromebook-test/bin/grade`.
- It does not authorize manual ticket phase or status edits.
- Attempt artifacts belong under `.lisa/attempts/T-050-01-02/1/work/`.
- Lisa later admits those artifacts into the shared work directory.
- Ticket-owned source commits must use `lisa commit-ticket` with exact paths.
- The ordinary Git index cannot be used for this ticket.

## Required operator-facing sentence

- The exact setup lead is `This folder isn't set up yet. Run: lisa init`.
- It must be the first line of each covered pre-init command failure.
- Technical detail may follow that line.
- The ticket names `loop`, `status`, `validate`, and project-aware `doctor`.
- The setup sentence is a first-contact contract, not an internal diagnostic.
- The sentence does not include backticks around `lisa init`.
- The sentence uses `isn't`, not `is not`.
- The sentence ends with `init`, without a final period in the quoted contract.
- Tests therefore need byte-level or prefix equality, not loose word matching.

## CLI dispatch

- `crates/lisa-cli/src/main.rs` owns Clap parsing and command dispatch.
- `main` resolves every `--path` through the private `resolve_path` helper.
- Relative paths become current-directory-relative absolute-looking paths.
- Each public command arm invokes its module and handles `Result` locally.
- Most failures are rendered as `Error: {detail}` on stderr.
- Each error arm exits with process status 1.
- There is no shared project-discovery preflight in `main.rs` today.
- There is no shared user-facing error renderer in `main.rs` today.
- `init`, `version`, plumbing commands, and guides do not require a project.
- `loop`, `status`, `validate`, and `doctor` all accept `--path`.
- Their command arms already resolve `--path` before entering their modules.
- That boundary can observe the same root for all four named commands.

## Existing initialization markers

- `lisa init` is implemented in `crates/lisa-cli/src/init.rs`.
- `plan_init_actions` scaffolds `CLAUDE.md` when absent.
- It also scaffolds `AGENTS.md` as a pointer to `CLAUDE.md`.
- It creates `docs/active/tickets` and the related story/work directories.
- It installs `docs/knowledge/rdspi-workflow.md`.
- It creates `.lisa.toml` with the current protocol version and defaults.
- It creates `.lisa/hooks`, `.lisa/signals`, and lifecycle hook files.
- A completely untouched folder has none of those Lisa-owned markers.
- A partially damaged or customized project can retain only some markers.
- Existing unit fixtures often model a project with `CLAUDE.md` and ticket dir.
- Many older fixtures do not create `.lisa.toml` because config has defaults.
- `config::load_config` deliberately accepts a missing `.lisa.toml`.
- It returns a default `LisaConfig` with no warnings when the file is absent.
- Therefore config loading alone cannot distinguish pre-init from default config.
- A pre-init predicate must not reclassify established partial-project tests.

## Loop behavior before this ticket

- `Commands::Loop` parses an optional client override first.
- It then calls `config::load_config`, which succeeds without `.lisa.toml`.
- It resolves defaults and calls `loop_cmd::run_loop`.
- `run_loop` first checks for `CLAUDE.md`.
- Missing `CLAUDE.md` returns `No CLAUDE.md found. Run `lisa init` first.`.
- Missing configured ticket directory returns a separate technical error.
- These checks occur before completion-seal or external-tool resolution.
- Dry-run uses the same structure checks before entering `run_dry`.
- An empty initialized ticket directory is accepted by dry-run.
- That empty dry-run currently prints `No tickets found in {absolute path}`.
- Non-dry loop behavior after structure checks has substantial side effects.
- Early shared pre-init detection is therefore safer for first-contact output.

## Status behavior before this ticket

- `crates/lisa-cli/src/status.rs` owns the board status command.
- `run_status` loads config, falling back to resolved defaults on config errors.
- It derives configured ticket and work directories.
- It resolves the completion seal for inspection.
- It errors if the configured ticket directory does not exist.
- That error currently leads with `Error: Ticket directory not found: ...`.
- It scans tickets once the directory exists.
- An empty ticket set prints `No tickets found in docs/active/tickets`.
- Empty status returns success immediately before any named sections.
- A non-empty board builds and validates the DAG.
- Parked remedies are derived from ticket dispositions in the work directory.
- Operator- and world-owned remedies appear under `Waiting on you`.
- Agent-owned remedies are excluded from that operator section.
- `print_waiting_on_you` emits nothing when the rendered remedy list is empty.
- Deferred completion notes are collected from two durable JSONL ledgers.
- Status delegates note rendering to `notes::print_notes`.
- `notes::print_notes` emits nothing for an empty slice.
- Therefore a non-empty board with no parks and no notes jumps directly to `DAG:`.
- Existing parked output is string-sensitive in `tests/parked_ux.rs`.
- Existing note ordering is covered in `tests/notes_ux.rs`.
- `Waiting on you` precedes `Notes for you`, which precedes `DAG:`.
- Non-empty section formatting includes a trailing blank line.

## Notes behavior before this ticket

- `crates/lisa-cli/src/notes.rs` owns list and acknowledgement behavior.
- Durable sources are `.lisa/completion-journal.jsonl` and provenance JSONL.
- `run_list` collects active queued notes and calls `print_notes`.
- `note_lines` returns an empty vector for an empty queue.
- `print_notes` returns without writing when that vector is empty.
- Consequently `lisa notes` succeeds with empty stdout and stderr.
- The black-box regression is named `empty_queue_renders_nothing` today.
- Non-empty note output begins with `Notes for you ({count})`.
- Each entry puts the plain summary before criterion and evidence.
- Acknowledgement output is `{ticket-id} acknowledged.`.
- Acknowledgement mutates only the provenance ledger.
- The ticket changes the list empty state, not acknowledgement behavior.
- The requested empty list sentence is exactly `Nothing to read.`.
- Status and standalone notes share a low-level printer today.
- Their empty-state needs are related but not identical.
- Standalone notes needs only the sentence, without an implied section wrapper.
- Status needs named sections to make absence legible.

## Validate behavior before this ticket

- `run_validate` and its private `validate` function live in `init.rs`.
- `validate` accumulates structured diagnostics in `ValidationResult`.
- Diagnostics have path, category, message, and error-or-warning severity.
- Optional tool checks execute before project structure checks.
- Missing `CLAUDE.md`, workflow, hooks, or ticket dir are independent errors.
- Config is loaded with defaults when `.lisa.toml` is missing.
- Ticket scanning uses `scan_tickets_with_diagnostics`.
- Per-file parse failures are retained as frontmatter errors.
- After scanning, an empty valid ticket list adds a readiness error.
- Its current message is `no tickets found. Create at least one ticket file.`.
- The diagnostic path is the configured ticket directory with a trailing slash.
- The function returns immediately after adding that error.
- `print_diagnostics` prints errors, then warnings.
- Any error causes a nonzero `Result` and a repeated summary on stderr in `main`.
- A no-error result says all checks passed and suggests `lisa loop`.
- `run_validate` then prints a config summary.
- The empty-board branch currently exits nonzero solely because empty is an error.
- The ticket explicitly permits choosing success-with-guidance or retaining failure.
- The field context says the nonzero contract forced a grader-created smoke ticket.
- The grader change is deferred, but the CLI can remove that prerequisite.
- The configured ticket path may differ from `docs/active/tickets`.
- Guidance must therefore interpolate `ticket_dir_rel`, not hard-code a path.
- A board with parse errors can also have zero successfully parsed tickets.
- Parse errors must remain errors rather than being mistaken for a clean empty board.

## Doctor behavior before this ticket

- `crates/lisa-cli/src/doctor.rs` owns dependency and project health reporting.
- `run_doctor` begins by loading config, which defaults when absent.
- It resolves runtime, agent client, and completion-seal intent.
- It runs external dependency checks before formatting the project section.
- `check_project_version` can mark the project check as skipped.
- The project section is omitted when no Lisa project version is found.
- Completion and cache checks still run outside a project.
- Doctor can clean stale Zellij plugin cache entries.
- Codex configuration can also be touched during trust pre-grant.
- Failure depends partly on the host's installed dependency state.
- Without an early pre-init boundary, first output is environment-dependent.
- The ticket's setup lead needs to precede all of that technical reporting.

## Test architecture

- `crates/lisa-cli/tests` contains black-box command fixtures.
- Tests invoke `env!("CARGO_BIN_EXE_lisa")` through `std::process::Command`.
- `tempfile` is already a dev dependency.
- Existing tests separately decode stdout and stderr.
- `help_surface.rs` demonstrates constant-backed exact snapshots.
- `notes_ux.rs` covers durable note list, ack, and restart behavior.
- `parked_ux.rs` pins exact leading status text for parked tickets.
- `status.rs` has unit tests but most assert only `Result` success or failure.
- `init.rs` has extensive validation unit coverage.
- The existing empty-ticket validation unit test asserts only nonzero `Result`.
- That test name and expectation encode the contract that must be reconsidered.
- Loop structure tests directly call `run_loop` on partial fixtures.
- Doctor tests largely isolate report helpers or use controlled fake binaries.
- A new black-box UX fixture can cover cross-command leading-line behavior.
- Exact output tests should isolate host-dependent code behind pre-init detection.
- Initialized snapshots should use controlled inputs and avoid real tool checks.

## Compatibility constraints

- No new CLI flags may appear in Clap help.
- Successful non-empty note rendering must remain byte-identical.
- Parked status rendering must remain byte-identical when remedies exist.
- Deferred note rendering must remain byte-identical when notes exist.
- DAG, waves, ready scheduling, and run-summary output must remain unchanged.
- Valid non-empty validation output and exit status must remain unchanged.
- Validation parse and setup errors must remain actionable and nonzero.
- Loop's partial-project technical checks remain useful after the setup lead boundary.
- Custom configured ticket paths must appear in empty-board guidance.
- The ordinary index contains Lisa-managed ticket metadata and must be preserved.

## Observed worktree state

- The branch is `main` tracking `origin/main`.
- `docs/active/tickets/T-050-01-01.md` is already modified.
- `docs/active/tickets/T-050-01-02.md` is already modified by Lisa.
- Those changes are not ticket-owned implementation files.
- The attempt directory contains the launch script and assignment document.
- No phase artifact existed before this Research artifact.
- No ticket-owned source file was staged, modified, or untracked at Research start.

## Research conclusions

- The first-contact lead belongs at a shared command-dispatch boundary.
- Empty list text belongs above the shared non-empty notes printer.
- Status needs explicit empty named sections without changing populated sections.
- Clean zero-ticket validation is distinguishable from parse-failed zero-ticket state.
- Exit behavior and text need black-box tests because the contract spans `main`.
- Existing module tests provide regression coverage for the unchanged populated paths.
