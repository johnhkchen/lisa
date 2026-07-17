# Design — T-048-02-01 status-and-unblock-ux

## Goals

The change must let an operator understand and resume parked work without
opening machine-readable Review artifacts.

The status command and dashboard must agree about which tickets need a person,
preserve the authored ask verbatim, and distinguish external waiting that Lisa
will check itself.

The unblock command must verify an optional check without modifying the live
project, bound its execution time, explain a decline in plain words, and reopen
only a genuinely eligible parked ticket.

The existing durable contract remains unchanged:

- `status: blocked` means parked;
- `status: open` restores normal scheduling;
- canonical `review-disposition.json` carries the remedy details.

## Decision 1: centralize parked-remedy discovery in `lisa-core`

Add a `parking` module with a small owned projection, conceptually:

```text
ParkedRemedy {
    ticket_id,
    remedy_owner,
    ask,
    check,
}
```

Expose a collector that accepts an iterator of tickets and a canonical work
directory.

It filters for `TicketStatus::Blocked`, parses each canonical disposition, and
retains only valid `ReviewDisposition::Block` values.

It sorts the result by ticket ID before returning.

### Alternatives

Status and plugin code could each parse `<work>/<id>/review-disposition.json`.
That is locally small but duplicates the most important semantics: blocked-only
filtering, legacy fallback behavior, invalid-document treatment, and ordering.

The collector could live in `lisa-cli`, with the plugin including or copying
it. That reverses the dependency boundary because the plugin already depends
on core but must not depend on the CLI binary crate for production behavior.

The disposition parser could gain ticket scanning directly. That module owns a
single document contract; project-level discovery is a distinct concern and
deserves a narrow module name.

### Rationale

Both consumers already link `lisa-core`, and core already owns both input
types. One projection prevents status/dashboard drift without introducing
configuration or rendering concerns into core.

## Decision 2: omit raw reason and steps from the human projection

`ParkedRemedy` carries only fields used by this ticket.

It intentionally omits:

- raw engineering `reason`;
- optional multi-step detail;
- `unstructured` parser metadata.

### Alternatives

Returning the entire `ReviewDisposition` would avoid a new struct but force
every caller to repeat enum matching and make it easy to render raw reason.

Carrying steps would support a future expanded help view but violates the
current one-line, ask-only surface and creates unused public data.

### Rationale

The projection itself enforces the ticket's information boundary. Status and
dashboard cannot accidentally expose the fields the operator should not need.

## Decision 3: render one shared semantic line shape

Both surfaces use the section heading `Waiting on you`.

Operator-owned line:

```text
<ticket-id>  <ask exactly as authored>
```

World-owned line:

```text
<ticket-id>  <ask exactly as authored> — Lisa checks on its own.
```

Agent-owned parked remedies are not classified as operator-owned asks.

The scheduler parks agent work only after exhausting its retry bound, but that
case is not an action a person was explicitly assigned by the structured owner.
It remains visible in ordinary blocked board state.

The section renders when at least one operator or world item exists.

### Alternatives

Using the raw block reason would preserve historical behavior but reintroduces
the engineering prose this story exists to remove.

Showing owner labels such as `[operator]` and `[world]` would aid debugging but
adds subsystem vocabulary to the human line.

Rendering world items in a separate `Waiting on the world` section is explicit
but consumes more vertical space and departs from the story's one-line hint.

### Rationale

The ticket ID provides the minimum identity needed by `lisa unblock <id>`.
Everything after the two-space separator is either the authored ask or the one
plain promise about Lisa's behavior. No reason, JSON field name, phase, status,
or owner token appears.

## Decision 4: status prints waiting lines before DAG mechanics

`run_status` discovers parked remedies immediately after scanning tickets and
prints the section before DAG counts, cycle detail, configuration, waves, and
run summary.

Cycle validation still occurs before returning success; output ordering does
not weaken validation.

### Alternatives

Appending the section near `Ready to schedule` would be less invasive but
fails the requirement that status opens with the human action.

