# Research: T-031-02 gate done on commit

## Ticket and workflow state

- T-031-02 is stored at
  `docs/active/tickets/T-031-02-gate-done-on-commit.md`.
- Its frontmatter is `phase: research`, `status: open`, and `agent: codex`.
- The prompt's shorter `T-031-02.md` path does not exist; ticket discovery already
  supports descriptive suffixes and preserves the real path on `Ticket.file_path`.
- T-031-01 is complete in repository history and supplies the native isolated
  transaction required by this ticket.
- The repository has unrelated modified and untracked files. They must remain
  outside ticket-scoped commits.
- The workflow requires all six phase artifacts and incremental commits during
  implementation. Lisa, rather than this session, owns ticket phase transitions.

## T-031-01 transaction boundary

- `crates/lisa-cli/src/commit_transaction.rs` defines `commit_ticket`.
- The transaction accepts a repository root, ticket ID, message, and explicit
  repository-relative include pathspecs.
- It acquires `.lisa-commit.lock` across snapshot, alternate-index staging,
  commit creation, guarded `HEAD` update, ordinary-index reconciliation,
  verification, cleanup, and unlock.
- Ticket content is staged through `GIT_INDEX_FILE`, never through the ordinary
  repository index.
- Existing ordinary staged entries are snapshotted, excluded from the ticket
  tree, and verified byte-for-byte after reconciliation.
- An include that overlaps an ordinarily staged path is rejected.
- The transaction prints the new commit ID only after its success path completes.
- Errors are actionable and the CLI exits nonzero.
- `crates/lisa-cli/src/main.rs` exposes the boundary as `lisa commit-ticket`.
- Required CLI arguments are `--ticket-id`, `--message`, and repeated
  `--include`; `--path` selects the repository root.
- The transaction does not edit phase or status. It commits the content already
  present at its explicit include paths.
- The transaction deliberately cannot infer which arbitrary working-tree source
  files belong to a ticket. The caller is responsible for ownership inputs.
- RDSPI already requires meaningful implementation units to be committed as they
  are completed. The completion boundary therefore owns remaining loop-managed
  ticket content: the real ticket file and its ticket work directory. Any
  outstanding source path must have been committed in an implementation unit
  before Review completion; sweeping the shared worktree would violate T-031-01.

## Plugin host-command capability

- `crates/lisa-plugin/src/lib.rs` runs inside Zellij's WASI sandbox.
- `State::load` captures `get_plugin_ids().initial_cwd` as the host project root.
- Relative ticket/work paths are prefixed with `/host` for WASI filesystem access.
- `strip_host_prefix` converts a WASI path back to a host-relative path.
- `PluginConfig.lisa_bin` carries the absolute native Lisa executable path.
- `lisa loop` emits that path into the generated plugin configuration.
- `State::load` requests `PermissionType::RunCommands` and subscribes to
  `EventType::RunCommandResult`.
- The existing `fire_notify` method demonstrates host command invocation with
  environment, cwd, and a context map used to attribute results.
- The current `RunCommandResult` handler recognizes only the `lisa_notify` key.
- A completion invocation can use a distinct context key containing the ticket
  ID so results are correlated with the correct pending transition.
- Zellij returns exit code, stdout, stderr, and the original context map.
- Native plugin tests construct `State` without a project root, so host calls
  need a testable command-building seam and deterministic result handler.

## Current artifact-driven phase advancement

- `check_artifact_advances` loops over running threads until no artifact-driven
  phase can advance.
- Research, Design, Structure, Plan, and Review use `Phase::artifact_filename`.
- Implement uses `review.md`, because `progress.md` is a living document.
- The function writes each next phase directly with `update_ticket_phase`.
- When Review's artifact is present, `Phase::next()` is `Done`; therefore this
  function is one current independent Done writer.
- It immediately logs phase completion/change and mutates the thread phase.
- Later in the same poll, rebuilding the DAG sees the on-disk Done state.

## Current idle-driven phase advancement

- `check_idle_signals` consumes pane-scoped and legacy idle signal files.
- Implement idle advances the ticket to Review.
- If `review.md` already exists after that transition, the same branch writes
  Done directly and changes `thread.current_phase` to Done.
- Review idle with `review.md` present also follows the generic next-phase path
  and writes Done directly.
- These are additional artifact/idle completion paths.
- Intermediate phase transitions do not require commits under this ticket; only
  the non-Done to Done boundary is durability-gated.

## Current stop and finish-up behavior

- `handle_stopped_signal` distinguishes transition handshakes from a stopped
  Review session.
- A stopped Review session calls `auto_complete_review`.
- `auto_complete_review` writes phase Done and status Done independently.
- It then completes the thread, emits Done provenance, releases the slot, and
  removes the thread before any commit exists.
- `check_review_timeouts` does not itself finish a ticket. It sends a finish-up
  prompt to the parked Review session.
