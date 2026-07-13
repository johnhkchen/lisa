# Research: pre-ownership miss hostile regression

## Ticket boundary

T-040-03-02 asks for one deterministic regression joining two behaviors that
were implemented by its dependencies.

The regression must drive a scheduler failure before provider ownership,
observe an appended durable assignment-transition row, and prove that the row
is retrievable through the CLI status surface.

The historical boundary is the rc.6 incident: scheduling could stop before an
agent owned its prompt without leaving a row that explained the missing run.

The ticket is test-oriented. Story S-040-03 explicitly places the hostile
regressions in `crates/lisa-plugin/src/lib.rs` tests and fixtures.

The story also excludes changes to the ledger contract or provider handshake
semantics. Those mechanisms belong to S-040-02 and the rc.7 lifecycle work.

## Dependency T-040-02-02: scheduler persistence

The plugin scheduler lives in `crates/lisa-plugin/src/lib.rs`.

`SeatAssignmentState` describes the pre-ownership lifecycle, including
`Starting`, `Delivering`, `Recovering`, and terminal retained failure states.

The timeout evaluator is `State::check_assignment_ack_timeouts_at`.

It accepts an injected `SystemTime`, so tests can cross deadlines without
sleeping or using a live provider.

For a fresh delivery whose acknowledgement deadline expires, the first scan
retries the tagged prompt and installs a new bounded deadline.

The next expired scan calls `fail_assignment_delivery` after the configured
retry allowance is exhausted.

`fail_assignment_delivery` changes the seat to `DeliveryFailed`, marks the
thread failed, retains the reservation for operator inspection, and returns a
path-specific transition outcome.

The same helper now calls `State::emit_assignment_transition`.

That writer resolves a complete identity from pane, ticket, attempt lease, and
thread. It derives the provider from the thread client and appends one typed
`AssignmentTransitionRecord` to `State::ledger_path`.

The terminal source-state guard is the exact-once boundary. A repeated failure
call cannot append after the seat has entered a terminal state.

The append is deliberately non-authoritative execution evidence. Its JSON has
an `assignment-transition` discriminator and no execution `outcome` or
`authoritative` members.

Existing plugin coverage includes
`preownership_terminal_transitions_append_once_and_coexist_with_later_done`.

That test directly invokes delivery, recovery, and startup helper methods. It
asserts row identity, state, provider, reason, timestamps, exact-once append,
and coexistence with a later Done execution record.

Existing timeout coverage also proves real recovery deadline behavior.

The current gap is not row serialization. It is the absence of one regression
that starts at the production timeout/miss path and ends at the CLI report.

## Dependency T-040-02-03: CLI reconstruction

The CLI binary is in `crates/lisa-cli/src/main.rs`.

Its `Status` command accepts `--ticket` and an optional `--ledger` override.

With `--ticket`, main resolves the ledger path and calls
`status::run_preownership_status` before loading config or scanning tickets.

The ledger report currently lives at the top of
`crates/lisa-cli/src/status.rs`, alongside the unrelated DAG status report.

`run_preownership_status` locks stdout and delegates to
`write_preownership_status`.

The writer opens the JSONL file, validates every nonblank row through
`ProvenanceLedgerRecord`, filters matching assignment-transition rows, and
renders them in append order.

The report includes attempt, pane, stable state, exact reason, provider,
start/end epoch seconds, and wall-clock duration.

It collects matches before writing. A malformed later line therefore cannot
leave partial output that appears complete.

`assignment_state_name` exhaustively maps the three stable failure states to
their kebab-case CLI spellings.

Existing CLI module tests cover mixed rows, no matches, and malformed input.

`crates/lisa-cli/tests/preownership_status.rs` is a black-box binary test. It
uses a literal fixture row and asserts exact `lisa status --ticket` output.

That test proves command parsing, path handling, ledger-only operation, and
formatting, but its fixture is static. It does not prove the scheduler created
the bytes it reports.

## Cross-crate test constraints

`lisa-plugin` is declared only as a `cdylib`; its private scheduler `State` is
tested inside the source module rather than through an integration-test API.

`lisa-cli` is currently a binary-only crate. Its status module cannot be added
as a normal dev-dependency from the plugin.

The CLI binary path environment variable is supplied only to integration tests
of the CLI package. It is not available to plugin library unit tests.

Spawning `cargo` recursively from a running Cargo test would introduce target
locks, environment coupling, and a non-unit test dependency on build order.

The existing real-Zellij delivery harness is model-free but intentionally
ignored unless its environment gate is enabled. It is not the portable native
regression requested by this story.

## Existing source-sharing pattern

Rust test modules can compile a source file with `include!` using a path based
on `CARGO_MANIFEST_DIR`.

A small module containing only the pre-ownership ledger reader and renderer
has no dependency on CLI config, Clap, Zellij, or plugin internals.

If main uses that module directly and the plugin test includes the same file,
the test exercises the exact query/render implementation behind the CLI
surface without launching a second process.

This is a test seam rather than a second implementation: formatting and parser
changes occur in one source file.

## Repository state and ownership

T-040-03-01 ran concurrently and modified `crates/lisa-plugin/src/lib.rs`.

Its source transaction completed as commit `b6a574a` before this ticket began
editing, leaving the plugin path clean.

Lisa-managed provenance, ticket phase edits, generated plugin docs, and
canonical work artifacts remain dirty or untracked. They are unrelated and
must not be included.

This attempt's phase artifacts belong only under
`.lisa/attempts/T-040-03-02/1/work` until Lisa admits them.

Any ticket-owned source paths must be committed through `lisa commit-ticket`
with exact repository-relative `--include` arguments.

## Verification surfaces

The primary filtered test can run under native `lisa-plugin` without Zellij,
network access, a provider process, or wall-clock sleeps.

Existing CLI unit and black-box tests must remain green after extracting the
pre-ownership report from the general status module.

The full native workspace suite checks both copies compiled from the shared
source and all scheduler interactions.

The WASM target check ensures the plugin source remains deployable even though
the CLI report inclusion is guarded by the plugin's native test module.

Formatting and diff checks protect the mechanical module extraction.
