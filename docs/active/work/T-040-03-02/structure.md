# Structure: pre-ownership hostile regression

## Modified files

### `crates/lisa-cli/src/main.rs`

Declare `mod preownership_status` next to the existing CLI modules.

In `Commands::Status` ticket-evidence mode, call
`preownership_status::run_preownership_status`.

Leave normal DAG status dispatch on `status::run_status`.

No argument, help text, path resolution, exit behavior, or command taxonomy
changes.

### `crates/lisa-cli/src/status.rs`

Remove the pre-ownership ledger imports and functions from the top of the
file.

Retain config loading, ticket scanning, DAG validation, and scheduling report
rendering.

Remove the three pre-ownership-specific unit tests and their literal row
helpers from this module after relocating them.

Retain every existing DAG-status unit test unchanged.

### `crates/lisa-plugin/src/lib.rs`

Within the existing native `#[cfg(test)] mod tests`, include the extracted CLI
module under a clearly named test-only module:

```rust
mod preownership_status_surface {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../lisa-cli/src/preownership_status.rs"
    ));
}
```

Add one historical regression near the existing pre-ownership provenance
tests.

The test uses private production scheduler methods already accessible from the
unit-test module.

No production plugin type, method, constant, or state transition changes.

## Created file

### `crates/lisa-cli/src/preownership_status.rs`

Own the ticket-focused provenance ledger query and rendering path.

Public interfaces:

```rust
pub fn run_preownership_status(
    ledger_path: &Path,
    ticket_id: &str,
) -> Result<(), String>
```

```rust
pub fn write_preownership_status<W: Write>(
    ledger_path: &Path,
    ticket_id: &str,
    output: &mut W,
) -> Result<(), String>
```

`run_preownership_status` remains the CLI stdout adapter.

`write_preownership_status` is the deterministic filesystem/query/render
boundary used by module tests and the plugin hostile regression.

Private interface:

```rust
fn assignment_state_name(state: AssignmentState) -> &'static str
```

This remains exhaustive over the stable provenance enum.

## Internal module organization

Imports are limited to:

- `std::fs::File`;
- `std::io::{BufRead, BufReader, Write}`;
- `std::path::Path`;
- core provenance types.

The module must not import CLI config, Clap, main command types, plugin types,
or Zellij APIs.

That self-contained boundary permits test-only compilation inside the plugin
without package-level coupling.

## Test organization in the extracted module

Move the existing literal execution row and assignment-row builder into the
new module's `#[cfg(test)] mod tests`.

Move these existing tests without semantic change:

- `preownership_status_filters_mixed_ledger`;
- `preownership_status_reports_no_matches`;
- `preownership_status_reports_malformed_line_before_writing`.

Because the entire source file is included by the plugin test module, its own
unit tests will also compile under the plugin library target.

That duplication is harmless but unnecessary. If it causes confusing test
counts, gate the embedded module's tests through source organization rather
than copying report logic.

The preferred simple shape accepts the duplicated three fast filesystem tests
because it avoids custom cfg flags and keeps one source file literal.

## Historical regression fixture

Test name:

```rust
fn rc6_preownership_delivery_miss_is_durable_and_cli_retrievable()
```

The name captures:

- historical release boundary;
- pre-ownership delivery miss;
- durable persistence;
- CLI retrieval.

Fixture state:

- ticket: `T-NAME`;
- pane: `10`;
- client: Codex;
- attempt: minted through `install_current_attempt`;
- ledger: temporary `provenance.jsonl`;
- seat: `Delivering`;
- generation: deterministic integer;
- acknowledgement deadline: injected base time;
- retries: `MAX_ASSIGNMENT_DELIVERY_RETRIES`.

Calling the timeout evaluator after the deadline takes the terminal production
branch immediately.

## Assertion layers

### Scheduler state

Require `FailureTransitionOutcome::DeliveryFailed` for the pane and ticket.

Require the seat remains `DeliveryFailed` and not `Owned`.

### Physical persistence

Read the physical ledger and require exactly one JSONL row.

Require raw absence of `outcome` and `authoritative`.

### Typed evidence

Decode through `ProvenanceLedgerRecord`.

Require schema-v3 assignment-transition identity, matching lease and pane,
provider `openai`, state `DeliveryFailed`, and production reason.

### CLI retrieval

Run the extracted writer over the same physical ledger.

Require the heading reports one failure and the body renders attempt, pane,
state, reason, provider, and timestamp fields.

## Deleted files

None.

## Attempt-private artifacts

The attempt owns only:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

All remain under `.lisa/attempts/T-040-03-02/1/work` until publication.

## Commit units

The extraction and its joined regression form one meaningful source unit: the
new test depends on the extracted exact CLI module.

Commit the four exact source paths together through `lisa commit-ticket`.

Do not include ticket frontmatter, provenance, generated docs, canonical work
artifacts, or any other dirty path.
