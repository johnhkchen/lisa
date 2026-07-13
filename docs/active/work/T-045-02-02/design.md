# Design — T-045-02-02 zellij-injects-launcher-only

## Goal

Connect every fresh native Codex ticket launch to the prerequisite
`lisa launch-codex` command using the exact atomically published assignment path.
The pane-facing input must remain a bounded launcher invocation.
The assignment body must remain exclusively in its private attempt file.
The existing `ExitThenFresh` policy must continue to create a new TUI per ticket.
Claude's command and delivery behavior must remain unchanged.

## Evaluation criteria

The selected design must satisfy these properties:

1. A Codex launch refers to the exact returned `AssignmentRef.path`.
2. No filename scan, nonce reconstruction, or newest-file heuristic is introduced.
3. The assignment path is shell-safe on the plugin-to-native-launcher edge.
4. The native launcher remains responsible for the no-shell Codex argv edge.
5. Zellij never receives the assignment body.
6. Every fresh Codex ticket gets a separate launcher/TUI invocation.
7. Routed models and lifecycle environment remain intact.
8. Launcher failure still writes the pane error signal.
9. Claude launch bytes and clear/reuse behavior remain unchanged.
10. The ownership/claim state machine is not redesigned in this ticket.
11. Primary, exit/recycle, and same-pane relaunch paths cannot diverge.
12. A native fixture proves the actual scheduler-produced transport shape.

## Option 1 — reconstruct the assignment filename in `CodexAdapter`

The adapter could combine `artifact_dir`, `attempt_id`, and a known nonce.

Advantages:

- launch call sites would retain their current signature;
- the scheduler would need little mechanical change.

Disadvantages:

- `SpawnContext` does not contain the publication nonce;
- adding only the nonce duplicates the writer's filename contract;
- reconstructed identity could disagree with the retained `AssignmentRef`;
- it weakens the producer-to-consumer handoff established by T-045-01-01;
- stale files can remain in an attempt directory.

This option is rejected.
The exact returned path is already the strongest and smallest authority.

## Option 2 — discover an assignment file by scanning the attempt directory

The adapter or scheduler could glob `assignment-*.md` and choose a candidate.

Advantages:

- no adapter interface change;
- it can be implemented locally at launch time.

Disadvantages:

- publication order and directory order are not assignment authority;
- recovery can leave multiple immutable files;
- a predecessor or superseded same-attempt nonce could be selected;
- it contradicts the exact reference retained in `State::assignment_refs`;
- it makes stale-claim rejection harder to reason about.

This option is rejected as an identity regression.

## Option 3 — place the assignment path in an environment variable

The plugin could add `LISA_ASSIGNMENT_PATH=<path>` to the existing Codex line.
The launcher could read it instead of receiving a positional argument.

Advantages:

- environment inheritance already exists for lifecycle identity;
- the shell command can remain visually compact.

Disadvantages:

- the prerequisite CLI explicitly accepts a positional assignment path;
- an inherited stale variable is less auditable than one invocation argument;
- it creates a second path transport without a need;
- the native argv fixture would no longer cover the plugin handoff directly.

This option is rejected.

## Option 4 — branch on Codex inside scheduler launch sites

Each scheduler path could inspect `route.agent` and manually build
`lisa launch-codex` for Codex while retaining `adapter.launch_command` for Claude.

Advantages:

- the adapter trait signature stays unchanged;
- the immediate edit can be made at each call site.

Disadvantages:

- provider-specific quoting and model logic leaks into the scheduler;
- five launch sites can drift;
- it breaks the existing adapter ownership boundary;
- future provider launch changes would require scheduler branches;
- tests would need to cover duplicated string construction.

This option is rejected because the adapter already owns launch differences.

## Option 5 — add assignment path to `SpawnContext`

`SpawnContext` could gain `assignment_path: Option<&Path>`.
Codex would require it while Claude would ignore it.

Advantages:

- the existing trait signature remains stable;
- all launch inputs live in one context object;
- test helper changes are mechanical.

Disadvantages:

- the value is not meaningful for reuse prompts and follow-up contexts;
- `Option` permits a fresh Codex launch to reach a late panic or fallback;
- contexts built solely for assignment reference delivery must invent `None`;
- the type no longer distinguishes pre-publication prompt construction from
  post-publication launch construction;
