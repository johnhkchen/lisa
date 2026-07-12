# Research: repetition inventory and classification

## Ticket boundary

- Ticket: `T-038-03-01`, a spike in story `S-038-03`.
- The acceptance criterion asks for an inventory and classification only.
- This ticket must not land source changes.
- The successor `T-038-03-02` owns any selected cleanup.
- The story limits that successor to small, individually test-backed changes.
- Larger refactors and harmless repetition stay in place.
- Stable provider, scheduler, lease, and CLI contracts are explicitly preserved.

## Repository shape relevant to the survey

- `crates/lisa-plugin/src/lib.rs` contains the active scheduler and plugin state.
- It is 14,909 lines, including a large native unit-test module.
- `State` owns ticket discovery, seats, leases, signal consumption, timeouts,
  completion, provenance, notifications, and UI conversion.
- There is no current `scheduler.rs`; historical documents referring to one
  predate the consolidation into `lib.rs`.
- `crates/lisa-plugin/src/adapter.rs` contains the provider adapter seam.
- It is 810 lines and implements native Claude and native Codex behavior.
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` is the
  deterministic real-Zellij delivery harness.
- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the metered live
  provider startup harness.
- Older ticket-owned harnesses remain under `docs/active/work/` as evidence.
- `crates/lisa-cli/src/templates.rs` writes and merges lifecycle-hook JSON.
- `crates/lisa-plugin/src/lib.rs` also contains recurring atomic-publication
  and test-fixture writing patterns.

## Scheduler signal-consumer family

- `check_heartbeat_signals` starts at `lib.rs:3096`.
- `check_process_start_signals` starts at `lib.rs:3146`.
- `check_shell_ready_signals` starts at `lib.rs:3178`.
- `check_codex_ack_signals` starts at `lib.rs:3210`.
- `check_awaiting_signals` starts at `lib.rs:3257`.
- `check_idle_signals` starts at `lib.rs:3299`.
- `check_transition_signals` starts at `lib.rs:3540`.
- `check_error_signals` starts at `lib.rs:3592`.
- Each opens `self.signal_dir` and treats a missing directory as inert.
- Most derive a pane id from `pane-<u32>.<suffix>`.
- Most consume a matching file with best-effort `remove_file`.
- Heartbeat, started, shell-ready, and ack signals carry payloads.
- Awaiting, stopped, cleared, error, and most idle handling use presence or a
  legacy body rather than the same payload contract.
- Idle retains compatibility with legacy `<ticket-id>.idle` names.
- Transition scanning deliberately handles two suffixes in one pass.
- Ordering in `poll_tick` is behavioral: awaiting must gate writers; errors
  must beat transition timeout handling; lifecycle evidence has lease rules.
- The repeated filename parse is separable from those behavioral differences.
- The repeated directory traversal and consumption policy is not uniform.

## Scheduler failure and recovery family

- `fail_assignment_delivery` starts at `lib.rs:1663`.
- `fail_assignment_recovery` starts at `lib.rs:1714`.
- `fail_startup_recovery` starts at `lib.rs:1961`.
- `fail_startup` starts at `lib.rs:2028`.
- `check_error_signals`, session timeout handling, and stale-thread detection
  also fail, fence, release, remove, alert, and log in related sequences.
- These sequences resemble one another textually but do not share authority.
- Some act on a pending seat; some act on a running thread.
- Some revoke an attempt lease and close a pane; others preserve or recycle it.
- Provenance emission occurs at specific teardown points while thread facts are
  still available.
- Reordering these effects can change single-writer and retry semantics.
- Existing regression tests exercise dropped acknowledgements, replacement
  starts, stale attempts, split-brain timelines, and completion gating.

## Scheduler timeout and liveness family

- Assignment acknowledgement deadlines are checked at `lib.rs:2175`.
- Transition timeouts are checked at `lib.rs:3923`.
- Review timeouts are checked at `lib.rs:4180`.
- Health evaluation starts at `lib.rs:4269`.
- Session timeouts start at `lib.rs:4331`.
- Hard stale-thread detection starts at `lib.rs:4438`.
- Several paths consult `is_pane_awaiting` and recent pane activity.
- Several locate a running thread, calculate elapsed time, log an event, and
  then reclaim or nudge work.
- Their clocks, exemptions, side effects, and fallback policies differ.
- Tests specifically pin active-session deferral and slow-test clock gaps.

## Atomic publication writing family

- `prepare_fresh_launch` writes a temporary launch script then renames it.
- `prepare_assignment` writes complete instructions then renames them.
- `write_pane_lease_marker` serializes an exact lease then renames it.
- `admit_artifact` copies admitted bytes to a temporary canonical file and
  renames it.
- `shell_readiness_probe` generates a shell command that performs the same
  temporary-file-to-destination publication inside the pane.
- All use same-directory rename as the completeness boundary.
- Temporary naming differs because collision and attribution needs differ.
- Error messages identify different operator-visible artifacts.
- Cleanup on failed rename is best effort in the host-side variants.
- Tests cover hostile payloads, occupied temporary paths, lease identity, and
  stale artifact rejection.

## Adapter repetition family

- `AgentAdapter` defines launch, assignment, reset, reuse, follow-up, signal,
  and readiness behavior.
- `assignment_reference` and `exit_command` already have trait defaults.
- Both native adapters return `ResetStrategy::ClearHandshake` verbatim.
- Both native adapters build the same `finish_up_prompt` and wrap it in
  `FollowUp::TypeIntoPane` verbatim.
- Both call `ticket_prompt`, but deliberately select different context files.
- Both reuse the assignment, but Codex conditionally adds an acknowledgement
  tag while Claude does not.
- Both expose signal capabilities, but the values differ.
- Both expose readiness mode, and the values intentionally differ.
- Launch commands are provider-specific and carry different safety flags.
- Adapter tests already assert reset, follow-up, assignment, launch, signal,
  and readiness behavior for each native provider.
- Several adapter tests compare adapter output with free-function output; this
  is a no-op compatibility proof rather than accidental test repetition.

## Maintained harness repetition family

- Both maintained shell harnesses set strict Bash mode.
- Both resolve a real Zellij binary, create isolated fixture roots, track a
  current session, launch `lisa loop`, discover panes, dump panes, wait with a
  bounded polling loop, create a Zellij wrapper, and clean child processes.
- Function names repeated across both include `fail`, `cleanup`,
  `session_action`, `dump_pane`, `wait_until`, `write_zellij_wrapper`, and
  `create_fixture`.
- The implementations close over different global names and evidence layouts.
- The deterministic harness uses stub providers and exact event assertions.
- The live harness invokes installed providers, records versions and hashes,
  samples observations, and supports case selection and prepare-only modes.
- The live harness calls the deterministic harness as a preflight.
- Shared sourcing would therefore create a runtime packaging and environment
  contract between two currently self-contained executable fixtures.
- Within the deterministic harness, `event_count_is` and
  `event_count_at_least` repeat the same file-existence and `awk` count logic;
  they differ only in the final comparison operator.
- The ignored Rust integration test is the standing end-to-end seam for the
  deterministic script.

## Historical harness evidence

- `docs/active/work/T-021-01/harness/` contains a multi-script Codex study and
  already centralizes common helpers in `00-common.sh`.
- `docs/active/work/T-031-03/harness/run.sh` and
  `docs/active/work/T-033-03-02/harness/run.sh` are ticket evidence.
- These files record completed investigations and are not product test entry
  points in the current workspace.
- Editing archived or admitted work would alter historical evidence rather
  than simplify an active maintenance surface.

## Recurring hook-writing family

- `settings_local_json` embeds Claude hook groups as literal JSON.
- `codex_hooks_json` embeds Codex hook groups as literal JSON.
- `merge_hooks` enumerates the Lisa hooks that must be merged into Claude
  settings.
- `merge_codex_hooks` enumerates the corresponding Codex hooks.
- Stop, startup, clear, heartbeat, and acknowledgement command strings recur.
- Provider schemas differ in matcher requirements and supported events.
- Generated JSON and merge behavior have extensive idempotence and ownership
  tests because user-owned hook content must be preserved.
- A single declarative hook schema would affect generation, merge semantics,
  upgrade matching, and tests together.

## Recurring test-fixture writing family

- The `lib.rs` unit tests repeatedly write ticket YAML frontmatter.
- They repeatedly create `pane-<id>.<suffix>` signal files.
- They repeatedly construct `State`, tickets, threads, seats, and leases.
- Focused helpers already exist for the newest scheduling scenarios, including
  `running_thread`, `fresh_slot`, `pane_name_schedule_state`,
  `consecutive_reuse_state`, `acknowledge_assignment`, and
  `codex_state_with_dag`.
- Older tests predate those helpers and use locally tailored fixture state.
- The module spans scheduler history from phase advancement through current
  attempt fencing, so superficially similar setup often encodes different
  authority assumptions.

## Constraints carried into classification

- A small candidate must remove repeated policy or parsing, not just lines.
- Its behavior must already have a focused test seam or admit a focused test.
- It must not erase an intentional provider distinction.
- It must not change signal ordering, lease admission, completion gating,
  historical evidence, or public CLI behavior.
- A cross-file abstraction with new runtime coupling is not small for this
  release-tightening story.
- Broad decomposition of `lib.rs` is architectural work, not a cleanup.
- No source file is changed by this research ticket.
