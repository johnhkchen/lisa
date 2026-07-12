# Design: Codex acknowledgment detector

## Decision summary

Add a small Codex-specific lifecycle classifier in the plugin crate. A pending
assignment is identified by ticket ID plus a monotonically unique assignment generation.
Lisa encodes that identity as a structured marker in the prompt it issues. The detector
returns true only for a `UserPromptSubmit` payload whose prompt carries an exactly matching
marker. All other valid or malformed lifecycle payloads return false.

This ticket defines and fixture-tests the detector boundary. It does not connect the
detector to scheduler promotion, hook-file transport, deadlines, recovery, or UI state.
Those behaviors remain in their dependent tickets.

## Design goals

- Use positive Codex lifecycle evidence.
- Attribute evidence to one pending ticket assignment.
- Distinguish repeated attempts for the same ticket.
- Reject events from the ticket previously resident in the pane.
- Reject reset-only evidence while the TUI is still idle.
- Fail closed on malformed or incomplete input.
- Avoid terminal scraping and transcript parsing.
- Avoid Claude event names and clear-handshake semantics.
- Keep scheduler mutation out of the classifier.
- Require no new dependency.
- Make fixture payloads directly reviewable.

## Non-goals

- Installing a new hook handler.
- Writing lifecycle payloads to `.lisa/signals`.
- Choosing the assignment-generation storage lifetime.
- Promoting `AssignedPendingAck` to `Owned`.
- Deduplicating scheduler transitions.
- Starting an acknowledgment deadline.
- Entering `Recovering`.
- Launching a fresh fallback session.
- Changing fresh Codex ownership behavior.
- Changing Claude reset or ownership behavior.
- Displaying assignment state in the dashboard.

## Option 1: treat `.cleared` as acknowledgment

The existing Codex adapter already receives a `SessionStart(clear)` event and normalizes
it into `pane-<id>.cleared`. The scheduler could regard this signal as acceptance.

Advantages:

- no new event type;
- no payload parsing;
- no new hook wiring.

Disadvantages:

- the event precedes the new ticket prompt;
- it proves only that reset completed;
- clear timeout can send the prompt without this event;
- it cannot identify ticket or generation;
- it is the same transport handshake used by Claude;
- it would mark ownership before Codex actually accepts the assignment.

Decision: reject. It violates the positive-evidence and provider-appropriate requirements.

## Option 2: infer acceptance from terminal contents

The plugin could inspect the pane for a spinner, echoed prompt, token counter, or other
rendered state after the delayed Enter.

Advantages:

- closely follows what a human sees;
- may appear immediately after submission.

Disadvantages:

- terminal text and layout are not stable semantic APIs;
- Zellij plugin state does not currently expose a clean transcript contract;
- rendered strings vary by Codex version, theme, width, and locale;
- old scrollback can be mistaken for current activity;
- ticket and generation attribution would still be fragile;
- the ticket explicitly excludes terminal-text scraping.

Decision: reject.

## Option 3: acknowledge on any `PostToolUse`

Codex already has a configured `PostToolUse` lifecycle hook. A heartbeat after prompt
delivery is positive agent activity.

Advantages:

- reuses an installed event;
- demonstrates that Codex executed work;
- does not depend on rendering.

Disadvantages:

- valid turns may not invoke a supported tool;
- long reasoning creates an unnecessary acknowledgment delay;
- the current normalized heartbeat drops session and turn identity;
- a late heartbeat from the previous ticket can be misattributed;
- the hook payload does not include the originating user prompt;
- correlating it requires a prior turn-binding event anyway.

Decision: reject as the primary acknowledgment. It remains useful liveness evidence.

## Option 4: acknowledge on `Stop`

The existing Stop hook is turn-scoped and includes session and turn identifiers.

Advantages:

- it is positive completion evidence;
- it already participates in Lisa lifecycle handling.

Disadvantages:

- it arrives after the whole turn rather than at acceptance;
- the seat could remain pending throughout productive work;
- current normalized signal files discard correlation fields;
- a delayed previous-ticket Stop event is stale;
- it does not directly contain the submitted assignment prompt.

Decision: reject as too late and insufficiently attributable by itself.

## Option 5: correlate `UserPromptSubmit` to an assignment marker

Codex emits `UserPromptSubmit` at turn scope and includes `session_id`, `turn_id`, and
`prompt`. Lisa can append a structured identity marker to the prompt it submits. The
detector compares that marker with the pending ticket and generation.

Advantages:

- it is a Codex-native lifecycle event;
- it occurs at the prompt-to-agent boundary;
- it carries the exact submitted prompt;
- it can bind Lisa identity to a Codex turn without transcripts;
- exact ticket and generation matching rejects stale activity;
- it is independent of Claude clear signals;
- it works even when the ticket uses no tools;
- it requires only JSON parsing already present in the crate.

Disadvantages:

- Lisa must add a `UserPromptSubmit` hook in later integration work;
- the prompt gains a small machine-readable line;
- scheduler code must create and retain a generation later;
- project hook trust and delivery remain operational prerequisites.

