# Design: T-039-02-01

## Goal

Add a durable characterization seam for the eight filesystem signal consumers
without changing scheduler behavior. The suite must make intentional differences
visible so the next ticket can introduce typed ingestion without accidentally
normalizing provider-specific or lifecycle-specific semantics.

## Option 1: rely on existing tests

The current module already exercises many individual consumer effects. This has
the smallest diff and all current tests pass. It does not meet the ticket well:
coverage is distributed across historical ticket sections, the full poll order
is not asserted, and legacy/deletion distinctions are not stated for every
consumer. A future structural edit would have difficulty identifying which
existing tests form the promised characterization suite.

This option is rejected because implicit aggregate coverage is not an explicit
regression contract.

## Option 2: black-box integration tests

An integration test could instantiate the plugin through public interfaces and
materialize signals on disk. That would resemble production use. The consumers,
`State`, slot state, and relevant transition types are private. Reaching them
through a full Zellij plugin lifecycle would require host simulation, add noise,
and make exact intermediate effects hard to observe. It could also turn a narrow
characterization task into a product testability refactor.

This option is rejected because it conflicts with the no-product-change
constraint and cannot precisely isolate all admission boundaries.

## Option 3: add tests directly to the existing inline module

Tests appended to `lib.rs` can access every private detail and reuse all helpers.
This is mechanically simple. The file is already very large, and a named suite
would be buried among roughly fourteen thousand lines of implementation and
historical tests. The following typed-ingestion ticket should be able to run and
inspect this suite as a distinct unit.

This option is viable but not preferred because it weakens discoverability.

## Option 4: a test-only child module

Add one `mod signal_consumer_characterization;` declaration inside the existing
`#[cfg(test)] mod tests`, backed by
`crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`. A child of
the inline test module retains access to the private parent fixtures and crate
state. Runtime builds do not compile the module. The production implementation
remains byte-for-byte unchanged apart from a test-only module declaration.

This option gives the suite a stable name, a focused file, and direct private
state access without exposing new APIs. It is selected.

## Suite shape

The suite will have three complementary layers.

First, one poll-order test reads the production source with `include_str!` and
asserts monotonically increasing positions for the eight call expressions. The
test records the actual order while tolerating comments and unrelated calls
between consumers. It deliberately checks `poll_tick`'s body rather than merely
listing an expected enum, because the behavior under characterization is the
real dispatch sequence.

Second, a filename/deletion matrix creates current and legacy-shaped records for
each consumer. It invokes each consumer independently and asserts whether the
current and legacy record remains. This captures that idle alone admits the
legacy `{ticket_id}.idle` family, while the seven pane-scoped consumers ignore
ticket-named legacy variants. Rejected current-shape payloads are used where
possible to prove scanner deletion occurs independently of semantic admission.

Third, focused effect tests create the minimum state needed for each consumer's
positive path. Each assertion combines accepted payload, deletion, and effect so
the suite is useful during the upcoming ingestion refactor.

## Payload distinctions to preserve

- Heartbeat, started, and shell-ready bodies are JSON `AttemptLease` values.
- Heartbeat admission is enforced in the scan method.
- Started and shell-ready delegate state admission after parsing.
- Ack is raw UTF-8 at ingestion and provider JSON/tag validation downstream.
- Awaiting, idle, transition, and error are presence signals.
- Presence signal bodies are ignored, including arbitrary text.
- Idle has both pane-current and ticket-legacy identity routes.
- Transition has two suffixes in one consumer.
- Error ownership comes from running threads, not merely slot binding.

## Delete timing interpretation

Filesystem observation cannot pause a private method between unlink and effect.
The stable observable contract is that a recognized file is one-shot even when
payload parsing or downstream admission rejects it. Tests will therefore pair a
recognized filename with malformed/stale/inapplicable content and assert both no
effect and deletion. This distinguishes delete-before-admission from a policy
that retains rejected records for replay.

For presence-only consumers, arbitrary bodies and inapplicable state establish
the same distinction. For the transition consumer, a malformed numeric pane ID
is especially strong evidence because the implementation deletes before pane ID
parsing. For idle, a recognized unknown legacy ticket is deleted before thread
lookup.

## Legacy filename interpretation

“Legacy filename handling” is treated as a cross-consumer dimension, not a claim
that every consumer supports a legacy form. The suite will affirm the positive
legacy idle route and negative behavior for ticket-named variants of all seven
other consumers. Negative legacy records must remain untouched because those
scanners do not recognize them.

## Effect observations

- Heartbeat refreshes clocks and clears awaiting/attention markers.
- Process-start promotes `Starting` to `ReadyForAssignment`.
- Shell-ready accepts only the reset successor and relaunches to `Starting`.
- Ack promotes a pending seat to `Owned` and records activity.
- Awaiting inserts the pane gate without refreshing activity.
- Idle legacy naming produces the current missing-artifact alert for a running
  Research thread while consuming the signal.
- Transition consumes a stopped signal, refreshes activity, and preserves an
  inapplicable idle slot; existing focused tests retain active transition edges.
- Error consumes arbitrary content and reclaims a matching running thread.

Using an inapplicable safe state for transition avoids native Zellij host calls.
The suite still observes its payload-insensitive dispatch and activity effect;
existing tests cover `WaitingForStop` and `WaitingForClear` state transitions.

## Source-change boundary

Only test code is ticket-owned:

- a test-only child module declaration in `crates/lisa-plugin/src/lib.rs`;
- the new characterization file under `crates/lisa-plugin/src/tests/`.

No runtime function, type, constant, configuration, or dependency will change.
Workflow artifacts remain attempt-private and are not included in the source
unit commit.

## Verification

Run the named characterization suite first. Then run plugin tests, workspace
tests, formatting checks, and Clippy with warnings denied. Inspect the diff to
confirm runtime code has no semantic changes. Commit the exact two source paths
through `lisa commit-ticket`.

