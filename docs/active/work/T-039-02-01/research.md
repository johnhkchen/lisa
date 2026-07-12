# Research: T-039-02-01

## Ticket boundary

- The ticket is `T-039-02-01`, `characterize-signal-consumers`.
- Its current phase is Research.
- It depends on completed ticket `T-039-01-01`.
- The acceptance criterion requests characterization before structural edits.
- The characterization must cover all eight `check_*_signals` consumers.
- It must record poll order, payload admission, legacy naming, deletion, and effect.
- The suite must pass against current behavior.
- Product behavior must not change.
- Attempt artifacts belong under `.lisa/attempts/T-039-02-01/1/work/`.
- Lisa publishes admitted artifacts later.
- Ticket phase and status frontmatter are Lisa-owned.

## Relevant code boundary

- All eight consumers are private methods on `State`.
- They live in `crates/lisa-plugin/src/lib.rs`.
- `State::poll_tick` invokes all eight.
- The consumers read from `State::signal_dir`.
- Signal files are filesystem records.
- Filename shape selects a consumer and usually identifies a pane.
- `pane_id_from_signal_filename` implements the shared pane-name grammar.
- That helper requires `pane-<u32><suffix>` exactly.
- Non-UTF-8 names, overflow, missing IDs, and wrong suffixes are rejected.
- The idle consumer retains one legacy ticket-ID filename form.
- Existing unit tests are an inline `#[cfg(test)] mod tests` in `lib.rs`.
- Inline tests can access private `State` methods and state.
- A child module of `tests` can also access those members through its parent.
- Integration tests cannot directly exercise these private consumers.

## Poll order

- `poll_tick` calls `check_heartbeat_signals` first.
- It then calls `check_awaiting_signals`.
- `deliver_ready_assignments` sits between awaiting and process-start ingestion.
- `check_process_start_signals` is the third signal consumer.
- `check_shell_ready_signals` is fourth.
- `check_codex_ack_signals` is fifth.
- `check_artifact_advances` sits between ack and idle ingestion.
- `check_idle_signals` is sixth.
- `check_transition_signals` is seventh.
- `check_error_signals` is eighth.
- Transition timeouts run after all eight consumers.
- Assignment-ack timeouts also run after all eight consumers.
- The comments attach behavioral meaning to the ordering.
- Heartbeats update clocks before health decisions.
- Awaiting signals gate later injection in the same tick.
- Ready assignment delivery occurs before newly observed process starts.
- Process-start evidence therefore stays observable at a scheduler boundary.
- Shell readiness precedes provider assignment acknowledgement.
- Ack evidence wins before timeout evaluation.
- Artifact publication precedes idle phase handling.
- Error evidence wins before transition-timeout fallback.

## Heartbeat consumer

- `check_heartbeat_signals` recognizes `pane-<id>.heartbeat` only.
- It reads the body as UTF-8.
- It parses the body as JSON `AttemptLease`.
- It removes the file after the read/parse attempt.
- Malformed and unreadable payloads have no state effect.
- Admission requires the addressed slot to exist.
- The slot ticket must equal the candidate ticket.
- The slot lease must equal the candidate lease.
- The candidate must equal the current lease authority.
- On admission it bumps slot and thread activity clocks.
- It clears attention debounce for the pane.
- It clears the awaiting-human gate for the pane.
- Legacy ticket-named heartbeat files are not recognized.

## Process-start consumer

- `check_process_start_signals` recognizes `pane-<id>.started` only.
- It reads JSON `AttemptLease` payloads.
- It deletes the file before acting on a successfully parsed candidate.
- Malformed payloads are one-shot and have no effect.
- `acknowledge_process_start` performs state admission.
- Admission is seat-, ticket-, generation-, and current-lease-scoped.
- A matching `Starting` seat becomes `ReadyForAssignment`.
- A stale candidate does not establish readiness or ownership.
- Ticket-named `.started` files are not recognized.

## Shell-ready consumer

