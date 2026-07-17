# Design — T-049-03-01 hash-stamped journal seal

## Decision summary

Journal-sealed completion will execute inside the plugin after the same durable
Requested and CommandInFlight transitions used by commit completion.

The plugin will recursively hash the prepared completed ticket and every file
under the canonical ticket work directory.

It will publish the prepared Done ticket through the existing sibling-temporary
publication primitive.

Only after that atomic file replacement succeeds will it append a Confirmed
journal row containing a typed journal receipt and the complete hash set.

The existing completion mask will provide logical cross-file atomicity if the
process stops between ticket publication and journal confirmation.

Commit-sealed completion will continue to launch the existing CLI transaction
and retain its current stdout, commit, retry, and parking behavior.

## Goals

Complete a Review-passing ticket when the run's pinned seal is journal.

Require hashes for the final Done ticket and every canonical work artifact.

Record deterministic project-relative paths and lowercase SHA-256 values.

Fail before Done publication if any required path cannot be enumerated or read.

Label the confirming row `seal: journal` and make its evidence unambiguous.

Preserve an honest commit receipt for commit-sealed rows.

Keep restart and lost-result reconciliation fail-closed.

Demonstrate tamper evidence by recomputing a changed artifact's digest.

Keep all pre-ladder journal rows readable.

## Non-goals

Do not change startup seal resolution.

Do not allow a run to switch seals after startup.

Do not hash-chain the journal itself.

Do not make journal sealing equivalent to repository history.

Do not duplicate the hash set into provenance.

Do not change Review disposition parsing.

Do not change the bounded native-command failure policy from T-049-04-01.

Do not add a no-repository runbook; T-049-03-02 owns it.

## Option A: add a native journal-completion CLI command

This option adds a second hidden command next to `complete-ticket`.

The plugin would launch it asynchronously, parse a serialized hash receipt from
stdout, and append the confirmation row.

Advantages:

- It mirrors the current commit-command lifecycle.
- Native command failures naturally enter the bounded retry/park path.
- Hashing stays outside the WASM binary.
- The CLI can reuse `sha2`, which it already depends on.

Costs:

- It adds a new public process boundary and receipt serialization protocol.
- It spreads one journal transaction across CLI main, commit transaction,
  plugin command construction, result parsing, and journal persistence.
- The CLI would prepare journal evidence while the plugin remains the only
  writer allowed to fold and publish the journal.
- A repo-less path vocabulary would have to coexist with the existing
  repository-relative command vocabulary.
- The ticket specifically centralizes journal mechanics in the completion
  domain and plugin journal modules.

This option is viable but introduces more boundary surface than the operation
needs.

## Option B: execute journal sealing directly in the plugin

This option branches at the pinned runtime seal after durable request and
in-flight rows exist.

The plugin hashes and publishes with the filesystem APIs it already uses for
assignments, canonical artifacts, dispositions, and the journal.

Advantages:

- No new command or stdout protocol is required.
- The journal writer and its hash receipt stay in one module boundary.
- Repo-less projects do not need `git_root` path conversion.
- The existing `RustPublication` primitive directly supplies atomic ticket
  replacement and hostile-path behavior.
- The existing completion mask already handles interruption before confirmation.
- Commit mode remains byte-for-byte on its existing command path.

Costs:

- `sha2` becomes a direct plugin dependency.
- Journal failures do not arrive through native command stderr.
- Shared successful-completion cleanup must be extracted from the commit result
  handler so both seals converge on one scheduler consequence.

This option is selected because it minimizes interfaces and uses the scheduler's
existing durable journal as the transaction coordinator.

## Option C: mark Done first and append hashes later

This option would update the ticket immediately, then enumerate and hash files,
then write the journal row.

It is rejected.

An unreadable artifact would leave durable Done bytes without a valid seal.

Although the in-flight mask could hide those bytes, avoidable partial work would
increase recovery complexity and violate the fail-before-publication requirement.

All artifact hashes must be available before the status flip.

## Option D: append the hash row before marking Done

This option would hash the current ticket and artifacts, append Confirmed, then
change the ticket frontmatter.

It is rejected.

The recorded ticket hash would immediately become stale when status and phase
change to Done.

A crash after confirmation but before the status flip would also expose a
Confirmed aggregate whose durable ticket is not Done.

The confirming row must be the last durable authority after Done publication.

## Receipt domain

Add two pure core types.

`CompletionContentHash` contains one stable path and one lowercase SHA-256
digest.

`CompletionSealReceipt` distinguishes `Commit { commit_id }` from
`Journal { content_hashes }`.

Constructors validate the receipt contract.

A content path must be non-empty.

A SHA-256 value must contain exactly 64 lowercase hexadecimal characters.

A commit receipt must carry a non-empty commit identifier.

A journal receipt must carry at least the ticket hash.

Journal paths must be unique and strictly sorted.

Receipt validation belongs in core because replayed journal bytes and newly
computed values must obey one contract.

Hash computation itself remains adapter I/O in the plugin.

## Journal schema

Advance new completion-journal rows from schema version 3 to schema version 4.

Keep the existing common `seal` field.

Keep confirmed commit rows' existing `commit_id` spelling for compatibility and
readability.

Add `content_hashes` to confirmed rows, defaulting to an empty collection when
reading older rows.

Use omission for the receipt field that does not apply:

- commit row: non-empty `commit_id`, no `content_hashes`;
- journal row: no `commit_id`, non-empty `content_hashes`.

