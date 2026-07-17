# Research — T-048-02-01 status-and-unblock-ux

## Scope observed

This ticket owns the human-facing surface for already parked Review blocks.

It adds three connected behaviors:

- status output that leads with actionable parked-ticket information;
- matching dashboard lines;
- an explicit command that verifies, then reopens, one parked ticket.

The ticket does not own automatic world rechecks or Review prompt authoring.

Those are assigned to dependent ticket T-048-02-02.

The ticket also does not own the block schema or parking scheduler policy.

Those shipped in T-048-01-01 and T-048-01-02 respectively.

## Assignment and repository state

The ticket frontmatter begins this attempt in `phase: research`.

Lisa owns frontmatter phase/status transitions for the active assignment.

Phase artifacts must stay under the current attempt's private work directory.

Implementation source units must be committed with `lisa commit-ticket` and
exact repository-relative include paths.

The ordinary worktree already contains unrelated modified and untracked Lisa,
ticket, and work-artifact paths.

Those paths are not available for this ticket to stage, rewrite, or commit.

Dependency T-046-07-01 is complete, so `main.rs` is available for the new
command and its purpose-first copy is now the baseline.

Dependency T-048-01-01 is complete, so structured Review dispositions are
available from `lisa-core`.

Dependency T-048-01-02 is also present on `main`, so durable parking semantics
can be consumed directly.

## Structured Review disposition

`crates/lisa-core/src/disposition.rs` is the parser and semantic boundary.

`ReviewDisposition::Block` contains:

- the original `reason`;
- typed `RemedyOwner` (`Agent`, `Operator`, or `World`);
- `ask`, preserved as an owned string;
- optional `steps`;
- optional inert `check` shell text;
- an `unstructured` fallback marker.

The parser preserves authored ask and check bytes rather than normalizing them.

A valid legacy two-field block becomes a complete safe fallback:

- operator owned;
- raw reason copied to ask;
- no steps;
- no check;
- `unstructured: true`.

Malformed outer documents remain `ReviewDisposition::Invalid`.

Parsing never evaluates the check.

The canonical file is named `review-disposition.json`.

For a parked ticket it remains at `<work_dir>/<ticket-id>/` after the live
thread and attempt lease have been released.

## Durable parking authority

`crates/lisa-plugin/src/lib.rs::apply_review_block_policy` consumes admitted
current-attempt Review artifacts.

Operator- and world-owned blocks park immediately.

Agent-owned blocks retry to a fixed per-loop bound before parking.

Parking first rewrites the real ticket to `status: blocked`.

Only after that durable write succeeds does the plugin release the slot and
remove the thread.

The canonical Review disposition is not copied into ticket frontmatter.

The parked ticket therefore has two durable inputs:

- scheduling authority in ticket status;
- human/check payload in canonical work.

There is intentionally no parallel parked-ticket database.

## DAG and scheduling behavior

`crates/lisa-core/src/dag.rs::Dag::can_start` rejects a ticket whose status is
`TicketStatus::Blocked`.

`Dag::get_ready_tickets` delegates to that rule and sorts the result.

The phase remains startable while a Review ticket is parked; blocked status is
the relevant exclusion.

Changing the ticket back to `TicketStatus::Open` is sufficient to restore
ordinary DAG eligibility when dependencies are done.

No unblock allow-list, retry token, or scheduler-only state is required.

`lisa_core::ticket::update_ticket_status` is the existing frontmatter writer.

It rewrites only the status field while retaining the rest of the ticket.

The plugin's `reconcile_unpark_transitions` observes an open ticket whose latest
parking provenance row is `Park` and appends the corresponding `Unpark` row.

Scheduling does not depend on that best-effort provenance append.

## CLI command surface

`crates/lisa-cli/src/main.rs` defines the Clap command inventory and dispatch.

The visible everyday path is ordered as init, validate, status, doctor, loop.

`Status` currently accepts project path and a separate hidden diagnostic form
for one ticket's pre-ownership ledger evidence.

There is no `Unblock` variant today.

Visible command descriptions use purpose-first operator language after
T-046-07-01.

The dispatch convention resolves relative paths against the current directory.

Operational functions usually return `Result<(), String>`.

Generic errors are prefixed with `Error:` by `main` and exit nonzero.

A failed remedy check is an expected operator outcome rather than an internal
stack or parser failure, so it needs a deliberate output boundary.

## Current status output

`crates/lisa-cli/src/status.rs::run_status` resolves configured ticket and work
directories, scans tickets, and constructs a DAG.

It currently prints, in order:

1. DAG count and cycle summary;
2. critical path and status counts;
3. scheduling configuration;
4. execution waves and ticket rows;
5. ready-to-schedule summary;
6. the shared run summary.

It does not read Review dispositions.

Its unit tests create temporary project fixtures but assert only success/error.

They do not capture or pin the normal status stdout.

The binary integration-test pattern uses `env!("CARGO_BIN_EXE_lisa")` and
`std::process::Command`, as seen in `preownership_status.rs` and help tests.

That pattern can assert ordering and exact new strings from real CLI output.

The configured `work_dir` is already resolved by the status path and is
currently used only for the final run summary.

## Dashboard projection and rendering

`crates/lisa-plugin/src/lib.rs::State::to_ui_state` converts scheduler state to
the self-contained types in `crates/lisa-plugin/src/ui.rs`.

The projection has access to both:

- every durable DAG ticket;
- the configured canonical work directory.

