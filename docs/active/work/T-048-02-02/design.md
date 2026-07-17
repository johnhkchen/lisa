# Design — T-048-02-02 ask authoring and auto-recheck

## Goals

The implementation must make two promises true.

First, an agent that blocks Review receives enough instruction to describe the
remedy in the structured contract and in language a person can act on.

Second, a parked world-owned remedy with an observable check is periodically
verified and reopened without an operator command when the check passes.

The design must preserve the existing authority model:

- the canonical disposition describes the block;
- ticket status controls scheduling;
- the native CLI owns process execution and snapshot isolation;
- the plugin owns scheduler cadence and transition provenance;
- ordinary DAG readiness seats a reopened ticket.

## Decision 1: keep detailed authoring rules in the workflow template

### Option A: add the rules only to the plugin's ticket prompt

The ticket prompt is sent directly for every attempt and already repeats the
minimal pass/block shapes. Adding all authoring rules there would make them
impossible for the agent to miss.

This option would also duplicate the Review contract in another long string.
The ticket specifically identifies `templates.rs`, whose workflow is installed
as the durable project instructions. The prompt already tells the agent to read
that workflow. Duplicating the schema and prose in both locations would create
an upgrade and drift problem.

Rejected as the primary location.

### Option B: add the rules to generated `CLAUDE.md` and `AGENTS.md`

Those files are always discovered by their respective clients. They are also
intentionally small pointers to one shared workflow. Adding phase-specific
protocol there would weaken the current single-source boundary and make every
project context noisier outside Review.

Rejected.

### Option C: extend the Review section of the rendered RDSPI workflow

The workflow is already the canonical phase contract injected into Lisa
projects. The generated context points at it, assignments require agents to
read it, and template tests compare installed and checked-in bytes. The Review
section is the natural location for schema, ownership, check, and language
rules.

Chosen.

The raw workflow body and checked-in rendered workflow will change together.
`templates.rs` tests will pin the required semantic phrases and example so
future template edits cannot silently remove them.

## Authoring contract content

The Review instructions will retain the exact pass form and add a complete
structured block form with these fields:

- `disposition: "block"`;
- a nonempty actionable `reason`;
- `remedy_owner` set honestly to `agent`, `operator`, or `world`;
- a one-sentence `ask`;
- optional `steps` when exact commands or steps are useful;
- `check` whenever the remedy is externally observable.

The prose will define owner meaning by who or what can actually change the
blocked reality, not by which subsystem reported it.

The ask rule will state all three required properties together:

- one sentence;
- addressed to a person who did not do the work;
- names the action rather than the subsystem.

The counter-example will quote the vague 2026-07-16 reason and replace it with
the ticket's plain action-oriented request:

`no stable Pages artifact has been deployed` becomes `Lisa needs the release
published; run: just release. Lisa will notice on its own once it's live.`

The example will be explicitly presented as a bad/good contrast, not a new
machine-valid JSON fixture. This keeps prose judgment separate from parser
syntax.

## Decision 2: reuse the existing safe native check runner

### Option A: execute checks directly inside the plugin

The scheduler has the timer and the parked remedy projection. It runs inside
WASI, however, and is not the established shell-execution boundary. Rebuilding
timeout, process-group, disposable snapshot, capture, and mutation-detection
behavior in the plugin would duplicate safety-critical code and could block the
event loop.

Rejected.

### Option B: have the plugin invoke `lisa unblock <id>`

This reuses existing native behavior with little code. The visible command is
an operator surface, though, and it permits owner types and no-check remedies
that are appropriate for explicit human action but not for automation. The
plugin would have to select IDs correctly and rely on a second scan not to
change classification before execution.

Rejected because the automation boundary should enforce its own eligibility.

### Option C: add a hidden native world-recheck command and invoke it from the plugin

The native command can reuse `run_check` exactly. It can rescan canonical
tickets and remedies itself, select only `remedy_owner: world` entries with a
real check, and reopen only passes. The plugin can launch the command
asynchronously through its existing host-command permission and result event.

Chosen.

This makes eligibility defense-in-depth: both the plugin avoids pointless host
calls when no world check exists and the native command independently refuses
operator, agent, malformed, passing, open, and checkless remedies.

## Native recheck semantics

A new internal function will load resolved project configuration, scan tickets,
and project parked remedies. It will iterate deterministic ticket order.

For each remedy it will continue only when:

- the owner is World;
- a nonblank parsed check exists;
- the matching ticket remains available from the same scan.

The existing `run_check` will execute each command with the existing five-second
timeout and read-only disposable snapshot.

Outcomes are deliberately narrow:

- Passed: update that ticket's status to Open and report its ID.
- Failed: leave all durable state unchanged.
- TimedOut: leave all durable state unchanged.
- ChangedFiles: leave all durable state unchanged.
- Infrastructure error: fail the hidden command closed with an error.

The hidden command will print only reopened ticket IDs, one per line. An empty,
successful stdout means the cadence ran and nothing changed. Ordinary failed
checks are not process failures and therefore do not create repeated scheduler
warnings.