- the borrow lifetime couples the full context to one temporary translated path.

This option is viable but not selected.

## Option 6 — make published assignment path an explicit launch argument

Change the adapter boundary to:

```text
launch_command(ctx, assignment_path)
```

Every fresh launch site must retain the successful `AssignmentRef`, translate its
path for the pane environment, and pass it to the adapter.
Claude accepts but ignores this parameter.
Codex includes it in `lisa launch-codex`.

Advantages:

- the type makes a published assignment mandatory for every fresh launch;
- there is no optional or reconstructed identity;
- the scheduler cannot build a launch before publication succeeds;
- Codex-specific formatting remains inside `CodexAdapter`;
- all primary and recovery sites follow the same contract;
- test contexts used for reuse delivery stay unchanged.

Disadvantages:

- every launch call and adapter unit test gains one argument;
- Claude receives an input it does not currently use;
- launch callers must keep the translated `PathBuf` alive for the call.

These costs are mechanical and explicit.
This option is selected.

## Published-reference flow

Every production launch path already calls `prepare_assignment` first.
The design changes those calls from success-only checks to a `match` that retains
the returned `AssignmentRef`.
Existing failure handling and lease revocation remain in the same branch.

After publication, the scheduler computes:

```text
pane_assignment_path = strip_host_prefix(assignment.path)
```

It passes that path to `adapter.launch_command`.
No code reads the assignment bytes during command construction.
No code searches `assignment_refs` merely to reconstruct the just-returned value.
`prepare_assignment` still retains the same reference in the map for later stories.

## Host-path translation

The plugin writes files through its `/host` view.
Commands typed into the pane run from the host project root.
Existing `ticket_dir` and `artifact_dir` values are translated with
`strip_host_prefix` before entering `SpawnContext`.
The assignment path will follow the identical rule.

Translation happens in the scheduler, not in the adapter.
This preserves the documented invariant that adapter paths are already host-relative.
It also lets a native test use ordinary temporary absolute paths unchanged.

## Codex launcher line

`CodexAdapter::interactive_line` will become assignment-aware.
Its logical shell shape is:

```text
LISA_BIN=<lisa> \
LISA_AGENT_CLIENT=codex \
LISA_PANE_ID=<pane> \
LISA_TICKET_ID=<ticket> \
LISA_ATTEMPT_ID=<attempt> \
<lisa> launch-codex [--model <model>] -- <assignment-path> || <error marker>
```

The `LISA_BIN` environment value and invoked executable come from the same resolved
`CodexAdapter.lisa_bin`.
This avoids invoking a different Lisa binary through `PATH` than hooks inherit.
The binary and all dynamic string values use the existing `shell_quote` helper.
Numeric pane and attempt IDs remain formatted numeric literals.

The `--` separator is placed immediately before the positional path.
This prevents a path beginning with `-` from becoming a Lisa option.
The optional model is expressed before that separator.
The native command then reconstructs native `OsString` Codex argv without a shell.

## Model handling

The current `model_flag` emits an interactive Codex flag fragment.
Its output already has the desired ` --model <shell-quoted-model>` shape.
The same helper can be retained because `launch-codex` exposes `--model`.
Only its documentation changes from direct Codex to native launcher semantics.

The model remains shell-parsed once when starting Lisa.
After Clap parses it, the prerequisite launcher passes it to Codex as one `OsString`.
The existing hostile argv fixture covers that downstream guarantee.

## Pane input boundary

The existing atomic `.lisa-launch-<pane>.sh` indirection remains.
Zellij receives only:

```text
sh '<attempt-work-dir>/.lisa-launch-<pane>.sh'
```

The script contains the bounded native launcher invocation.
The assignment body remains in the separately published nonce-bearing Markdown file.
This preserves protection against PTY limits and composer paste behavior.

Removing the script and injecting the full `lisa launch-codex ...` line directly
would also be bounded, but it would discard the established atomic launch transport,
duplicate path-quoting checks at the Zellij edge, and broaden this ticket.
The existing script is appropriately understood as the launcher line indirection.

