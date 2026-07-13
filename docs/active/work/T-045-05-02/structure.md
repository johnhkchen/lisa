# Structure — T-045-05-02 field regression assertions

## Change inventory

Modify `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`.

Modify `docs/knowledge/live-codex-review-boundary.md`.

Create no Rust source file.

Create no default Cargo integration test.

Delete no file.

Retain all RDSPI artifacts only in this attempt's private work directory.

## Harness constants

Replace the single `TICKET_ID` constant with:

- `PRIMARY_TICKET_ID=T-FIELD-REVIEW-01`;
- `SUCCESSOR_TICKET_ID=T-FIELD-REVIEW-02`;
- a two-element ticket list used by generic loops.

The legacy case addresses only `PRIMARY_TICKET_ID`.

The current case addresses both IDs in dependency order.

## Runtime state

Retain existing current-case globals for case, root, session, panes, and processes.

Add sampler-local state files rather than more shell globals where possible.

State-transition memory lives under the active evidence case.

Process de-duplication keys live under the active evidence case.

Lease-transition memory lives under the active evidence case.

The attempt-private claim gates remain under each fixture ticket's work directory.

## Fixture writer

`create_fixture(kind, lisa_bin)` remains the only project constructor.

It will create two ticket Markdown files.

The first ticket retains the prior Review recovery wording.

The second ticket uses equivalent wording and depends on the first.

Both tickets receive canonical `docs/active/work/<ticket>/review.md`.

The same `AGENTS.md` protocol applies to either assignment environment.

The Git baseline continues to include the entire fixture before Lisa starts.

## Agent protocol

`write_agents_protocol` remains generic over inherited environment variables.

The current-attempt work directory is derived from `LISA_TICKET_ID` and
`LISA_ATTEMPT_ID`.

The immutable assignment lookup remains limited to that directory.

The per-ticket gate remains `.field-claim-gate`.

The exact `lisa claim` remains within the first shell tool action.

No stale probe is added to agent instructions.

This keeps agent ownership behavior identical for predecessor and successor.

## Dashboard row parser

Add a helper that extracts the scheduler thread row for one ticket.

It accepts a dashboard snapshot string and ticket ID.

It selects the bracket-prefixed thread row, not the story progress row.

Add a helper that maps that row to the known assignment state label.

Known labels are:

- `starting`;
- `delivering`;
- `delivered-awaiting-claim`;
- `owned`;
- `claim-timed-out`;
- `delivery-failed` when exposed.

## State transition recorder

Replace `record_state_once(state, dashboard)` with ticket-scoped recording.

`record_ticket_state(ticket, dashboard)` compares the detected state to
`.last-state-<ticket>`.

On change it appends:

```text
timestamp<TAB>ticket<TAB>state
```

to `state-events.tsv`.

It does not suppress a state that returns later.

For current tickets, observing `delivered-awaiting-claim` starts gate handling.

For the primary ticket, gate handling writes its gate immediately.

For the successor, gate handling first runs the stale-claim probe.

The historical failed banner is appended as a ticket-scoped `FAILED` state.

## Stale-claim probe

Add `probe_stale_successor_claim`.

It runs at most once, guarded by an evidence marker.

It derives numeric pane ID from `CURRENT_AGENT_PANE`.

It invokes `CURRENT_LISA_BIN claim` with:

- fixture root as `--path`;
- successor ticket ID;
- attempt ID zero;
- nonce zero;
- actual `LISA_PANE_ID` in the environment.

It records stdout, stderr, and exit status under the current evidence case.

Success is an immediate harness failure.

Nonzero without `[stale-attempt]` is also a harness failure.

Only after a passing rejection does the recorder create the successor gate.

## Process sampler

Keep `process-snapshots.txt` as raw diagnostics.

Add `sample_process_events` over the same `ps -axo pid,ppid,command` input.

For each ticket, match its exact `assignment-1-*.md` path segment.

Classify current Lisa `launch-codex` as `launcher`.

Classify the provider child beginning with `codex` as `codex`.

Use `role:pid` as the de-duplication key.

Append first observation to `process-events.tsv` with:

```text
timestamp<TAB>ticket<TAB>role<TAB>pid<TAB>ppid<TAB>assignment
```

The raw command remains in snapshots if parsing fails.

## Lease sampler

