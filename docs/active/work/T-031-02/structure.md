# Structure: T-031-02 gate done on commit

## Change inventory

### Modify `crates/lisa-core/src/ticket.rs`

Add one combined frontmatter operation:

```rust
pub fn update_ticket_done(path: &Path) -> Result<(), TicketError>;
```

The helper reads once, transforms both `phase` and `status`, and writes once.
It reuses the module's existing frontmatter parsing/replacement conventions.

Add unit tests beside the existing phase/status update tests.

### Modify `crates/lisa-cli/src/commit_transaction.rs`

Expose a completion wrapper around `commit_ticket`:

```rust
pub(crate) struct CompleteTicketRequest {
    pub repo_root: PathBuf,
    pub ticket_id: String,
    pub message: String,
    pub ticket_file: PathBuf,
    pub work_dir: PathBuf,
}

pub(crate) fn complete_ticket(
    request: CompleteTicketRequest,
) -> Result<CommitTransactionResult, CommitTransactionError>;
```

The wrapper validates exact ticket-owned paths, saves ticket bytes, calls the
combined core Done update, then delegates Git work to `commit_ticket` with the
ticket file and work directory as explicit includes.

On transaction failure it restores original ticket bytes. If restoration also
fails, return a combined actionable error.

Add process tests to the existing temporary Git fixture.

### Modify `crates/lisa-cli/src/main.rs`

Add a `CompleteTicket` Clap subcommand with:

- `--path` repository root;
- `--ticket-id`;
- `--message`;
- `--ticket-file` repository-relative real ticket path;
- `--work-dir` repository-relative ticket work directory.

Dispatch to `complete_ticket` and print the commit ID only on success.

### Modify `crates/lisa-plugin/src/lib.rs`

Add completion state types, pending state storage, command construction,
completion request/result methods, DAG masking, and routed call sites.

No adapter module change is needed; the state machine is provider-neutral.

### Create/update work artifacts

- `docs/active/work/T-031-02/research.md`
- `docs/active/work/T-031-02/design.md`
- `docs/active/work/T-031-02/structure.md`
- `docs/active/work/T-031-02/plan.md`
- `docs/active/work/T-031-02/progress.md`
- `docs/active/work/T-031-02/review.md`

The ticket file is not edited by this session.

## Core ticket helper organization

The combined helper belongs next to `update_ticket_phase` and
`update_ticket_status` because it is frontmatter behavior, not Git behavior.

Internal transformation order:

1. Read ticket text.
2. Locate the YAML frontmatter boundary.
3. Replace the existing phase line with `phase: done`.
4. Replace the existing status line with `status: done`.
5. Require both fields to exist.
6. Write the fully transformed content once.

The function must not silently add missing fields or partially write after one
replacement succeeds.

## Native completion wrapper organization

### Request validation

Reuse `normalize_includes` for repository-relative path safety. Require:

- exactly one normalized ticket-file path;
- exactly one normalized work-directory path;
- ticket filename/path is not equal to the work directory;
- nonempty ticket ID and message through the underlying transaction validation.

The wrapper should resolve the ticket file under the discovered/canonical
repository root before reading it.

### Preparation guard

An internal guard owns:

- ticket path;
- original bytes;
- whether durable completion succeeded.

Its explicit rollback restores bytes on transaction failure. A defensive Drop
may attempt best-effort restoration before success, but explicit errors remain
the authoritative reporting path.

### Delegation

The wrapper constructs:

```rust
CommitTransactionRequest {
    repo_root,
    ticket_id,
    message,
    includes: vec![ticket_file, work_dir],
}
```

No Git command is duplicated outside `commit_ticket`.

### Success output

The existing result's `commit_id` is forwarded unchanged. The plugin treats a
zero exit plus plausible hash output as the native verification receipt.

## Plugin completion state types

Add near the existing transition state definitions:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionSource {
    Artifact,
    Idle,
    Stopped(u32),
    Manual,
    ObservedDone,
}