## Fresh-TUI behavior

No new process-reuse mechanism is needed.
`CodexAdapter::reset_strategy()` already returns `ExitThenFresh`.
On a resident Codex pane, scheduling injects `/exit`, waits for shell return,
then injects the new attempt's script invocation.
On an empty pane, scheduling injects the script invocation immediately.

Because the invocation is `lisa launch-codex` and that native command starts Codex,
each successful invocation creates one new interactive child.
There is no `/clear` or reused Codex composer in this boundary.
The fixture will assert distinct tickets produce distinct published assignment paths
and distinct fresh launch scripts/invocations.

## Assignment state scope

This ticket changes transport, not ownership evidence.
The existing `SeatAssignmentState`, startup grace, ack retry, and claim consumption
are owned by dependent story `S-045-03`.
They will not be broadly rewritten here.

The transport fixture stops at launch dispatch.
It proves the initial input boundary without claiming that a launcher invocation alone
establishes ownership.
Any current later acknowledgment path remains available until the evidence-tier ticket
replaces it.
This preserves the story's declared separation and avoids prematurely inventing an
intermediate scheduler state.

## Fixture design

Add a native scheduler fixture in `crates/lisa-plugin/src/lib.rs`.
Native plugin tests use no-op Zellij host functions, so they are a stub-pane boundary.
The fixture creates two ready Codex tickets and two empty pane slots.
It sets a recognizable Lisa binary and schedules once.

For each assigned ticket, the fixture will inspect:

- the retained nonce-bearing `AssignmentRef`;
- the complete assignment file body;
- the atomically published `.lisa-launch-<pane>.sh`;
- the `SessionLaunch` pane command recorded by the scheduler;
- the queued Enter for the actual `send_line_to_pane` call;
- the slot's resolved Codex client and `Starting` state.

It will assert:

1. two distinct tickets are assigned;
2. two pane submissions are queued;
3. each pane command is only `sh <script-path>`;
4. neither pane command contains assignment body text;
5. each script invokes the fixture Lisa binary and `launch-codex`;
6. each script contains its own exact shell-quoted assignment path;
7. no script contains the assignment body;
8. no script directly invokes bare `codex`;
9. no `/clear` is involved;
10. the two assignment paths and scripts are distinct.

This is stronger than an adapter-only string assertion because it executes the actual
scheduler publication and pane-send path up to the native Zellij stub.
It remains free of real Zellij, Codex, and model tokens as required.

## Existing test updates

Adapter tests will use a stable fixture assignment path.
Codex command-shape expectations will change from direct `codex` to
`'/abs/lisa' launch-codex ... -- '<assignment>'`.
They will assert the assignment body is absent and the exact path is present.
Routing and fallback tests will expect the resolved Lisa launcher.

Claude adapter tests will pass the same fixture path and retain exact current output.
This directly guards the ticket's Claude-untouched constraint.

Scheduler tests that currently assert launch scripts contain no assignment reference
must distinguish body from path.
Codex scripts should contain the assignment path after this ticket.
Claude scripts should remain path-free.
Assertions about `LISA_ASSIGNMENT` body markers remain valid because only the filename
enters the launch script.

## Error behavior

Assignment publication failure continues to abort before any pane injection.
Launch-script publication failure continues to abort before any pane injection.
The native launcher rejects missing or non-file assignment paths before spawning Codex.
The shell `||` fallback continues to create `pane-<id>.error` on launcher or child failure.
No new silent fallback to bare Codex is introduced.

## Compatibility

The implementation adds no dependency and no serialized data shape.
It uses existing `Path`, `PathBuf`, `shell_quote`, and publication helpers.
The hidden CLI command is already committed and tested by the prerequisite.
The plugin WASM continues to perform only filesystem publication and Zellij input.
Native child construction remains outside WASM in `lisa-cli`.

## Decision summary

Make the exact published assignment path a required `launch_command` argument.
Retain the returned reference at every launch site, translate it for the pane,
and let `CodexAdapter` construct a shell-quoted `lisa launch-codex` invocation.
Keep atomic script indirection and `ExitThenFresh` unchanged.
Prove two-ticket behavior through the native scheduler's stub-pane fixture.
