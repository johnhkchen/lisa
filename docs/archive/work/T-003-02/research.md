# T-003-02 Research: Artifact-Phase Advance

## What This Ticket Needs

When a Claude session completes a phase, it writes an artifact file (e.g., `research.md`) to `docs/active/work/{ticket_id}/`. Lisa should detect this and advance the ticket's phase in its YAML frontmatter.

## Existing Infrastructure

### Phase-to-Artifact Mapping (types.rs:60-69)

`Phase::artifact_filename()` already maps phases to artifact filenames:
- Research → `research.md`
- Design → `design.md`
- Structure → `structure.md`
- Plan → `plan.md`
- Implement → `progress.md`
- Ready/Review/Done → None

`Phase::next()` returns the successor phase (e.g., Research → Design).

### Ticket Phase Update (ticket.rs:398-406)

`ticket::update_ticket_phase(path, new_phase)` exists. It reads the ticket file, rewrites the `phase:` line in YAML frontmatter, and writes back. Preserves all other content.

### Activity Events (types.rs:426-474)

`ActivityEvent` already has the variants the ticket requires:
- `PhaseCompleted { ticket_id, phase }` — a thread completed a phase
- `TicketPhaseChanged { ticket_id, old_phase, new_phase }` — a ticket's phase changed
- `ArtifactCreated { ticket_id, phase, path }` — an artifact was created

### Scheduler Helpers (scheduler.rs:557-569)

- `ticket_work_dir(ticket_id)` — returns `config.work_dir / ticket_id`
- `phase_artifact_exists(ticket_id, phase)` — checks if a phase artifact file exists (uses `path.exists()`)

### Plugin State (lib.rs)

- `State.config.work_dir` — prefixed with `/host/` during `load()` for WASI access
- `State.threads` — HashMap<TicketId, Thread>, tracks running agents
- `State.last_phases` — HashMap<TicketId, Phase>, snapshot for change detection
- `State.dag` — the computed DAG with all ticket data

### Current Poll Loop (lib.rs:230-274)

`poll_tick()` runs every 5 seconds:
1. `rebuild_dag()` — re-scans ticket files, detects phase changes via `last_phases`
2. If changes: marks Done tickets as completed, frees slots, updates thread phases
3. `schedule_ready_tickets()` — fills idle slots

Currently, `rebuild_dag()` only detects changes that are **already written to ticket frontmatter** by the agent. There is no code that scans work directories for artifact files and proactively advances phases.

### Thread Parking (scheduler.rs / types.rs)

- `Thread::park()` sets status to `ThreadStatus::Parked`
- `State` doesn't directly use `Scheduler.park_thread()` — it manages threads in its own `HashMap<TicketId, Thread>` and doesn't use the `Scheduler` struct at all in the plugin.

## Key Observations

1. **The plugin and scheduler have duplicated thread management.** `lib.rs` has its own `threads: HashMap<TicketId, Thread>` and doesn't use the `Scheduler` struct's thread tracking. The new artifact-scanning logic should be added to `lib.rs` `State` methods, not to `Scheduler`.

2. **WASI filesystem**: Work dir paths need `/host/` prefix, already handled in `load()` for `config.work_dir`.

3. **The right place** to add artifact scanning is in `poll_tick()`, between `rebuild_dag()` and `schedule_ready_tickets()`. This way, when an artifact appears, the ticket file gets updated, and on the next poll the DAG reflects the new phase.

4. **Alternatively**, artifact scanning could happen inside or after `rebuild_dag()` — detect artifacts, update ticket files, then the next `rebuild_dag()` picks up the changes naturally. But this means a 1-tick delay. Doing it inline (scan → update file → rebuild DAG) within a single tick is better.

5. **File I/O concern**: `ticket::update_ticket_phase()` uses `std::fs::read_to_string` and `std::fs::write`. These work in WASI with the `/host/` prefix. The ticket file paths are stored in `Ticket::file_path` after parsing.

6. **Phase advance logic**: For a ticket in phase X with artifact for X present → advance to X.next(). The `Implement` phase's artifact is `progress.md`; when present, advance to `Review`. When advancing to `Review`, the thread should be parked.

7. **Edge case**: An agent might write multiple artifacts in one session (e.g., writes `research.md` then `design.md` before the next poll). The scanning logic should detect the **most advanced** artifact and advance accordingly, not just one step.

## Files That Need Changes

- `crates/lisa-plugin/src/lib.rs` — add `handle_artifact_advance()` method to `State`, call it from `poll_tick()`
- No changes needed to `lisa-core` — all required APIs already exist (`Phase::artifact_filename()`, `Phase::next()`, `ticket::update_ticket_phase()`, `ActivityEvent` variants)

## Boundaries and Constraints

- Only scan tickets that have active threads (are in `self.threads` with `Running` status)
- Don't re-advance tickets that are already past a phase (check current phase matches)
- Must handle the case where work dir doesn't exist yet (ticket just started, no artifacts)
- Thread parking only happens when advancing to `Review` phase specifically
- The ticket frontmatter update must use the WASI-accessible path (with `/host/` prefix) stored in the ticket's `file_path`
