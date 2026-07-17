# Structure — T-048-02-02 ask authoring and auto-recheck

## Change overview

The ticket changes seven existing files and creates no source module.

The implementation stays within three existing boundaries:

1. workflow/template copy;
2. native parked-check execution;
3. plugin scheduling and host-effect orchestration.

No type in `lisa-core` needs to change. The structured disposition, parked
remedy projection, ticket status mutation, and Unpark provenance schema already
exist.

## `crates/lisa-cli/data/rdspi-workflow.md`

### Role

This file is the raw mechanics body rendered by `templates::RDSPI_WORKFLOW`.

### Modification

Expand the Review phase's disposition instructions.

Retain the exact passing shape:

```json
{"disposition":"pass","reason":null}
```

Add the structured blocking shape with:

- `disposition`;
- `reason`;
- `remedy_owner`;
- `ask`;
- optional `steps`;
- observable `check`.

State the allowed owner values and the honest-owner rule.

State that `check` is required whenever the remedy is externally observable.

State the complete one-sentence ask rule.

Add the exact bad/good Pages release counter-example.

Keep the existing post-Review instruction that the agent remains on the ticket
and waits for Lisa.

### Interface impact

The rendered installed workflow changes. No Rust API changes.

## `docs/knowledge/rdspi-workflow.md`

### Role

This is the checked-in installed/rendered workflow used by this repository and
read by live ticket assignments.

### Modification

Mirror the Review text added to the raw workflow body while retaining the
single leading purpose paragraph rendered from `PURPOSE_PARAGRAPH`.

### Invariant

The complete file must remain byte-equal to `RDSPI_WORKFLOW`.

## `crates/lisa-cli/src/templates.rs`

### Role

This module renders agent context and owns tests for the workflow contract.

### Production modification

No production function needs a new branch. `RDSPI_WORKFLOW` already includes
the raw workflow body and generated contexts already point agents at it.

### Test modification

Extend `test_review_disposition_contract_is_injected` or split it into focused
tests that assert the rendered workflow contains:

- `remedy_owner`;
- `agent`, `operator`, and `world` values;
- `ask`, `steps`, and `check` fields;
- honest owner wording;
- the externally observable check requirement;
- the complete ask language rule;
- exact bad Pages wording;
- exact improved release action wording.

The existing equality test remains the integration guard between raw data and
checked-in project context.

## `crates/lisa-cli/src/unblock.rs`

### Role

This module owns safe execution of parked remedy checks and durable reopening.

### New internal result

Add an automation entry point:

```rust
pub(crate) fn run_world_rechecks(root: &Path) -> Result<Vec<String>, String>
```

The returned vector contains only ticket IDs whose status this call changed to
Open. It is sorted because `collect_parked_remedies` is sorted.

### Internal organization

Keep `run_unblock` as the operator-specific entry point and preserve all pinned
messages.

`run_world_rechecks` will:

1. load and resolve config;
2. scan configured tickets;
3. collect canonical parked remedies;
4. retain only World remedies with `Some(check)`;
5. run each check with `CHECK_TIMEOUT`;
6. update the matching ticket status only on `CheckResult::Passed`;
7. collect reopened IDs.

The function reuses these existing private internals unchanged where possible:

- `run_check`;
- `ReadOnlySnapshot`;
- process-group timeout handling;
- capture sanitation;
- mutation fingerprinting.

### Failure rules

- Ordinary failed check: continue, no result ID.
- Timeout: continue, no result ID.
- ChangedFiles: continue, no result ID.
- Missing/nonworld/checkless remedy: never execute.
- Config, scan, check infrastructure, or status-write error: return `Err`.

No provenance write belongs in this module.

### Unit tests

Add focused tests only if a behavior is cheaper to prove below the CLI fixture
boundary. Existing `run_check` timeout and mutation tests remain authoritative
for execution mechanics.

## `crates/lisa-cli/src/main.rs`

### Role

This file declares CLI commands and dispatches native operations.

### New hidden command

Add a hidden plumbing variant:

```rust
RecheckWorld {
    #[arg(long, default_value = ".")]
    path: PathBuf,
}
```

Use a command name that Clap renders as `recheck-world`.

Do not add it to the everyday path or operator command lists.

### Dispatch

Resolve `path`, call `unblock::run_world_rechecks`, and print each reopened
ticket ID on its own line.

On `Err`, print the normal hidden-command error and exit nonzero.

An empty successful vector prints nothing and exits zero.

The existing `Unblock` command and its success/decline copy remain unchanged.

## `crates/lisa-cli/tests/parked_ux.rs`

### Role

