# Structure: Codex acknowledgment detector

## Change inventory

### Create

- `crates/lisa-plugin/src/codex_ack.rs`
- `crates/lisa-plugin/tests/fixtures/codex_ack/matching-prompt-submit.json`
- `crates/lisa-plugin/tests/fixtures/codex_ack/still-idle-clear.json`
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-ticket.json`
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-generation.json`
- `docs/active/work/T-033-01-02/research.md`
- `docs/active/work/T-033-01-02/design.md`
- `docs/active/work/T-033-01-02/structure.md`
- `docs/active/work/T-033-01-02/plan.md`
- `docs/active/work/T-033-01-02/progress.md`
- `docs/active/work/T-033-01-02/review.md`

### Modify

- `crates/lisa-plugin/src/lib.rs`

### Delete

- None.

## Module boundary

`codex_ack.rs` owns the provider-specific data contract between a Lisa assignment and a
Codex lifecycle payload. It has no access to `State`, `AgentSlot`, `TransitionState`,
`SeatAssignmentState`, Zellij APIs, filesystem state, or timers.

The module performs two operations:

1. encode a pending assignment identity into prompt text;
2. classify a Codex lifecycle JSON payload against that identity.

This boundary makes the detector deterministic and fixture-testable. Scheduler wiring in
`T-033-01-03` can call it without moving provider JSON parsing into `lib.rs`.

## Public crate-private interface

### `CodexAssignmentRef`

```rust
pub(crate) struct CodexAssignmentRef<'a> {
    pub ticket_id: &'a str,
    pub generation: u64,
}
```

Properties:

- borrowed ticket ID avoids allocation at call sites;
- `u64` makes generation comparison explicit and cheap;
- derives `Debug`, `Clone`, `Copy`, `PartialEq`, and `Eq`;
- it does not own scheduler lifecycle or allocation policy.

### `tag_codex_assignment`

```rust
pub(crate) fn tag_codex_assignment(
    prompt: &str,
    assignment: CodexAssignmentRef<'_>,
) -> String
```

Responsibilities:

- serialize a private marker value with `serde_json`;
- preserve prompt contents;
- ensure exactly one newline separates prompt and marker;
- emit the canonical `LISA_ASSIGNMENT ` prefix;
- return a ready-to-submit prompt string.

It does not decide when a prompt should be tagged or generate the generation value.

### `detect_codex_ack`

```rust
pub(crate) fn detect_codex_ack(
    payload_json: &str,
    pending: CodexAssignmentRef<'_>,
) -> bool
```

Responsibilities:

- deserialize the minimal lifecycle envelope;
- require `hook_event_name == "UserPromptSubmit"`;
- require a string `prompt` field;
- locate canonical whole-line assignment metadata;
- deserialize the marker payload;
- compare both ticket and generation;
- return false for every parse error or mismatch.

The function intentionally does not return or log payload contents. It is a classifier,
not a telemetry or policy engine.

## Private types

### `LifecycleEvent`

Minimal deserialization shape:

```rust
struct LifecycleEvent<'a> {
    hook_event_name: &'a str,
    prompt: Option<&'a str>,
}
```

Use borrowed strings where Serde permits. Unknown fields are ignored, allowing fixtures
and live payloads to retain session, turn, model, permission, and path data.

### `AssignmentMarker`

Serialization and deserialization shape:

```rust
struct AssignmentMarker<'a> {
    ticket_id: Cow<'a, str>,
    generation: u64,
}
```

If borrowed deserialization complicates the implementation, use an owned `String`; the
payload is small and classification is infrequent. Correctness is preferred over a
fragile lifetime optimization.

The marker type remains private so its JSON encoding is controlled by the helper rather
than constructed across the scheduler.

## Constants

Define one private constant:

```rust
const ASSIGNMENT_PREFIX: &str = "LISA_ASSIGNMENT ";
```

Tests in the module validate the emitted representation through the public helper and
detector rather than importing the constant externally.

## Parsing organization

The detector's internal sequence is linear:

```text
payload JSON
  -> LifecycleEvent
  -> exact event-name guard
  -> prompt field guard
  -> prompt lines
  -> exact prefix strip
  -> AssignmentMarker JSON
  -> ticket equality + generation equality
  -> boolean acknowledgment
```

