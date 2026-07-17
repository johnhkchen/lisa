# Structure — T-048-02-01 status-and-unblock-ux

## Change inventory

The implementation is divided into four source units:

1. shared parked-remedy discovery;
2. status/dashboard presentation;
3. unblock and read-only check execution;
4. black-box operator fixtures and help locks.

No existing file is deleted.

No ticket, story, workflow template, or shared work artifact is modified by
implementation.

## `crates/lisa-core/src/parking.rs` — new

This module owns project-level discovery of structured parked remedies.

### Public type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub check: Option<String>,
}
```

The type is owned so CLI and plugin projections do not borrow parsed JSON or a
ticket scan.

It deliberately does not expose raw reason, steps, or unstructured metadata.

### Public collector

```rust
pub fn collect_parked_remedies<'a>(
    tickets: impl IntoIterator<Item = &'a Ticket>,
    work_dir: &Path,
) -> Vec<ParkedRemedy>
```

Responsibilities:

- require `TicketStatus::Blocked`;
- build `<work_dir>/<ticket-id>/review-disposition.json`;
- call `parse_review_disposition`;
- retain only `ReviewDisposition::Block`;
- copy owner, ask, and check into the projection;
- sort by ticket ID.

Invalid, pass, absent, or unreadable files produce no projection.

They do not change the underlying blocked board state.

### Unit tests

Fixtures create blocked/open tickets and canonical work files.

Tests cover:

- structured operator block projection;
- structured world block projection with check;
- legacy fallback becoming operator-owned;
- open ticket exclusion;
- invalid/pass/missing disposition exclusion;
- stable ID sorting;
- raw reason and steps not existing on the public type/surface by construction.

## `crates/lisa-core/src/lib.rs` — modify

Add:

```rust
pub mod parking;
```

No re-export aliases are introduced.

Callers use `lisa_core::parking::{...}` explicitly.

## `crates/lisa-cli/src/status.rs` — modify

Add a narrow waiting renderer over `&[ParkedRemedy]`.

Conceptual interface:

```rust
fn print_waiting_on_you(remedies: &[ParkedRemedy])
```

Behavior:

- filter to `Operator` and `World`;
- return without output if neither exists;
- print `Waiting on you`;
- print one deterministic line per item;
- preserve ask bytes within the line;
- suffix world items with ` — Lisa checks on its own.`;
- print one blank separator line.

`run_status` calls the core collector after scanning tickets and before the DAG
header is printed.

The existing DAG construction, validation, waves, ready summary, and run
summary remain in their current order after the new conditional section.

Unit tests can exercise the renderer semantically through shared line-format
helpers; full stdout ordering belongs to the binary fixture.

## `crates/lisa-plugin/src/ui.rs` — modify

### New UI type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingItem {
    pub ticket_id: String,
    pub ask: String,
    pub checks_on_own: bool,
}
```

This type has display semantics only and does not expose core owner vocabulary
to the line renderer.

### `PluginState` extension

Add:

```rust
pub waiting_items: Vec<WaitingItem>,
```

Initialize it to empty in `Default`.

Most tests using struct update syntax remain source-compatible.

Any complete literal fixture receives `waiting_items: Vec::new()`.

### New renderer

```rust
fn render_waiting_on_you(
    state: &PluginState,
    width: usize,
    output: &mut Vec<String>,
)
```

The renderer uses the existing vector-of-lines pattern.

It prints nothing when the vector is empty.

It prints a plain `Waiting on you` heading and one line per item.

The operator line has only ID and ask.

The world line adds the `Lisa checks on its own` promise.

Long terminal rows may be visually clipped to width only after the complete
semantic line is constructed; tests use enough width to prove verbatim asks.

ANSI styling is limited to heading/ID decoration and is not inserted inside
the ask string.

### Operations ordering

`render_operations_view` calls `render_waiting_on_you` before
`render_attention_banner` and `render_threads`.

The dashboard title/status bar remains the terminal chrome at line zero.

The human section is the first content section.

### UI tests

Add direct tests that assert:

- empty waiting state appends no lines;
- operator heading and exact ask render;
- world item includes the self-check promise;
- raw reason/owner/JSON vocabulary is absent from a fixture output;
- waiting appears before Attention/Threads in the full Operations view;
- DAG and Activity preset behavior is unchanged.

## `crates/lisa-plugin/src/lib.rs` — modify

