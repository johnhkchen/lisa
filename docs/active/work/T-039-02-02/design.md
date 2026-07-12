# Design: T-039-02-02

## Goal

Introduce a single typed filesystem signal-ingestion boundary used by all eight
signal consumers while preserving every characterized behavior. The boundary
normalizes filesystem mechanics, not provider semantics or scheduler admission.

## Design forces

- All consumers scan the same directory but recognize different record shapes.
- Three consumers deserialize `AttemptLease` JSON.
- One consumer forwards raw provider JSON.
- Four logical signal kinds use presence only.
- Idle has both pane and legacy ticket targets.
- Transition recognizes two presence kinds during one scan.
- Deletion timing differs at the filename-recognition boundary.
- Lease currency depends on mutable scheduler state and must remain downstream.
- Poll ordering must remain visible in `poll_tick`.
- The existing characterization module cannot change.

## Option 1: one eager scan per poll

This approach would scan `signal_dir` once at the start of `poll_tick`, parse all
known files into a large batch, delete them, then pass subsets to consumers.

Advantages:

- Only one filesystem directory scan per tick.
- A complete snapshot could make ingestion performance predictable.
- A single batch type could enumerate every signal kind.

Disadvantages:

- It changes timing: later signal consumers currently see records created after
  earlier consumers run in the same tick.
- It would move deletion of all recognized records to the beginning of the poll.
- Artifact and assignment operations interleaved between consumers would no
  longer occur between scans.
- It makes the required poll order less direct because ingestion order and state
  application order become separate concepts.
- It risks changing transition behavior for records produced mid-tick.

Decision: reject. The ticket requests a boundary, not a snapshot or performance
rewrite, and unchanged characterization is the minimum behavioral constraint.

## Option 2: generic callback-based scanner

This approach would introduce a helper that accepts suffixes and a closure. The
helper would scan, delete, and invoke the closure with a path or body.

Advantages:

- Small code change.
- Removes repeated `read_dir` boilerplate.
- The caller could customize parsing through closures.

Disadvantages:

- Closures would keep the payload contract implicit.
- Lease JSON, provider JSON, and presence would be arbitrary callback choices.
- Deletion policy would be configured procedurally rather than represented in
  types.
- The result would be a filesystem iteration helper, not a typed signal boundary.
- Provider uniformity mistakes would remain easy because any consumer could read
  or parse any body.

Decision: reject. It does not satisfy the intent of typed signal records.

## Option 3: one record enum plus one request enum

This approach introduces a focused module with:

- a request enum describing the logical scan requested by a consumer;
- a record enum describing the typed records yielded by that scan;
- one ingestion function that performs directory access, recognition, payload
  reading/deserialization, and one-shot deletion.

Advantages:

- Every consumer crosses one named boundary.
- Record variants make payload distinctions explicit.
- The request preserves the existing one-scan-per-consumer scheduling shape.
- Transition can remain one request producing stopped and cleared variants.
- Idle targets can distinguish pane and legacy ticket identity.
- Lease parsing is centralized without moving current-lease admission.
- Delete-before-admission stays inside the filesystem boundary.

Disadvantages:

- A caller receives an enum and must select the expected variants.
- Request and record enums have a deliberately constrained pairing that Rust's
  type system does not fully encode at the return type.
- Adding a new signal requires updating both request recognition and record
  variants.

Decision: choose. The pairing is small and exhaustively implemented in one
module, while downstream matches make unexpected variants harmless and visible.

## Option 4: specialized typed methods on an ingestor

This approach would expose methods such as `leases(suffix)`, `payloads(suffix)`,
`presence(suffix)`, `idle()`, and `transitions()` returning separate record
types.

Advantages:

- Each method has a precise return type.
- Consumers cannot accidentally match a record from another family.
- Lease and provider payload differences are strongly expressed.

Disadvantages:

- Suffix parameters can accidentally grant the wrong payload policy to a new
  signal kind.
- Hard-coded methods avoid that risk but create several public boundary entry
  points rather than one uniform ingestion operation.
- The common recognition and deletion logic still needs internal branching.
- The method surface obscures the complete supported signal protocol compared
  with one exhaustive request enum.

Decision: viable but not selected. Exhaustive request and record enums better
document the protocol as a closed set while retaining typed payload variants.

## Chosen boundary

Add a private `signal` module to `lisa-plugin` and expose crate-private types:

```text
SignalRequest
  Heartbeats
  ProcessStarts
  ShellReady
  CodexAcknowledgements
  Awaiting
  Idle
  Transitions
  Errors

SignalRecord
  Heartbeat { pane_id, lease }
  ProcessStarted { pane_id, lease }
  ShellReady { pane_id, lease }
  CodexAcknowledgement { pane_id, payload }
  Awaiting { pane_id }
  Idle { target }
  Stopped { pane_id }
  Cleared { pane_id }
  Error { pane_id }

IdleTarget
  Pane(u32)
  LegacyTicket(TicketId)
```

The single operation is conceptually:

```text
ingest(signal_dir, request) -> Vec<SignalRecord>
```

An empty vector represents a missing/unreadable directory, no matches, or only
recognized records whose required payload could not be read or parsed.

## Payload policy

