# Research: land small demonstrated cleanups

## Ticket and workflow boundary

- Ticket `T-038-03-02` is in Research.
- Its only implementation inputs are the classifications produced by
  predecessor `T-038-03-01`.
- The predecessor inventory authorizes consideration of C-01 through C-04.
- C-05 through C-14 are explicitly deferred or intentional repetition.
- The clean-gate dependency `T-038-02-03` is complete.
- The ticket requires every landed cleanup to have a passing proof.
- Final verification requires `cargo test --workspace` and clean clippy.
- Phase artifacts belong in `.lisa/attempts/T-038-03-02/1/work/`.
- Lisa, not this attempt, publishes artifacts and updates ticket frontmatter.
- Source commits must use exact-path `lisa commit-ticket` transactions.

## Repository state

- The workspace contains the `lisa-core`, `lisa-plugin`, and `lisa-cli` crates.
- Scheduler behavior and most scheduler unit tests live in
  `crates/lisa-plugin/src/lib.rs`.
- Provider behavior and provider-specific unit tests live in
  `crates/lisa-plugin/src/adapter.rs`.
- The deterministic real-Zellij harness lives in
  `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.
- Its Rust test entry point is
  `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`.
- `CLAUDE.md` defines `cargo test --workspace` and `just check` as standard
  repository verification commands.
- No ordinary-index entries are staged at the start of this attempt.
- Lisa has modified `.lisa/provenance.jsonl` and the assigned ticket file.
- Those workflow-owned changes are outside this ticket's source ownership.
- The branch is already ahead of its remote because completed tickets share it.

## Predecessor inventory

- C-01 is repeated `pane-<u32>.<suffix>` filename parsing in scheduler signal
  consumers.
- C-02 is identical native-adapter `reset_strategy` behavior.
- C-03 is identical native-adapter Review follow-up construction.
- C-04 is repeated event-log counting in the deterministic shell harness.
- Each candidate is confined to one maintained file.
- The predecessor names a specific proof seam for each candidate.
- The predecessor forbids allowing C-01 to absorb scanner behavior.
- The predecessor treats provider-specific assignment and reuse behavior as
  intentional and outside C-02/C-03.
- The predecessor treats cross-harness extraction as future-epic scope.

## C-01 current filename parsing

- `State::check_heartbeat_signals` reads `pane-<id>.heartbeat` names.
- `State::check_process_start_signals` reads `pane-<id>.started` names.
- `State::check_shell_ready_signals` reads `pane-<id>.shell-ready` names.
- `State::check_codex_ack_signals` reads `pane-<id>.ack` names.
- `State::check_awaiting_signals` reads `pane-<id>.awaiting` names.
- `State::check_transition_signals` recognizes both `.stopped` and `.cleared`.
- `State::check_error_signals` reads `pane-<id>.error` names.
- Each grammar requires the literal `pane-` prefix.
- Each grammar requires one exact suffix at the end of the filename.
- The middle component must parse as a Rust `u32`.
- Filename conversion currently uses `OsStr::to_str`, rejecting non-UTF-8.
- The repeated chains return no pane id for a wrong prefix.
- They return no pane id for a wrong suffix.
- They return no pane id for an empty or non-numeric middle component.
- They return no pane id for overflow beyond `u32::MAX`.
- Exact suffix stripping prevents trailing extensions from being admitted.
- Leading zeroes are accepted because `u32::from_str` accepts them.
- The helper seam can remain private to `lib.rs`.
- It needs no scheduler state and performs no I/O.

## C-01 behavioral boundaries

- Directory iteration remains separately implemented by each consumer.
- Missing signal directories remain inert.
- Payload reads remain separately implemented.
- Attempt-lease admission remains separately implemented.
- Signal deletion timing remains consumer-specific.
- Activity-clock updates remain consumer-specific.
- Poll ordering remains unchanged.
- Transition scanning must continue to recognize two suffixes in one pass.
- Transition files with a recognized suffix are currently removed before their
  pane-id parse result is acted upon.
- Other consumers currently remove only filenames that produce a valid pane id.
- Idle scanning retains legacy `<ticket-id>.idle` support.
- The predecessor did not include idle parsing in C-01.
- Whole-loop or callback extraction would cross into deferred C-05.
- A table-driven unit test can exercise the pure grammar independently.
- A Unix-only case can construct a non-UTF-8 `OsString` for rejection proof.
- Existing signal-consumer tests provide regression coverage for state effects.

## C-02 current adapter reset policy

- `AgentAdapter` declares `reset_strategy` without a default.
- `ClaudeCodeAdapter` returns `ResetStrategy::ClearHandshake`.
- `CodexAdapter` returns the identical value.
- `ResetStrategy::FreshExec` remains available for future adapters.
- Scheduler dispatch calls the method through resolved trait objects.
- Existing tests assert the Claude native result.
- Existing tests assert the Codex native result.
- Resolver tests assert results for missing, routed, invalid, and mixed tickets.
- A trait default does not remove the override seam.
- Removing both identical implementations makes the two native adapters inherit
  the shared policy through dynamic dispatch.
- No public API changes because the trait is crate-private.

## C-03 current adapter follow-up policy

- `AgentAdapter` declares `follow_up` without a default.
- Both native implementations call `finish_up_prompt` with the same three
  `FollowUpContext` fields.
- Both wrap the returned string in `FollowUp::TypeIntoPane`.
- `FollowUp::SpawnCommand` remains available for future adapters.
- `FollowUpContext::pane_id` is retained for a future non-interactive adapter.
- Existing Claude and Codex tests compare the complete `FollowUp` value.
- The shared construction depends only on imported `finish_up_prompt`.
- A trait default leaves future adapters free to override delivery mechanics.
- Provider launch, assignment, reuse, readiness, and signals remain explicit.

## C-04 current event counting

- `event_count_is` accepts an event kind and exact expected count.
- `event_count_at_least` accepts an event kind and lower bound.
- Both initialize `actual` to zero.
- Both test for `$CURRENT_ROOT/evidence/events.log` existence.
- Both invoke the same tab-delimited `awk` program.
- Only the final comparison operator differs.
- A script-local `event_count` function can print the existing computed value.
- Command substitution can supply that value to both predicates.
- A missing log must still produce zero.
- The helper must preserve whitespace-free integer output.
- The harness is self-contained and should not source another script.
- The ignored Rust integration test invokes the harness with the built Lisa CLI.
- The integration test requires real Zellij, zsh, jq, and the WASM target.
- The current environment exposes `zellij` and `jq` executables.
- The harness asserts exact and lower-bound counts across success and recovery
  scenarios and requires a printed PASS receipt.

## Deferred repetition that must remain

- C-05: whole signal-consumer loops.
- C-06: scheduler failure and reclaim paths.
- C-07: timeout and liveness loops.
- C-08: atomic publication paths.
- C-09: repeated helpers across maintained Zellij harnesses.
- C-10: admitted historical harness evidence.
- C-11: Claude/Codex lifecycle-hook schemas and merge lists.
- C-12: broad scheduler test-fixture construction.
- C-13: provider-specific assignment and reuse construction.
- C-14: independent adapter compatibility assertions.
- These sites are expected to remain visible after implementation.
- The final Review artifact must name them for the release-readiness report.

## Verification constraints

- C-01 needs focused parser tests plus existing plugin regressions.
- C-02 and C-03 have focused adapter tests that exercise inherited behavior.
- C-04 needs the explicitly run ignored integration test and its PASS receipt.
- Formatting should be checked before commits.
- Each meaningful source unit should be committed independently with exact paths.
- Workspace tests run after all units are integrated.
- Clippy should run across all targets and features with warnings denied.
- Final status must show no ticket-owned staged, modified, or untracked files.
