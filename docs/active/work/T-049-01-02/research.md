# Research — T-049-01-02 seal visibility and ledger field

## Ticket boundary

T-049-01-02 starts after T-049-01-01 introduced the completion-seal domain.

The ticket makes the already-resolved tier visible to people and durable data.

It does not introduce a third tier or change completion mechanics.

The runtime tiers are `commit` and `journal`.

Configured `auto` is intent, not a resolved tier and must not reach output rows.

The acceptance surface has four parts:

- `lisa doctor` output;
- `lisa status` output;
- every newly written provenance-ledger record;
- every newly written completion-journal record.

Old JSON rows must continue loading.

An absent `seal` means commit-sealed pre-ladder history.

## Existing completion vocabulary

`crates/lisa-core/src/completion.rs` owns completion-seal types.

`CompletionSeal` is a two-variant enum: `Commit` and `Journal`.

Serde encodes the variants as lowercase `commit` and `journal`.

`CompletionSeal::default()` is `Commit`.

That default already matches the ticket's legacy classification rule.

`CompletionSealMode` separately represents `Auto`, `Commit`, and `Journal`.

`resolve_completion_seal` combines configured mode with commit support.

`ResolvedCompletionSeal` retains the selected tier and any auto-fallback reason.

`crates/lisa-cli/src/completion_seal.rs` owns native environment probing.

`resolve_for_run` probes a repository, identity, HEAD, and metadata as required.

Explicit journal mode skips the Git probe entirely.

Auto selects journal when commit support is unavailable.

Explicit commit returns a formatted error when commit support is unavailable.

`RunCompletionSeal::seal()` exposes the resolved `CompletionSeal`.

The module is currently private to the CLI crate but is reachable by siblings.

## Configuration boundaries

`crates/lisa-cli/src/config.rs` loads `.lisa.toml`.

`ResolvedConfig.completion_mode` retains configured intent.

It deliberately does not claim to be a runtime resolution.

Missing configuration defaults to `CompletionSealMode::Auto`.

`lisa loop` calls `resolve_for_run` once before scheduler side effects.

The loop sends only `commit` or `journal` into generated Zellij KDL.

`crates/lisa-core/src/types.rs` parses that KDL into `PluginConfig`.

`PluginConfig.completion_seal` defaults to `Commit`.

Missing and malformed KDL values therefore fail closed to commit.

The plugin already has the pinned tier as `self.config.completion_seal`.

No plugin-side re-probe is required or possible.

## Doctor surface

`crates/lisa-cli/src/doctor.rs` loads and resolves project configuration.

Doctor currently builds dependency reports and prints one assembled string.

`format_report` renders the general dependency section.

`run_doctor` adds project-version, cache, and optional Codex-trust sections.

Doctor currently checks Git unconditionally through `build_checks`.

That general dependency behavior survived T-049-01-01 intentionally.

The loop-specific dependency helper can omit Git for journal loops.

The seal visibility line is not present today.

Doctor's output assembly is internal and unit tests test `format_report` pieces.

Black-box CLI tests can capture actual stdout via `CARGO_BIN_EXE_lisa`.

Doctor requires Zellij, the configured provider, and embedded WASM to succeed.

Tests that only need copy can use a small formatter without process fixtures.

The requested exact commit copy is:

`completion seal: commit-sealed — finished work lands as history`

The requested exact journal copy is:

`completion seal: journal-only — finished work is recorded but not undoable`

The journal copy contains no `git` spelling.

## Status surface

`crates/lisa-cli/src/status.rs` loads configuration with a forgiving fallback.

On config-load failure it uses `ResolvedConfig::default()`.

It scans tickets, builds the DAG, prints statistics, waves, and a run summary.

The current config line reports thread count and session timeout.

No seal line is printed.

Status uses `println!` directly rather than writing through an injected writer.

Existing unit tests mostly assert success or error, not captured output.

An extracted seal-line formatter can be asserted by status and doctor fixtures.

