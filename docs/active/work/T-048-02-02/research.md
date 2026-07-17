# Research — T-048-02-02 ask authoring and auto-recheck

## Ticket state and scope

- Ticket `T-048-02-02` is open and starts in Research.
- It belongs to story `S-048-02`, “the ask surface.”
- The ticket has two independent but related deliverables.
- The first is instruction copy for agents authoring blocking Review dispositions.
- The second is automatic verification of world-owned parked remedies.
- The ticket depends on `T-048-01-02` and `T-047-01-01`.
- Both dependencies are complete in the current history.
- `T-048-01-02` established parking policy and transition provenance.
- `T-047-01-01` last serialized changes to `templates.rs`.
- `T-048-02-01`, also complete, established status and manual unblock behavior.
- The working tree contains Lisa-managed ticket/journal changes and published work from other tickets.
- Those paths are unrelated and must not be included in this ticket's source commits.

## Agent-context and template flow

- `crates/lisa-cli/src/templates.rs` owns installed project context templates.
- `RDSPI_WORKFLOW` is a `LazyLock<String>` in that module.
- Its stable purpose paragraph comes from `lisa_core::context::PURPOSE_PARAGRAPH`.
- Its mechanics body comes from `crates/lisa-cli/data/rdspi-workflow.md`.
- The rendered result must equal `docs/knowledge/rdspi-workflow.md` byte for byte.
- A template test asserts that equality.
- `lisa init` installs or upgrades the rendered workflow in projects.
- The plugin's per-ticket assignment tells the agent to read the workflow document.
- The current workflow Review section defines `review.md` and `review-disposition.json`.
- It currently documents only the legacy minimal block shape with `disposition` and `reason`.
- It does not describe `remedy_owner`, `ask`, `steps`, or `check`.
- It does not state the one-sentence language rule from the ticket.
- It does not carry the required 2026-07-16 counter-example.
- `templates.rs::test_review_disposition_contract_is_injected` currently pins only:
  - the workflow reference in generated `CLAUDE.md`;
  - the pass JSON;
  - the minimal block JSON;
  - the pass/block reason validity rule.
- The generated `CLAUDE.md` and `AGENTS.md` point to the workflow rather than duplicating it.
- Keeping detailed Review authoring rules in the workflow preserves that existing boundary.

## Structured Review disposition contract

- `crates/lisa-core/src/disposition.rs` is the parser and validator.
- `RemedyOwner` has three serialized values: `agent`, `operator`, and `world`.
- `ReviewDisposition::Block` retains:
  - `reason`;
  - `remedy_owner`;
  - `ask`;
  - optional `steps`;
  - optional `check`.
- A structured block requires both `remedy_owner` and a nonblank `ask`.
- `steps`, when present, must be an array of nonblank strings.
- `check`, when present, must be a nonblank string.
- Unknown or malformed structured fields fall back to the legacy unstructured block behavior.
- A legacy block remains valid and is treated as operator-owned.
- Its reason is also used as the fallback ask.
- Parsing stores check text but never executes it.
- Parser tests explicitly prove check content remains inert.
- The parser does not attempt to judge sentence quality or owner honesty.
- Those semantic judgments therefore belong in agent instructions and review practice.

## Parking and scheduling authority

- `T-048-01-02` made ticket frontmatter status the durable parking authority.
- `status: blocked` excludes a ticket from ordinary DAG readiness.
- `status: open` restores ordinary DAG eligibility.
- No separate parked allow-list is used for scheduling.
- `crates/lisa-core/src/parking.rs` projects canonical parked remedies.
- `collect_parked_remedies` considers only blocked tickets.
- It reads `docs/active/work/<ticket>/review-disposition.json` through the typed parser.
- It returns ticket ID, remedy owner, ask, and optional check.
- Results are sorted by ticket ID for deterministic consumers.
- Invalid, missing, or passing dispositions produce no remedy projection.
- The status CLI and plugin dashboard both consume this projection.
- Operator-owned asks are shown as direct action requests.
- World-owned asks are annotated that Lisa checks on its own.
- Agent-owned parked blocks are not shown in the external waiting list.

## Manual unblock execution boundary

- `crates/lisa-cli/src/unblock.rs` implements `lisa unblock`.
- `run_unblock` loads and resolves project configuration.
- It scans the configured ticket and work directories.
- It declines unknown, nonblocked, or remedy-less tickets in pinned plain language.
- When a remedy has a check, `run_check` executes it.
- A passed check permits the ticket status to change to open.
- A failed, timed-out, or mutating check leaves the status blocked.
- A remedy without a check can be reopened manually.
- Check execution has a fixed five-second timeout.
- The shell runs with null stdin and bounded temporary stdout/stderr captures.
- On Unix it receives its own process group.
- Timeout handling kills the shell process group, including descendants.
- Check output is reduced to one sanitized observation line.
- ANSI controls, tabs, other control characters, and excessive length are removed.
- Checks run in a disposable snapshot, not the live project directory.
- Git-visible tracked and untracked paths are copied when Git is available.
- A bounded tree-copy fallback excludes `.git`, `target`, `node_modules`, and attempt state.
- The snapshot is recursively made read-only.
- `TMPDIR`, `TMP`, and `TEMP` point to a separate scratch directory.
- The snapshot is fingerprinted before and after command execution.
- A changed fingerprint cannot pass even if permissions were altered first.
- Existing tests cover pass, failure, timeout, write isolation, mutation detection, and output sanitation.

