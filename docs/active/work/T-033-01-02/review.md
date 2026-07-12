# Review: Codex acknowledgment detector

## Outcome

Implemented the fixture-proven Codex acknowledgment classifier required by
`T-033-01-02`. The detector recognizes provider-native `UserPromptSubmit` lifecycle
evidence only when the submitted prompt carries the exact Lisa pending ticket and
assignment generation. Clear-only, previous-ticket, and previous-generation payloads do
not acknowledge.

The meaningful source unit is durable in Lisa's isolated ticket commit:

- commit: `9cafcea7c7b8a2f571fa5cde435a8742014df281`;
- message: `feat: detect ticket-scoped Codex acknowledgments`.

The commit contains exactly six ticket-owned source and fixture paths. The ticket and
work artifacts remain outside that source commit for Lisa's completion transaction.

## Source changes

### Created `crates/lisa-plugin/src/codex_ack.rs`

The new private module defines the provider contract between a pending Lisa assignment
and a Codex lifecycle payload.

It contains:

- `CodexAssignmentRef` with ticket ID and generation;
- canonical assignment marker serialization;
- `tag_codex_assignment` for prompt metadata;
- minimal lifecycle payload deserialization;
- `detect_codex_ack` for exact fail-closed classification;
- nine focused unit tests.

The module is deliberately independent of:

- scheduler `State`;
- `SeatAssignmentState` mutation;
- `TransitionState` and `/clear` sequencing;
- Zellij pane contents;
- Codex transcript contents;
- signal-file polling;
- Claude hooks and handshake behavior;
- acknowledgment timeouts and recovery.

### Modified `crates/lisa-plugin/src/lib.rs`

Added only the private module declaration:

```text
mod codex_ack;
```

No scheduler logic, adapter behavior, prompt delivery, pane naming, or UI behavior was
changed by this ticket.

## Fixture changes

Created four checked-in lifecycle JSON fixtures under
`crates/lisa-plugin/tests/fixtures/codex_ack/`:

- `matching-prompt-submit.json`;
- `still-idle-clear.json`;
- `stale-previous-ticket.json`;
- `stale-previous-generation.json`.

The fixtures preserve the documented Codex lifecycle field shape, including
representative session, transcript-path, working-directory, model, permission, turn,
event, source, and prompt values where relevant. Production classification intentionally
deserializes only `hook_event_name` and `prompt`; extra provider fields remain tolerated.

## Detector contract

### Pending identity

A pending assignment is represented by:

```text
ticket_id + assignment generation
```

Both are required. Ticket identity excludes activity from the pane's previous ticket.
Generation excludes delayed activity from a prior delivery attempt for the same ticket.

The generation is a `u64`. This module compares it but does not allocate or persist it;
that belongs to the dependent scheduler integration.

### Prompt marker

The helper appends one structured prompt line:

```text
LISA_ASSIGNMENT {"ticket_id":"T-...","generation":42}
```

The JSON object is produced by `serde_json`, so ticket content is escaped correctly. The
detector requires the prefix at the beginning of a prompt line and parses the remainder
as the marker schema. It does not accept an arbitrary substring match.

### Lifecycle evidence

The detector requires `hook_event_name == "UserPromptSubmit"`.

Current official Codex documentation identifies this as a turn-scoped lifecycle event
and documents `session_id`, `turn_id`, and the prompt about to be submitted. This choice
was made using the `openai-docs` skill after the repository knowledge base correctly
flagged hook payloads as version-sensitive. The skill's manual-fetch helper failed on a
missing content-hash header; its prescribed official-docs MCP fallback supplied the
current Hooks reference.

This event is Codex-native acceptance-boundary evidence. It is not inferred from prompt
echo, spinner text, terminal layout, transcript format, `.cleared`, or Claude behavior.

### Fail-closed behavior

The detector returns false for:

