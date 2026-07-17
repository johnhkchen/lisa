# Structure — T-050-01-02 never-a-dead-end surfaces

## Change set overview

- Modify `crates/lisa-cli/src/main.rs`.
- Modify `crates/lisa-cli/src/notes.rs`.
- Modify `crates/lisa-cli/src/status.rs`.
- Modify `crates/lisa-cli/src/init.rs`.
- Add `crates/lisa-cli/tests/never_dead_end.rs`.
- Do not modify CLI help, config schema, core types, plugin code, or grader code.
- Do not create a new runtime module.

## `crates/lisa-cli/src/main.rs`

### New private constants

- Add `SETUP_FIRST_LINE` with the exact required sentence.
- Keep the constant private to the binary crate.
- Define the default ticket marker as `docs/active/tickets` at the helper site.
- Do not export either value through `lisa-cli/src/lib.rs`.

### New private project predicate

Shape:

```rust
fn has_lisa_project_marker(root: &Path) -> bool
```

- Return true when `root/.lisa.toml` exists.
- Return true when `root/docs/active/tickets` is a directory.
- Return false only when neither marker is present.
- Do not load or parse configuration in this predicate.
- Do not canonicalize the root again.
- Do not require `CLAUDE.md` because it is not Lisa-specific.
- Do not check hooks or workflow files; command validation owns those details.

### New private preflight

Shape:

```rust
fn require_lisa_project(root: &Path) -> Result<(), String>
```

- Return `Ok(())` for a project-like root.
- Return one fully rendered stderr document for an untouched root.
- First line must equal `SETUP_FIRST_LINE`.
- Follow it with a blank line.
- Follow with `Technical detail:` and the inspected marker names/root.
- The helper performs no writes and invokes no external process.

### New private exit helper or inline call pattern

- Each guarded arm calls the preflight immediately after `resolve_path`.
- On error, write the returned document directly with `eprintln!`.
- Exit with code 1.
- Do not route this error through the existing `Error: {e}` wrappers.
- Preserve the existing wrapper for all errors after preflight succeeds.

### Guarded command arms

- `Commands::Doctor` is guarded before `doctor::run_doctor`.
- `Commands::Validate` is guarded before `init::run_validate`.
- Ordinary `Commands::Status` is guarded before `status::run_status`.
- The `status --ticket` plumbing path may share the same guard because it is project-root scoped.
- `Commands::Loop` is guarded before parsing project configuration.
- Client syntax parsing can remain before or after only if the setup lead remains first.
- Preferred ordering is path resolution, project preflight, then client/config parsing.
- `Commands::Notes` is not guarded by this ticket.
- `Commands::Init` is never guarded.
- Hidden plumbing commands remain unchanged.

### Main module tests

- Integration tests own process output and exit behavior.
- Optional small unit tests may pin predicate classification.
- Avoid calling `std::process::exit` from a unit test.

## `crates/lisa-cli/src/notes.rs`

### Preserved helpers

- Keep `note_lines(&[]) == Vec::new()`.
- Keep all populated note line construction byte-identical.
- Keep `print_notes` as a populated/embedded renderer.
- Keep the trailing blank line after populated notes.
- Keep acknowledgement behavior unchanged.

### `run_list` empty branch

- Collect notes from the same durable paths.
- If the collection is empty, print exactly `Nothing to read.`.
- Otherwise call the existing `print_notes` helper.
- Return `Ok(())` in both branches.
- Produce no stderr on a clean empty queue.

### Notes unit test

- Add or rename a unit test to pin the empty list sentence through a writer only if practical.
- The integration fixture is the authoritative stdout assertion.
- Retain the formatter test for populated notes.

## `crates/lisa-cli/src/status.rs`

### `print_waiting_on_you`

- Continue deriving lines through `waiting_on_you_lines`.
- If no lines exist, print:

```text
Waiting on you
Nothing waiting.

```

- Return after the empty block.
- Leave populated rendering byte-identical.

### New status note wrapper

Shape:

```rust
fn print_status_notes(notes: &[QueuedNote])
```

- Import `QueuedNote` alongside `collect_notes` if needed.
- When empty, print:

```text
Notes for you
Nothing to read.

```

- When populated, delegate to `crate::notes::print_notes(notes)`.
- Do not change `notes::print_notes` itself.

### `run_status`

- Keep early empty-ticket-board behavior unchanged.
- Keep DAG validation and collection ordering unchanged.
- Invoke `print_waiting_on_you` at the current location.
- Collect notes from the same journal and ledger paths.
- Replace the direct notes printer call with the status wrapper.
- Keep `DAG:` and everything after it byte-identical.

### Status tests

- Add pure formatter capture only if current stdout functions allow it cheaply.
- Prefer black-box coverage for exact empty-section output.
- Existing `parked_ux` snapshots protect populated Waiting behavior.
- Existing `notes_ux` ordering protects populated Notes behavior.