Moving all status rendering to a returned string would make unit capture easy
but is unnecessary refactoring outside this ticket.

### Rationale

A binary fixture can capture stdout without changing the existing print-based
implementation. The operator sees the only potentially required action before
any DAG or scheduling vocabulary.

## Decision 5: add a first-class dashboard waiting vector

Add a small UI-owned `WaitingItem` with:

- ticket ID;
- ask;
- whether Lisa checks on its own.

Add `waiting_items: Vec<WaitingItem>` to `PluginState` and its default.

`State::to_ui_state` calls the shared core collector over durable DAG tickets
and maps operator/world remedies into this UI type.

Render the section near the top of the Operations view, before the existing
attention banner and thread table.

### Alternatives

Reusing `ParkedThread` is incompatible with the shipped policy because durable
parks remove their thread and release the seat.

Reclassifying waiting asks as `HealthAlert` would provide a visible banner but
would add failure labels and suggested-action machinery that the ticket says
not to show.

Adding another dashboard pane or modal is explicitly outside story scope.

### Rationale

The new vector models durable parked remedies instead of live sessions. The
existing vector-of-lines rendering and direct tests remain intact.

## Decision 6: make `unblock` an everyday operator command

Add visible Clap syntax:

```text
lisa unblock <id> --path <project>
```

The ticket ID is positional because it is the primary object and matches the
story spelling.

The command description names the outcome: verify what changed and let the
ticket run again.

Dispatch resolves the project path with the existing helper and delegates to a
new `unblock` module.

### Alternatives

Adding `--unblock` to `lisa status` would mix observation and mutation and does
not match the requested command.

Keeping it hidden as plumbing would make the human recovery path undiscoverable.

Accepting multiple IDs would complicate partial success and check output with
no acceptance need.

### Rationale

One explicit ticket per invocation makes every status write and decline easy to
understand. The command belongs beside status in the everyday help ordering.

## Decision 7: validate durable state before executing anything

The unblock flow is:

1. load configured ticket/work directories;
2. scan tickets and find the exact ID;
3. require `status: blocked`;
4. parse the canonical disposition;
5. require a valid `ReviewDisposition::Block`;
6. run its check if present;
7. on success or absent check, write `status: open`;
8. print that the ticket can run again.

No check runs for an unknown, open, done, or malformed ticket.

### Alternatives

Opening any blocked ticket even if its disposition is missing would be a useful
manual escape hatch but bypasses the structured contract this command is meant
to verify.

Accepting a CLI-provided replacement check would turn unblock into remediation
or arbitrary command execution and leave no durable review record.

### Rationale

The canonical block remains the authority for what to verify. Fail-closed input
handling prevents an unrelated blocked dependency from being mistaken for a
parked Review remedy.

## Decision 8: represent a failed check as a normal declined outcome

The module returns a domain result that distinguishes:

- reopened with a success message;
- declined with a plain message;
- operational failure with an internal error string.

Main prints reopened messages to stdout.

It prints declined messages directly to stderr and exits nonzero without the
generic `Error:` prefix.

Operational configuration, scanning, and write failures keep the established
`Error:` path.

### Alternatives

Returning every decline as `Err(String)` reuses existing dispatch but prefixes
the result with `Error:`, making an expected observation sound like a crash.

Returning exit zero on decline could make shell automation believe the ticket
was reopened.

### Rationale

Nonzero accurately reports that the requested state change did not happen,
while the dedicated output path guarantees no stack trace or Rust error chain.

## Decision 9: run checks in a disposable project snapshot

The check runner creates a temporary directory outside the live project.

For Git projects it asks Git for tracked and non-ignored untracked paths and
copies their current filesystem contents into the snapshot.

For small non-Git fixtures it recursively copies the root while excluding
repository control and common build/attempt caches.

Symlinks are not followed into the live tree; unsafe links are omitted from the
execution view.

After copying, remove write bits from every snapshot file and directory while
preserving executable bits.

