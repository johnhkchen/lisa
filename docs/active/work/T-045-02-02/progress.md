# Progress — T-045-02-02 zellij-injects-launcher-only

## Status

Implementation, verification, and the isolated source commit are complete.
All ticket-owned source paths are clean.
Review artifacts remain.

## Baseline completed

The ordinary Git index was confirmed empty before source edits.
The worktree already contained unrelated Lisa runtime ledgers and materialized
epic/story/ticket files.
Those paths are not owned by this ticket and have remained untouched.

Baseline focused tests passed:

```text
cargo test -p lisa-plugin adapter::tests
```

Result:

- 25 passed;
- 0 failed.

The prerequisite native launcher fixture passed:

```text
cargo test -p lisa-cli --test codex_launcher
```

Result:

- 1 passed;
- 0 failed.

The baseline established that the CLI already preserves the hostile assignment path
as one exact Codex child argv element.
It also established that the plugin still directly described `codex` before this work.

## Adapter launch contract completed

Modified `crates/lisa-plugin/src/adapter.rs`.

`AgentAdapter::launch_command` now requires:

- `&SpawnContext`;
- an already-published pane-addressable `&Path`.

The path is a required argument rather than optional context.
This makes it impossible to construct a fresh provider launch through the adapter
without first selecting the exact durable assignment reference.

`ClaudeCodeAdapter` accepts the new parameter as `_assignment_path`.
Its returned `build_claude_command` is unchanged.
Existing exact Claude command assertions remain green.

## Codex launcher invocation completed

`CodexAdapter::interactive_line` now invokes the resolved Lisa binary instead of
directly invoking Codex.

The produced fixed shape is:

```text
LISA_BIN=<bin> LISA_AGENT_CLIENT=codex LISA_PANE_ID=<pane> \
LISA_TICKET_ID=<ticket> LISA_ATTEMPT_ID=<attempt> \
<bin> launch-codex [--model <model>] -- <assignment-path> || <error marker>
```

Completed details:

- the resolved Lisa binary is shell-quoted as both environment and executable;
- lifecycle ticket/pane/attempt values remain;
- routed model remains shell-quoted and precedes `--`;
- assignment path is shell-quoted and is the sole positional after `--`;
- the full assignment body is never interpolated;
- direct Codex safety flags were removed from plugin command composition;
- pane error signal fallback remains unchanged.

The native CLI continues to own direct Codex flags and `OsString` argv construction.

## Scheduler path threading completed

Every production fresh-launch site now retains the `AssignmentRef` returned by
`State::prepare_assignment` and passes its translated path to `launch_command`.

Converted paths:

1. primary empty-pane dispatch;
2. primary cross-provider or `ExitThenFresh` recycle preparation;
3. `FreshExec` launch branch;
4. same-pane startup recovery relaunch;
5. launch after the prior client exits.

Each path uses `strip_host_prefix(&assignment_ref.path)`.
Existing publication failure handling remains at each site.
No directory scan, nonce reconstruction, or fallback assignment path was added.
The retained `State::assignment_refs` map remains the exact scheduler reference.

## Shared-worktree serialization deviation

While implementation was in progress, concurrent ticket `T-045-03-01`
(`claim-is-ownership-proof`) modified the same `lib.rs`.
Its initial diff contained claim ingestion and ownership admission only.
Those changes were identified as foreign and preserved.

After this ticket had made non-overlapping assignment-path launch edits in `lib.rs`,
the neighboring ticket ran its isolated transaction.
Commit `67a7f0e` (`feat(plugin): own assignments from exact claims`) serialized the
shared file and included the already-present launch-path hunks along with its claim work.

Consequences:

- current HEAD contains the production `lib.rs` path-threading changes;
- this ticket's remaining `lib.rs` diff contains its acceptance fixture and comment updates;
- `adapter.rs` remains this ticket's uncommitted interface/command implementation;
- HEAD without the current adapter diff is temporarily not a compiling interface pair;
- this ticket's isolated commit will restore a coherent committed adapter boundary.

No foreign claim logic will be included by this ticket's transaction.
The final Review will record the cross-ticket serialization explicitly rather than
misattribute the neighboring claim behavior.

This is a shared-file dependency overlap in the materialized DAG:
`T-045-03-01` depends on the claim command ticket but not on this launcher ticket,
despite both editing `lib.rs`.
The source changes themselves do not conflict semantically.

## Adapter tests updated

All adapter `launch_command` calls now supply a stable nonce-bearing fixture path.

Updated assertions cover:

- exact Claude command preservation;
- route-selected Codex launcher use;
- resolved absolute Lisa binary;
- PATH fallback to bare `lisa`;
- optional routed model placement;
- exact quoted assignment path after `--`;
- absence of direct `codex --dangerously...`;
- absence of `AGENTS.md`, `Read the ticket`, and `LISA_ASSIGNMENT` body markers;
- existing error marker.

Focused post-edit adapter result:

```text
cargo test -p lisa-plugin adapter::tests
```

- 25 passed;
- 0 failed.

## Stub-pane acceptance fixture completed

Added:

```text
codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
```

The fixture uses the native plugin test's no-op Zellij host boundary.
It creates ten independent ready Codex tickets, limits scheduling to two empty panes,
and observes the first two per-ticket launches.

For both active tickets it verifies:

- a current attempt lease exists;
- a distinct nonce-bearing assignment file exists;
- the file contains the full `Read the ticket` and `AGENTS.md` body;
- a distinct `.lisa-launch-<pane>.sh` exists;
- the script invokes the resolved hostile-path Lisa binary with `launch-codex`;
- the script includes its own exact shell-quoted assignment path;
- the script contains no assignment body or direct Codex invocation;
- the recorded pane input is exactly `sh <launch-script-path>`;
- the pane input contains neither assignment body nor `launch-codex` script body;
- `send_line_to_pane` queued one Enter per pane;
- both slots became fresh Codex sessions in `Starting`;
- no `/clear` transition is involved.

Focused result:

```text
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
```

- 1 passed;
- 0 failed.

This test starts no real Codex process and consumes no model tokens.
It is the ticket's required fixture/stub-pane evidence.

## Plugin package verification completed

Ran:

```text
cargo test -p lisa-plugin
```

Result:

- 391 passed;
- 0 failed;
- 0 ignored.

This includes:

- all adapter tests;
- the new two-ticket launcher fixture;
- assignment atomicity tests;
- fresh/recycled Codex state-machine tests;
- Claude reset/reuse tests;
- same-pane recovery tests;
- concurrent claim-consumer tests now present on HEAD;
- signal and completion regressions.

## Deviations from Plan

The planned implementation shape was followed.
One deviation was necessary because the neighboring isolated transaction committed
the shared `lib.rs` production hunks before this ticket could commit them.
No reversion or history rewrite was attempted because that would destroy valid shared
work and violate repository safety.

The planned fixture considered adding a test-only pane-write recorder.
That was not necessary.
Existing production observations were sufficient:

- `pending_enters` proves `send_line_to_pane` was called for each pane;
- `SessionLaunch.command` records the exact submitted line;
- the published script records the native launcher body.

Avoiding a new test-only `State` field kept the production state shape smaller.

## Formatting and diff verification completed

The following checks passed:

```text
cargo fmt --all -- --check
git diff --check -- crates/lisa-plugin/src/adapter.rs crates/lisa-plugin/src/lib.rs
```

Before the isolated commit, the ordinary index remained empty.
The ticket-owned working diff named exactly:

- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/lib.rs`.

The diff audit confirmed:

- no full assignment body enters the launch line;
- no production direct Codex invocation remains in `CodexAdapter`;
- Claude command construction is unchanged;
- all adapter launch calls supply the published path;
- no claim-consumer source outside the already committed shared HEAD is pending;
- no unrelated runtime or planning file is included.

## Workspace verification completed

Ran:

```text
cargo test --workspace
```

The command passed all enabled unit, integration, and doc tests.
Observed principal crate totals included:

- CLI library: 19 passed;
- CLI binary: 269 passed;
- plugin: 391 passed;
- prerequisite launcher integration: 1 passed;
- all core and remaining integration suites passed;
- doc tests passed.

No enabled test failed.

## WASM and repository check completed

Ran:

```text
just check
```

The command passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- a second complete `cargo test --workspace` run.

This verifies the new adapter signature and path handling compile for the actual
Zellij WASM target as well as native fixtures.

## Isolated commit completed

Created the ticket source commit only through Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-045-02-02 \
  --message "feat(plugin): launch Codex with exact assignment reference" \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
5f02b0f2682608fad4dc6779e3387b23f73ef6a8
```

Commit stat:

- `crates/lisa-plugin/src/adapter.rs`: modified;
- `crates/lisa-plugin/src/lib.rs`: modified;
- 164 insertions;
- 41 deletions.

The larger line count is primarily the explicit two-ticket fixture and mechanical
adapter call-site test updates.

`git show --check 5f02b0f` passed.
The commit contains exactly the two requested include paths.
No ordinary `git add`, `git commit`, or broad staging command was used.

Both ticket-owned source paths are clean relative to HEAD.
The ordinary index is empty.

## Post-commit verification completed

Reran:

```text
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
cargo test -p lisa-cli --test codex_launcher
```

Results:

- stub-pane fixture: 1 passed, 0 failed;
- native argv fixture: 1 passed, 0 failed.

This confirms the serialized commit boundary preserved both halves of the contract:
the plugin sends only per-ticket launcher transport, and the launcher sends the exact
path as one Codex argv element.

## Final implementation status

All Plan steps are complete.
No ticket-owned source remains staged, modified, or untracked.
No real Codex/Zellij field run was performed or claimed; that remains the later
field-validation story's explicit boundary.

Only `review.md` and `review-disposition.json` remain for this ticket.