Deserialization will reconstruct a typed receipt based on the row's `seal`.

A commit row with hashes, a journal row with a commit ID, or a journal row with
no hashes is invalid history and fails closed.

Legacy rows without `seal` remain commit-sealed and require their old commit ID.

The aggregate will retain `confirmed_receipt` rather than only a commit string.

A compatibility accessor can continue exposing `confirmed_commit_id` to the
existing commit tests.

## Hash-set construction

Prepare the final Done ticket bytes before hashing.

Use a private sibling preparation path and the existing
`ticket::update_ticket_done` parser so frontmatter semantics have one source.

Remove the preparation file after reading its complete bytes, on success or
error where possible.

Hash the prepared ticket bytes directly.

Recursively traverse the canonical work directory.

Sort directory entries before descending so filesystem enumeration order cannot
affect the receipt.

Read every non-directory artifact as bytes and hash it with SHA-256.

Report directory enumeration, metadata, and read failures with the exact path.

Represent recorded paths relative to `project_root`.

Reject required paths outside the project root rather than recording ambient
absolute host paths.

Sort all `CompletionContentHash` values by their recorded path before building
the receipt.

The ticket path and work path are distinct by configuration; receipt uniqueness
validation catches accidental collisions.

## Symbolic links and unusual entries

Directories are traversed but are not themselves hashed.

Regular files are hashed by content.

Symbolic links are read through their visible artifact path; a dangling or
unreadable target yields a named read-and-hash failure.

Special non-file entries are rejected instead of opened, preventing hangs on
FIFOs or device nodes.

This is fail-closed and gives deterministic unreadable-artifact fixtures without
depending on process privilege or mode-bit enforcement.

## Publication sequence

The chosen sequence is:

1. Validate completion authority and passing Review disposition as today.
2. Append Requested with `seal: journal`.
3. Append CommandInFlight with `seal: journal`.
4. Prepare complete Done ticket bytes in a non-authoritative sibling.
5. Read and hash every canonical work artifact.
6. Build and validate the complete typed journal receipt.
7. Atomically publish the Done ticket bytes through `RustPublication`.
8. Rescan and verify durable Done frontmatter.
9. Append Confirmed with the journal receipt.
10. Run common successful-completion cleanup, provenance, slot release, and
    dependent scheduling.

No confirmation is possible before steps 4–8 succeed.

## Interruption model

An interruption before step 7 leaves the original Review ticket unchanged.

An interruption during step 7 exposes either the original complete ticket or
the complete Done ticket because publication is one sibling rename.

An interruption after step 7 but before step 9 leaves Requested/InFlight in the
journal.

On restart, `mask_completion_transaction` projects the prior Review state over
the Done bytes and prevents dependent scheduling.

Reconciliation reruns the same generation and recomputes the receipt.

Because Done preparation is idempotent, the second publication has identical
ticket semantics.

An interruption after step 9 is replay-safe because the aggregate is Confirmed
and no new completion command is requested.

## Failure behavior

Hash-set construction failures return one named error before Done publication.

The pending in-flight aggregate remains the scheduler fence.

The plugin logs a completion rejection whose lead sentence states that the
journal seal could not be created and whose detail names the failing path.

The unresolved generation remains eligible for existing reconciliation and
deadline parking rather than being mislabeled as a successful completion.

No Confirmed row and no Done scheduler transition are emitted.

This ticket will not guess an operator remedy based on a Rust filesystem error.

The existing deadline path supplies the bounded eventual park if the condition
persists.

## Shared success path

Extract a helper that accepts ticket ID, pending completion facts, and a typed
receipt.

The helper verifies durable Done, journals Confirmed, removes pending state,
rebuilds the DAG, records phase activity, emits provenance, releases the slot,
and schedules dependents.

Commit result handling will continue validating exit code and hex commit ID,
then call the helper with a commit receipt.

Journal completion will call the helper with the computed journal receipt.

Activity text will say `Completion commit verified ... at <id>` for commit and
`Journal seal verified ... with <n> content hashes` for journal.

The typed receipt prevents journal success from fabricating a commit-shaped
identifier.

## Testing design

Core tests validate accepted and rejected receipt shapes.

Journal tests prove schema-4 commit and journal rows reconstruct typed receipts.

Legacy schema-1 through schema-3 commit rows continue reconstructing.

Mixed seal/receipt rows fail closed.

A repo-less scheduler fixture uses a project tree with no `.git` directory,
dispatches Review completion under `CompletionSeal::Journal`, and verifies Done,
Confirmed, seat release, and dependent eligibility.

That fixture parses the recorded hash set and independently recomputes every
ticket/artifact hash from visible bytes.

It then mutates one artifact and proves the recorded digest differs from the
new digest while all other recorded evidence remains unchanged.

An unreadable fixture places a dangling symlink in the canonical work tree,
asserts a path-specific error, and proves the ticket remains Review with no
Confirmed row.

An injected publication failure observes that the live ticket still contains
the original Review bytes and that no confirmation is appended.

Existing commit-command argv and completion-result tests remain unchanged to
guard Tier 1 semantics.

## Dependency decision

Add `sha2 = "0.10"` to `lisa-plugin`.

The workspace lock already contains this exact package through `lisa-cli`, so
no dependency version or lockfile graph change is expected.

Using the established implementation is preferable to maintaining custom
cryptographic code in the scheduler.