- malformed lifecycle JSON;
- missing or wrongly typed required event fields;
- any event other than `UserPromptSubmit`;
- missing prompt text;
- marker-like prose embedded mid-line;
- marker data in an unrelated payload field;
- malformed marker JSON;
- wrong ticket ID;
- wrong generation.

Unknown extra payload fields are ignored. The function does not panic or mutate state on
bad evidence.

## Acceptance-criterion assessment

### Matching pending ticket and generation returns true

Met. `matching_fixture_acknowledges_pending_assignment` loads the checked-in matching
`UserPromptSubmit` fixture and classifies it against pending ticket `T-033-01-02`,
generation `42`. Result is true.

### Still-idle event returns false

Met. `clear_fixture_is_not_acknowledgment` loads a `SessionStart` payload with
`source: clear`. It represents a cleared Codex conversation before the assignment prompt
has been accepted. Result is false.

### Stale previous-ticket event returns false

Met. `previous_ticket_fixture_is_stale` uses the same representative session but carries
ticket `T-033-01-01`, generation `41`. Result against the current pending assignment is
false.

### Stale assignment generation returns false

Met. `previous_generation_fixture_is_stale` carries the current ticket ID but generation
`41`. Result against pending generation `42` is false. This directly proves ticket-only
matching cannot claim a retried assignment.

### No Claude handshake assumption

Met structurally. The production classifier does not import or examine `TransitionState`,
reset strategy, `.cleared`, `.stopped`, `.heartbeat`, Claude event names, or shared hook
scripts. The word Claude occurs only in module documentation describing the excluded
dependency.

### No terminal-render assumption

Met structurally. The module accepts a JSON string and pending identity. It performs no
Zellij access, terminal reads, rendering checks, filesystem reads, or transcript reads.

## Test coverage

### Focused detector suite

Command:

```text
cargo test -p lisa-plugin codex_ack
```

Result after isolated commit reconciliation: 9 passed, 0 failed.

Coverage includes:

- four acceptance fixtures;
- malformed JSON;
- wrong lifecycle event with a matching marker;
- marker embedded in prose rather than on its own line;
- marker located in an unrelated field;
- JSON-sensitive ticket ID round-trip;
- forward compatibility with unknown event fields.

### Plugin package suite

Command:

```text
cargo test -p lisa-plugin --lib
```

Result: 260 passed, 0 failed.

The suite includes existing Codex and Claude adapter tests, scheduler lifecycle tests,
reuse and recycle tests, timeout tests, pane-name tests, and UI tests.

### Strict lint

Command:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Result: passed.

The new APIs are not scheduler-consumed until `T-033-01-03`, so each future-consumed item
has a narrow `allow(dead_code)` annotation naming that dependent ticket. No module-wide or
crate-wide lint suppression was added.

### Workspace suite

Command:

```text
cargo test --workspace
```

Result: passed.

Observed unit-test counts:

- `lisa-cli`: 268 passed;
- `lisa-core`: 147 passed;
- `lisa-plugin`: 260 passed.

Integration and documentation tests also passed.

### Formatting and fixture validation

- `cargo fmt --all`: passed and applied expected formatting to the new module.
- all four fixture files pass `jq -e` JSON validation.
- ticket-owned source paths pass `git diff --check` before commit.

## Coverage strengths

- The acceptance truth table uses checked-in JSON rather than constructing only Rust
  values in test code.
- Positive and stale fixtures share realistic session context, ensuring the decision is
  based on assignment correlation rather than pane/session coincidence.
- Same-ticket stale-generation coverage exercises the most important retry ambiguity.
- Exact line matching prevents incidental prompt prose from acting as metadata.
- Structured JSON serialization covers ticket identifiers requiring escaping.
- Unknown-field coverage allows Codex to add payload fields without breaking detection.
- Full workspace verification exercises the module in the repository's normal build.

## Coverage gaps and dependent work

### Scheduler integration is intentionally absent

