# Progress — T-049-01-02 seal visibility and ledger field

## Status

Implementation is complete.

All ticket-owned source changes are committed through `lisa commit-ticket`.

All focused tests and the full workspace suite pass.

No ticket-owned path remains staged, modified, or untracked.

## Completed — provenance schema

Added `seal: CompletionSeal` to terminal execution records.

Added the same field to assignment-transition records.

Added the same field to parking-transition records.

Each field uses `#[serde(default)]`.

Because `CompletionSeal::default()` is commit, old rows classify correctly.

New row serialization includes either `"seal":"commit"` or `"seal":"journal"`.

The provenance writer now stamps `self.config.completion_seal`.

Unpark rows preserve the seal from the corresponding park row.

The provenance schema version advanced from 5 to 6.

The ledger documentation now describes the field and compatibility rule.

## Completed — completion journal

Added a common defaulted seal field to the journal record envelope.

Every requested, in-flight, rejected, and confirmed row now carries the tier.

The plugin adapter passes its pinned configuration seal into journal append.

Replay carries the deserialized seal into `CompletionJournalAggregate`.

Legacy schema-1 rows without a seal reconstruct as commit-sealed.

The journal schema version advanced to 2.

The reader accepts both schema 1 and schema 2.

Transitions in one active completion generation must use one seal.

A mixed-tier transition fails closed before bytes change.

A later generation after a terminal aggregate may adopt a newly configured tier.

## Completed — doctor and status

Added one exhaustive shared plain-language formatter.

Commit copy is:

`completion seal: commit-sealed — finished work lands as history`

Journal copy is:

`completion seal: journal-only — finished work is recorded but not undoable`

The journal line contains no `git` wording.

Doctor prints the line in a dedicated completion section.

Status prints the same line in its config summary.

Explicit commit and journal configurations show their requested runtime tier.

Auto performs the existing read-only environment resolution for inspection.

The state-reading helper cannot emit configured-only `auto` as a seal.

## Completed — fixtures

Core tests assert all new provenance shapes serialize a seal.

Legacy execution, assignment, and parking rows assert default commit classification.

Journal tests assert new schema-2 rows carry their pinned seal.

Journal legacy replay asserts schema-1 missing fields become commit.

Journal mismatch tests assert fail-closed, byte-preserving behavior.

Doctor and status module fixtures assert both exact lines.

A Unix black-box CLI fixture invokes both commands for both tiers.

The fixture extracts the actual output line and compares it byte-for-byte.

## Commits

`ca51ad4` — Label provenance with completion seals.

Includes:

- `crates/lisa-core/src/provenance.rs`;
- `docs/knowledge/provenance-ledger.md`.

`503acd7` — Carry seals through plugin audit rows.

Includes:

- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ownership.rs`.

`6a1f757` — Show completion seals in doctor and status.

Includes:

- `crates/lisa-cli/src/completion_seal.rs`;
- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/status.rs`.

`78049aa` — Exercise seal visibility through CLI fixtures.

Includes `crates/lisa-cli/tests/seal_visibility.rs`.

`94a6081` — Version additive seal audit schemas.

Includes the core schema, journal schema, and ledger documentation.

## Verification

`cargo test -p lisa-core provenance::tests`

Result: 17 passed.

`cargo test -p lisa-plugin completion_journal::tests`

Result: 7 passed.

`cargo test -p lisa-cli completion_seal::tests`

Result: 5 passed.

`cargo test -p lisa-cli doctor::tests`

Result: 50 passed.

`cargo test -p lisa-cli status::tests`

Result: 13 passed, including adjacent preownership tests selected by the filter.

`cargo test -p lisa-cli --test seal_visibility`

Result: 1 black-box fixture passed.

`cargo fmt --all -- --check`

Result: passed.

`cargo test --workspace --quiet`

Result: passed after the final schema-version adjustment.

## Plan deviation

The original design proposed retaining existing schema-version numbers because
Serde could parse the additive field without branching.

Final review found that the provenance contract explicitly says shape changes
advance the version and the journal also has a versioned envelope.

The implementation therefore advanced provenance to 6 and journal to 2.

Backward compatibility remains intact: provenance structures default a missing
field, and journal replay accepts both versions 1 and 2.

This deviation makes the durable format more accurately self-describing.

## Remaining

Only the Review artifacts remain.
