# Progress: Durable completion journal reconstruction

## Status

Implementation is complete and verified.

The completion adapter now persists its aggregate as a versioned append-only
JSONL journal at `.lisa/completion-journal.jsonl`.

Requested, CommandInFlight, Rejected, and Confirmed transitions are validated
against the pure core reducer before they become durable.

Each logical append republishes the complete old history plus one new compact
record through `publication.rs`'s nonce-named sibling temporary and atomic
rename.

A fresh plugin State reconstructs typed aggregate state from the journal before
initial DAG construction.

## Completed: journal module

Created `crates/lisa-plugin/src/completion_journal.rs`.

The module defines schema version 1.

Every record carries the completion generation identity as explicit fields:

- completion ID;
- attempt ID;
- numeric generation.

Requested additionally stores prior Phase and TicketStatus.

CommandInFlight stores the exact CorrelationId.

Confirmed stores the matching correlation plus verified commit ID.

Rejected stores optional correlation, reason text, and retryability.

The runtime transition and aggregate types use lisa-core domain identities.

The private serde schema is isolated inside the plugin persistence adapter.

No parser was added for CompletionGenerationId Display.

## Completed: strict reconstruction

Missing journal files reconstruct to an empty aggregate map.

Non-empty files must end with a newline.

Empty interior lines are rejected.

Malformed JSON is rejected with a 1-based line number.

Unknown schema versions are rejected explicitly.

Every parsed line is folded in file order.

No malformed or unsupported line is skipped.

The fold uses `lisa_core::completion::reduce` for legal transition semantics.

Generation-key mismatch and correlation mismatch are both fail-closed.

Repeated transitions after pending/confirmed state are rejected.

Retryable rejection can accept a later Request.

A different attempt key can begin a fresh aggregate after a terminal confirmed
ticket is reset.

## Completed: atomic logical append

Append reads and validates the entire existing history.

It validates the proposed transition before changing destination bytes.

It serializes exactly one compact JSON object plus newline.

It creates the parent directory if necessary.

It uses RustPublication with a Nonce temporary policy.

The temporary prefix is one fixed sibling filename component.

The prior destination remains intact when validation fails.

Successful publication returns the exact post-transition aggregate.

The plugin updates its in-memory aggregate only after publication succeeds.

## Completed: State reconstruction

Added State fields for:

- completion journal path;
- journal health;
- reconstructed completion aggregate map.

Production load sets `/host/.lisa/completion-journal.jsonl`.

Load reconstructs the journal before scanning tickets into the initial DAG.

Malformed journal restore logs an operator-visible error and marks durability
unhealthy.

An unhealthy non-empty journal blocks new scheduling and all-done termination.

New completion transitions also refuse to run while journal health is false.

Empty journal paths remain no-op durability for legacy pre-load native tests.

## Completed: durable aggregate truth

`reconciliation_state` consults reconstructed aggregate state before the live
pending map or DAG fallback.

It can now return exact correlation-bearing CommandInFlight after restart.

Requested and CommandInFlight persistently suppress duplicate request effects.

Confirmed suppresses duplicate completion while the ticket remains durably
Done.

A reset non-Done ticket can become Eligible for a different attempt key.

PendingCompletion now retains exact CompletionGenerationId and CorrelationId.

Result handling no longer recomputes identity from ticket-only command context.

## Completed: restart-safe DAG masking

Factored completion transaction masking into one State helper.

Live PendingCompletion has first precedence.

Without a live pending record, reconstructed Requested or CommandInFlight state
supplies prior phase/status.

The helper is used by normal `rebuild_dag`.

The same helper masks startup scan results after journal reconstruction.

A Done ticket written by an in-flight completion command therefore remains
Review/non-Done after plugin restart.

Confirmed stops masking and exposes durable Done.

No ticket bytes are changed by masking.

## Completed: request/launch ordering

Existing effect identity, current lease, dependency, and ticket path validation
remain before command acceptance.

The command builder is validated before persistence in production.

Requested is atomically journalled with prior phase/status.

CommandInFlight is atomically journalled with the generation-derived
correlation.

Only after both transitions succeed does the adapter insert
PendingCompletion.

Only after pending insertion does it call the Zellij host command API.

Any journal failure becomes a structured LaunchFailed rejection and prevents
the host effect.

The existing native inert-executor behavior remains for tests with an unset
journal path.

## Completed: result ordering

Nonzero exit and invalid commit output journal retryable Rejected before pending
state is removed.

Stale-authority results also durably terminate the old in-flight aggregate
before live pending removal.

If Rejected publication fails, the adapter retains pending state and the DAG
mask.

Successful command output must still be a 40- or 64-character hexadecimal
commit ID.

Durable Done verification now scans raw ticket files directly.

This was necessary because the restart-safe journal mask intentionally keeps
the normal DAG at the prior phase while CommandInFlight.

Confirmed is journalled with exact correlation and commit ID.

Only after Confirmed publication succeeds does the adapter remove pending,
rebuild unmasked Done, log completion, emit provenance, release the seat, and
schedule dependents.

Confirmation persistence failure leaves pending and masking intact and emits no
authoritative success effects.

## Completed: provenance compatibility

No `lisa-core::provenance` type or schema version changed.