It currently builds `ParkedThread` rows only from retained in-memory threads
whose `ThreadStatus` is `Parked`.

The new parking policy does not retain such threads, so that vector is not the
source for durable parked Review blocks.

`PluginState` currently carries tickets, active/parked threads, activity,
alerts, slots, timing, modal state, pause state, and active view.

It has no structured waiting item.

The Operations view renders the dashboard title, an attention banner, the
thread table, and filtered activity.

`render_attention_banner` is concerned with Review tickets and health alerts.

It shows titles, artifact names, wait times, failure labels, and suggested
actions; that is broader and more technical than the ticket's ask-only line.

The existing line renderer returns a vector of strings and is directly unit
tested without launching Zellij.

Tests can assert a new waiting section appears before thread/attention detail
and that the ask remains a verbatim substring.

All `PluginState` fixtures generally use `..PluginState::default()`, limiting
the cost of adding one defaulted vector field.

## Shared discovery boundary

Both CLI status and plugin projection must answer the same question:

“Which blocked tickets have a valid canonical Block disposition, and what are
their typed owner, ask, and optional check?”

The inputs live in `lisa-core` types and project files.

If CLI and plugin each reproduce filtering/parsing/sorting, their user surfaces
can drift on legacy fallback, invalid documents, or ticket ordering.

`lisa-core` already owns both ticket types and disposition parsing and is linked
by both consumers.

There is no existing parked-remedy collector module.

The collector needs no CLI config knowledge: callers can supply tickets and the
resolved work directory.

Stable ticket-ID sorting is important because DAG storage uses a `HashMap`.

## Check execution constraints

The `check` field is authored shell text whose exit code is the result.

This ticket is the first execution boundary for that previously inert text.

Requirements constrain it in three independent ways:

- bounded wall-clock execution;
- no live project mutation;
- plain outcome text with no stack trace.

`std::process::Command` has no built-in timeout API.

A bounded runner must spawn, poll with `try_wait`, terminate at a deadline, and
collect a small observation from stdout/stderr.

Piped output can deadlock if a child fills the pipe while the parent only polls.

Redirecting stdout/stderr to temporary files avoids that wait/output coupling.

Killing only `/bin/sh` can leave its descendant alive.

On Unix, placing the child in its own process group and killing the group closes
that timeout hole.

Lisa's supported execution environment is Unix-oriented (shell hooks, Zellij,
and existing Unix permission code), while tests still benefit from a guarded
non-Unix fallback.

Running the authored check in the live root and comparing `git diff` afterward
would detect mutation only after damage occurred.

It would also miss ignored files and non-Git side effects.

A disposable project snapshot gives the check the same relative project view
without granting it the live tree as its working directory.

Removing write bits supplies the read-only fixture behavior requested by the
acceptance criterion.

A before/after snapshot fingerprint also rejects a write attempt if permissions
are bypassed (for example by a privileged test process).

The temporary tree can be discarded after every outcome.

For Git projects, copying `git ls-files --cached --others --exclude-standard`
avoids ignored build trees such as this repository's 20 GB `target` directory
and 447 MB attempt store while retaining current tracked and untracked project
inputs.

Small non-Git fixtures need a recursive fallback with known repository-control
and build-cache directories excluded.

Shell text may still name absolute external paths; complete operating-system
sandboxing is not exposed portably by Rust's standard library.

The contract calls checks read-only, while the implementation boundary can
guarantee that relative project writes affect only disposable state and are
reported as failure.

## Failure observations and brand voice

The ticket supplies the target shape: “That didn't work yet — …”.

The useful observation is the first non-empty line emitted by the check.

Newlines, control characters, unbounded output, raw exit codes, and Rust I/O
errors are unsuitable for the operator surface.

A short single-line observation can be selected from stderr/stdout and capped.

When no useful line exists, the command still needs a plain fallback such as
“it still isn't ready.”

Timeout and attempted-write outcomes need purpose language, for example that it
took too long or tried to change project files.

The ticket status must remain blocked for every decline path.

Only a zero exit status, no detected snapshot mutation, and no timeout permits
the real status update.

## Verification surfaces

Focused core tests can pin collector filtering, legacy fallback, and sorting.

CLI binary fixtures can pin:

- Waiting on you appears before DAG detail;
- operator ask bytes appear and reason/owner jargon does not;
- world lines say Lisa checks on its own;
- failed checks decline and retain blocked status;
- passing and absent checks reopen the ticket;
- a fresh DAG then returns the reopened ticket as ready;
- timeout returns promptly and retains blocked status;
- a relative write attempt cannot touch the live fixture and is declined.

Plugin UI tests can pin the same waiting line before the existing operational
sections without a real Zellij process.

Workspace tests cover exhaustive enum matches and all `PluginState` fixtures.

Formatting, diff hygiene, focused crate tests, and the complete workspace suite
are the repository's established completion checks.

## Constraints carried into Design

- Durable blocked/open ticket status remains the only scheduling authority.
- Canonical Review disposition remains the only ask/check payload store.
- Invalid or absent disposition data must not manufacture an unblock check.
- Operator asks must be displayed without replacing or trimming their content.
- Status and dashboard need one shared discovery meaning and deterministic order.
- Expected check failure must not surface through generic `Error:` diagnostics.
- A check must never run with the live repository as its working directory.
- Timeout must cover the shell process tree, not only the immediate child.
- No source unit may depend on the attempt-private artifact directory.
- Automatic world-owned recheck remains for T-048-02-02.
