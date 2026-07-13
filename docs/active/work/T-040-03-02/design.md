# Design: join the scheduler miss to the CLI report

## Decision summary

Add one plugin-native historical regression that drives the real delivery
timeout path to terminal failure, then queries the generated ledger through
the exact reader/renderer used by `lisa status --ticket`.

Extract that reader/renderer from the broad CLI `status.rs` file into a small
`preownership_status.rs` module so it can be compiled by both the CLI and the
plugin test without introducing a production dependency cycle.

No scheduler, schema, handshake, retry, or user-visible output behavior will
change.

## Option 1: rely on the two existing tests

The plugin already proves it appends typed transition rows, and the CLI already
proves it renders a literal row.

Advantages:

- no source change;
- each component remains independently tested;
- existing tests are already fast and deterministic.

Disadvantages:

- no test crosses the producer/consumer boundary;
- coordinated drift could leave both isolated tests green;
- the acceptance criterion explicitly asks for one driven miss whose appended
  row is surfaced through the CLI path;
- the historical rc.6 failure is not named and pinned end to end.

Rejected because it does not close the ticket's stated proof gap.

## Option 2: extend the real-Zellij delivery harness

The shell harness can already force bounded startup and delivery failures. It
could run `lisa status --ticket` against the resulting ledger.

Advantages:

- exercises a physical Zellij session and the built CLI/WASM boundary;
- uses the command-line process exactly as an operator would;
- does not require a Rust source-sharing seam.

Disadvantages:

- intentionally ignored unless a host-specific environment flag is present;
- depends on Zellij installation and terminal behavior;
- slower and less portable than the deterministic native story gate;
- overlaps the later authorized field-report ticket's concerns.

Rejected as the primary regression. It may remain useful field evidence, but
it cannot be the always-on deterministic proof.

## Option 3: spawn the CLI binary from a plugin unit test

The plugin test could look for `target/debug/lisa` and invoke the command.

Advantages:

- exercises the real process and Clap dispatch;
- produces a familiar black-box assertion.

Disadvantages:

- Cargo does not provide `CARGO_BIN_EXE_lisa` to another package's unit test;
- focused plugin tests do not guarantee the CLI binary was built;
- guessing target paths breaks custom target directories and profiles;
- recursively invoking Cargo risks lock contention and makes the test depend
  on build order.

Rejected because it is not a hermetic native test.

## Option 4: turn lisa-cli into a library dependency

Create `lisa-cli/src/lib.rs`, expose the report, and add `lisa-cli` as a plugin
dev-dependency.

Advantages:

- conventional Rust dependency and public API;
- no source inclusion;
- future CLI logic could be reused by other crates.

Disadvantages:

- broadens a binary crate's architecture for one regression seam;
- requires deciding which CLI modules form a stable library;
- risks compiling unrelated CLI modules and build-script behavior;
- creates a package dependency relationship that does not exist in production.

Viable, but rejected as disproportionate to the ticket.

## Option 5: move report logic into lisa-core

Both plugin tests and CLI could call a new core report function.

Advantages:

- simple dependency direction;
- one compiled implementation;
- no test-only source inclusion.

Disadvantages:

- CLI text formatting and filesystem diagnostics are presentation concerns;
- expands the core contract beyond typed provenance data;
- contradicts the existing separation where core owns schema and CLI owns
  operator rendering.

Rejected because it puts the seam in the wrong architectural layer.

## Option 6: extract and include the exact CLI module

Move only the pre-ownership reader/renderer into
`crates/lisa-cli/src/preownership_status.rs`.

Main declares the module and calls it for ticket evidence mode.

The plugin's `#[cfg(test)]` module includes that exact file using
`include!(concat!(env!("CARGO_MANIFEST_DIR"), ...))`.

Advantages:

- one source of truth for parsing and rendering;
- production CLI dispatch uses the extracted module directly;
- plugin regression can drive private scheduler state and query its output;
- no new package dependency or production plugin code;
- remains deterministic and native.

Disadvantages:

- cross-crate source inclusion is an unusual test seam;
- the included module must remain self-contained;
- module-level tests compile in both packages unless explicitly gated.

Selected as the narrowest reliable approach.

## Test scenario

Use the existing `preownership_failure_state` helper to build a complete
Codex-owned fixture with pane 10, ticket `T-NAME`, a current attempt lease, a
thread, and a temporary ledger.

Install `SeatAssignmentState::Delivering` with zero retries and an expired
acknowledgement deadline.

Call `check_assignment_ack_timeouts_at` once with a time after that deadline.

Because no retry allowance remains, production code must execute
`fail_assignment_delivery` and append a `DeliveryFailed` row.

This is preferable to directly calling the helper: it drives the actual miss
evaluator that classifies absence of acknowledgement.

The fixture has no ownership signal, so the seat must never become `Owned`.

## Durable assertions

Assert the ledger exists and contains exactly one physical nonblank line.

Decode the line as `ProvenanceLedgerRecord` and require the assignment variant.

Assert ticket, attempt lease, pane, provider, state, and exact production
timeout reason.

Assert execution-only fields are absent from the raw JSON.

These checks prove append durability and semantics independently of rendering.

## CLI surface assertion

Call `preownership_status_surface::write_preownership_status` with the physical
ledger path, ticket ID, and a byte buffer.

Assert the output includes a one-row heading, attempt/pane correlation,
`delivery-failed`, the exact scheduler reason, `openai`, and all timestamp
labels.

The timestamps are production wall-clock values, so assert stable labels and
relationships rather than hard-coded epoch seconds.

The load-bearing historical discriminator is the report heading plus named
failure state. Before S-040-02 the scheduler produced no row, so the CLI reader
would report no matching failures or fail to open a ledger.

## Module extraction behavior

`run_preownership_status`, `write_preownership_status`, and the exhaustive
state-name mapping move without logic changes.

`write_preownership_status` becomes public so the regression can supply an
in-memory writer while main still uses the stdout wrapper.

Existing focused CLI tests move with the module.

The general `status.rs` retains only DAG/config status behavior and its tests.

Main changes its module declaration and ticket-mode call site only.

Normal `lisa status` and ticket evidence output remain byte-for-byte stable.

## Failure policy

If the joined regression reveals missing persistence or incompatible CLI
decoding, this ticket will block rather than change the dependency-owned
scheduler or schema contracts.

Mechanical extraction defects in the test seam may be corrected here.

Any unexplained lifecycle behavior will be documented in Review and given a
blocking disposition, consistent with the story's honest boundary.