This integration target exercises the built Lisa process against filesystem
fixtures.

### Fixture helpers

Add a helper that invokes:

```text
lisa recheck-world --path <root>
```

Reuse existing helpers for project creation, tickets, dispositions, parsed
status, and DAG readiness.

For timeout coverage, use a command that exceeds the production five-second
bound and assert a conservative elapsed ceiling. Existing millisecond unit
coverage remains the precise process-group timing test; the integration case
pins the real command constant.

### New cases

1. Passing World check:
   - no `unblock` command;
   - hidden recheck exits zero;
   - stdout contains ticket ID;
   - status is Open;
   - DAG reports ready.

2. Failing World check:
   - hidden recheck exits zero;
   - stdout/stderr empty;
   - original ticket bytes unchanged;
   - status remains Blocked;
   - DAG remains not ready.

3. Passing Operator check:
   - not executed automatically;
   - status remains Blocked.

4. World write attempt:
   - live sentinel absent;
   - status remains Blocked.

5. World timeout:
   - process returns within a bounded interval;
   - status remains Blocked.

These tests distinguish automatic verification from the visible manual
`unblock` UX already covered in this file.

## `crates/lisa-plugin/src/lib.rs`

### State extension

Add:

```rust
world_recheck_in_flight: bool
```

Default false through the derived `Default` implementation.

The flag is process-local coordination, not scheduling authority.

### Pure command builder

Add a method near `build_completion_command`:

```rust
fn build_world_recheck_command(
    &self,
) -> Result<(Vec<String>, BTreeMap<String, String>), String>
```

It validates configured `lisa_bin` and `project_root` and returns exact argv
plus context containing `lisa_world_recheck`.

### Eligibility predicate

Add a method that reads the current DAG and canonical work projection and
returns whether at least one remedy is:

- owner World;
- check present.

This avoids spawning a host command on empty boards or boards that only need a
person. The native command repeats the same selection as its authority check.

### Effect request method

Add:

```rust
fn request_world_recheck(&mut self)
```

Behavior:

- return if `world_recheck_in_flight`;
- return if no eligible remedy;
- build argv/context;
- on builder failure, log a warning and remain not in flight;
- set in-flight before invoking the host effect;
- invoke with empty environment and host project cwd.

Setting the flag before the effect call prevents reentrant duplicate requests.

Native tests with empty project root remain host-inert.

### Result handler

Add:

```rust
fn handle_world_recheck_result(
    &mut self,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
)
```

Behavior:

- always clear in-flight;
- on exit zero and empty trimmed stdout, return;
- on exit zero and nonempty stdout:
  - parse nonblank lines as display IDs;
  - rebuild DAG;
  - reconcile Unpark transitions;
  - schedule ready tickets;
  - log one information entry;
- on failure, log one warning without changing scheduler state.

The durable scan determines which tickets actually reopened; stdout is
observational and should not directly mutate a specific ticket.

### Lifecycle integration

In `PermissionRequestResult(Granted)`:

- retain timer start and initial scheduling;
- request the startup world recheck.

In `poll_tick`:

- retain the existing five-second timer chain;
- request a cadence recheck once per tick before termination/rearm logic.

In `RunCommandResult`:

- recognize `lisa_world_recheck` before notification handling;
- call the result handler;
- render afterward.

Completion context handling remains first and unchanged.

### Plugin tests

Add helpers to create a blocked World Review fixture with:

- canonical structured disposition;
- configured ticket/work directories;
- a latest Park provenance row;
- one idle slot;
- enough slot timing state to permit scheduling.

Add tests for:

- command argv/context;
- eligibility excludes Operator and checkless World;
- in-flight suppression;
- successful result after durable Open:
  - one Unpark row;
  - ticket scheduled on the available seat;
  - repeated handling/reconciliation adds no second row;
- empty success preserves ledger and blocked status;
- failed result clears in-flight and preserves ledger/status.

### Existing interfaces retained

- `review_block_action` still marks World parks recheck eligible.
- `apply_review_block_policy` still writes Blocked before releasing a seat.
- `reconcile_unpark_transitions` remains the sole plugin provenance writer.
- `schedule_ready_tickets` remains the only seat allocation path.
- `POLL_INTERVAL_SECS` remains five seconds.

## Commit units

The file structure supports three meaningful isolated commits.

1. Agent authoring contract:
   - raw workflow;
   - checked-in workflow;
   - template tests.

2. Native world recheck:
   - `unblock.rs`;
   - `main.rs`;
   - CLI fixtures.

3. Scheduler cadence and provenance/reseat integration:
   - plugin `lib.rs`.

Each unit can be verified and committed with exact include paths.