- `check_shell_ready_signals` recognizes `pane-<id>.shell-ready` only.
- It reads JSON `AttemptLease` payloads.
- It deletes the file before invoking acknowledgement logic.
- Malformed payloads are consumed without an effect.
- `acknowledge_shell_ready` enforces the reset successor boundary.
- Only the exact current successor in `ResettingStartup` is admitted.
- Admission triggers the bounded same-pane relaunch path.
- The resulting assignment state becomes a replacement `Starting` state.
- Ticket-named `.shell-ready` files are not recognized.

## Codex acknowledgement consumer

- `check_codex_ack_signals` recognizes `pane-<id>.ack` only.
- It reads the body as an arbitrary UTF-8 string.
- It does not parse the payload in the scanner itself.
- It deletes the file before acknowledgement admission.
- `acknowledge_codex_assignment` parses and validates the provider payload.
- The payload must contain the exact pending ticket and generation tag.
- Slot lease and current lease authority must also match.
- A matching pending seat becomes `Owned`.
- Successful admission bumps activity and logs one acknowledgement.
- Duplicate, malformed, or stale acknowledgements are consumed without promotion.
- Ticket-named `.ack` files are not recognized.

## Awaiting consumer

- `check_awaiting_signals` recognizes `pane-<id>.awaiting` only.
- The body is presence-only and is never read.
- It removes the file before inserting the pane into `awaiting_human`.
- A first insertion logs an informational event.
- A duplicate still deletes the file but does not log another insertion.
- It deliberately does not bump activity.
- Ticket-named `.awaiting` files are not recognized.

## Idle consumer

- `check_idle_signals` first clears the in-memory idle alert list.
- It recognizes every UTF-8 filename ending in `.idle`.
- It deletes each recognized file immediately.
- `pane-<u32>.idle` is the current filename form.
- Current pane naming resolves the assigned ticket through `agent_slots`.
- The slot must be in `TransitionState::Idle`.
- A current pane signal bumps activity before thread/phase admission.
- `{ticket_id}.idle` is the legacy filename form.
- Legacy naming resolves the ticket directly from the filename.
- The signal body is ignored.
- A running thread is required for a phase effect.
- Research through Review generally require an admitted phase artifact.
- Implement advances to Review on idle and attempts progress publication.
- Missing artifacts can produce an idle-without-artifact alert.
- The consumer owns both legacy admission and phase effects.

## Transition consumer

- `check_transition_signals` handles two suffixes in one scan.
- It recognizes pane-prefixed names ending in `.stopped` or `.cleared`.
- It deletes a stopped/cleared file before parsing the numeric pane ID.
- Thus malformed pane-prefixed transition names are consumed.
- The body is ignored.
- Valid stopped and cleared signals bump pane activity.
- Stopped signals dispatch to `handle_stopped_signal`.
- Cleared signals dispatch to `handle_cleared_signal`.
- Stopped can advance `WaitingForStop` to `WaitingForClear`.
- Cleared can deliver the next prompt and return the slot to `Idle`.
- Transition effects depend on slot state and awaiting-human gates.
- `.idle` is explicitly left for the idle consumer.
- Ticket-named stopped and cleared files are not recognized.

## Error consumer

- `check_error_signals` recognizes `pane-<id>.error` only.
- The body is presence-only and ignored.
- It removes the file before resolving an outcome.
- A recovering assignment is failed through recovery handling.
- Otherwise the running thread map is the ownership authority.
- A matching running thread is failed and removed.
- Its slot is released.
- Provenance and an error alert are emitted.
- An unknown or idle pane is consumed and logged harmlessly.
- Ticket-named `.error` files are not recognized.

## Existing test infrastructure and constraints

- `tempfile` is already available to plugin unit tests.
- `install_current_attempt` constructs matching current lease authority.
- Existing fixture helpers construct scheduling and recovery states.
- Many individual effects already have focused regression tests.
- Existing tests do not present the eight contracts as one named suite.
- Existing tests do not lock the complete call order in one assertion.
- Several current tests already prove deletion of successful signals.
- Malformed start and stale heartbeat tests establish fail-closed patterns.
- The requested suite should make the cross-consumer differences explicit.
- Test-only modularization avoids changes to runtime methods or data structures.
- The worktree already contains Lisa-owned changes to ticket/provenance files.
- Those existing changes are outside ticket source ownership.
- Any source commit must use exact paths through `lisa commit-ticket`.

