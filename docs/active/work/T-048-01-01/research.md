# Research — T-048-01-01 structured-block-schema

## Ticket boundary

The ticket is the first task in story S-048-01, `legible-block-contract`.

It owns two data-contract changes in `lisa-core`:

1. the parsed representation of a blocking Review disposition;
2. provenance row types for later scheduler park and unpark behavior.

The ticket explicitly does not change scheduler policy or execute a remedy
check. T-048-01-02 consumes the new contracts to implement parking. Later
story S-048-02 owns status/dashboard copy, operator unblock commands, check
execution, rechecking, and authoring guidance.

The current ticket phase is `research`. The ordinary worktree already contains
Lisa-managed ticket and provenance changes plus unrelated published work. Those
paths are not ticket-owned and must not be staged or committed by this attempt.

## Current Review disposition module

`crates/lisa-core/src/disposition.rs` owns the Review disposition parser.

The public `ReviewDisposition` enum currently has three variants:

- `Pass`;
- `Block { reason: String }`;
- `Invalid { reason: String }`.

The type derives `Debug`, `Clone`, `PartialEq`, and `Eq`.

`parse_review_disposition` accepts a filesystem path. It reads the whole file
as UTF-8 text, parses it through `serde_json::from_str`, and delegates the
resulting `serde_json::Value` to a private `validate_document` function.

Read failure and JSON syntax failure produce `ReviewDisposition::Invalid`.
This is the module's fail-closed boundary: only an exact typed `Pass` can grant
completion authority.

`validate_document` requires a JSON object with both `disposition` and `reason`.
It removes those fields from the object and ignores any fields left over.
Consequently the current parser already tolerates additive unknown fields.

A pass is valid only when `disposition` is the string `"pass"` and `reason` is
JSON null. A block is valid only when `disposition` is `"block"` and `reason`
is a non-empty, non-whitespace JSON string. All other relationships become
`Invalid` with an explanatory string.

The existing block parser preserves the exact reason string, including leading
or trailing whitespace when the string contains at least one non-whitespace
character. Validation calls `trim()` only to test emptiness; it does not mutate
the stored string.

The module does no shell parsing, process creation, environment access, or
command execution. Its only side effect is reading the disposition file.

## Existing disposition tests

The unit tests live beside the parser in `disposition.rs`.

The helper `parse_document` writes supplied text to a temporary
`review-disposition.json` and calls the public path parser. This exercises the
filesystem and JSON boundaries together.

Current coverage includes:

- exact pass parsing;
- a block with a reason;
- missing files;
- malformed JSON;
- missing, null, empty, and whitespace-only block reasons;
- pass with a non-null reason;
- pass without a reason;
- an unknown disposition string;
- a non-object document.

The current tests compare parsed blocks directly to
`ReviewDisposition::Block { reason }`, so changes to that public variant affect
core and plugin compilation even when runtime behavior is unchanged.

## Disposition consumers

`crates/lisa-core/src/completion.rs` stores `ReviewDisposition` inside
`DurableCompletionInputs`. Its reconciliation rule uses
`matches!(..., ReviewDisposition::Pass)`, so any block shape remains
ineligible without scheduler changes.

The completion module's unit tests construct a block directly as a non-passing
fixture. `crates/lisa-core/tests/completion_state_machine.rs` does the same in
its reference harness.

`crates/lisa-plugin/src/lib.rs` imports the parser and enum. The plugin has two
matches that bind only the block reason:

- `passing_review_disposition` turns a block into
  `CompletionRejection::DispositionBlocked`;
- `review_protocol_blocker` renders the reason in its attention diagnostic.

Other plugin matches use `ReviewDisposition::Block { .. }` and are insensitive
to added fields. Existing direct reason-binding patterns are exhaustive and
must name ignored fields if the variant grows.

The plugin's completion behavior is deliberately outside the ticket. Pass is
still the only completion-granting value; a structured or unstructured block
still rejects completion.

## Story-level block vocabulary

The story identifies three remedy-owner classes:

- `agent`: the running agent owns a bounded retry before parking;
- `operator`: a person must supply the remedy;
- `world`: external reality must change and can later be rechecked.

For a block, the proposed JSON fields are additive to `disposition` and
`reason`:

- `remedy_owner`, restricted to the three owner strings;
- `ask`, one plain-language sentence;
- `steps`, an optional list of concrete actions;
- `check`, an optional read-only shell probe stored for later use.

The ticket calls `steps` and `check` optional. It describes a block with
missing or malformed structure as operator-owned, using the raw `reason` as
its ask and carrying an `unstructured` flag. The legacy two-field block is the
most important missing-structure case.

The parser is not responsible for proving that `ask` is grammatically one
sentence, that a step is operationally effective, or that `check` is actually
read-only. The later execution boundary owns command safety. At parse time the
check remains inert data.