The scheduler does not yet allocate a generation, tag the delivered prompt, preserve the
hook payload, invoke the detector, or promote assignment state. Those changes belong to
`T-033-01-03`, whose acceptance criterion exercises the real pending-to-owned transition
and duplicate/stale state handling.

### Hook transport is not installed in this ticket

Generated `.codex/hooks.json` still configures the prior events only. No handler currently
persists `UserPromptSubmit` payloads for the plugin. Adding transport together with
generation-aware scheduler consumption is required before this classifier affects live
ownership. Keeping it out of this ticket avoids landing a partially consumed signal.

### Timeout and fallback remain absent

No finite ack deadline, recovery state entry, or fresh-session fallback exists yet.
`T-033-01-04` owns those behaviors after ack-gated promotion lands.

### No live-token proof

The story defines this classifier stage as fixture-proven and free of live tokens. The
live repeated-reuse proof remains in `S-033-03`. This ticket therefore validates the
documented lifecycle contract deterministically but does not claim a fresh live Codex run.

## Open concerns

### `UserPromptSubmit` is a pre-send lifecycle boundary

Official documentation describes the prompt as about to be sent and permits other
matching hooks to block it. Lisa's eventual capture hook must be non-blocking and exit
successfully. A separate user hook could still block the same prompt after the event was
emitted. If live testing shows this creates a meaningful false-positive path, the
scheduler integration should require correlated later activity from the same Codex turn
before promotion. This is an operational edge, not observed evidence in the fixture-only
scope.

### Hook schema and delivery can drift

The repository already records Codex lifecycle version drift. The detector intentionally
uses only `hook_event_name` and `prompt`, but removal or renaming of `UserPromptSubmit`
fields would cause safe false negatives and then exercise bounded recovery. The live
harness must be rerun on Codex upgrades.

### Generation allocation must avoid reuse

The classifier assumes a generation uniquely identifies a delivery attempt. The next
ticket must allocate generations so a released/recreated pending assignment cannot reuse
an old value within the scope where delayed events may arrive. A monotonically increasing
counter or equivalently unique token satisfies the contract.

### Marker is model-visible prompt metadata

The marker line is part of the submitted prompt. It is intentionally short and explicit,
but live integration should confirm it does not distract the agent. It should not be
removed from provider-visible prompt text unless another documented hook correlation
field can carry Lisa's pre-submit generation.

## Critical issues requiring human attention

None found in the implemented classifier, fixtures, tests, lint, or isolated commit.

The dependent integration tickets remain critical to the full story: until they land,
recycled Codex seats continue to remain pending and this detector is not invoked at
runtime. The narrow dead-code annotations make that temporary boundary visible.

## Files created

- `crates/lisa-plugin/src/codex_ack.rs`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/matching-prompt-submit.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/still-idle-clear.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-ticket.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-generation.json`;
- all six RDSPI work artifacts under `docs/active/work/T-033-01-02/`.

## Files modified

- `crates/lisa-plugin/src/lib.rs`.

## Files deleted

- None.

## Repository and commit ownership review

- Source commit contains exactly the six planned paths.
- Commit was created through `cargo run -p lisa-cli -- commit-ticket`.
- No ordinary `git add`, `git commit`, or broad staging command was used.
- Ticket-owned source and fixture paths are clean after commit.
- No ticket-owned source path is staged in the ordinary index.
- Work artifacts remain untracked for Lisa's final completion transaction.
- Existing unrelated modified and untracked paths were preserved.
- Ticket phase and status frontmatter were not manually edited.
- Lisa retains responsibility for phase transitions, Done publication, and seat release.

## Final assessment

`T-033-01-02` is complete at its defined detector boundary. Lisa now has a small,
provider-native classifier that binds Codex prompt-submission evidence to an exact pending
ticket and assignment generation, rejects reset-only and stale lifecycle events, fails
closed on malformed input, and carries no Claude handshake or terminal-render assumptions.
The contract is ready for `T-033-01-03` to wire into ack-gated ownership.
