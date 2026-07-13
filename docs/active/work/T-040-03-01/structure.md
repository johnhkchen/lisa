# Structure: blocking Review regression

## File inventory

### Modified source

`crates/lisa-plugin/src/lib.rs`

Add one test to the existing `#[cfg(test)] mod tests` section near
`review_disposition_gates_artifact_completion_and_dependents`.

No production function, type, public interface, manifest, or dependency is
changed.

### Attempt-private artifacts

The current attempt writes:

- `.lisa/attempts/T-040-03-01/1/work/research.md`
- `.lisa/attempts/T-040-03-01/1/work/design.md`
- `.lisa/attempts/T-040-03-01/1/work/structure.md`
- `.lisa/attempts/T-040-03-01/1/work/plan.md`
- `.lisa/attempts/T-040-03-01/1/work/progress.md`
- `.lisa/attempts/T-040-03-01/1/work/review.md`
- `.lisa/attempts/T-040-03-01/1/work/review-disposition.json`

These are not source transaction inputs. Lisa later admits and publishes them
after verifying the attempt lease.

### Unchanged files

No changes are planned for:

- `crates/lisa-core/src/disposition.rs`; parsing already works;
- `crates/lisa-plugin/src/publication.rs`; commit publication already works;
- `crates/lisa-cli`; no command behavior changes;
- ticket frontmatter; Lisa owns phase and status transitions;
- canonical `docs/active/work/T-040-03-01`; Lisa owns publication.

## Test placement

Place the new test immediately after the generic disposition-gate regression.
This keeps Review authorization coverage together while giving the historical
incident an independent, searchable name.

Proposed test name:

```rust
test_t039_06_02_blocking_review_never_prepares_done
```

The name encodes:

- the source incident (`T-039-06-02`);
- the input condition (blocking Review);
- the prohibited scheduler effect (Done preparation).

## Internal fixture components

The test uses existing test-module imports and helpers where appropriate:

- `tempfile::tempdir` for isolation;
- `lisa_core::ticket::scan_tickets` for ticket parsing;
- `Dag::from_tickets` for dependency state;
- `State`, `PluginConfig`, `AgentSlot`, and `TransitionState` from the module;
- `Thread`, `ThreadStatus`, and `Phase` for runtime state;
- `install_current_attempt` for lease synchronization;
- `write_review_disposition` for attempt-private JSON evidence.

No new reusable helper is needed. The scenario deliberately spells out its
state so a reader can audit the historical boundary in one place.

## Temporary filesystem layout

```text
<temp>/
  tickets/
    T-REVIEW.md
    T-DEPENDENT.md
  work/
    T-REVIEW/
      review.md                    # admitted during poll
      review-disposition.json      # admitted during gate
  attempts/                        # helper fallback/private lease storage
  provenance.jsonl                 # must never be created
```

The exact attempt-private path is derived by `State::attempt_work_dir`; the
test does not hard-code its layout.

## Ticket fixture boundary

`T-REVIEW.md` represents the assigned ticket:

```yaml
id: T-REVIEW
status: review
phase: review
```

`T-DEPENDENT.md` represents downstream work:

```yaml
id: T-DEPENDENT
status: open
phase: ready
depends_on: [T-REVIEW]
```

The reviewed ticket begins in Review both on disk and in its runtime thread.
The dependent begins ready in phase terminology but cannot run because its
dependency is not Done.

## Runtime state boundary

The state owns:

- the parsed two-ticket DAG;
- configured ticket/work directories;
- a configured temporary ledger path;
- one agent slot assigned to the reviewed ticket;
- one running Review thread;
- one current attempt lease shared by map, thread, and slot.

The dependent has no thread or slot. This makes accidental scheduling visible.

## Artifact inputs

Write `review.md` before polling. This reproduces the historical trigger: the
Review artifact is already present when the scheduler evaluates advancement.

Write the exact valid block JSON contract:

```json
{"disposition":"block","reason":"resolve the hostile review finding"}
```

The reason is nonempty and actionable, ensuring the parser returns
`ReviewDisposition::Block` rather than `Invalid`.

## Production path exercised

The test calls:

```text
check_artifact_advances
  -> admit review.md
  -> request_review_completion
  -> admit review-disposition.json
  -> parse ReviewDisposition::Block
  -> log warning and return
```

It must not reach:

```text
request_completion
  -> pending_completions.insert
  -> native complete-ticket command
  -> successful result publication
  -> emit authoritative Done provenance
  -> release slot / schedule dependent
```

## Assertion organization

Group assertions by contract rather than implementation order.

### Transaction refusal

- `pending_completions` has no reviewed ticket.
- Assertion message states this failed on the pre-gate unconditional path.

### Assignment retention

- thread exists;
- phase is Review;
- status is Running;
- slot ticket remains reviewed ticket;
- slot attempt lease remains installed lease;
- current lease map remains installed lease.

### Durable non-Done state

- ticket file still contains `status: review`;
- ticket file still contains `phase: review`;
- ledger file does not exist.

### Dependency safety

- DAG says dependent dependencies are not all Done;
- dependent thread does not exist.

### Operator evidence

- activity log contains `Completion blocked` and exact reason.

## Ordering

1. Create ticket files and DAG.
2. Construct state with ledger path.
3. Install slot, thread, and attempt lease.
4. Write Review artifacts.
5. Poll artifact advancement.
6. Assert transaction refusal first.
7. Assert assignment and durable state.
8. Assert provenance and dependency state.
9. Assert visible reason.

The transaction-refusal assertion comes first because it is the direct
historical discriminator and gives the clearest failure on a regression.

## Source ownership and transaction

Only `crates/lisa-plugin/src/lib.rs` is ticket-owned source. It is committed as
one meaningful regression-test unit through `lisa commit-ticket` with that
single exact include.

No broad include, ordinary index staging, or ordinary commit is permitted.
Unrelated dirty files remain untouched.