The ticket preserves the raw reason independently of the structured ask.
Existing plugin diagnostics and completion rejection currently consume reason,
not ask.

## Current provenance module

`crates/lisa-core/src/provenance.rs` owns provenance schemas, serialization,
time conversion, and append-only JSONL writes.

`SCHEMA_VERSION` is currently 3. Existing schema-v3 work added assignment
transition rows while retaining schema-v2 execution-row compatibility.

`ProvenanceRecord` represents a terminal execution attempt. It contains the
ticket ID, complete `AttemptLease`, outcome, authority and fencing flags,
requested and actual routes, epoch-second timestamps, duration, usage, cost,
spawn concurrency, and pane ID.

`AssignmentTransitionRecord` represents a bounded transition that ended before
provider ownership. It contains its own record discriminator, ticket ID,
attempt lease, pane, provider, assignment state, reason, start/end timestamps,
and wall-clock duration.

`ProvenanceRecordType` currently has one kebab-case serialized variant:
`AssignmentTransition`.

`ProvenanceLedgerRecord` is an untagged enum. Its variants are ordered as
assignment transition first and execution second. The required field sets
distinguish them while preserving the historical execution JSON shape, which
has no explicit `record_type`.

All provenance structs derive serde serialization/deserialization and equality
appropriate to their numeric contents. `ProvenanceRecord` uses `PartialEq`
because it contains `f64`; assignment transition uses full `Eq`.

## Current provenance append boundary

`append_record` accepts a terminal execution record.

`append_assignment_transition_record` accepts a pre-ownership transition.

Both delegate to private generic `append_serialized`. That helper creates the
parent directory, serializes one compact JSON object, appends a newline, opens
the target with create-and-append semantics, and writes the entire line.

The helper never reads or rewrites prior rows. The append tests confirm that
multiple records survive, each row remains parseable, and failed appends do not
disturb an existing colliding target.

`system_time_to_epoch` converts `SystemTime` to UTC epoch seconds and saturates
pre-epoch inputs to zero. Writers compute durations separately using saturating
subtraction.

## Existing provenance readers

`crates/lisa-cli/src/preownership_status.rs` reads the untagged ledger and acts
only on `AssignmentTransition` rows.

The plugin reads `ProvenanceLedgerRecord` in several tests and production
helpers. Some matches have exactly two exhaustive arms: `Execution` and
`AssignmentTransition`. Adding another ledger variant will require those
matches to explicitly ignore or handle park transitions.

The future scheduler ticket needs a typed append entry point and a replayable
typed ledger variant. This ticket does not call the new writer from plugin
state because no park/unpark transition exists yet.

## Park/unpark provenance requirements

The acceptance criteria require park and unpark record types with:

- ticket identity;
- attempt identity;
- remedy owner class;
- timestamps;
- append coverage;
- replay coverage.

The story further says durations must make stranded time queryable. Existing
provenance conventions represent an interval with `started_at`, `ended_at`, and
`wall_clock_secs`. Park is the start of stranded time; unpark closes that
interval. The exact producer-side timestamp meaning will be consumed by the
next ticket, while this ticket establishes serializable fields.

`AttemptLease` is the repository's existing attempt-scoped identity. It carries
both `ticket_id` and numeric `attempt_id`; provenance rows also repeat the
top-level ticket ID for query convenience and compatibility with existing
record shapes.

No current core type represents remedy owner. Defining it in disposition makes
the parsed block and provenance records share one vocabulary rather than using
unvalidated strings at the ledger boundary.

## Compatibility constraints

The disposition JSON contract is additive. Existing exact pass files must
remain pass, and existing exact block files must remain a typed block rather
than becoming invalid.

Malformed outer JSON, missing disposition, invalid pass relationships, unknown
dispositions, and unusable reasons still cannot grant completion. The new
fallback applies to the structure of an otherwise valid block, not to an
untrustworthy document or absent actionable reason.

The raw reason is compatibility data. It must not be trimmed, replaced by ask,
or lost when structured fields fail validation.

The provenance ledger is mixed-shape and append-only. Existing schema-v2
execution and schema-v3 assignment rows must continue to deserialize. New rows
need an explicit discriminator so serde's untagged replay can identify them.

## Verification boundaries

Focused disposition unit tests can cover every owner, optional values,
fallback behavior, legacy preservation, and inert check content without
involving the plugin.

Focused provenance unit tests can cover compact serialization, one-line
append, heterogeneous replay, and preservation of old row shapes.

Core completion and state-machine tests catch direct constructor changes.
Plugin compilation and tests catch exhaustive consumer matches.

Workspace tests provide the final compatibility check. No integration test
should observe a spawned command because parsing has no execution boundary.
