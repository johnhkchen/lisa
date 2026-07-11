# Structure: append-only ignore and mutation report

## Change overview

The implementation remains within the `lisa-cli` crate and root CLI
documentation. No new crate, module, persistent metadata file, or public command
is introduced. The primary code change is in `init.rs`, where the action model,
gitignore planner, execution reporting, and focused tests already belong.

## Files modified

### `crates/lisa-cli/src/init.rs`

Responsibilities added or changed:

- distinguish unchanged no-ops from safety-preservation skips;
- plan `.lisa/.gitignore` with an append-only merge;
- execute through an injectable writer for output tests;
- record successful file creates and updates;
- render the exact post-run file mutation report;
- constrain executable-bit changes to hooks written by the current run;
- revise and extend init regression tests.

This remains the single source for init planning and execution. No second
upgrade pipeline is created.

### `README.md`

Expand the `lisa init` CLI reference to document:

- replace-only-with-proven-ownership behavior for static templates;
- preservation/safety-skip behavior for customized or unreadable files;
- append-only behavior for `.lisa/.gitignore`;
- exact real-run file mutation reporting;
- the operator instruction to inspect reported paths before committing.

### `docs/active/work/T-030-02/research.md`

Maps the existing planner, execution loop, tests, templates, and documentation.

### `docs/active/work/T-030-02/design.md`

Records alternatives and decisions for ignore merging, action categories,
reporting, output capture, and hook modes.

### `docs/active/work/T-030-02/structure.md`

Defines this file/interface blueprint.

### `docs/active/work/T-030-02/plan.md`

Sequences implementation and verification work.

### `docs/active/work/T-030-02/progress.md`

Tracks completed work, tests, commits, and deviations during implementation.

### `docs/active/work/T-030-02/review.md`

Provides the final change summary, test assessment, and open concerns.

## Files intentionally unchanged

### `crates/lisa-cli/src/templates.rs`

`LISA_GITIGNORE` remains the canonical ordered list of current Lisa-required
rules. The new planner derives required rules from it. The legacy gitignore
constant may remain for historical context even though the append-only planner
does not use ownership history.

### `crates/lisa-cli/src/main.rs`

The public `run_init(root, dry_run)` signature remains compatible, so CLI parsing
and dispatch require no change.

### Ticket and story files

Ticket `phase` and `status` fields remain untouched. Lisa advances phases from
artifact detection.

## Action model

Replace the current four-way enum with five explicit variants:

```rust
enum InitAction {
    CreateDir(PathBuf),
    CreateFile { path: PathBuf, content: String },
    UpdateFile { path: PathBuf, content: String },
    NoOp { path: PathBuf, reason: String },
    SafetySkip { path: PathBuf, reason: String },
}
```

### Category boundaries

- `CreateDir`: an absent directory will be created.
- `CreateFile`: an absent file will receive the supplied content.
- `UpdateFile`: a readable existing file will receive intentionally merged or
  proven-owned replacement content.
- `NoOp`: the target already satisfies the declared policy or is intentionally
  preserve-if-present without a proposed update.
- `SafetySkip`: init declined a possible update because ownership, readability,
  or structured validity was insufficient.

### Display contract

```text
  create  path/
  create  path
  update  path
  no-op   path (reason)
  skip    path (reason)
```

The distinction is stable in both dry-run and real-run planned output because
both use the same action vector and `Display` implementation.

## Existing planner classification changes

Convert these conditions to `NoOp`:

- directory already exists;
- `CLAUDE.md` already exists under its preserve-if-present policy;
- `AGENTS.md` already exists under its preserve-if-present policy;
- exact current static template;
- `.lisa.toml` unchanged after merge;
- existing hook infrastructure directory;
- structured JSON compares equal after merge;
- gitignore already contains every required rule.

Convert or retain these conditions as `SafetySkip`:

- unknown static-template content;
- unreadable static-template content;
- unreadable `.lisa.toml`;
- malformed or unreadable Claude settings JSON;
- malformed or unreadable Codex hooks JSON;
- unreadable or non-UTF-8 `.lisa/.gitignore`.

## Gitignore planner interface

Add a private helper adjacent to `plan_owned_template`:

```rust
fn plan_append_only_gitignore(path: PathBuf, required: &str) -> InitAction
```

### Absent path

Return `CreateFile` with `required.to_string()`.

### Readable existing path