`State::to_ui_state` calls `collect_parked_remedies` over `self.dag.tickets()`
and `self.config.work_dir`.

It maps:

- `RemedyOwner::Operator` to `WaitingItem { checks_on_own: false }`;
- `RemedyOwner::World` to `WaitingItem { checks_on_own: true }`;
- `RemedyOwner::Agent` to no waiting item.

The core collector already sorts, so mapping preserves stable order.

The resulting vector is assigned in the final `ui::PluginState` literal.

No poll tick, timer, scheduling, provenance, or thread teardown path changes.

Focused projection tests may build a blocked ticket plus canonical disposition
and assert `to_ui_state().waiting_items` if direct UI rendering coverage does
not fully exercise the file boundary.

## `crates/lisa-cli/src/unblock.rs` — new

This module has three layers: project command, check evaluator, and disposable
snapshot helpers.

### Public command outcome

```rust
pub enum UnblockOutcome {
    Reopened(String),
    Declined(String),
}
```

The enum is public only within the binary crate (`pub(crate)` if sufficient).

Messages are already final operator copy.

### Public command function

```rust
pub fn run_unblock(root: &Path, ticket_id: &str)
    -> Result<UnblockOutcome, String>
```

Responsibilities:

- resolve config defaults using the existing config module;
- scan the configured ticket directory;
- find exact ticket ID;
- reject a ticket not currently blocked;
- parse canonical block through the core collector/parser;
- evaluate optional check;
- leave status untouched for every decline;
- call `ticket::update_ticket_status(..., TicketStatus::Open)` only on pass or
  no check;
- return `Reopened("<id> can run again.")`.

Configuration/scanning/frontmatter write errors remain `Err(String)`.

Missing/invalid block structure is an operational refusal with actionable
plain wording, not a fabricated pass.

### Internal check result

```rust
enum CheckResult {
    Passed,
    Failed(String),
    TimedOut,
    ChangedFiles,
}
```

The production wrapper supplies `Duration::from_secs(5)`.

Tests call an internal duration-taking function.

### Snapshot builder

Conceptual helpers:

```rust
fn snapshot_project(root: &Path, destination: &Path) -> io::Result<()>;
fn git_visible_paths(root: &Path) -> Option<Vec<PathBuf>>;
fn copy_path_safely(root: &Path, relative: &Path, destination: &Path);
fn copy_small_tree(...);
fn set_tree_read_only(path: &Path, read_only: bool);
fn fingerprint_tree(path: &Path) -> io::Result<Vec<u8>>;
```

Git path discovery uses NUL-delimited output to preserve spaces/newlines.

Paths are validated as relative and prevented from escaping the snapshot.

Regular files are copied with current contents.

Directories are created as needed.

Symlinks are never followed to an external target; safe internal file targets
may be copied as regular snapshot content, otherwise omitted.

The non-Git fallback skips `.git`, `target`, `node_modules`, and
`.lisa/attempts` directory roots.

The fingerprint visits entries in sorted relative-path order and hashes:

- path bytes;
- entry kind;
- Unix permission bits when available;
- regular file contents.

The runner fingerprints after making the tree read-only and again after the
process group exits.

### Process runner

Conceptual signature:

```rust
fn run_check(root: &Path, check: &str, timeout: Duration)
    -> Result<CheckResult, String>
```

It creates:

- one temporary snapshot directory;
- one disposable scratch directory for `TMPDIR`/`TMP`/`TEMP`;
- anonymous temporary files for stdout and stderr.

It launches `/bin/sh -c` in the snapshot.

On Unix, `CommandExt::process_group(0)` creates a new process group.

At timeout, `libc::kill(-pid, SIGKILL)` terminates that group, followed by
`child.wait()`.

The non-Unix guarded path falls back to `child.kill()`.

Polling sleeps for a small fixed interval (for example 10 ms).

Output reading is capped before conversion to a display observation.

### Observation helper

```rust
fn observed_line(stderr: &[u8], stdout: &[u8]) -> Option<String>
```

It chooses stderr first, removes control characters except ordinary spaces,
trims display whitespace, and caps the result by characters.

The decline formatter supplies exact strings for ordinary failure, timeout,
and attempted mutation.

### Unit tests

Tests use temporary tiny projects and short timeouts.

They cover:

- zero exit passes;
- nonzero exit surfaces one stderr observation;
- stdout is the fallback observation;
- multiline/long/control output is reduced safely;
- silent failure gets the plain fallback;
- timeout returns within a bounded test window;
- a descendant sleep is terminated with the group;
- relative write attempt cannot touch the live root;
- a write that succeeds in disposable state is classified as changed files;
- successful read sees an existing fixture file;
- spaces in project paths and check output are preserved.

## `crates/lisa-cli/src/main.rs` — modify

Add `mod unblock;`.

Add visible `Commands::Unblock` after Status in display order.

Fields:

- positional `ticket_id: String`;
- `--path`, default `.`.

Description:

`Verify what changed and let a waiting ticket run again.`

After-help example:

`lisa unblock T-001 --path ./my-project`

Dispatch behavior:

- `Reopened(message)`: print stdout, return success;
- `Declined(message)`: print stderr directly, exit 1;
- `Err(error)`: retain `Error: {error}`, exit 1.

Add no plumbing footer entry.

## `crates/lisa-cli/Cargo.toml` — modify

Promote `tempfile = "3"` from dev-only to normal dependency because disposable
snapshot/output lifetime is production behavior.

Add Unix-target `libc = "0.2"` for process-group termination.

Keep the existing dev dependency list otherwise unchanged.

## `Cargo.lock` — modify if Cargo requires it

`tempfile` and `libc` already exist transitively/in dev resolution.

The `lisa-cli` package dependency list may change to record direct `libc`.

No package version update is intended.

## `crates/lisa-cli/tests/parked_ux.rs` — new

Black-box fixture helpers create:

- `CLAUDE.md`;
- default ticket/work directories;
- one or more valid ticket Markdown files;
- canonical `review-disposition.json` files.

The test binary is `env!("CARGO_BIN_EXE_lisa")`.

Cases:

1. `status` with one operator park:
   - stdout begins with `Waiting on you`;
   - line contains ticket ID and exact ask;
   - line precedes `DAG:`;
   - raw reason, owner label, field names, and steps are absent.
2. world park status:
   - exact ask appears;
   - line states `Lisa checks on its own`.
3. failing check:
   - exit is nonzero;
   - stderr is the exact one-line decline;
   - no `Error:`, stack, exit code, or JSON vocabulary;
   - ticket remains blocked;
   - fresh DAG does not return it ready.
4. passing check:
   - stdout is exact success copy;
   - status becomes open;
   - fresh DAG returns it ready.
5. absent check:
   - same reopen and DAG result without spawning a check.
6. write attempt:
   - decline is plain;
   - live sentinel does not exist;
   - ticket stays blocked.

The timeout string is pinned by unit coverage to avoid a five-second binary
fixture delay.

## `crates/lisa-cli/tests/help_surface.rs` — modify

Update the exact top-level help snapshot for the new visible command.

Add `unblock` to the complete command inventory.

Add an exact operator command help snapshot with its purpose-first description,
usage, path option, and example.

Add `unblock` to visible operator ordering between status and doctor.

Keep plumbing/internal grouping assertions unchanged.

The existing jargon-free operator help assertion now covers unblock copy.

## Commit boundaries

### Commit 1: shared discovery and waiting surfaces

Exact include candidates:

- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-core/src/parking.rs`;
- `crates/lisa-cli/src/status.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

This is one meaningful cross-crate unit: both surfaces consume one collector.

### Commit 2: unblock execution and operator fixtures

Exact include candidates:

- `crates/lisa-cli/Cargo.toml`;
- `Cargo.lock` if changed;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/unblock.rs`;
- `crates/lisa-cli/tests/help_surface.rs`;
- `crates/lisa-cli/tests/parked_ux.rs`.

This unit introduces the command, its safety boundary, and its black-box
contract together.

Attempt-private RDSPI artifacts are not passed to `commit-ticket`; Lisa
publishes admitted artifacts during completion.

## Files explicitly out of scope

- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/data/rdspi-workflow.md`;
- scheduler poll/timer methods;
- provenance record definitions;
- ticket frontmatter of T-048-02-01;
- `docs/active/work/T-048-02-01/`.

## Final ownership audit

Before Review:

- every source path above is committed with exact includes;
- none remains modified, staged, or untracked;
- ordinary index contents remain untouched;
- unrelated active ticket/work/Lisa changes remain present and uncommitted;
- phase artifacts exist only in the current attempt directory;
- review artifacts are written last.