Decision: choose this option. It is the narrowest event with direct assignment
correlation and the earliest positive lifecycle evidence available after submission.

## Assignment identity

Define a borrowed value:

```text
CodexAssignmentRef {
    ticket_id: &str,
    generation: u64,
}
```

The ticket ID identifies logical work. The generation identifies one delivery attempt.
Generation must be unique for a pane assignment lifetime; the later scheduler integration
owns how values are allocated and persisted.

Ticket-only matching is deliberately insufficient. A retry of the same ticket can leave
a delayed `UserPromptSubmit` event from generation N after generation N+1 is pending.

## Marker format

Use one exact, line-oriented marker:

```text
LISA_ASSIGNMENT {"ticket_id":"T-033-01-02","generation":7}
```

The JSON object is serialized by `serde_json`; it is not assembled by unescaped string
interpolation. The prefix gives the detector a narrow scan target. The marker occupies a
whole line appended to the issued prompt. The parser requires exactly one structured
object after the prefix and exact field equality.

This is prompt metadata, not terminal rendering. The provider reports it back in a
lifecycle payload at submission time.

## Detector API

Expose crate-private helpers from a new `codex_ack` module:

```text
tag_codex_assignment(prompt, assignment) -> String
detect_codex_ack(payload_json, pending_assignment) -> bool
```

`tag_codex_assignment` appends the canonical marker to arbitrary prompt text.

`detect_codex_ack` parses a minimal lifecycle envelope. It returns true only when:

1. the JSON is a valid object;
2. `hook_event_name` is exactly `UserPromptSubmit`;
3. `prompt` is present as a string;
4. a whole prompt line begins with the canonical prefix;
5. the remainder parses as the marker schema;
6. ticket ID equals the pending ticket;
7. generation equals the pending generation.

Malformed JSON, missing fields, wrong event types, malformed marker JSON, and all
mismatches return false. Fail-closed boolean behavior fits a polling classifier: bad
evidence is ignored, never promoted.

## Session and turn fields

Fixtures retain realistic `session_id` and `turn_id` values. The first classifier does
not require expected values for either field because the assignment generation is minted
by Lisa before Codex creates a turn, and the existing scheduler does not yet know the new
Codex session or turn ID.

The lifecycle event still establishes the provider turn associated with the prompt. A
future transport envelope may retain its session and turn IDs for diagnostics or later
activity correlation without changing the core marker decision.

## False-positive controls

- `SessionStart(clear)` has no submitted prompt and returns false.
- A `UserPromptSubmit` for the prior ticket has a different ticket ID and returns false.
- A retry event for the same ticket with an old generation returns false.
- An arbitrary event containing the marker in another field returns false.
- A different lifecycle event with a prompt-like field returns false.
- A marker embedded mid-line returns false.
- A malformed marker returns false.
- A terminal screenshot or transcript string is never accepted as input.

## Fixture strategy

Create checked-in JSON fixtures under
`crates/lisa-plugin/tests/fixtures/codex_ack/`:

- matching `UserPromptSubmit`;
- still-idle `SessionStart(clear)`;
- stale previous-ticket `UserPromptSubmit`;
- stale same-ticket previous-generation `UserPromptSubmit`.

The fixtures use documented Codex field names and preserve realistic unrelated fields.
Tests load them with `include_str!`, run the public classifier boundary, and assert the
required truth table. Additional inline negative cases exercise malformed input and exact
line matching.

## Placement

The detector belongs in `crates/lisa-plugin/src/codex_ack.rs` because:

- it classifies a native Codex adapter event;
- its immediate consumer is plugin scheduler state;
- no CLI command or core ticket model needs this provider-specific schema;
- keeping it out of `lisa-core` avoids provider protocol leakage.

`crates/lisa-plugin/src/lib.rs` declares the private module only. It does not yet mutate
assignment state in response to detection.

## Compatibility

- Existing prompts are unchanged unless the new tagging helper is called.
- Existing scheduler transitions are unchanged.
- Existing generated hook files are unchanged in this ticket.
- Existing Claude code is untouched.
- Unknown extra event fields are ignored by Serde.
- No WASM-incompatible filesystem or process API is introduced.

## Verification

- Run focused `codex_ack` unit tests.
- Run all `lisa-plugin` library tests.
- Run `cargo test --workspace`.
- Run `cargo fmt --all`.
- Run strict Clippy for `lisa-plugin` all targets.
- Inspect the ticket commit for exact source and fixture ownership.
- Confirm unrelated dirty paths remain untouched.

## Final decision

Implement a provider-specific, fail-closed classifier over documented Codex
`UserPromptSubmit` JSON. Correlate the prompt to a structured Lisa ticket/generation
marker. This provides fixture-proven acknowledgment semantics at the correct contract
boundary while leaving hook transport and pending-to-owned scheduler mutation to the
next ticket in the linear story.