## Current CLI command surface

- `crates/lisa-cli/src/main.rs` defines the Clap command enum and dispatch.
- `Unblock` is a visible everyday operator command.
- Plumbing commands such as `complete-ticket` are hidden.
- The binary module `unblock` is private to `main.rs`.
- The reusable `lisa-cli` library currently exposes only `capture_usage` test support and commit transactions.
- `run_loop` is invoked after config is loaded and resolved in `main.rs`.
- Dry-run exits through `run_dry` before real launch side effects.
- Real loop launch discovers the native Lisa executable with `current_exe`.
- That absolute executable path is passed into the generated Zellij layout as `lisa_bin`.

## Scheduler timer and host-command boundary

- `crates/lisa-plugin/src/lib.rs` owns scheduler state and event handling.
- `POLL_INTERVAL_SECS` is five seconds.
- Permission grant starts the poll timer and attempts initial scheduling.
- Timer events flush deferred pane input and invoke `poll_tick` when the poll timer is due.
- `poll_tick` observes lifecycle signals, advances artifacts, applies block policy, rebuilds the DAG, and schedules ready tickets.
- The timer is rearmed at the end of every nonterminal poll.
- The plugin runs inside Zellij's WASI environment.
- Direct project paths inside the plugin are rooted at `/host`.
- Native commands cannot use `/host`; they run in the host environment.
- `get_plugin_ids().initial_cwd` supplies the host project root.
- `PluginConfig::lisa_bin` supplies the native Lisa executable.
- The plugin already has `RunCommands` permission.
- It already subscribes to `RunCommandResult`.
- Completion transactions and notification hooks use asynchronous host commands.
- Command result contexts distinguish completion and notification results.
- The existing effect boundary accepts argv, environment, cwd, and a context map.
- Native tests avoid host calls when `project_root` is empty.

## Existing park and unpark provenance

- `crates/lisa-core/src/provenance.rs` defines schema version 5 parking rows.
- Parking transition types are Retry, Park, and Unpark.
- A parking record carries the exact attempt lease and remedy owner.
- World parks set `recheck_eligible: true`.
- Operator and exhausted-agent parks set it false.
- Park rows include interval start and end timestamps.
- `State::reconcile_unpark_transitions` replays the latest parking row per ticket.
- A latest Park plus durable open status causes one Unpark row to be appended.
- The Unpark preserves owner, attempt, retry metadata, and recheck eligibility.
- Its wall-clock interval starts at the prior Park timestamp.
- After append, the latest row is Unpark, making reconciliation idempotent.
- Scheduling does not depend on successful provenance append.
- An open status begins a new scheduling episode even if provenance is temporarily unwritable.
- Reconciliation is currently called during plugin load and after each DAG rebuild.
- Existing tests cover world recheck eligibility and status-driven unpark provenance.
- No code currently executes an eligible world's check automatically.

## Existing fixture coverage

- `crates/lisa-cli/tests/parked_ux.rs` provides black-box CLI fixtures.
- Fixtures create a project path containing spaces.
- They use real ticket and canonical work files.
- They invoke the built Lisa binary.
- They assert status copy and process exit behavior.
- A passing manual check reopens a ticket and makes it DAG-ready.
- A failing manual check remains blocked and non-ready.
- A write attempt cannot touch the live project.
- These fixtures currently call `lisa unblock`; none exercise loop-driven automation.
- Plugin tests in `crates/lisa-plugin/src/lib.rs` build native scheduler state directly.
- The existing park-policy replay uses real temporary ticket/work/ledger paths.
- It verifies seat release, durable blocked status, and Park rows.
- The existing agent-bound replay changes blocked status back to open manually.
- It then invokes unpark reconciliation and verifies exactly one Unpark row and reseating.
- The scheduler's host command calls are normally inert in native tests.
- Completion tests observe an explicit test-only effect collection at their boundary.

## Constraints and invariants

- Auto-recheck may verify; it must not perform the remedy.
- Only world-owned parked tickets are eligible for automatic reopening.
- A world remedy without an observable check cannot be verified automatically.
- Operator-owned and agent-owned remedies must not be reopened by the cadence.
- A nonzero check, timeout, mutation attempt, malformed disposition, or execution error must fail closed.
- Failing checks must not rewrite frontmatter or append false Unpark evidence.
- Check execution cannot block the WASM scheduler thread.
- At most one cadence invocation should be outstanding per scheduler instance.
- Reopening must flow through `status: open` and ordinary DAG scheduling.
- Unpark provenance must remain an observation of that durable change.
- Source commits must use exact ticket-owned paths through `lisa commit-ticket`.
- Phase artifacts remain private under the current attempt directory until Lisa publishes them.
