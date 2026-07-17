# Research: block triage proposal

## Assignment boundary

T-049-07-01 begins in Research and requires all six RDSPI phases.

The ticket extends operator-owned Review parking after T-049-05-01.

It does not change which Review dispositions block completion.

It does not authorize an agent to settle a block automatically.

Its output is advice attached to an already parked ticket.

Failure, timeout, or disabled triage must preserve the existing park.

The T-046-06-03 field incident is the required regression case.

## Existing Review disposition model

`crates/lisa-core/src/disposition.rs` owns fail-closed disposition parsing.

`ReviewDisposition::Block` carries reason, remedy owner, ask, optional steps,
optional check, and the legacy-unstructured marker.

`RemedyOwner` has Agent, Operator, and World variants.

Legacy bare-reason blocks become Operator-owned and unstructured.

The parser does not currently contain a triage or proposal type.

The canonical disposition remains in the configured work directory.

Private attempt copies live under `.lisa/attempts/<ticket>/<generation>/work`.

Artifact admission requires current lease evidence and canonical publication.

## Existing level-triggered parking

`crates/lisa-plugin/src/lib.rs` owns the live park flow.

`apply_review_block_policy` examines current live Review threads.

Operator and World blocks park immediately.

Agent blocks receive two bounded scheduler retries before parking.

Parking writes `status: blocked` before best-effort provenance.

It then releases the seat, removes the thread, and rebuilds the DAG.

`reconcile_orphaned_review_blocks` handles a vanished writing thread.

It correlates canonical bytes with the newest private attempt generation.

It rejects already-consumed generations using parking provenance.

It performs the same status, provenance, seat, and DAG consequence.

The reconciler runs at load, each poll, and scheduling admission.

T-049-05-01 therefore makes the park itself level-triggered and durable.

No proposal may become part of that status transition's success condition.

## Existing parked projection

`crates/lisa-core/src/parking.rs` owns `ParkedRemedy`.

`collect_parked_remedies` includes only status-blocked tickets with valid Block
dispositions.

The projection carries ticket ID, owner, ask, reason, and optional check.

An unstructured block receives `LEGACY_BLOCK_ASK` as its plain lead.

The raw reason is preserved below that lead.

`crates/lisa-cli/src/status.rs` renders Operator and World remedies.

It omits Agent-owned remedies from Waiting on you.

It currently renders the ask first and the reviewer's raw note second.

`crates/lisa-plugin/src/lib.rs` maps the same projection into UI values.

`crates/lisa-plugin/src/ui.rs` renders the same two-line ordering.

There is no durable supplementary parked-state document today.

## Existing provenance

`crates/lisa-core/src/provenance.rs` owns append-only JSONL records.

The current schema version is six.

The ledger is an untagged union of execution, assignment, and parking rows.

Parking rows use Retry, Park, and Unpark discriminators.

They retain the source attempt lease, owner, bounds, and interval.

Append functions create parents and append one compact JSON line.

The plugin writes park transitions after durable status succeeds.

Existing ledger readers generally ignore malformed or irrelevant rows.

Adding disjoint record-type discriminators is compatible with the union model.

There is no triage-attempt or proposal-action provenance today.

## Existing configured agent capacity

`PluginConfig.max_threads` is the global configured concurrency ceiling.

Optional provider caps further restrict Claude or Codex concurrency.

The scheduler counts Running ticket threads against the global ceiling.

It resolves a ticket route before applying provider limits.

Two times `max_threads` physical panes exist to absorb transitions.

Park release frees the blocked ticket's ordinary scheduler seat.

The plugin can also run bounded host commands through Zellij RunCommands.

World rechecks use that host-command path with an in-flight guard.

RunCommandResult context keys attribute completion and recheck results.

Host commands are currently not counted as agent threads.

A triage host process therefore needs explicit capacity accounting.

## Existing agent launch surfaces

The configured default client is Claude or Codex.

Per-ticket frontmatter may resolve a different client and model.

Interactive ticket work uses provider adapters and terminal panes.

`lisa agent-exec` already drives headless `codex exec --json` for diagnostics.

It translates streaming events and extracts usage but is ticket-signal oriented.

Claude's normal path is interactive and hook-driven.

There is no provider-neutral one-shot structured-output runner.

The CLI already has `libc`, `tempfile`, and process APIs needed for a timeout.

The plugin already receives the absolute Lisa binary in its layout config.

## Configuration path

`crates/lisa-cli/src/config.rs` parses `.lisa.toml`.

Known top-level sections and keys are explicitly validated.

`ResolvedConfig` applies defaults and loop CLI overrides.

`crates/lisa-cli/src/loop_cmd.rs` serializes resolved values into KDL.

`PluginConfig::from_config_map` parses the KDL values leniently in WASM.

No triage section or bound exists today.

The ticket explicitly requires a disabled configuration behavior.

The disabled state must be visible rather than inferred from tool absence.

## Explicit operator commands

`lisa unblock <ticket>` is the current parked-ticket command.

It optionally runs a read-only check, then writes status Open.

World checks automatically reopen only observable World-owned remedies.

Neither command records a proposal apply or dismiss decision.

No command currently executes or acknowledges prepared proposal steps.

Clap commands are defined and dispatched in `crates/lisa-cli/src/main.rs`.

Ticket status updates use `lisa_core::ticket::update_ticket_status`.

## T-046-06-03 incident evidence

The preserved legacy reason names two criteria/evidence mismatches.

The Codex leg measured 225 MiB against an approximately 200 MiB criterion.

The runbook had already been calibrated to 300 MiB from field evidence.

The old-Zellij variant proved managed mode bypassed the seeded old binary.

The written criterion still asked for string-guided recovery.

The reviewer recommended conforming reruns or explicit criterion amendments.

The operator note records that two sentence-level amendments resolved it.

No run evidence was changed.

The correct triage classification is criteria versus cited evidence.

The correct recommendation is to amend the stale criteria to match evidence.

## Constraints surfaced by the codebase

The park must complete before triage launch or result processing.

Proposal absence must be a valid steady state.

Only Operator-owned parks qualify for this ticket's first responder.

One source park generation needs at most one bounded triage attempt.

A plugin restart must not spend again for the same source generation.

The triage process needs a hard wall-clock timeout and process-group cleanup.

Its output must parse fail-closed before becoming operator-visible.

Its filesystem access should be read-only because it only investigates.

The output must include summary, recommendation, and prepared steps.

Status and dashboard must place those fields before the raw reason.

Apply and dismiss must be explicit invocations, not scheduler inference.

Both actions must append durable provenance.

Wrong advice must be dismissible without disturbing the underlying park.

Existing unmodified rendering must remain exact when no active proposal exists.

Source changes must be committed only with scoped `lisa commit-ticket` calls.
