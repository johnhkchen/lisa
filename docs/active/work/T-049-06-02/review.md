# Review — T-049-06-02 Notes for you queue

## Disposition

Pass. The implementation satisfies both acceptance criteria and is ready for Lisa's
completion flow.

## Changes reviewed

### Durable queue and acknowledgment model

- Added `lisa_core::notes`, which derives active notes from confirmed completion
  journal records whose review disposition is `note`.
- A note is identified by ticket ID, attempt ID, and generation. An acknowledgment
  suppresses only that exact durable fact, so a later generation can surface a new
  note for the same ticket.
- Added append-only `note-acknowledged` provenance rows and registered them in the
  mixed provenance ledger schema. The schema version advances from 7 to 8.
- Reduction is deterministic, ignores requested/in-flight completion states, and
  tolerates malformed or torn history using the repository's durable-ledger rules.

### Operator CLI and status

- Added `lisa notes` to list active notes and `lisa notes ack <ticket-id>` to record
  explicit acknowledgment.
- Rendering uses a separate `Notes for you (N)` section, leads with the plain summary,
  and places the quoted criterion and evidence citation beneath it.
- Empty queues render no section. Duplicate or nonexistent acknowledgments fail
  without appending another provenance fact.
- `lisa status` keeps urgent `Waiting on you` first, then shows optional Notes for
  you before the DAG. The operator help surface includes the new command.

### Dashboard projection

- Added durable note items to plugin UI state and rendered Notes for you separately
  from Waiting on you, attention, and thread operations.
- The plugin re-reduces journal and provenance state while projecting the dashboard,
  so restart and acknowledgments are reflected without an in-memory-only note path.
- The display order is summary, quoted criterion, then evidence, matching the CLI.

## Acceptance coverage

The rendering criterion is covered by core formatter/UI tests, CLI status tests, and
plugin rendering tests. These assert the dedicated heading, summary-first order,
visible separation from Waiting on you, and absence of empty sections. A plugin
fixture writes a valid requested/in-flight/confirmed journal sequence, constructs a
state, drops it, reloads a fresh state, and verifies that the note survives.

The lifecycle criterion is covered by real-binary CLI tests and core reducer tests.
They verify listing, provenance append on acknowledgment, clearing in a fresh CLI
process, non-resurfacing after restart, duplicate acknowledgment rejection, and a
later generation resurfacing independently. The plugin restart fixture also verifies
the queue clears after acknowledgment.

Zero-effect assertions compare DAG-ready tickets, ticket bytes, and completion
aggregate before and after acknowledgment. They also assert that notes create no
thread, seat, or parking state. The implementation therefore has no scheduling,
capacity, or completion transition side effect.

## Verification

- `cargo test -p lisa-core` — passed (242 unit tests plus integrations).
- `cargo test -p lisa-cli --test notes_ux` — passed (3 black-box tests).
- `cargo test -p lisa-cli --test help_surface` — passed (6 tests).
- `cargo test -p lisa-cli status::` — passed (15 focused tests).
- `cargo test -p lisa-plugin --lib` — passed (437 tests).
- `cargo test --workspace --no-fail-fast` — passed.
- `cargo fmt --all -- --check` — passed.
- `just check` — passed, including the WASM target check and full suite.
- `git diff --check` — passed.

The real-Zellij delivery-boundary test remains ignored because it requires external
Zellij tooling and the WASM target; this is its existing environment gate, not a
ticket regression.

## Commit and ownership audit

- `7bb43ee50ab691da9a38557543064f035df7167e` — core durable queue and provenance
  acknowledgment.
- `479a2f8b6f2a45e1aaffa1f2e64bcdb6842fa48d` — CLI command, status/help rendering,
  and black-box lifecycle tests.
- `406fd9407eb237c085619212ab0ed3d68219cfa6` — dashboard projection/rendering and
  restart/zero-effect fixture.

Each commit was made with `lisa commit-ticket` and exact repository-relative include
paths. Their file lists match the planned source units. No ticket-owned source file
is staged, modified, or untracked. Remaining worktree changes are Lisa-managed
journal/ticket/publication state, including concurrent ticket state, and were kept
outside these commits.

## Open concerns

None blocking. Disputing a note intentionally remains outside this feature: the
operator creates ordinary ticket work, as required, and no automatic mutation is
introduced here.
