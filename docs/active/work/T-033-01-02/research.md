# Research: Codex acknowledgment detector

## Ticket boundary

- Ticket: `T-033-01-02`.
- Current phase at entry: `research`.
- The ticket asks for detection only.
- It does not ask the scheduler to promote a seat to `Owned`.
- Promotion is owned by dependent ticket `T-033-01-03`.
- Ack timeout and recovery are owned by `T-033-01-04`.
- Dashboard rendering is outside this story and belongs to `S-033-02`.
- Live consecutive reuse proof belongs to `S-033-03`.
- The acceptance criterion requires captured lifecycle fixtures.
- It also requires ticket and assignment-generation attribution.
- Still-idle and stale-previous-ticket events must not acknowledge.
- Claude handshake and terminal rendering are explicitly excluded.

## Existing assignment model

- Scheduler implementation is in `crates/lisa-plugin/src/lib.rs`.
- `SeatAssignmentState` was introduced by `T-033-01-01`.
- Its variants are `AssignedPendingAck`, `Owned`, and `Recovering`.
- Assignment state is keyed by physical pane ID.
- Absence from the map means that the pane has no assignment.
- `AgentSlot.ticket_id` remains the ticket reservation.
- `TransitionState` remains reset and provider-switch transport state.
- `has_session` records whether a provider TUI is resident.
- `last_client` records which provider occupies or is entering the pane.
- These facts are deliberately separate from acknowledged ownership.
- Reused or recycled Codex seats enter `AssignedPendingAck`.
- Fresh Codex assignments retain the existing `Owned` behavior.
- Claude assignments retain the existing `Owned` behavior.
- Existing clear and exit timeouts preserve pending assignment state.
- No code currently classifies a Codex acknowledgment.
- No assignment generation is currently stored by the scheduler.

## Existing handoff transport

- Provider adapters live in `crates/lisa-plugin/src/adapter.rs`.
- Both native clients currently use `ResetStrategy::ClearHandshake`.
- Same-provider reuse sends `/clear` into the resident TUI.
- The scheduler waits for a `.cleared` signal.
- It then sends the next ticket prompt into the pane.
- Text and Enter are deliberately separated by a delay.
- Cross-provider recycling sends `/exit` first.
- After an exit grace period it launches the incoming provider.
- These transitions prove input delivery attempts, not agent acceptance.
- `.cleared` means Codex created a cleared conversation.
- It does not mean the next ticket prompt was submitted.
- A clear timeout also sends the prompt but is not an acknowledgment.
- Terminal text is not read by the scheduler.
- Zellij pane rendering is therefore not an existing semantic input.

## Existing prompts

- `ticket_prompt` in `crates/lisa-plugin/src/lib.rs` builds shared RDSPI text.
- It accepts a ticket directory, ticket ID, and provider context filename.
- Claude receives `CLAUDE.md` in the prompt.
- Codex receives `AGENTS.md` in the prompt.
- Aside from that context filename, the prompt body is shared.
- A reused Codex session receives the bare prompt through `reuse_prompt`.
- A fresh Codex launch wraps the prompt in the CLI shell command.
- The current prompt contains the ticket ID in its path and work directory.
- It does not contain a unique assignment-generation identifier.
- Reusing the same ticket would therefore reproduce equivalent prompt text.
- Ticket identity alone cannot reject a stale event from an older attempt.

## Existing Codex lifecycle hooks

- Generated hook configuration lives in `crates/lisa-cli/src/templates.rs`.
- Generated project configuration lives at `.codex/hooks.json`.
- Current configured events are `PostToolUse`, `Stop`, and `SessionStart`.
- `SessionStart` is matched on the `clear` source.
- The handlers invoke shared `.lisa/hooks` shell scripts.
- `PostToolUse` writes a heartbeat signal.
- `Stop` writes a stopped signal and captures usage.
- `SessionStart(clear)` writes a cleared signal.
- Signal filenames are pane-scoped.
- The shared scripts presently depend on `LISA_PANE_ID`.
- The scripts do not preserve the lifecycle JSON payload.
- The scheduler consumes normalized files under `.lisa/signals`.
- No current signal contains session ID, turn ID, ticket ID, or generation.
- Existing hook scripts have unrelated working-tree modifications.
- Those modifications are not owned by this ticket.

## Current official Codex hook contract

