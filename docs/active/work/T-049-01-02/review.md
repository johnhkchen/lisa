# Review — T-049-01-02 seal visibility and ledger field

## Disposition

Pass.

The ticket's two acceptance criteria are satisfied.

The completion tier is visible in doctor and status using the required copy.

Every new provenance and completion-journal row carries a typed seal.

Old rows without the field parse and classify as commit-sealed history.

All ticket-owned source changes are committed and the final workspace suite passes.

## User-visible behavior

`lisa doctor` now includes a completion section.

Commit-tier output says:

`completion seal: commit-sealed — finished work lands as history`

Journal-tier output says:

`completion seal: journal-only — finished work is recorded but not undoable`

`lisa status` includes the same line in its configuration summary.

The strings are defined once in `completion_seal.rs`.

The journal wording never names Git.

Configured commit and journal are already runtime tier choices and display directly.

Configured auto uses the existing read-only environment probe.

The inspection path returns only `CompletionSeal`, so `auto` cannot leak to output.

Explicit commit still displays commit when prerequisites are missing, leaving doctor
free to render its complete dependency diagnostics instead of aborting early.

## Provenance changes

`ProvenanceRecord` now carries a public `CompletionSeal`.

`AssignmentTransitionRecord` carries the same field.

`ParkingTransitionRecord` carries the same field.

All three fields are `#[serde(default)]`.

The enum's existing default is commit.

That makes missing fields mean pre-ladder commit-sealed history by construction.

The plugin stamps new rows from `PluginConfig.completion_seal`.

This is the immutable tier pinned by native loop startup and transported in KDL.

No plugin-side environment probe was added.

Unpark records preserve the seal of the durable park record they close.

The provenance schema version is now 6.

The field table, examples, version history, and compatibility rule are documented.

## Completion-journal changes

The common serialized journal envelope now carries `seal`.

This covers requested, command-in-flight, rejected, and confirmed states uniformly.

The plugin supplies its pinned tier to the append boundary.

Replay returns both the typed transition and its typed seal.

`CompletionJournalAggregate` retains that seal after reconstruction.

Rows in one active completion generation must agree on their tier.

A mismatch returns an actionable invalid-history error and leaves bytes unchanged.

The completion-journal schema version is now 2.

The reader accepts schema 1 and schema 2.

Schema-1 rows omit the field and therefore default to commit.

The existing legacy reconciliation-deadline compatibility remains intact.

## Files changed

### CLI

- `crates/lisa-cli/src/completion_seal.rs`
  adds inspection resolution and shared wording;
- `crates/lisa-cli/src/doctor.rs`
  renders the completion section and module fixtures;
- `crates/lisa-cli/src/status.rs`
  renders the shared line in the config summary and adds fixtures;
- `crates/lisa-cli/tests/seal_visibility.rs`
  runs black-box doctor/status fixtures for both tiers.

### Core

- `crates/lisa-core/src/provenance.rs`
  adds defaulted typed fields, advances schema version, and tests compatibility.

### Plugin

- `crates/lisa-plugin/src/completion_journal.rs`
  persists, reconstructs, and validates the tier;
- `crates/lisa-plugin/src/lib.rs`
  propagates the pinned tier through every production audit writer;
- `crates/lisa-plugin/src/ownership.rs`
  updates direct record fixtures for the additive field.

### Documentation

- `docs/knowledge/provenance-ledger.md`
  documents new writes and missing-field semantics.

No files were deleted.

## Test coverage

Core provenance coverage proves:

- new execution rows serialize commit seals;
- new assignment and parking rows serialize journal seals;
- schema-v2 execution rows without the field load as commit;
- schema-v3 assignment rows without the field load as commit;
- schema-v4 parking rows without the field load as commit;
- mixed ledgers remain distinguishable through the untagged reader.

Completion-journal coverage proves:

- every new state row uses schema 2 and carries its seal;
- schema-1 missing-field histories replay as commit;
- the aggregate exposes the reconstructed tier;
- mixed tiers in one generation fail before publication;
- malformed, torn, empty, unknown-version, and invalid-order histories still fail.

CLI coverage proves:

- both exact strings match the ticket byte-for-byte;
- the journal line contains no `git` spelling;
- doctor output assembly includes both variants;
- status output assembly includes both variants;
- a built-binary fixture invokes both commands for commit and journal projects.

## Verification results

Focused provenance suite: 17 passed.

Focused completion-journal suite: 7 passed.

Focused completion-seal suite: 5 passed.

Focused doctor suite: 50 passed.

Focused status selection: 13 passed.

Black-box seal-visibility fixture: 1 passed.

`cargo fmt --all -- --check`: passed.

`cargo test --workspace --quiet`: passed after the final source commit.

No warnings or failures remain from ticket-owned code.

## Commit record

- `ca51ad4` — Label provenance with completion seals;
- `503acd7` — Carry seals through plugin audit rows;
- `6a1f757` — Show completion seals in doctor and status;
- `78049aa` — Exercise seal visibility through CLI fixtures;
- `94a6081` — Version additive seal audit schemas.

Every commit was produced through `lisa commit-ticket` with exact include paths.

## Worktree review

All ticket-owned source and test paths are clean.

The ordinary index has no staged ticket-owned paths.

The remaining worktree changes are Lisa-managed ledgers, ticket transitions,
published work artifacts, and concurrent-ticket state.

They were neither included in nor altered by this ticket's isolated commits.

Concurrent repository commits landed during the attempt; review used the five
ticket-specific hashes rather than treating the intervening commits as owned work.

## Open concerns

None blocking.

Doctor continues its pre-existing unconditional Git dependency diagnostic even
for journal mode. T-049-01-01 deliberately retained that doctor behavior, and the
current ticket changes visibility rather than the dependency policy.

The black-box fixture is Unix-only because it creates executable shell stubs.

The shared formatter and module fixtures remain platform-independent, so copy
coverage is still present on non-Unix targets.

No data migration is required; existing JSONL rows are intentionally unchanged.