Status has the project root and the configured mode needed by `resolve_for_run`.

Unlike loop startup, status is observational and should not launch side effects.

The native seal probe itself is read-only.

## Provenance ledger schema

`crates/lisa-core/src/provenance.rs` owns the JSONL types and append helpers.

The ledger is intentionally mixed-version and mixed-shape.

`ProvenanceLedgerRecord` is an untagged enum over three record structures.

`ProvenanceRecord` is the terminal execution row.

`AssignmentTransitionRecord` captures failures before provider ownership.

`ParkingTransitionRecord` captures retry, park, and unpark transitions.

All three shapes are described as provenance records in the module contract.

Each shape has multiple constructors in core tests and plugin code.

The current schema constant is version 5.

The ticket calls the schema change additive.

Serde ignores unknown fields by default, helping forward tolerance.

A field with `#[serde(default)]` will use `CompletionSeal::default()` when absent.

Because that default is commit, it directly expresses pre-ladder classification.

New writes serialize public struct fields unless explicitly skipped.

Therefore writers must supply the plugin's pinned seal to every constructor.

The plugin constructs assignment rows in `emit_assignment_transition`.

It constructs parking rows for retry, park, and unpark paths.

It constructs terminal execution rows in `emit_provenance`.

Several native plugin tests construct rows directly.

Core provenance tests already keep a literal schema-v2 execution fixture.

That fixture is an existing natural compatibility check for a missing field.

## Completion journal schema

`crates/lisa-plugin/src/completion_journal.rs` owns journal persistence.

The journal schema version is currently 1.

`JournalRecord` contains `schema_version` and a flattened tagged body.

The body variants are requested, command-in-flight, rejected, and confirmed.

`CompletionJournalTransition` is the typed adapter-facing representation.

`JournalRecord::from_transition` converts transitions into serialized rows.

`JournalRecord::into_transition` reconstructs transitions during replay.

`append` folds all old bytes before applying and atomically publishing new bytes.

`load` and `fold_bytes` reject torn, malformed, empty, or invalid sequences.

There are already literal legacy rows without reconciliation deadlines.

Those legacy fixtures also omit any seal field.

Adding `seal` at the `JournalRecord` envelope makes it common to every state.

Serde defaulting at that envelope can classify every old row as commit.

The transition type currently contains no seal.

The plugin adapter calls `journal_completion_transition` from completion paths.

That adapter has access to `self.config.completion_seal`.

## Replay and classification

Provenance readers deserialize directly into the typed row structures.

Adding a defaulted typed field exposes the inferred commit classification.

Completion-journal replay converts a deserialized record back to a transition.

For replay to retain classification, the transition or aggregate must carry it.

At minimum the deserialized record can expose the default during conversion.

If the transition drops it, tests cannot distinguish a legacy row after load.

The ticket explicitly asks replay/reconstruction paths to treat absence as commit.

Retaining seal through transition and aggregate state makes that fact observable.

One aggregate should not silently combine different seals within a generation.

Existing histories are uniformly inferred commit.

New histories should be uniformly stamped from the pinned plugin configuration.

## Documentation and readers

`docs/knowledge/provenance-ledger.md` documents the provenance schema.

The source module links to that document as the field table and query guide.

Adding a durable field changes that documented public audit surface.

`crates/lisa-cli/src/preownership_status.rs` reads mixed provenance rows.

`crates/lisa-cli/src/run_summary.rs` also reads provenance segments loosely.

Typed deserialization compatibility prevents these readers from breaking.

## Constraints

No existing row may need migration or rewriting.

No `auto` value may be written as a seal.

Doctor and status need the same wording, not parallel copy definitions.

The journal-tier line must not say `git`.

The resolved runtime fact should come from existing completion resolution.

The plugin should stamp the pinned `PluginConfig.completion_seal`.

The schema addition should not repurpose existing outcome or state fields.

Ticket-owned source changes must be committed only through `lisa commit-ticket`.

Existing dirty Lisa-managed files and another ticket's work must remain untouched.