- The eventual `review.md`, idle signal, or stopped signal routes through one of
  the completion paths above.
- Thus timeout/finish-up coverage is primarily about ensuring the resumed path
  cannot bypass the same completion transaction.

## Current manual completion behavior

- The `d` key opens the mark-done modal.
- Review tickets may be manually completed even with a running thread.
- Implement tickets with `review.md` are also eligible.
- `mark_ticket_done` checks dependencies, then independently writes phase Done
  and status Done.
- A status write failure is logged but completion continues after phase changed.
- It immediately completes/removes the thread, emits provenance, releases the
  slot, rebuilds the DAG, and schedules dependents.
- This is the most direct violation of the required commit-before-publish order.

## Current generic Done publication

- `poll_tick` rebuilds the DAG after phase/signal/timeout checks.
- It then finds running threads whose newly scanned ticket phase is Done.
- Each is completed, emits provenance, releases its slot, and is removed.
- `sweep_stale_slots` releases slots assigned to any Done ticket.
- `audit_threads` removes threads whose ticket is Done or missing.
- `schedule_ready_tickets` relies on the rebuilt DAG; `Dag` treats a dependency
  as satisfied exactly when its phase is Done.
- `check_all_done` requires every DAG ticket Done and no running threads.
- It emits the all-done event and completion notification, sets `terminated`,
  and stops rearming the poll timer.
- Therefore preventing early frontmatter publication is not sufficient by
  itself: every generic teardown/sweep must distinguish verified completion
  from unverified external or pending Done state.

## Provenance behavior

- `emit_provenance` appends one JSONL record while the thread still exists.
- Existing completion paths can call it directly, and the generic poll sweep can
  call it again if state cleanup is incomplete.
- Failed completion attempts must not emit `RunOutcome::Done`.
- The eventual successful result should call the existing emitter exactly once,
  immediately before removing the thread.
- A pending-attempt set provides a natural deduplication boundary for duplicate
  review/idle/stop/manual triggers.

## State-machine requirements exposed by the code

- Completion is asynchronous because the commit executes as a host command.
- A ticket needs at most one in-flight completion attempt.
- The request side must validate dependencies, locate the actual ticket path,
  prepare both Done fields, and launch the isolated transaction.
- The result side must correlate the response, verify exit success, and only
  then publish all in-memory consequences.
- Repeated triggers while pending must be idempotent.
- Success must remove the pending attempt before rebuilding/scheduling so the
  new durable Done state becomes authoritative.
- Failure must restore non-Done ticket content, leave the thread and slot
  recoverable, keep dependents blocked, and retain a visible actionable error.
- A failed attempt must be retryable; permanent suppression would strand Review.
- A reused provider seat is protected by retaining the slot assignment until
  the success result publishes completion.

## Ticket content preparation constraint

- The native transaction stages existing file content; it does not synthesize
  frontmatter changes.
- Writing Done in WASM before launching the command creates a window where a
  poll or external observer can see uncommitted Done.
- It also requires exact rollback after command failure.
- Moving preparation into the native command narrows the mutation/commit window
  to the serialized transaction and allows restoration before a failure exits.
- The CLI already depends on `lisa-core`, whose ticket module owns frontmatter
  update behavior.
- A completion-specific native wrapper can save original bytes, update both
  fields, invoke the existing isolated transaction, and restore original bytes
  on pre-commit failure.
- The ticket file and `docs/active/work/<id>` remain explicit includes.
- The plugin can pass the real suffixed ticket path, avoiding the prompt's
  assumed `<id>.md` filename.

## Testing surface

- `lib.rs` has extensive native state tests with temporary ticket/work trees,
  synthetic threads, agent slots, signals, and direct method calls.
- Host command APIs are not available in ordinary native tests, so request
  construction and result publication should be separable from the actual
  Zellij call.
- Existing tests around artifact advancement, idle completion, stopped Review,
  manual mark-done, dependent scheduling, reused Codex slots, and provenance
  provide fixtures to extend or replace.
- CLI process tests can validate frontmatter preparation, isolated commit
  contents, ordinary-index preservation, and rollback on failure.
- Focused plugin tests should assert no Done publication before a simulated
  successful command result and no teardown after a simulated failure.
- Workspace tests, WASM release build, and plugin Clippy are explicit acceptance
  requirements.

## Boundaries and constraints

- T-031-03 owns provider-contract and live mixed-provider validation.
- This ticket should remain provider-neutral; completion belongs to scheduler
  state, not adapter-specific code.
- Non-Done phase auto-advancement remains local frontmatter mutation.
- Reset/failure/timed-out outcomes are not Done transitions and remain outside
  the completion transaction.
- External tools can still edit ticket files; the scheduler must not treat an
  unverified external Done edit as equivalent to its own successful transaction
  for an active thread.
- The ticket frontmatter itself must not be manually advanced by this work
  session; Lisa will detect the written artifacts.
