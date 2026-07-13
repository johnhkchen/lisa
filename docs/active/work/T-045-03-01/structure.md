# Structure — T-045-03-01 claim is ownership proof

## Change summary

This ticket modifies the plugin scheduler and its signal characterization tests.
It creates no production module because the existing `signal` boundary and scheduler
state machine already provide the correct component split.

Ticket-owned source paths:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

No source file is deleted.

## `crates/lisa-plugin/src/signal.rs`

### Imports

Import `AssignmentClaim` from `lisa_core::claim` alongside the existing core lease
and ticket types.

This keeps the wire schema single-sourced in `lisa-core`.
The plugin must not declare a local claim struct or manually extract JSON fields.

### `SignalRequest`

Add `Claims` adjacent to assignment-related signal families.
The intended request ordering is lifecycle evidence followed by claims, hook
acknowledgements, and the remaining presence/transition families.

`SignalRequest` remains crate-private and copyable.
No public API changes.

### `SignalRecord`

Add a typed record:

```text
Claim {
    pane_id: u32,
    claim: AssignmentClaim,
}
```

The record carries the trusted routing component from the filename separately from
the self-asserted assignment identity in the body.
The scheduler consumer must compare them through slot authority.

### `ingest_path`

Add the `Claims` match arm.
It recognizes the exact `.claim` suffix using
`pane_id_from_signal_filename`.

After a valid filename is recognized:

1. read the complete body as text;
2. deserialize `AssignmentClaim`;
3. remove the path unconditionally;
4. return a typed record only when both read and parse succeed.

The implementation can use a small generic JSON helper if that materially reduces
duplication, but should avoid broad refactoring of the existing raw provider path.
Claim JSON is typed; provider hook JSON remains intentionally raw.

### Signal unit tests

Extend the existing typed/raw/presence distinction test or add a focused claim test.
Assertions:

- exact `pane-7.claim` yields the typed claim record;
- the path is removed;
- malformed JSON under a valid filename yields no record and is removed;
- invalid pane names remain untouched.

The test uses a nonce above ordinary small values so the `u128` type is exercised at
the plugin boundary, while core retains maximum-value round-trip coverage.

## `crates/lisa-plugin/src/lib.rs`

### Core import

Add `AssignmentClaim` to the plugin's core imports.
Keep `AttemptLease`, ticket types, and other imports organized according to existing
formatting.

### Scheduler admission method

Add a private method near `acknowledge_codex_assignment`:

```text
fn admit_assignment_claim(
    &mut self,
    pane_id: u32,
    claim: &AssignmentClaim,
) -> bool
```

Responsibilities:

- require the pane to be in an active unowned assignment state;
- resolve slot ticket and slot attempt lease;
- compare claim ticket and attempt with state and slot;
- require exact current lease authority;
- resolve and compare the retained `AssignmentRef` lease and nonce;
- insert `Owned` only after every check passes;
- return whether the transition occurred.

Non-responsibilities:

- reading or deleting signal files;
- parsing JSON;
- logging;
- bumping activity;
- timeout selection;
- hook evidence interpretation;
- filesystem assignment lookup.

The method borrows data long enough to evaluate equality, then mutates
`seat_assignments` only at the end.
No partial state mutation occurs on rejection.

### Claim signal consumer

Add a private method near other signal consumers:

```text
fn check_claim_signals(&mut self)
```

It requests `SignalRequest::Claims`, matches `SignalRecord::Claim`, and calls
`admit_assignment_claim`.

On success it:

- calls `bump_pane_activity`;
- logs one `ActivityEvent::Info` containing pane ID and wording that identifies a
  claim rather than a hook acknowledgement.

On rejection it performs no scheduler side effect beyond the ingestion layer's
one-shot file consumption.

### Poll integration

Insert:

```text
self.check_claim_signals();
```

after shell-ready ingestion and before Codex acknowledgement ingestion.
Update the nearby comment so it describes claim ownership separately from the
existing provider prompt signal.

The remaining poll sequence is unchanged.
Claim processing remains before assignment acknowledgement timeout evaluation.

### Lifecycle cleanup

Add `claim` to the suffix array in `clear_pane_lifecycle_signals`.
This is best-effort residue cleanup only; exact admission remains the safety fence.

### Scheduler acceptance test

Add one high-level test near the existing fresh-dispatch ownership tests.
Suggested name:

`delivered_assignment_becomes_owned_on_exact_claim_without_hook`

Fixture structure:

- create a provider scheduling state and signal directory;
- call `schedule_ready_tickets`;
- obtain the current lease and retained assignment reference;
- publish exact process-start evidence;
- observe `ReadyForAssignment`;
- call `deliver_ready_assignments`;
- observe `Delivering`, `seat_is_owned == false`, and dashboard `delivering`;
- assert no `.ack` file exists;
- write a wrong-nonce claim and run the claim consumer;
- assert the claim is consumed and the seat remains delivered/unowned;
- write the exact claim and run the claim consumer;
- assert the claim is consumed, state is `Owned`, and dashboard says `owned`;
- assert the activity log records the claim transition.

Using a fresh Codex fixture matches the incident motivating E-045.
The admission implementation itself remains provider-neutral because it does not
inspect the slot's client or parse provider payloads.

### Focused method tests

If the high-level acceptance test does not make stale lease and wrong-pane rejection
clear enough, add a table-style unit test directly around `admit_assignment_claim`.
Avoid duplicating every CLI producer rejection because final scheduler checks are the
target here.

Minimum negative identities:

- wrong nonce;
- stale or wrong attempt;
- wrong ticket/pane route;
- missing retained assignment reference.

The exact claim must succeed only after the negative cases leave state unchanged.

## `signal_consumer_characterization.rs`

### Poll-order characterization

Update the expected source call sequence to include:

```text
self.check_claim_signals();
```

between shell readiness and Codex acknowledgement.
Rename the test if its fixed consumer count is embedded in the name.

### Recognized one-shot cases

Add `("claim", "pane-7.claim", "not-json")` to the recognized-record cases.
Dispatch that case through `state.check_claim_signals()`.
The assertion proves a malformed body under a recognized strict filename is deleted
before admission.

### Legacy filename cases

Add `("claim", "T-LEGACY.claim")` to the legacy-name matrix.
Dispatch through the claim consumer.
Assert it remains because idle alone admits legacy ticket-addressed filenames.

### Claim consumer behavior

Add a focused state fixture containing:

- current attempt authority;
- slot ticket and attempt lease;
- a `Delivering` assignment state;
- an exact retained `AssignmentRef`.

Write a wrong claim first and verify:

- file consumed;
- not owned;
- no activity clock bump.

Write the exact claim next and verify:

- file consumed;
- state `Owned`;
- activity clock bumped;
- claim-specific activity log present.

The assignment reference can be constructed directly in this characterization test;
the atomic writer is already independently covered.

## `signal_ingestion_regression.rs`

### Typed record contract

Extend `every_request_produces_its_exact_typed_record_contract` with one claim file,
request, expected record, and deletion assertion.
Renumbering later fixture pane IDs is optional because files are consumed between
independent requests.

### Recognition policy

Add a malformed exact claim body assertion if not fully covered by `signal.rs`.
Add an invalid claim filename assertion only if it improves the strict-family matrix
without duplicating an identical generic parser check.

### Poll interleaving

Insert the claim consumer call between shell-ready and hook acknowledgement in the
expected ordered operations.

## Internal dependency direction

```text
lisa-core::claim::AssignmentClaim
             ↓
plugin signal ingestion (filename + JSON shape)
             ↓
plugin scheduler admission (pane + state + lease + retained nonce)
             ↓
SeatAssignmentState::Owned
             ↓
existing UI reduction and activity output
```

The scheduler never depends on the CLI crate.
The CLI and plugin independently depend on the shared core wire type.

## Mutation ordering

The only authoritative mutation is the final insertion of `Owned`.
All comparisons precede it.
Activity clocks and logs follow successful insertion.

Signal deletion occurs earlier at ingestion by existing one-shot convention.
This means an operationally malformed or semantically stale claim does not retry
automatically.
The agent can issue a corrected claim, producing a new complete file.

## Compatibility boundaries

- `AssignmentClaim` JSON remains unchanged.
- pane claim filename remains unchanged from the CLI producer.
- `AttemptLease` serialization remains unchanged.
- `SeatAssignmentState::Owned` remains the sole owned truth.
- UI types and labels remain unchanged.
- existing hook files and templates remain unchanged.
- CLI command syntax and output remain unchanged.
- no artifact path or publication behavior changes.

## Commit boundary

The four plugin paths form one meaningful source unit:

- signal record support;
- scheduler admission;
- consumer ordering;
- acceptance and regression coverage.

Commit them together through:

```text
lisa commit-ticket --ticket-id T-045-03-01 \
  --message "feat(plugin): own assignments from exact claims" \
  --include crates/lisa-plugin/src/signal.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/tests/signal_consumer_characterization.rs \
  --include crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
```

The exact command may omit a test path if implementation demonstrates it needs no
change, but it must never include unrelated workflow or runtime files.

## Completion shape

After implementation:

- every ticket-owned source path is committed and clean;
- phase artifacts remain private to the attempt directory;
- `progress.md` records implementation, verification, and deviations;
- `review.md` and exact pass/block JSON complete Review;
- ticket frontmatter remains untouched.
