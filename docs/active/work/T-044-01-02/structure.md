# Structure: verb-forward command help and examples

## Change set

The implementation changes two ticket-owned Rust files and creates no runtime
module, configuration, fixture, or dependency.

Modified source files:

1. `crates/lisa-cli/src/main.rs`
2. `crates/lisa-cli/tests/help_surface.rs`

Attempt artifacts remain under
`.lisa/attempts/T-044-01-02/1/work/` and are not source commit inputs.

## `crates/lisa-cli/src/main.rs`

### Existing boundary

The private `Commands` enum remains the only help-metadata boundary touched.
The private `Cli` struct and its top-level `before_help`, `about`, and
`after_help` stay unchanged. The `main` function and all command modules stay
unchanged.

### Init variant

Retain:

- the existing doc-comment purpose;
- `display_order = 0`;
- `dry_run` and `path` fields and their attributes.

Extend the existing command attribute with:

- `after_help = "Example: lisa init --path ./my-project"`.

### Validate variant

Retain:

- the existing verb-forward purpose;
- `display_order = 1`;
- `path` and `check_tools` fields.

Extend its command attribute with:

- `after_help = "Example: lisa validate --path ./my-project --check-tools"`.

### Status variant

Retain:

- the existing verb-forward purpose;
- `display_order = 2`;
- `path`, `ticket`, and `ledger` fields;
- the `ledger` requirement on `ticket`.

Extend its command attribute with:

- `after_help = "Example: lisa status --path ./my-project"`.

### Doctor variant

Retain:

- the existing verb-forward purpose;
- `display_order = 3`;
- the `path` field.

Extend its command attribute with:

- `after_help = "Example: lisa doctor --path ./my-project"`.

### Loop variant

Retain:

- the existing verb-forward purpose;
- `display_order = 4`;
- `path`, `max_threads`, `client`, and `dry_run` fields.

Extend its command attribute with:

- `after_help = "Example: lisa loop --path ./my-project --max-threads 3"`.

### Unchanged variants

The following enum variants receive no edits:

- SetupGuide
- HooksGuide
- Version
- AgentExec
- CaptureUsage
- CommitTicket
- CompleteTicket

Their hidden/plumbing behavior remains the predecessor's responsibility and is
still covered by existing tests.

### Unchanged runtime

No changes occur below the enum definition. In particular:

- `Cli::parse()` is unchanged;
- all `match cli.command` arms are unchanged;
- path resolution is unchanged;
- implementation module calls are unchanged;
- exit codes and error handling are unchanged.

## `crates/lisa-cli/tests/help_surface.rs`

### Module contract comment

Update the top module documentation so its pinned properties include
command-specific purpose/example snapshots. Keep the predecessor/story context
and black-box execution note.

### Snapshot data type

Add a private test-only record near the operator command constants:

```rust
struct OperatorHelpSnapshot {
    command: &'static str,
    expected: &'static str,
}
```

No traits are required. The records are iterated by shared reference.

### Snapshot collection

Add a constant array of five `OperatorHelpSnapshot` values, one per member of
`OPERATOR_COMMANDS`, in the same canonical order:

1. init
2. validate
3. status
4. doctor
5. loop

Each expected raw string contains the full stdout for:

`lisa <command> --help`

Each string includes:

- exact rendered purpose;
- blank-line spacing;
- generated usage;
- generated options and defaults;
- built-in help option;
- final `Example:` line;
- trailing newline.

### New snapshot test

Add a test named `operator_help_matches_snapshots` after the top-level snapshot
test. It loops through the snapshot collection, captures stdout with the
existing `help_stdout` helper, and compares exact strings.

The assertion diagnostic names the relevant command so failures are local even
though the loop covers five records.

### Existing tests retained

Keep `all_twelve_subcommands_resolve` unchanged. It still proves that hidden and
plumbing commands parse.

Keep `top_level_help_matches_snapshot` unchanged. It still pins the everyday
path and category layout.

Keep `plumbing_commands_are_separate_and_internal_hidden` unchanged. It still
protects the top-level grouping.

Keep `about_line_and_operator_help_are_jargon_free` substantively unchanged.
It will automatically scan `after_help` example content because it captures the
entire stdout for each operator command.

### Consistency invariant

The snapshot array and `OPERATOR_COMMANDS` must represent the same five names.
The test can assert equal lengths and compare each snapshot's command with the
corresponding operator constant before invoking the binary. This prevents one
list from silently omitting a command while the other still drives jargon and
grouping tests.

## File-level dependency flow

`main.rs` command attributes
→ Clap derive-generated command metadata
→ built `lisa` binary help stdout
→ `help_surface.rs` black-box capture
→ exact command-specific snapshot assertion
→ existing banned-jargon assertion.

No production Rust API is introduced between files. The integration test sees
only the executable contract.

## Commit units

### Unit 1: help metadata

Commit only:

- `crates/lisa-cli/src/main.rs`

This is a meaningful source unit because it independently makes every required
help screen render an example.

### Unit 2: regression coverage

Commit only:

- `crates/lisa-cli/tests/help_surface.rs`

This independently locks the public help output and the five-command set.

Exact includes prevent unrelated worktree state from entering either isolated
transaction.

## Deletions and new dependencies

- No files are deleted.
- No production files are created.
- No Cargo manifest changes are needed.
- No external snapshot crate is added.
- No generated `.snap` files are added.
- No feature gates are added.

## Review boundary

Review should compare the rendered five help screens with the selected
invocations, confirm the top-level output remains byte-for-byte stable, and
confirm both source files are clean after isolated commits. It should not
evaluate command execution behavior because this ticket does not alter it.