Add `sample_lease_state` after signal sampling.

Read the actual pane lease path using the numeric agent pane ID.

Parse ticket ID and attempt ID with `jq`.

Use `ticket:attempt` as the present identity.

Use `absent` when the marker does not exist or is not readable.

Append only identity transitions to `lease-events.tsv`.

The structured row shape is:

```text
timestamp<TAB>status<TAB>ticket<TAB>attempt
```

The existing captured-signals directory continues to preserve lease bodies by digest.

## Sampler initialization

`start_sampler` creates these new empty files:

- `process-events.tsv`;
- `lease-events.tsv`;
- `.seen-processes`.

It removes or initializes last-state and last-lease markers for the fresh case.

It keeps the 100-millisecond interval.

## Wait helpers

Change `state_was_seen` to accept ticket and state.

It filters columns two and three.

Change `claim_was_captured` to accept a ticket.

It loads captured `.claim` JSON and matches the ticket.

Change `ticket_is_done` to accept a ticket.

Add `all_current_tickets_are_done` for the final wait.

Legacy wait addresses primary `FAILED` only.

Current waits address both tickets independently.

## Evidence capture

`capture_fixture_evidence` copies both final ticket files.

Use a `tickets-final/` directory instead of one `ticket-final.md`.

Copy both attempt directories beneath `attempt-snapshot/<ticket>/`.

Copy both canonical work directories beneath `work-snapshot/<ticket>/`.

Retain journal, provenance, layout, Git log, status, and final screens.

The runbook evidence tree is updated accordingly.

## Per-ticket assertion helper

Add `assert_current_ticket(ticket)`.

It discovers exactly one launch script.

It discovers exactly one immutable assignment.

It derives the nonce from that filename.

It asserts the launch script calls `launch-codex` with the pane-relative exact path.

It asserts exactly one captured claim JSON matches ticket, attempt, and nonce.

It asserts the exact state sequence appears once and in order.

It asserts no failure state occurs.

It asserts one structured launcher and one structured Codex process row.

## No-reinjection helper

Add `assert_no_duplicate_injection(ticket)`.

Its primary state proof is exact transition sequence cardinality.

It additionally parses terminal snapshot sections.

Within each section it counts the ticket's tagged `LISA_ASSIGNMENT` line.

No section may contain more than one line.

Exactly one assignment file and one launch script are already required.

The helper emits no receipt itself.

## Fresh-boundary helper

Add `assert_fresh_boundary` after both tickets are Done and evidence is captured.

It extracts the one launcher and Codex PID per ticket.

It requires predecessor and successor PIDs to differ for both roles.

It requires the lease event order:

1. present predecessor attempt one;
2. absent;
3. present successor attempt one.

It requires captured lease JSON for both exact identities.

It requires stale successor rejection evidence.

It requires no attempt-zero claim signal.

## Completion helper

Add `assert_exact_completions`.

It reads copied or live completion journal JSONL with `jq -s`.

It validates one record for each state and ticket.

It validates attempt and generation one.

It validates confirmed commit ID shape.

It reads provenance JSONL with `jq -s`.

It validates one authoritative unfenced Codex Done record per ticket.

It validates total relevant cardinality two.

## Current-case order

Prepare fixture and start loop.

Start sampler.

Wait for predecessor passive wait, exact claim, and ownership.

Wait for predecessor Done.

Wait for successor passive wait.

The state recorder performs and records stale rejection before opening the gate.

Wait for successor exact claim and ownership.

Wait for both tickets Done.

Stop sampler and capture evidence.

Run all offline assertions over retained evidence.

Print granular receipts, then stop the case.

## Runbook structure

Update Purpose from one current ticket to a two-ticket lifecycle.

Update expected receipts.

Update fixture instruction and state sequencing.

Document producer-side stale rejection and why it does not synthesize ownership.

Document transition, process, and lease ledgers.

Document exact journal/provenance cardinalities.

Remove the paragraph delegating these assertions to this ticket.

Retain authorization, prerequisites, debug overrides, redaction, and interpretation limits.

## Commit boundary

One meaningful source unit spans the harness and its runbook.

Commit both together with one isolated ticket transaction.

Exact include paths are the two modified files.

Evidence and RDSPI artifacts are not source commit includes.

After commit, both source paths must be clean.

The ordinary index must remain untouched.
