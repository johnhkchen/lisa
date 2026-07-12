# Design: gate ownership on acknowledgment

## Goal

Connect the existing ticket-scoped Codex acknowledgment detector to scheduler ownership.
A recycled Codex assignment remains pending until a `UserPromptSubmit` payload proves that
Codex submitted the prompt for the exact ticket and assignment generation.

## Non-goals

- Do not change fresh Codex immediate ownership.
- Do not change any Claude assignment behavior.
- Do not treat clear, heartbeat, stop, idle, or terminal output as acknowledgment.
- Do not add an acknowledgment deadline.
- Do not enter `Recovering`.
- Do not add fresh-session recovery.
- Do not project assignment state into the dashboard.
- Do not persist live seat assignment state across plugin restarts.

## Decision 1: identity storage

### Option A: store generation in a parallel map

Keep `SeatAssignmentState::AssignedPendingAck` fieldless and add a
`pending_assignment_generations: HashMap<u32, u64>`.

Advantages:

- small change to enum matches;
- generation storage can be removed independently.

Costs:

- two maps can drift;
- pending state can exist without identity;
- release and recovery must update both maps;
- tests can construct invalid scheduler states accidentally.

Decision: reject. The generation is part of pending assignment truth.

### Option B: create a separate assignment record

Replace the map value with a struct containing state, ticket, and generation.

Advantages:

- centralizes all assignment metadata;
- may suit later UI and recovery work.

Costs:

- duplicates `AgentSlot::ticket_id` in this ticket;
- broadens a narrowly introduced prerequisite model;
- requires larger cleanup and reconciliation rules.

Decision: reject for now. The existing split deliberately keeps routing reservation in
`AgentSlot` and assignment truth in the map.

### Option C: carry generation in the pending enum variant

Change the variant to `AssignedPendingAck { generation: u64 }`.

Advantages:

- pending state is always classifiable;
- `Owned` and `Recovering` remain compact;
- release remains a single map removal;
- exact matching can read state and slot ticket together;
- future recovery can consume the pending generation during its transition.

Costs:

- existing pattern matches and tests need updates;
- `SeatAssignmentState` is no longer uniformly fieldless.

Decision: choose Option C.

## Decision 2: generation allocation

### Options considered

- timestamp-derived generations;
- random generations;
- process-local monotonic counter;
- pane-local counters.

Timestamps can collide under fast scheduling and require clock assumptions. Random values
need a randomness source in WASM. Pane-local counters need another map and can repeat after
seat cleanup. A process-local monotonic `u64` counter is deterministic and sufficient for
the lifetime of in-memory pending state.

Decision: add `next_assignment_generation` to `State`. Allocation increments with checked
or saturating wrap handling before each pending Codex assignment. Generation zero remains
available as the default pre-allocation value; actual assignments receive a positive value.
The counter is scheduler identity, not a user-visible sequence.

## Decision 3: tag at the adapter boundary

### Option A: tag in `schedule_ready_tickets`

This works for bare reuse prompts, but the fresh interactive command already embeds and
shell-quotes the prompt inside `CodexAdapter`. Rewriting the completed shell string would be
fragile and provider-specific.

### Option B: change the global `ticket_prompt`

This would affect Claude, tests, and any non-assignment use. The helper also lacks generation
context.

### Option C: pass optional generation through `SpawnContext`

The scheduler already creates `SpawnContext` at each delivery boundary. Codex can tag the
prompt before embedding it in either a launch command or bare reuse delivery. Claude ignores
the optional field and preserves exact output.

Decision: choose Option C. Add `assignment_generation: Option<u64>` to `SpawnContext` and a
Codex-only prompt helper in the adapter. Pending recycled Codex paths pass `Some(generation)`.
Owned/fresh and Claude paths pass `None`.

## Decision 4: native payload transport

### Option A: infer acknowledgment from heartbeat

A heartbeat does not carry ticket or generation. Delayed tool activity can belong to the
previous ticket. This violates the detector contract.

Decision: reject.

### Option B: write only a boolean `.ack` file

The hook would classify or discard the JSON outside the plugin. That duplicates the detector,
requires passing expected generation to shell state, and makes stale protection harder to
test centrally.

Decision: reject.

### Option C: persist the raw `UserPromptSubmit` payload

Add an `on-ack.sh` hook. It copies standard input to a pane-scoped temporary file, then
atomically renames it to `pane-<id>.ack`. The plugin reads the raw payload and invokes the
existing detector against current scheduler state.

Advantages:

- the detector remains the only classifier;
- JSON is not parsed or interpolated by shell;
- attribution uses the existing pane environment;
- the plugin decides against current ticket/generation state;
- temporary writes are not visible as complete acknowledgments.

Decision: choose Option C.

## Decision 5: promotion API

Introduce a focused scheduler method:

```rust
fn acknowledge_codex_assignment(&mut self, pane_id: u32, payload: &str) -> bool
```

The method:

1. reads the current slot ticket;
2. requires `AssignedPendingAck { generation }`;
3. calls `detect_codex_ack` with ticket and generation;
4. replaces the state with `Owned` only on true;
5. returns true only when the transition occurred.

The boolean makes “exactly once” directly testable. It also avoids logging duplicate or stale
events as successful transitions.

The method does not inspect route or transition state. Only scheduling can create a pending
Codex assignment, so the pending variant is the provider gate. This keeps acknowledgment
independent of whether prompt delivery followed clear or exit transport.

## Decision 6: signal consumption

Add `check_codex_ack_signals` alongside existing signal scanners. For every correctly named
`pane-<id>.ack` file it:

- reads the payload;
- removes the file regardless of validity;
- calls the focused promotion method;
- bumps pane activity when a valid promotion occurs;
- logs one informational transition only when promotion occurs.

Unreadable files are removed and treated as no acknowledgment. Unknown pane IDs, unassigned
panes, owned panes, stale generations, and malformed payloads are consumed without mutation.

The scanner runs in `poll_tick` before future recovery/deadline evaluation. It does not replace
heartbeat handling because prompt submission and tool activity represent different facts.

## Decision 7: hook installation

Add `ON_ACK_HOOK` to the CLI's versioned templates. Extend both generated Codex hooks and merge
logic with a matcher-less `UserPromptSubmit` entry invoking `.lisa/hooks/on-ack.sh`.

Extend init's hook-script inventory so new installations create the file and existing managed
installations update it through normal ownership-aware planning. Extend validation's required
script list. Existing user `UserPromptSubmit` hooks remain present; `ensure_hook` deduplicates
Lisa's command by script path.

No Claude settings entry is added. This is a Codex-native lifecycle event and the plugin only
uses it for states created by Codex scheduling.

## Shell safety

The hook uses:

```sh
tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
cat > "$tmp" && mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
```

The pane ID comes from Lisa's numeric scheduler environment. Payload bytes flow only through
standard input and `cat`. Quoted paths avoid word splitting. Rename is within one directory,
so readers see the previous complete file or the new complete file, never a partial write.

## Test design

### Scheduler transition test

Create a state with one recycled Codex seat, schedule a ticket, and capture its generated
pending generation. Assert:

- no acknowledgment leaves the state pending and not-owned;
- stale-ticket payload returns false and leaves it pending;
- stale-generation payload returns false and leaves it pending;
- matching payload returns true and makes it owned;
- the same payload a second time returns false and remains owned.

This directly satisfies the ticket acceptance criterion.

### Signal integration test

Write a matching payload to `pane-<id>.ack`, run the scanner, and assert the file is consumed
and the state is owned. A rejected payload test verifies consumption without promotion.

### Adapter tests

Assert Codex reuse and launch outputs carry the marker when a generation is present. Assert
Claude output remains equal to the pre-change free function. Assert Codex with no generation
retains existing output, preserving fresh assignment behavior.

### CLI tests

Assert generated hooks include `UserPromptSubmit` and `on-ack.sh`. Assert merge preserves a
user hook and adds exactly one Lisa hook across repeated merges. Assert init/validate fixtures
include the new script through existing inventory-driven tests.

## Failure behavior

- Missing hook: seat remains pending; recovery ticket will bound that state later.
- Invalid JSON: no promotion.
- Partial hook write: atomic rename prevents observation.
- Unknown pane: no promotion.
- Released assignment: no promotion.
- Stale ticket or generation: no promotion.
- Duplicate matching event after ownership: no promotion.
- Counter exhaustion: saturating behavior avoids reuse during the process lifetime; practical
  exhaustion is unreachable.

## Chosen design summary

Make the pending assignment self-identifying, tag only Codex prompts at their adapter delivery
boundary, transport raw native `UserPromptSubmit` payloads through an atomic pane-scoped signal,
and promote through one exact-match scheduler method. This design uses the detector as the only
truth gate, preserves all Claude and fresh-Codex contracts, and gives the next ticket a clean
place to evaluate acknowledgment deadlines after matching payloads have been consumed.