Set the shell's current directory to the snapshot.

Capture a content/metadata fingerprint before and after execution.

Any detected change makes the check fail even if its exit status is zero.

Restore temporary permissions only for cleanup and discard the snapshot.

### Alternatives

Run in the live project under chmod and restore permissions. This temporarily
changes the operator's project and risks leaving it unusable after interruption.

Run in the live project and compare Git status. This detects only some changes
after they have happened and cannot safely revert concurrent/user work.

Reject shell text containing apparent mutation commands. Shell syntax is too
expressive for a reliable deny-list and aliases or scripts evade lexical checks.

Require platform sandbox tools. macOS and Linux expose different facilities,
and Lisa's one-command install cannot assume Bubblewrap or another daemon.

### Rationale

The disposable view gives useful relative-path observations and makes project
mutation recoverable by construction. Read-only permissions supply immediate
failure in the ordinary case; the fingerprint closes the privileged-process
test case where permissions might be bypassed.

## Decision 10: bound the complete shell process group

Use `/bin/sh -c <check>` with a production timeout of five seconds.

Redirect stdout and stderr to temporary files so output cannot fill a pipe and
block the child while the parent polls.

On Unix, start the shell in its own process group.

Poll `try_wait` at a short interval until exit or deadline.

At the deadline, terminate the process group and wait for the shell.

Tests call the runner with a shorter injected duration.

### Alternatives

Calling a system `timeout` binary is concise but not portable to macOS.

Killing only the shell leaves child processes such as `sleep` or `curl` alive.

Using blocking `Command::output` cannot enforce a deadline.

### Rationale

The explicit process group makes the wall-clock bound apply to the authored
probe rather than only to one wrapper PID. Temporary output files keep the poll
loop independent of output volume.

## Decision 11: reduce observations to one safe plain line

On ordinary nonzero exit, select the first non-empty line from stderr, then
stdout.

Strip control characters, trim surrounding whitespace for display, and cap the
line length.

Format the decline as:

```text
That didn't work yet — <observation>.
```

When no line is available, use `it still isn't ready`.

Timeout uses `it took longer than 5 seconds`.

Detected writes use `it tried to change project files`.

Avoid printing exit codes, shell command text, parser variants, paths to Review
JSON, or Rust I/O chains in expected declines.

### Alternatives

Printing complete stderr preserves diagnostics but can include stack traces,
multi-line tool logs, secrets, and subsystem jargon.

Mapping every failure to one generic sentence loses the authored observation
the story explicitly wants quoted.

### Rationale

One bounded line preserves the most useful observation and maintains the same
at-a-glance interaction as the waiting section.

## Decision 12: verify readiness through a fresh DAG

Integration coverage will not merely inspect the rewritten YAML.

After a passing or absent-check unblock, the fixture rescans tickets, rebuilds
`Dag`, and asserts the ticket appears in `get_ready_tickets`.

Failure, mutation, and timeout fixtures assert both the on-disk blocked status
and absence from ready tickets.

### Rationale

This directly proves the acceptance phrase “the next scheduling pass seats the
ticket” at the scheduler eligibility boundary without launching a full Zellij
loop.

## Compatibility and scope boundaries

No disposition wire format changes.

No provenance schema changes.

No completion or lease behavior changes.

No automatic world check execution is introduced in plugin timer paths.

No Review template wording changes; T-048-02-02 owns that serialized file.

Existing attention banners remain for Review gates and health failures.

The new waiting section precedes rather than replaces those signals.

Existing status waves and run summary remain byte-compatible after the new
leading section when no parked operator/world remedy exists.

## Verification decision

Add core unit tests for discovery and sorting.

Add plugin UI tests for operator and world lines, ask preservation, ordering,
and absence of reason/owner labels.

Add black-box CLI fixtures for status plus every unblock outcome.

Add focused runner tests for timeout and mutation isolation.

Run formatting, focused core/CLI/plugin tests, workspace check, full workspace
tests, diff checks, and post-commit ownership audits.