## `crates/lisa-cli/src/init.rs`

### `validate`

- Keep all setup, tool, config, hook, directory, and scan diagnostics.
- After surfacing scan errors, inspect `scan.tickets.is_empty()`.
- Remove the synthetic `readiness` Error diagnostic for clean emptiness.
- Return `ValidationResult` immediately with current diagnostics and zero counts.
- Any parse errors already in diagnostics remain errors.
- Any missing setup errors accumulated earlier remain errors.

### `run_validate`

- Call `validate` once as today.
- If the result has errors, preserve current diagnostic rendering and `Err` behavior.
- If there are no errors and `ticket_count == 0`, resolve configured ticket dir.
- Print one guidance paragraph using that relative path.
- Return `Ok(())` immediately.
- Do not print the generic all-checks-passed line in this branch.
- Do not print the config summary in this branch.
- For `ticket_count > 0`, use the existing output path byte for byte.

### Config path resolution

- Reuse `config::load_config` and `config::resolve_config`.
- Use `ResolvedConfig.ticket_dir` to obtain the operator-visible path.
- Normalize only the display suffix needed to avoid duplicate `/`.
- Do not alter config parsing or default values.

### Validation tests

- Replace any test whose sole contract is empty-board failure.
- Name the success test to document exit 0 explicitly.
- Keep incomplete-project empty fixtures nonzero when other errors exist.
- Keep malformed-ticket zero-parse cases nonzero.
- Pin custom ticket-dir interpolation if the integration fixture can do so cheaply.

## `crates/lisa-cli/tests/never_dead_end.rs`

### Shared command runner

- Define a local `lisa(args)` helper returning `std::process::Output`.
- Do not depend on shell parsing.
- Pass paths through argument arrays to support spaces.
- Decode output with `String::from_utf8_lossy` only in assertions.

### Untouched-root fixture

- Use `tempfile::tempdir()` with no Lisa files.
- Run each guarded command against `--path <temp>`.
- Commands are `loop --dry-run`, `status`, `validate`, and `doctor`.
- Assert exit code 1.
- Assert stdout is empty.
- Assert stderr starts with the exact setup sentence plus newline.
- Assert the technical detail follows, not precedes, the sentence.
- Use a table loop or individual tests with command names in failures.

### Initialized empty-board fixture

- Invoke `lisa init --no-history --path <temp>`.
- Assert init succeeds before exercising empty output.
- This creates current hooks, workflow, config, and directories.
- It avoids copying internal scaffolding assumptions into the fixture.
- It also models the release-audit first-run path directly.

### Empty notes assertion

- Run `lisa notes --path <initialized-root>`.
- Assert exit 0.
- Assert stdout exactly `Nothing to read.\n`.
- Assert stderr exactly empty.

### Empty validate assertion

- Name test `validate_empty_board_exits_zero_with_ticket_guidance`.
- Run `lisa validate --path <initialized-root>` without tool checks.
- Assert exit 0.
- Assert stdout equals the one paragraph plus newline.
- Assert it contains `docs/active/tickets/`.
- Assert stderr exactly empty.
- The name records the selected schedulable exit contract.

### Empty status assertion

- A zero-ticket status returns before optional sections, so add one ready fixture ticket.
- Write a minimal valid ticket into the initialized ticket directory.
- Leave work dispositions and note ledgers absent.
- Run `lisa status`.
- Assert `Waiting on you\nNothing waiting.\n\n` exists.
- Assert `Notes for you\nNothing to read.\n\n` exists.
- Assert Waiting precedes Notes and Notes precedes `DAG:`.
- Assert stderr is empty and exit is 0.

### Populated snapshots

- Existing black-box suites remain the primary populated snapshots.
- If empty section additions intentionally affect their whole output, update only expected empty fragments.
- Do not loosen exact populated line assertions.

## Commit boundaries

- First meaningful unit: source behavior in the four existing Rust files.
- Include exactly `crates/lisa-cli/src/main.rs`, `notes.rs`, `status.rs`, and `init.rs`.
- Second meaningful unit: focused black-box regression file.
- Include exactly `crates/lisa-cli/tests/never_dead_end.rs`.
- If existing tests require expectation updates, include those exact test paths in the test commit.
- Use `lisa commit-ticket --ticket-id T-050-01-02` for each unit.
- Do not include private attempt artifacts in source commits.

## Unchanged boundaries

- `crates/lisa-cli/src/lib.rs` gains no exports.
- `crates/lisa-core` is unchanged.
- `crates/lisa-plugin` is unchanged.
- `docker/chromebook-test/bin/grade` is unchanged.
- `docs/active/tickets/T-050-01-02.md` remains Lisa-managed.
- `docs/active/work/T-050-01-02` is not written by this attempt.
- Ordinary staged entries, if any, remain untouched.