No completion row was added to `.lisa/provenance.jsonl`.

The acceptance fixture parses the post-confirmation row through the existing
`ProvenanceLedgerRecord::Execution` shape.

It asserts outcome Done and authoritative true.

Existing schema-v2 execution plus schema-v3 assignment-transition compatibility
tests pass unchanged.

## Completed: restart acceptance regression

Added
`completion_journal_reconstructs_restart_states_before_authoritative_provenance`.

The test configures a real temporary ticket/work tree, current AttemptLease,
completion command builder, journal, and provenance path.

It drives the production artifact completion adapter.

It observes exactly one Requested and one CommandInFlight record.

It creates a fresh State and uses the same restore helper invoked by load.

The fresh aggregate equals the original typed in-flight aggregate, including:

- generation key;
- correlation;
- prior phase;
- prior status.

It changes ticket bytes to Done and proves the restarted journal mask retains
Review.

It delivers a valid correlated command result.

It creates another fresh State and reconstructs Confirmed plus the exact commit
ID.

It asserts exactly three accepted-state records and no publication temporary
residue.

It asserts no provenance exists in-flight and one backward-compatible
authoritative execution record exists after Confirmed.

## Focused test results

Passed:

- five journal-module tests;
- one plugin restart/provenance integration test;
- all six tests selected by `completion_journal`;
- full plugin native suite: 354 passed;
- completion single-gateway structural regression within that suite;
- existing completion result success/failure regressions;
- existing level-triggered reconciliation regression;
- existing provenance compatibility regressions.

No focused test is ignored.

## Lint and target results

Passed:

- `cargo fmt --all -- --check`;
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `cargo check -p lisa-plugin --target wasm32-wasip1` through `just check`.

## Workspace verification

Passed `cargo test --workspace --no-fail-fast`.

Observed totals include:

- CLI library transaction tests: 14 passed;
- CLI binary tests: 267 passed;
- CLI integration suites: 1 atomic provider, 3 help, and 1 preownership passed;
- core unit tests: 195 passed;
- core generated completion ordering regression: 1 passed;
- recorded livelock regression: 1 passed;
- plugin native tests: 354 passed;
- doc tests: passed with zero cases.

The real-Zellij delivery boundary remains ignored under its existing declared
environment requirement.

Passed `just check`, including WASM check and the workspace suite.

## Deviations from Plan

The concurrent T-042-01-03 `lib.rs` work completed before adapter integration.

Its source commit was `27bddc1`, followed by completion commit `a10feeb`.

This ticket integrated only after `lib.rs` became clean, so no concurrent hunk
was consumed.

The first restart test exposed that normal `rebuild_dag` could no longer be used
to verify raw Done once the durable journal mask existed.

Result verification was adjusted to scan raw ticket files before Confirmed,
then rebuild normally after confirmation.

This deviation strengthens the intended mask rather than weakening it.

Rejected was included in the journal in addition to the three acceptance states
so command failure remains retryable across restart instead of reconstructing
forever as CommandInFlight.

A reset/new-attempt fold case was added to preserve existing ticket reset
behavior for different attempt keys.

## Repository ownership

Ticket-owned source paths are exactly:

- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`.

The ordinary Git index is empty.

Lisa-managed `.lisa/provenance.jsonl` and active ticket changes remain excluded.

Shared admitted work artifacts remain excluded.

The unrelated untracked `crates/lisa-plugin/docs/` tree remains untouched.

Concurrent T-042-01-07 work artifacts and ticket state remain untouched.

## Remaining

Primary source commit completed through `lisa commit-ticket`:

`5e6df88b5d1f984a7d61104d238f0ed48ddf3f4b`

Message:

`feat(plugin): persist completion aggregate journal`

It contains exactly:

- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Post-commit review identified one additional restart fence: a reconstructed
Requested/CommandInFlight ticket is intentionally masked to Review, and Review
is normally startable in the DAG. Scheduling must skip that aggregate until
reconciliation resolves it.

A focused scheduling skip and assertion were prepared and passed the exact
restart test plus WASM Clippy.

Concurrent T-042-01-07 then resumed its own `lib.rs` test work after the primary
commit. This ticket withdrew only its uncommitted scheduling-fence hunks so
neither isolated transaction can consume the other's source.

T-042-01-07 committed its isolated tests as `ec6ae05` and completed as
`604233d`. The scheduling fence was then reapplied on that clean admitted
baseline; its diff contains only the 20 ticket-owned lines described above.

Final verification after that reapplication passed:

- all 358 `lisa-plugin` tests;
- native `lisa-plugin` Clippy with warnings denied;
- WASM `lisa-plugin` Clippy with warnings denied;
- the full workspace test suite, including 358 plugin tests and all doc tests.

The follow-up was committed through `lisa commit-ticket` as:

`5ce1474b4ca67473fd14a258a18ff31aae0c9732`

Message:

`fix(plugin): fence reconstructed completion scheduling`

It contains exactly `crates/lisa-plugin/src/lib.rs`.

The two Review artifacts are the only remaining attempt-private outputs.

Verify source cleanliness and inspect the committed diff.

Write Review artifacts and remain on T-042-02-02 for Lisa's completion gate.