- The project knowledge base marks Codex hook schemas as version-sensitive.
- Current official documentation was checked during Research.
- Codex supports a `UserPromptSubmit` lifecycle hook.
- It runs at turn scope.
- It receives one JSON object on standard input.
- Common fields include `session_id`, `cwd`, and `hook_event_name`.
- Turn-scoped hooks add `turn_id`.
- `UserPromptSubmit` adds the submitted `prompt` string.
- The event occurs when a user prompt is about to be sent.
- This is provider lifecycle evidence, not terminal rendering.
- The event matcher is ignored for `UserPromptSubmit`.
- Project-local hooks require the project config layer to be trusted.
- Lisa launches Codex with `--dangerously-bypass-hook-trust`.
- Hook payload parsing should use documented fields only.
- Transcript contents are explicitly not a stable hook interface.

## Candidate evidence already available

- `SessionStart(clear)` is conversation reset evidence.
- It is too early to prove that the next prompt was accepted.
- `PostToolUse` is later positive activity.
- It can be absent for tickets that require no supported tool.
- It cannot directly identify which Lisa prompt created its turn.
- `Stop` is turn completion evidence.
- Waiting for it would acknowledge only after work may already be complete.
- It also needs correlation to the submitted assignment turn.
- `UserPromptSubmit` directly carries the submitted prompt.
- It also carries provider session and turn identifiers.
- Its payload can be correlated without transcript or terminal inspection.

## Attribution requirements

- Pane identity alone is insufficient because panes are reused.
- Session identity alone is insufficient because `/clear` changes conversations.
- Ticket identity alone is insufficient because a ticket can be retried.
- Turn identity is generated by Codex only after prompt submission.
- Lisa needs an identifier known before it sends the prompt.
- The ticket calls this an assignment generation.
- A unique generation marker in the issued prompt supplies that identifier.
- A pending assignment can retain its expected ticket and generation.
- The lifecycle detector can compare the received prompt marker to both.
- Session correlation can reject activity from a different resident session.
- A malformed or unrelated event must fail closed.
- Unknown extra payload fields should not break classification.

## Fixture precedent

- CLI integration fixtures exist under `crates/lisa-cli/tests/fixtures`.
- They capture newline-delimited Codex exec events.
- The plugin currently has no dedicated fixture directory.
- Unit tests in `crates/lisa-plugin/src/lib.rs` are mostly inline.
- `include_str!` can load checked-in JSON without runtime filesystem access.
- That is compatible with native unit tests and the WASM crate boundary.
- Separate JSON fixtures make positive and negative evidence reviewable.
- Fixtures should preserve realistic Codex field names and extra fields.
- Tests should not construct only idealized Rust values.

## Module and dependency boundaries

- `lisa-plugin` already depends on `serde` and `serde_json`.
- No new third-party dependency is required for payload parsing.
- A small detector module can remain independent of scheduler state mutation.
- `adapter.rs` should continue to own provider transport behavior.
- `lib.rs` should continue to own scheduler state transitions.
- The detector can own marker formatting and event classification.
- Keeping it provider-specific prevents Claude semantics leaking into it.
- A boolean result matches the ticket's immediate acceptance language.
- A parse error should remain distinguishable from a valid non-ack event.

## Test constraints

- Tests must cover a matching pending assignment.
- Tests must cover `SessionStart(clear)` as still-idle/reset evidence.
- Tests must cover a previous ticket in the same pane/session.
- Tests must cover an older generation of the same ticket.
- Tests should cover a different session as additional stale evidence.
- Tests should cover malformed lifecycle input failing closed.
- Tests should verify prompt text alone is not terminal scraping.
- Tests should verify no Claude event names or handshake files are referenced.
- Full plugin tests must remain green.
- Workspace tests should catch template and cross-crate regressions.

## Working-tree constraints

- The repository is already dirty with unrelated user and ticket work.
- `.lisa/hooks` files are modified outside this ticket.
- `crates/lisa-cli/src/agent_exec.rs` is modified outside this ticket.
- Numerous docs and active ticket files are unrelated and untracked.
- Ticket-owned changes must avoid those paths.
- Source changes must be committed with `lisa commit-ticket` only.
- Exact repository-relative include paths are required.
- Work artifacts are left for Lisa's completion transaction.
- Ticket frontmatter phase and status must not be edited manually.

## Research conclusion

The current code has an explicit pending-assignment state but no provider-native
acknowledgment classifier or assignment generation. Codex's documented
`UserPromptSubmit` payload is the narrow lifecycle boundary that exposes the submitted
prompt together with session and turn identity. A detector can correlate a unique Lisa
assignment marker in that prompt to the pending ticket and generation, reject clear-only
and stale events, and remain independent of Claude signals, terminal contents, scheduler
promotion, and recovery policy.
