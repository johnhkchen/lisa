# Structure — T-049-01-02 seal visibility and ledger field

## Modified files

### `crates/lisa-cli/src/completion_seal.rs`

Add shared brand copy for the two runtime seal variants.

Add observational tier selection for state-reading commands.

Keep strict loop-start resolution unchanged.

Public-to-crate interfaces:

- `visibility_line(seal: CompletionSeal) -> &'static str`;
- `resolve_for_inspection(root, mode) -> CompletionSeal`.

Unit tests exhaust both variants and the three configured modes.

### `crates/lisa-cli/src/doctor.rs`

Resolve the observational seal after resolved configuration is available.

Append the shared line to the project-facing doctor output.

Retain existing dependency, cache, and trust behavior.

Add a fixture helper/test that renders both tier lines through doctor assembly.

### `crates/lisa-cli/src/status.rs`

Resolve the observational seal from root and configured mode.

Print the shared line directly after the general config line.

Add fixture tests for explicit commit and journal configurations.

### `crates/lisa-core/src/provenance.rs`

Import `CompletionSeal` from the completion module.

Add a defaulted public `seal` field to:

- `ProvenanceRecord`;
- `AssignmentTransitionRecord`;
- `ParkingTransitionRecord`.

Update sample constructors to make new writes explicit.

Assert serialized rows contain the field.

Add old-row parse/classification tests for all three shapes.

Keep the existing literal schema-v2 execution fixture field-free.

### `crates/lisa-plugin/src/completion_journal.rs`

Import `CompletionSeal`.

Add defaulted `seal` to the serialized `JournalRecord` envelope.

Add `seal` to `CompletionJournalAggregate` and expose an accessor.

Change journal conversion flow:

`transition + seal -> JournalRecord -> bytes -> JournalRecord -> seal + transition`.

Change `append` to accept the resolved seal.

Change `apply_transition` to accept and validate the seal.

Update every journal test append call with an explicit tier.

Add new-write, legacy-default, and mixed-tier rejection coverage.

### `crates/lisa-plugin/src/lib.rs`

Pass `self.config.completion_seal` into journal append.

Stamp `self.config.completion_seal` into each provenance constructor.

Update direct provenance fixtures with explicit seals where required.

No new scheduler state is needed because `PluginConfig` already stores the tier.

### `crates/lisa-plugin/src/ownership.rs`

Update direct execution-record test fixtures with an explicit seal.

Production ownership logic remains unchanged.

### `docs/knowledge/provenance-ledger.md`

Add `seal` to examples and field tables.

Document `commit` and `journal` values.

Document missing-field compatibility as pre-ladder commit history.

Align the displayed current schema version with the source constant.

## Data flow

Native loop startup resolves configured intent once.

The generated layout transports the runtime tier.

`PluginConfig` parses and retains it.

All plugin provenance writers read the same retained value.

The completion-journal adapter supplies it to each appended record.

Replay reconstructs it into each completion aggregate.

Doctor and status independently inspect the current environment for display.

They render the same exhaustive copy function.

## Compatibility boundary

Serde defaulting is the only legacy migration mechanism.

No existing JSONL file is rewritten.

No field is renamed or removed.

No discriminator changes.

No journal state-machine ordering changes.

No provenance untagged-variant ordering changes.

## Invariants

Every new provenance row has `seal`.

Every new completion-journal row has `seal`.

Every missing seal deserializes as `CompletionSeal::Commit`.

Every row in one active completion generation has the same seal.

`auto` never appears in a durable `seal` field.

Journal visibility copy never contains `git`.

Doctor and status copy are byte-identical for the same tier.

## Ordering

Core provenance types land before plugin constructors consume their new field.

Journal schema and plugin propagation land together to avoid unstamped writes.

CLI visibility is independent and can land after persistence.

Documentation accompanies the core audit-schema commit.

## Files not modified

`crates/lisa-core/src/completion.rs` already has the needed enum and default.

`crates/lisa-core/src/types.rs` already transports the pinned plugin tier.

`crates/lisa-cli/src/loop_cmd.rs` already pins and transports the tier.

Ticket frontmatter phase and status remain Lisa-owned.

Shared work artifacts remain Lisa publication-owned.