1. Build an ordered list from non-empty trimmed lines in `required`.
2. Build a set or repeated comparison view of trimmed existing logical lines.
3. Select required rules not present in the existing view.
4. If none are missing, return `NoOp` with `already up to date`.
5. Clone the original existing string without reconstruction.
6. If it is nonempty and does not end in `\n`, append one `\n` separator.
7. Append each missing rule and a trailing `\n` in template order.
8. Return `UpdateFile` with the resulting content.

### Read failure

Return `SafetySkip` with a preservation reason. Do not fall back to the current
template.

### Planner wiring

Replace only the `.lisa/.gitignore` call to `plan_owned_template`:

```rust
actions.push(plan_append_only_gitignore(
    root.join(".lisa/.gitignore"),
    templates::LISA_GITIGNORE,
));
```

All workflow and hook template calls remain unchanged apart from renamed no-op
and safety-skip variants.

## Execution interface

Retain the public wrapper:

```rust
pub fn run_init(root: &Path, dry_run: bool) -> Result<(), String>
```

Add a private implementation:

```rust
fn run_init_with_writer<W: Write>(
    root: &Path,
    dry_run: bool,
    out: &mut W,
) -> Result<(), String>
```

`run_init` locks stdout and delegates. All init `println!` calls move to
`writeln!(out, ...)` inside the private implementation. Output errors map to a
clear `Failed to write init output` string.

## Successful mutation record

Add a small private value type:

```rust
enum FileMutationKind {
    Created,
    Updated,
}

struct FileMutation {
    kind: FileMutationKind,
    path: PathBuf,
}
```

The record is appended only after a successful `fs::write`:

- `CreateFile` records `Created`;
- `UpdateFile` records `Updated`;
- `CreateDir`, `NoOp`, and `SafetySkip` record nothing.

The vector preserves planner/execution order, producing deterministic output.

## Hook mode boundary

After content execution, inspect the successful mutation vector. On Unix, set
`0755` only when:

- the mutation path is one of the four active `.lisa/hooks/*.sh` targets; and
- its content was created or updated successfully in this run.

`on-notify.sample` remains non-executable. Existing no-op or safety-skipped hook
paths receive no permission change.

## Report rendering

After all writes and required chmod operations succeed, emit:

```text
Initialization complete.

Files changed:
  created  <path>
  updated  <path>
```

For an empty vector, emit `  none`.

The next steps gain an initial instruction:

```text
  1. Inspect the files reported above before your next commit
```

Existing ticket creation, validation, and loop instructions shift down one
number.

## Test organization

Tests remain in `init.rs` and use private helpers through the module test scope.

### Action category tests

- Update existing matches from `Skip` to `NoOp` or `SafetySkip` according to the
  explicit policy table.
- Count actual creates using `CreateDir | CreateFile`, not “not skip.”
- Assert display strings contain `no-op` and `skip` labels.

### Append-only merge tests

- customized rule is preserved and missing rules are appended;
- no trailing newline gains exactly one separator;
- surrounding whitespace prevents duplicate required rules;
- current rules yield `NoOp`;
- a second real init changes no bytes;
- invalid UTF-8 yields `SafetySkip` and remains unchanged.

### Combined vend regression

Revise the existing project-modified fixture:

- workflow additions remain byte-for-byte unchanged;
- custom stop and notification content remain unchanged;
- gitignore retains its exact original prefix including `hooks/ntfy-topic`;
- `claude/` and `codex/` appear only as appended rules;
- planning is read-only;
- real init performs the planned append;
- a temporary Git repository reports `.lisa/hooks/ntfy-topic` ignored.

### Output and write-set tests

- Capture a dry run with creates, update, no-op, and safety skip present.
- Assert dry run contains all four labels and writes no files.
- Snapshot relevant file bytes before a real run.
- Capture real output and parse/report expected created and updated paths.
- Assert unchanged and safety-skipped paths do not appear under `Files changed`.
- Run init again and assert the report says `none` for content writes.

## Change ordering

1. Refine action variants and update classifications/tests mechanically.
2. Add and wire the append-only gitignore planner.
3. Revise the combined regression and add edge-case tests.
4. Introduce writer-based execution and mutation record.
5. Limit hook chmod behavior and add report tests.
6. Update README contract.
7. Format and run focused/full verification.

## Structural conclusion

The implementation preserves the existing plan-then-execute architecture. The
gitignore gains one policy-specific planner, while every static template remains
under the ownership-aware helper from T-030-01. Explicit action categories and a
successful-write record connect planner intent, filesystem effects, and operator
output without a new subsystem or persistent state.