The command will not append provenance. It changes the same durable scheduling
authority as manual unblock. The scheduler remains responsible for observing
that change and recording the Unpark interval.

## Decision 3: schedule one asynchronous aggregate recheck

### Option A: launch one host command per parked ticket

This gives fine-grained result attribution but multiplies command events and
requires an in-flight set keyed by ticket. It also repeats config and board
scans and makes simultaneous status updates harder to reason about.

Rejected.

### Option B: launch one aggregate command per scheduler cadence

The hidden command already scans all world-owned remedies. One asynchronous
invocation is enough for the board and naturally prevents shell work from
blocking the plugin event loop.

Chosen.

State will carry one `world_recheck_in_flight` boolean. A request is suppressed
while the previous command is outstanding. This prevents overlapping commands
when a check reaches the same duration as the five-second poll interval.

The plugin will request the first command when permissions are granted. That is
the first point at loop startup where `RunCommands` is available. It will then
request another command from the existing `poll_tick` cadence whenever no
invocation is in flight.

No second timer will be introduced.

## Host command construction

The plugin will build an argv vector rather than a shell string:

- configured absolute `lisa_bin`;
- hidden `recheck-world` subcommand;
- `--path`;
- absolute host project root.

The host cwd will also be the project root. No dynamic value is interpolated
into shell syntax.

The command context will carry a distinct `lisa_world_recheck` key so the
existing `RunCommandResult` branch can distinguish it from completion and
notification effects.

If the binary or host project root is unavailable, the request fails closed.
Production layouts already supply both; directly constructed native tests can
exercise the pure builder without invoking Zellij.

## Command result handling

On any attributed result, the in-flight flag is cleared.

For exit zero with empty stdout, no state transition or activity entry is
needed. This avoids churn from the expected repeated failing/not-ready case.

For exit zero with one or more reopened IDs, the plugin will:

1. rebuild the DAG from durable ticket files;
2. reconcile Unpark provenance from latest Park rows and open statuses;
3. schedule ready tickets through the ordinary selector.

This ordering ensures the Unpark row exists before the fresh scheduling episode
can mutate more runtime state, while provenance remains non-authoritative if its
append fails.

The handler may log one concise informational activity naming the reopened IDs.

For a nonzero or missing exit code, the plugin clears in-flight state and logs
one warning. It does not change ticket, thread, lease, seat, or provenance
state. The normal next timer may try again.

## Loop start and cadence interpretation

Plugin permission grant is the operational loop-start boundary. Before it, the
plugin is not authorized to launch host commands. On grant it currently starts
the timer and attempts initial scheduling; the recheck request will join that
same startup boundary.

Blocked tickets cannot be seated by the initial schedule. When a startup check
passes, the asynchronous result handler rebuilds and performs the next schedule
pass immediately. Thus no operator input and no extra five-second wait are
required.

Subsequent requests use the existing five-second poll chain. There is no new
config knob because the ticket explicitly asks for the existing cadence.

## Provenance behavior

The hidden native command writes only Open status. The plugin's existing
`reconcile_unpark_transitions` then reads the latest Park row and creates the
Unpark row with:

- the original exact attempt lease;
- World owner;
- `recheck_eligible: true`;
- original interval start;
- current interval end and wall-clock duration.

Repeated command success cannot append repeated Unpark rows because the ticket
is no longer blocked and because the latest transition becomes Unpark.

A failing check makes no status change, so reconciliation finds no reopened
ticket and appends no row. This is the no-churn property.

## Testing strategy

Template tests will assert:

- the structured fields are present in the rendered workflow;
- all three owner values are named;
- ownership must be honest;
- check is required when externally observable;
- the complete one-sentence language rule is present;
- the exact bad and good 2026-07-16 strings are present.

Native check tests will retain the existing low-timeout unit coverage.

Black-box CLI fixtures will invoke the hidden automation command, never
`unblock`, and cover:

- passing World check reopens;
- failing World check leaves ticket bytes/status unchanged;
- operator-owned passing check remains blocked;
- write attempt remains disposable and blocked;
- timeout remains bounded and blocked.

Plugin-native tests will cover:

- exact host argv/context construction;
- only eligible World checks trigger a request decision;
- duplicate requests are suppressed while in flight;
- an observed open World park produces one Unpark row and is seated on the
  result handler's schedule pass;
- an empty successful result changes no ticket or provenance state;
- a failed command clears in-flight state without mutation.

Workspace tests and formatting will verify integration across the CLI, core,
and plugin crates.

## Expected file ownership

- Modify `crates/lisa-cli/data/rdspi-workflow.md`.
- Modify `docs/knowledge/rdspi-workflow.md` to match rendered output.
- Modify `crates/lisa-cli/src/templates.rs` tests.
- Modify `crates/lisa-cli/src/unblock.rs` for the automation function.
- Modify `crates/lisa-cli/src/main.rs` for the hidden command.
- Modify `crates/lisa-cli/tests/parked_ux.rs` for process-level fixtures.
- Modify `crates/lisa-plugin/src/lib.rs` for cadence, effect handling, and tests.

No core schema, config, public operator command, new timer, or new artifact file
is required.