### Lease records

- Heartbeat, process start, and shell ready deserialize directly into
  `AttemptLease`.
- A typed lease record exists only if the body was readable UTF-8 and valid JSON.
- The ingestion boundary does not inspect slots, current leases, seat state, or
  generations beyond deserialization.
- Heartbeat retains explicit slot/current-lease admission in its consumer.
- Process start and shell ready retain their existing acknowledgement methods.

### Provider payload records

- Codex acknowledgement produces a raw `String` payload record.
- The ingestion module does not import or call Codex acknowledgement parsing.
- This keeps native provider wire syntax separate from provider-neutral leases.
- The existing downstream acknowledgement method remains the authority.

### Presence records

- Awaiting, stopped, cleared, and error records contain only pane identity.
- Their file bodies are never read.
- This makes it impossible for consumers to accidentally rely on body text.
- The stopped/cleared distinction is represented by separate enum variants.

### Idle records

- Idle carries an `IdleTarget`, not a body.
- Pane targets carry only the parsed pane ID.
- Legacy targets carry the full filename stem as a ticket ID.
- The consumer remains responsible for slot transition checks and ticket lookup.

## Filename recognition and deletion policy

Recognition must reproduce current observable behavior exactly.

### Strict pane-first recognition

- Heartbeat, process start, shell ready, acknowledgement, awaiting, and error
  require a fully valid `pane-<u32>.<suffix>` filename before deletion.
- Names with invalid pane numbers remain untouched.
- Once recognized, the file is deleted regardless of payload read/parse success.

### Broad transition recognition

- Transition requires UTF-8, the `pane-` prefix, and either `.stopped` or
  `.cleared` suffix.
- It deletes the file before parsing the pane ID.
- A malformed pane number produces no typed record but remains consumed.
- Both transition kinds are recognized during the same directory scan.

### Broad idle recognition

- Idle recognizes every UTF-8 filename ending in `.idle`.
- It deletes before pane parsing or target admission.
- A `pane-` stem with an invalid number produces no typed record.
- A non-pane stem becomes a legacy ticket target.

## Consumer responsibilities after refactor

- `check_heartbeat_signals` iterates typed heartbeat records and performs exact
  current-attempt admission and activity/gate effects.
- `check_process_start_signals` dispatches typed lease records.
- `check_shell_ready_signals` dispatches typed lease records with current time.
- `check_codex_ack_signals` dispatches raw payload records and applies existing
  ownership/activity effects.
- `check_awaiting_signals` applies presence effects only.
- `check_idle_signals` retains alert clearing and all phase/artifact logic; only
  filename/delete/target parsing moves.
- `check_transition_signals` applies activity and state-machine dispatch based on
  typed stopped/cleared variants.
- `check_error_signals` retains recovery and thread-authority logic.

## Poll order decision

- Keep every `self.check_*_signals()` call exactly where it is.
- Do not introduce a top-level batch.
- Each check method calls `signal::ingest` when reached.
- Existing non-signal work remains interleaved exactly as before.
- The source-order characterization remains unchanged and meaningful.

## Module visibility

- The module is private to `lisa-plugin`.
- Boundary types and function are `pub(crate)` only as needed by `lib.rs`.
- No `lisa-core` public API is introduced.
- No adapter API changes are needed.
- The boundary consumes `AttemptLease` and `TicketId` from `lisa-core` because
  those are already shared domain types.

## Testing strategy

- Keep the T-039-02-01 characterization module byte-for-byte unchanged.
- Run its focused filter immediately after implementation.
- Add unit tests in the new module for record typing and subtle deletion rules.
- Cover lease parsing versus malformed payload consumption.
- Cover raw provider payload preservation.
- Cover presence records ignoring unreadable/arbitrary bodies by not reading.
- Cover idle legacy target and malformed pane deletion.
- Cover transition combined scan and malformed pane deletion.
- Retain existing `lib.rs` consumer tests as downstream state-effect coverage.
- Run workspace tests and Clippy with warnings denied.

## Risks and mitigations

- Risk: flattening all payloads into strings.
  Mitigation: distinct enum fields and lease deserialization at ingestion.
- Risk: treating tagged Codex JSON as an `AttemptLease`.
  Mitigation: a dedicated raw provider payload variant.
- Risk: moving current-lease admission into filesystem code.
  Mitigation: ingestion has no access to `State`.
- Risk: deleting malformed pane names that strict consumers previously retained.
  Mitigation: explicit strict versus broad recognition helpers and unit tests.
- Risk: losing legacy idle support.
  Mitigation: `IdleTarget::LegacyTicket` and characterization coverage.
- Risk: changing poll timing.
  Mitigation: preserve eight calls and eight on-demand scans.
- Risk: accidentally consuming `.idle` in transition.
  Mitigation: request-specific recognition and the unchanged characterization.

## Decision summary

Use one private, typed, request-driven ingestion function. Centralize directory
iteration, filename recognition, payload acquisition, deserialization, and
one-shot deletion. Keep scheduler admission and all state effects in the eight
existing consumers. Preserve each consumer's call site and timing. Encode the
provider and compatibility asymmetries in record variants and recognition
branches so the code documents why signals are not uniformly interchangeable.