Marker lines that fail JSON parsing are skipped. A later valid marker may match, although
the tagging helper emits only one. This keeps unrelated prompt prose harmless and lets
the classifier fail closed without surfacing parser errors as control flow.

## Root module change

Add:

```rust
mod codex_ack;
```

next to `mod adapter`, `mod pane_name`, and `mod ui` in
`crates/lisa-plugin/src/lib.rs`.

No imports or calls are added to scheduler code in this ticket. The module's tests count
as its current consumers, and crate-private items are annotated for the dependent
scheduler ticket if strict dead-code lint requires it.

Prefer avoiding blanket `allow(dead_code)`. A narrow attribute with a comment naming
`T-033-01-03` is acceptable because the detector intentionally lands one ticket before
its scheduler consumer.

## Fixture structure

All fixtures are single JSON objects with current documented Codex hook field names.

Common realistic fields:

- `session_id`;
- `transcript_path`;
- `cwd`;
- `hook_event_name`;
- `model`;
- `permission_mode` where supported;
- `turn_id` for turn-scoped events.

### `matching-prompt-submit.json`

- event: `UserPromptSubmit`;
- current ticket: `T-033-01-02`;
- generation: `42`;
- prompt ends with the matching structured marker;
- expected result: true.

### `still-idle-clear.json`

- event: `SessionStart`;
- source: `clear`;
- no prompt or turn ID;
- expected result: false.

This captures the post-reset/pre-prompt boundary that must not become ownership.

### `stale-previous-ticket.json`

- event: `UserPromptSubmit`;
- same representative Codex session;
- previous ticket ID;
- an older generation;
- expected result against the current pending assignment: false.

### `stale-previous-generation.json`

- event: `UserPromptSubmit`;
- same current ticket ID;
- generation `41` rather than `42`;
- expected result: false.

This is the direct proof that ticket matching alone is insufficient.

## Unit-test organization

Tests live in `codex_ack.rs` under `#[cfg(test)]` so they can exercise private marker
details while keeping the production API narrow.

Helper constants load fixtures using repository-relative `include_str!` paths. Compile
time inclusion means tests do not depend on current working directory or WASI file access.

Primary tests:

- `matching_fixture_acknowledges_pending_assignment`;
- `clear_fixture_is_not_acknowledgment`;
- `previous_ticket_fixture_is_stale`;
- `previous_generation_fixture_is_stale`.

Additional contract tests:

- tagging round-trips ticket IDs safely through JSON;
- malformed payload returns false;
- wrong event with a matching marker returns false;
- embedded mid-line marker returns false;
- marker text in an unrelated JSON field returns false.

## Dependency impact

- `serde` already has derive enabled in `lisa-plugin`.
- `serde_json` already exists in production dependencies.
- No Cargo manifest change is planned.
- No native-only dependency or API is introduced.
- The module remains compilable for `wasm32-wasip1`.

## Ownership and commit structure

One meaningful source unit is expected:

```text
feat: detect ticket-scoped Codex acknowledgments
```

Exact source includes:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/codex_ack.rs`;
- the four `crates/lisa-plugin/tests/fixtures/codex_ack/*.json` files.

The commit must use `lisa commit-ticket --ticket-id T-033-01-02` through the repository
CLI if the installed binary lacks the command. Work artifacts are not included in the
source commit; Lisa owns their final completion transaction.

## Change ordering

1. Add the root module declaration.
2. Add the assignment value and canonical tagging helper.
3. Add lifecycle and marker parsing.
4. Add captured-shape fixtures.
5. Add the fixture truth-table tests.
6. Add malformed and exact-match tests.
7. Format and run focused tests.
8. Run package and workspace verification.
9. Commit exact source paths through Lisa.
10. Confirm ticket-owned source paths are clean.

## Structural invariants

- The detector never reads pane contents.
- The detector never reads a transcript path.
- The detector never checks `.cleared`, `.stopped`, or `.heartbeat` files.
- The detector never references Claude hook semantics.
- The detector never changes scheduler state.
- True requires exact event type, ticket ID, and generation.
- Invalid evidence is equivalent to no acknowledgment.
- Fixture files remain immutable test inputs.
- Ticket frontmatter remains untouched.