#[derive(Debug, Clone, Copy)]
struct PendingCompletion {
    prior_phase: Phase,
    source: CompletionSource,
}
```

Add to `State`:

```rust
pending_completions: HashMap<TicketId, PendingCompletion>
```

`State` already derives Default, and the map naturally defaults empty.

## Plugin command boundary

Add a pure builder:

```rust
fn build_completion_command(
    lisa_bin: &str,
    project_root: &Path,
    ticket_id: &str,
    ticket_file: &Path,
    work_dir: &Path,
) -> Result<(Vec<String>, BTreeMap<String, String>), String>;
```

The command vector starts with the configured Lisa binary and uses the
`complete-ticket` subcommand. It passes no shell string and therefore needs no
shell escaping.

The context map contains `lisa_completion=<ticket-id>`.

Production dispatch converts `Vec<String>` to `Vec<&str>` and calls
`run_command_with_env_variables_and_cwd` with the project root cwd.

## Plugin request method

Add:

```rust
fn request_completion(&mut self, ticket_id: TicketId, source: CompletionSource);
```

Responsibilities:

- deduplicate pending attempts;
- verify dependency and path/config prerequisites;
- capture prior phase;
- insert pending state;
- launch command;
- remove pending and log error if launch preparation cannot occur.

It never updates phase/status, completes threads, emits provenance, releases
slots, rebuilds for readiness, or schedules dependents.

## Plugin result method

Add:

```rust
fn handle_completion_result(
    &mut self,
    ticket_id: &str,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
);
```

Split internal paths into:

- `completion_failed`: remove pending, rebuild masked/restored state, log error;
- `publish_completion`: remove pending, rebuild durable Done, update/log thread
  state, emit provenance, release/remove thread, and schedule.

The event handler routes completion context before notification context.

## DAG masking boundary

In `rebuild_dag`, tickets are mutable before `Dag::from_tickets`.

For each ticket with a pending completion whose scanned phase is Done:

- replace phase with `PendingCompletion.prior_phase`;
- restore a non-Done in-memory status consistent with the current thread/ticket
  snapshot (normally the prior parsed status captured in pending state).

Therefore `PendingCompletion` should also retain `prior_status: TicketStatus`.

This mask applies only to the in-memory ticket vector. It does not rewrite disk.

## Completion call-site changes

### `check_artifact_advances`

When `next_phase == Done`, call `request_completion(..., Artifact)`, mark the loop
as having handled the artifact, and do not call `update_ticket_phase`.

Avoid infinite looping: a pending request causes subsequent iterations to skip.

### `check_idle_signals`

- Implement idle still writes Review.
- Existing `review.md` then requests completion with `Idle`.
- Generic Review idle requests completion instead of writing Done.
- Intermediate phase behavior remains unchanged.

### `auto_complete_review`

Collapse this method to dependency/path diagnostics plus
`request_completion(..., Stopped(pane_id))`. Remove all direct Done writes and
teardown behavior.

### `mark_ticket_done`

Retain modal/dependency validation and replace writes/teardown/rebuild/schedule
with `request_completion(..., Manual)`.

### `poll_tick` observed-Done sweep

Replace direct generic teardown for running threads found Done with completion
requests using `ObservedDone`. Already pending tickets are deduplicated.

Verified success is published directly by the result handler, so the old poll
teardown loop is removed.

### `sweep_stale_slots` and `audit_threads`

They operate on the masked DAG during pending requests. Add explicit pending
guards as defense in depth so neither can release/remove a pending ticket.

## Publication ordering

Successful result handling order is fixed:

1. Validate result receipt.
2. Remove pending mask.
3. Rebuild and verify phase/status Done.
4. Log completion events.
5. Complete thread.
6. Emit Done provenance.
7. Release slot.
8. Remove thread.
9. Schedule dependents.

Termination remains a subsequent poll concern.

## Test module changes

### Core

Add focused combined-update tests near existing update tests.

### CLI

Extend `GitRepo` fixture helpers only where required to read ticket bytes and
commit paths. Add success and rollback process tests.

### Plugin

Reuse current ticket/thread/slot fixtures. Add small helpers to seed:

- configured fake Lisa binary/project root;
- pending completion without invoking a real host process when needed;
- simulated command results;
- provenance ledger temp path.

Update old tests that asserted immediate Done to assert pending-before-result and
Done-after-result.

## Architectural boundaries

- `adapter.rs` remains unchanged.
- Provider-specific hooks remain unchanged.
- DAG dependency semantics remain unchanged.
- Intermediate RDSPI phase writes remain unchanged.
- Timed-out/failed provenance remains unchanged.
- T-031-03 remains responsible for prompt/provider contract and live regression.
- No broad working-tree stage or ownership inference is introduced.
