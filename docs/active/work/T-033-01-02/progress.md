# Progress: Codex acknowledgment detector

## Status

Implementation is complete and verified. The meaningful source unit is ready for the
required isolated Lisa commit. Review remains after commit reconciliation.

## Completed phases

- Research: complete in `research.md`.
- Design: complete in `design.md`.
- Structure: complete in `structure.md`.
- Plan: complete in `plan.md`.
- Implement: source and tests complete.
- Review: pending isolated commit and final handoff assessment.

## Implemented source changes

### Provider-specific module

Created `crates/lisa-plugin/src/codex_ack.rs`.

The module owns:

- borrowed pending-assignment identity;
- canonical assignment marker serialization;
- prompt tagging;
- minimal Codex lifecycle payload parsing;
- exact `UserPromptSubmit` classification;
- ticket and generation correlation;
- fail-closed handling for invalid evidence.

The module deliberately does not own:

- scheduler assignment state;
- hook transport;
- signal files;
- Zellij pane access;
- transcript parsing;
- timeout policy;
- ownership promotion;
- Claude behavior.

### Root module registration

Modified `crates/lisa-plugin/src/lib.rs` only to add:

```text
mod codex_ack;
```

No scheduler methods, state fields, transitions, prompts, or provider adapters changed.

### Assignment identity

Added `CodexAssignmentRef` with:

- `ticket_id: &str`;
- `generation: u64`.

This is the exact identity the dependent scheduler ticket will retain while a recycled
Codex assignment is pending. Generation allocation remains outside this detector.

### Canonical marker

Added a structured prompt line:

```text
LISA_ASSIGNMENT {"ticket_id":"...","generation":...}
```

The marker object is serialized with `serde_json`; ticket IDs are not interpolated into
JSON manually. The tagging helper preserves the prompt body and places the marker at a
line boundary.

### Fail-closed detector

`detect_codex_ack` returns true only when:

- the payload is valid JSON;
- `hook_event_name` is exactly `UserPromptSubmit`;
- the payload has a string `prompt` field;
- a prompt line starts with the canonical prefix;
- the marker remainder is valid JSON;
- marker ticket ID equals the pending ticket;
- marker generation equals the pending generation.

Every error and mismatch returns false. The detector ignores unknown extra lifecycle
fields for compatibility with evolving Codex payloads.

## Added fixture coverage

Created four JSON fixtures under
`crates/lisa-plugin/tests/fixtures/codex_ack/`:

- `matching-prompt-submit.json`;
- `still-idle-clear.json`;
- `stale-previous-ticket.json`;
- `stale-previous-generation.json`.

The payloads use the documented Codex lifecycle shape, including representative session,
turn, transcript, working-directory, model, and permission fields. The detector reads
only the stable semantic fields it needs.

Fixture truth table against pending `T-033-01-02`, generation `42`:

| Fixture | Event | Identity | Ack |
| --- | --- | --- | --- |
| matching | `UserPromptSubmit` | ticket current, generation 42 | true |
| still idle | `SessionStart(clear)` | no submitted assignment | false |
| previous ticket | `UserPromptSubmit` | ticket `T-033-01-01`, generation 41 | false |
| previous generation | `UserPromptSubmit` | ticket current, generation 41 | false |

## Defensive test coverage

Added five additional tests beyond the four fixture cases:

- malformed payload JSON fails closed;
- matching marker on `Stop` is not acknowledgment;
- marker text embedded mid-line is not accepted;
- marker text in an unrelated JSON field is not accepted;
- JSON-sensitive ticket IDs round-trip and unknown event fields are ignored.

Total new detector tests: 9.

## Verification completed

### Formatting

Command:

```text
cargo fmt --all
```

Result: passed.

The initial `cargo fmt --all -- --check` reported only expected formatting differences in
the new detector module. `cargo fmt --all` applied them. Existing unrelated dirty paths
remain outside ticket ownership.

### Fixture syntax

Command shape:

```text
jq -e . crates/lisa-plugin/tests/fixtures/codex_ack/*.json
```

Result: all four fixtures are valid JSON.

### Focused detector tests

Command:

```text
cargo test -p lisa-plugin codex_ack
```

Result: 9 passed, 0 failed.

### Plugin package tests

Command:

```text
cargo test -p lisa-plugin --lib
```

Result: 260 passed, 0 failed.

This includes all existing adapter, scheduler, transition, Codex, Claude, pane-name, and
UI unit tests.

### Strict lint

Command:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Result: passed.

The detector APIs have narrow dead-code annotations naming `T-033-01-03`, because this
ticket intentionally lands the classifier one dependency before scheduler consumption.

### Workspace tests

Command:

```text
cargo test --workspace
```

Result: passed.

Observed package counts include:

- `lisa-cli`: 268 unit tests passed;
- `lisa-core`: 147 unit tests passed;
- `lisa-plugin`: 260 unit tests passed;
- integration and documentation tests passed.

### Static source review

Checks completed:

- ticket-owned source diff passes `git diff --check`;
- fixtures were summarized with `jq` to confirm their attribution dimensions;
- detector source contains no filesystem access;
- detector source contains no transcript reads;
- detector source contains no terminal or Zellij access;
- detector source contains no Claude event or signal classification;
- ordinary Git index has no staged paths.

## Deviations from Plan

### Marker type uses an owned string

Structure allowed an owned `String` if borrowed deserialization complicated the code.
The implementation uses `String` for `AssignmentMarker.ticket_id`. The payload is tiny,
classification is infrequent, and the owned shape keeps serialization and deserialization
straightforward. This is an anticipated choice, not a scope deviation.

### Documentation verification path

The `openai-docs` Codex manual helper failed because the manual response omitted its
expected content-hash header. Following the skill fallback, the official developer docs
MCP fetched the current Hooks page. It confirmed `UserPromptSubmit` fields and semantics.
No source scope changed because of the fallback.

### No live-token capture

The story explicitly describes the detector proof as fixture-based and free of live
tokens. The checked-in fixtures preserve the documented lifecycle payload shape; no live
Codex invocation was added to this implementation ticket.

## Unchanged behavior

- Recycled Codex assignments still remain `AssignedPendingAck` indefinitely until the
  dependent scheduler tickets land.
- Fresh Codex assignments still use the existing immediate-owned contract.
- Claude assignments and clear handshakes are unchanged.
- Codex generated hook configuration is unchanged.
- Shared `.lisa/hooks` scripts are unchanged by this ticket.
- Prompt delivery does not yet call `tag_codex_assignment`.
- Scheduler polling does not yet call `detect_codex_ack`.
- No acknowledgment deadline exists yet.

## Remaining implementation work in dependent tickets

`T-033-01-03` must:

- allocate and retain an assignment generation;
- tag the issued reused-Codex prompt;
- transport the `UserPromptSubmit` payload to the scheduler;
- invoke this detector against the pending assignment;
- promote matching pending state to owned exactly once;
- reject stale and duplicate signals at the state transition boundary.

`T-033-01-04` must:

- add a finite acknowledgment deadline;
- enter `Recovering` on timeout;
- perform at most one fresh-session fallback;
- surface terminal recovery failure explicitly.

These items are outside this detector ticket's acceptance boundary.

## Commit plan

Meaningful unit message:

```text
feat: detect ticket-scoped Codex acknowledgments
```

Exact source includes:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/codex_ack.rs`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/matching-prompt-submit.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/still-idle-clear.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-ticket.json`;
- `crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-generation.json`.

The commit will use the Lisa isolated transaction. Work artifacts remain for Lisa's final
completion commit. Ticket frontmatter has not been manually edited.
