# Review: verb-forward command help and examples

## Disposition recommendation

Pass. The acceptance criterion is implemented, both ticket-owned source files
are committed through Lisa's isolated transaction, focused and repository-wide
verification passes, and no source concern remains open.

## Summary

Each operator command's own `--help` now combines its existing plain,
verb-forward purpose with one concrete invocation. The examples live in
variant-level Clap `after_help` metadata, so they appear after generated option
documentation without changing the predecessor's top-level help layout.

The black-box help regression test now snapshots the complete output for all
five operator commands. This positively locks both the purpose and example for
each command, while the existing independent jargon scan continues to reject
banned terminology anywhere in operator-facing help.

## Files modified

### `crates/lisa-cli/src/main.rs`

Five `Commands` variants gained `after_help` strings:

- Init
- Validate
- Status
- Doctor
- Loop

The rendered examples are:

- `lisa init --path ./my-project`
- `lisa validate --path ./my-project --check-tools`
- `lisa status --path ./my-project`
- `lisa doctor --path ./my-project`
- `lisa loop --path ./my-project --max-threads 3`

The purpose copy was deliberately retained:

- Set up a project to run with Lisa.
- Check tickets and setup for problems before a run.
- Show which tickets are ready or waiting and why.
- Check that Lisa's required tools are installed.
- Start a run over ready tickets in parallel where they do not collide.

In source, these remain the established doc-comment strings from the
predecessor state. All begin with direct verbs and pass the jargon policy.

No runtime code changed. Command parsing, defaults, dispatch, and implementation
modules are byte-for-byte unchanged outside the five metadata attributes.

### `crates/lisa-cli/tests/help_surface.rs`

Added an inline full-output snapshot for each operator command. The new
`operator_help_matches_snapshots` test invokes the built binary with each
`<cmd> --help` pair and compares all stdout, including:

- purpose;
- usage;
- option documentation;
- built-in help option;
- blank-line structure;
- concrete example;
- final newline.

The snapshot collection is cross-checked against `OPERATOR_COMMANDS` for count,
order, and command identity. This prevents a future sixth operator or a reordered
collection from silently escaping command-specific coverage.

The module-level test contract was updated from four to five protected
properties. Existing predecessor assertions remain intact.

## Acceptance-criterion evaluation

### `lisa init --help`

- Purpose is plain and starts with `Set`.
- Output includes `Example: lisa init --path ./my-project`.
- Full output is snapshotted.
- Full output passes the jargon scan.

### `lisa validate --help`

- Purpose is plain and starts with `Check`.
- Output includes a path and the real `--check-tools` flag.
- Full output is snapshotted.
- Full output passes the jargon scan.

### `lisa status --help`

- Purpose is plain and starts with `Show`.
- Output includes a concrete project path.
- Full output is snapshotted.
- Full output passes the jargon scan.

### `lisa doctor --help`

- Purpose is plain and starts with `Check`.
- Output includes a concrete project path.
- Full output is snapshotted.
- Full output passes the jargon scan.

### `lisa loop --help`

- Purpose is plain and starts with `Start`.
- Output includes a concrete path and thread count.
- Full output is snapshotted.
- Full output passes the jargon scan.

### Regression behavior

- Dropping any example changes its full snapshot and fails the test.
- Replacing an example with a different command changes the snapshot and fails.
- Dropping any purpose changes the snapshot and fails.
- Reintroducing a banned term fails the independent jargon test.
- Removing or renaming an operator breaks the canonical-list/snapshot contract
  or command-resolution test.
- Moving plumbing back into the primary list still fails the predecessor's
  grouping and top-level snapshot assertions.

## Test coverage

### Focused help surface

Command:

```text
cargo test -p lisa-cli --test help_surface
```

Result: 5 passed, 0 failed.

Covered properties:

- all twelve Lisa-owned subcommands still resolve;
- top-level help remains the accepted snapshot;
- all five operator help screens match exact snapshots;
- plumbing remains separated and internal commands hidden;
- top-level about and all operator help remain jargon-free.

### CLI crate

Command:

```text
cargo test -p lisa-cli
```

Result: pass. All applicable CLI unit, integration, and doc tests passed. The
existing real-Zellij integration remained ignored under its documented
environment gate.

### Workspace and WASM

Command:

```text
just check
```

Result: pass.

This successfully ran the plugin check for `wasm32-wasip1` and the full
workspace test suite. This verifies that the CLI metadata changes do not break
the plugin build or other workspace crates.

### Formatting and diff hygiene

- `cargo fmt --all -- --check`: pass.
- `git diff --check`: pass before the second source commit.
- Direct rendering of all five help screens: pass and visually inspected.
- Top-level help snapshot: unchanged and passing.

## Commit review

Two atomic ticket-owned source units were committed:

1. `d12cf4d101d588bda189bf70d2b6e671e8816ddb`
   - `T-044-01-02: add operator command examples`
   - exact include: `crates/lisa-cli/src/main.rs`

2. `4d4c75beaafde5a9ff2dd0be41cae4fa4bcf8c2f`
   - `T-044-01-02: snapshot operator command help`
   - exact include: `crates/lisa-cli/tests/help_surface.rs`

Both were created with `lisa commit-ticket`. Neither source path remains
modified, untracked, or staged. The ordinary index contains no staged paths.

## Risk assessment

### Runtime risk

Very low. The change is static Clap help metadata. Help rendering occurs before
runtime dispatch, and no parser field or dispatch arm changed.

### Maintenance risk

Low and intentional. Full snapshots couple tests to option help and Clap
spacing. That is appropriate for this curated user-facing surface but means
future deliberate copy or option changes require explicit snapshot review.

### Example longevity

The examples use current stable flags and a neutral illustrative path. If a
flag is renamed, the corresponding full snapshot draws attention to updating
the operator example at the same time.

## Open concerns and limitations

- No functional execution test runs the example commands against
  `./my-project`; examples are static help copy and are validated as rendered
  CLI contracts. This matches the ticket, which requests help output rather
  than tutorial fixtures.
- The banned-jargon list is policy-driven and finite. The ticket extends its
  coverage to the new examples but does not redefine brand policy.
- Terminal wrapping could differ after a future Clap upgrade. The locked
  dependency makes current CI deterministic, and any upgrade will receive a
  reviewable snapshot diff.
- No critical issue, TODO, or human-action blocker remains.

## Worktree boundary

The repository contains unrelated/managed Lisa state and active planning files.
They were present before implementation or appeared through Lisa artifact
publication. Neither source commit included them. Ticket-owned source is clean,
and attempt artifacts remain in the required private attempt directory for Lisa
to admit and publish.

## Conclusion

The implementation satisfies the full acceptance criterion: every operator
command has a plain imperative purpose and concrete `Example:` line, the
predecessor's black-box snapshot harness now locks all five command-specific
screens, and the separate jargon guard prevents brand regression. The ticket is
ready for Lisa's completion transaction.
