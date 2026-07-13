# Review: Durable completion journal reconstruction

## Disposition

Pass.

The acceptance criterion is satisfied.

Requested, CommandInFlight, Rejected, and Confirmed completion transitions are
now persisted in a dedicated atomic JSONL journal and reconstructed before the
plugin builds its startup DAG.

The restart test proves Requested, CommandInFlight, and Confirmed recovery.

Provenance records and readers were not changed.

## Source commits

Primary implementation:

`5e6df88b5d1f984a7d61104d238f0ed48ddf3f4b`

`feat(plugin): persist completion aggregate journal`

This commit contains exactly:

- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Post-review scheduling fence:

`5ce1474b4ca67473fd14a258a18ff31aae0c9732`

`fix(plugin): fence reconstructed completion scheduling`

This commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`.

Both commits were created through `lisa commit-ticket` with exact include
paths.

The ordinary Git index is empty.

No ticket-owned source file remains modified, staged, or untracked.

## Files changed

### `crates/lisa-plugin/src/completion_journal.rs`

This new module owns the persistence adapter.

It defines a versioned JSONL record schema for completion transitions.

Each record carries the complete generation identity:

- completion/ticket ID;
- attempt ID;
- generation number.

Requested records also retain the prior ticket phase and status.

CommandInFlight records retain the exact correlation ID.

Confirmed records retain the matching correlation and verified commit ID.

Rejected records retain the failure reason, retryability, and correlation when
one exists.

The loader requires a complete newline-terminated journal.

Malformed JSON, empty records, unknown schema versions, illegal transition
order, key mismatches, and correlation mismatches fail closed.

Missing journal files reconstruct as an empty aggregate set.

Folding delegates transition legality to `lisa_core::completion::reduce`.

Logical append first loads and validates all existing bytes.

It validates the proposed record against the reconstructed aggregate.

It then publishes the complete extended history through the existing
`publication::RustPublication` sibling-temporary and rename contract.

Readers therefore observe either the prior complete journal or the new complete
journal, never a partially appended terminal line.

Accepted historical records are never edited or removed.

### `crates/lisa-plugin/src/lib.rs`

Plugin state now carries:

- the completion journal path;
- journal health;
- reconstructed aggregates.

Production startup loads the journal before initial DAG construction.

Requested and CommandInFlight aggregates mask durable Done frontmatter back to
their recorded prior phase/status.

This prevents an unverified ticket commit from becoming scheduler truth after
a plugin restart.

The scheduler also explicitly fences reconstructed unresolved aggregates.

That second guard is necessary because Review is normally a startable DAG
state even after Done has been masked.

A malformed journal blocks scheduling and all-done termination.

The adapter persists Requested before it accepts responsibility for launching
the completion command.

It persists CommandInFlight before the host command is launched.

Launch and command failures persist Rejected before ephemeral pending state is
cleared.

Successful command handling scans the raw ticket bytes for authoritative Done.

Only after that verification does it persist Confirmed.

Provenance publication, seat release, and dependent scheduling remain after
confirmation.

`PendingCompletion` retains the exact generation key and correlation, keeping
live result attribution aligned with the durable aggregate.

Reconciliation state prefers the reconstructed durable aggregate over legacy
ephemeral facts.

## Acceptance mapping

### Atomically journal Requested

The completion request gateway appends Requested through atomic whole-history
publication before command launch begins.

The record includes the exact generation and the pre-Done phase/status needed
for restart masking.

### Atomically journal CommandInFlight

The adapter appends CommandInFlight with its correlation before invoking the
host command.

The reducer requires the same generation key and a legal Requested predecessor.

### Atomically journal Confirmed

The result path first validates key and correlation and verifies raw durable
Done frontmatter.

It then appends Confirmed with correlation and commit ID before provenance,
release, or rescheduling.

### Reconstruct the same aggregate after restart

`completion_journal_reconstructs_restart_states_before_authoritative_provenance`
constructs fresh plugin states from persisted journal bytes.

It verifies Requested reconstruction.

It verifies CommandInFlight reconstruction with exact correlation.

It verifies Confirmed reconstruction with exact commit identity.

It verifies unresolved Done is masked to its prior Review state.

It installs the reconstructed DAG and verifies unresolved completion cannot
schedule replacement work.

It verifies confirmation removes the mask and permits normal durable Done
reconciliation.

### Keep provenance backward-compatible

The completion journal is a separate `.lisa/completion-journal.jsonl` file.

No provenance schema, record enum, serializer, parser, or query was modified.

The acceptance test also checks that an existing execution provenance record
remains readable and retains its original variant.

## Test coverage

The journal module has focused tests for:

- Requested, CommandInFlight, and Confirmed restart reconstruction;
- malformed JSON;
- a torn non-newline-terminated tail;
- empty records;
- unknown schema versions;
- invalid generation keys;
- invalid correlations;
- invalid transition ordering;
- retryable rejection followed by a new request generation;
- reset followed by a different attempt key;
- failed validation preserving the prior journal bytes.

The plugin integration test covers the full restart boundary and provenance
compatibility.

The post-review assertion covers the otherwise subtle Review scheduling edge.

Final verification after the last source change:

- `cargo test -p lisa-plugin --lib --no-fail-fast`: 358 passed;
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`: passed;
- `cargo test --workspace --no-fail-fast`: passed, including all 358 plugin
  tests and all doc tests.

Earlier verification on the primary implementation also passed `just check`,
including the workspace suite and WASM check.

The existing real-Zellij integration remains ignored under its declared
environment requirement; this ticket does not change that boundary.

## Failure behavior

The implementation intentionally fails closed when journal truth is ambiguous.

An invalid journal is surfaced as unhealthy plugin state.

It does not silently discard a torn last record.

It does not infer confirmation merely from a reachable commit.

It does not schedule tickets or report all-done while the journal is unhealthy.

An append failure leaves the previous destination bytes intact and keeps
completion authority from advancing ephemerally past durable truth.

Rejected is journalled even though the acceptance criterion names three states.

That additional state prevents a failed command from reconstructing forever as
CommandInFlight and preserves retry semantics across restart.

## Open concerns and limitations

Whole-history append is O(journal size) because atomic publication replaces the
complete file.

Completion transitions are low volume, so this is acceptable for the current
scope; compaction or immutable segmenting can be considered if the journal
becomes large.

The design assumes the plugin event loop is the sole journal writer.

Two independent plugin processes writing the same project could race the
read-fold-publish sequence.

Multi-process serialization was not part of this ticket and remains an
operational constraint.

The journal reconstructs completion authority, masking, and scheduling fences.

It does not reconstruct panes, threads, or leases, which have separate recovery
contracts.

Bounded replay and lost-result deadline policy belong to T-042-02-03.

No critical defect, acceptance gap, or human-blocking issue remains for this
ticket.

## Repository handoff

Lisa-managed changes to `.lisa/provenance.jsonl`, ticket frontmatter, and
admitted work artifacts were deliberately excluded from source commits.

The unrelated untracked `crates/lisa-plugin/docs/` tree was not touched.

Concurrent T-042-01-07 changes were committed independently before the final
scheduling fence was reapplied and verified.

This ticket is ready for Lisa's Review admission and completion transaction.

The agent must remain on T-042-02-02 until Lisa confirms that transaction.
