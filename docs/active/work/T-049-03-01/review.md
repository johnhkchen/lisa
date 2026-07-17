# Review — T-049-03-01 hash-stamped journal seal

## Outcome

T-049-03-01 is ready to complete.

A run pinned to `CompletionSeal::Journal` can now finish a Review-passing ticket
without a repository, Git root, native completion command, or configured Lisa
binary.

The successful journal confirmation contains SHA-256 hashes for the final Done
ticket and every file beneath the canonical ticket work directory.

The confirming envelope says `seal: journal` and contains no fabricated commit
identifier.

Commit-sealed completion retains its existing command, commit-ID, retry, park,
provenance, and scheduling behavior.

## Commits

`d916e5e` — Add typed completion seal receipts.

`685e915` — Validate replayed journal hash evidence.

`1dc2904` — Implement hash-stamped journal completion.

Every commit used `lisa commit-ticket` with exact repository-relative includes.

No ordinary `git add`, ordinary `git commit`, or broad include was used.

## Core completion domain

`crates/lisa-core/src/completion.rs` now defines
`CompletionContentHash`.

Each binding contains a non-empty stable path and exactly 64 lowercase
hexadecimal SHA-256 characters.

`CompletionSealReceipt` distinguishes:

- a non-empty commit identifier for `Commit`;
- a non-empty, strictly path-sorted, path-unique hash set for `Journal`.

The receipt exposes its matching runtime seal.

Commit evidence and journal evidence cannot be silently interchanged.

Journal receipt construction revalidates every binding even when Serde created
the private fields directly.

That extra validation prevents malformed persisted hash values from bypassing
the public content-hash constructor during journal replay.

## Journal schema

`crates/lisa-plugin/src/completion_journal.rs` advances new rows to schema 4.

Requested, command-in-flight, failure-observed, and rejected row semantics are
unchanged.

Confirmed rows now project the typed receipt into additive evidence fields.

Commit rows retain the established `commit_id` field and omit content hashes.

Journal rows omit `commit_id` and carry `content_hashes`.

The common envelope retains the existing `seal` field.

Replay chooses the required receipt shape from that envelope seal.

The fold rejects:

- commit confirmations containing content hashes;
- commit confirmations missing a commit ID;
- journal confirmations containing a commit ID;
- journal confirmations missing hashes;
- malformed SHA-256 bindings;
- empty, duplicate, or unsorted journal hash sets;
- receipt evidence whose tier differs from the active aggregate seal.

Schema-3 commit confirmations remain readable and reconstruct their prior commit
receipt.

Pre-ladder rows without `seal` continue defaulting to commit-sealed history.

## Hash construction

The plugin uses the workspace's established `sha2` 0.10 dependency.

Cargo.lock gained only the direct `lisa-plugin` dependency edge; the package and
transitive graph were already present through the CLI.

The seal transaction prepares the final ticket frontmatter before computing its
ticket digest.

The recorded ticket hash therefore describes the visible Done bytes, not the
pre-completion Review bytes.

The canonical work directory is traversed recursively.

Directory entries are sorted before processing, and the complete receipt is
sorted again by its project-relative path.

Regular files are read as complete byte strings.

Readable symbolic-link artifacts are hashed through their visible work path.

Dangling or unreadable symbolic links fail at the same named read boundary.

Special non-file objects are rejected without being opened, avoiding FIFO or
device hangs.

Directory enumeration, entry inspection, artifact reading, and unsupported
object errors all name the exact failing path.

Paths outside `project_root` are rejected rather than recorded as ambient host
paths.

## Atomic Done publication

The live ticket is not modified while hashes are being computed.

Original ticket bytes are copied to a nonce-bearing sibling preparation file.

The existing core ticket updater changes status and phase only in that private
preparation file.

The prepared bytes are read, hashed, and retained in memory.

The preparation sibling is removed on successful and handled failure paths.

Only after all required hashes validate does `RustPublication` write a complete
sibling temporary and rename it over the live ticket.

The visible ticket is therefore either the exact prior Review file or the exact
complete Done file.

An injected hostile-interruption fixture observes the original Review bytes at
the publication boundary, refuses publication, and proves those exact bytes
remain authoritative with no sibling residue.

An unreadable artifact fails before the authoritative rename and likewise
preserves the exact Review ticket.

## Cross-file interruption safety

Requested and CommandInFlight journal rows are durable before the journal seal
transaction starts.

Confirmed is appended only after the atomic Done publication and a durable Done
rescan.

If execution stops after the ticket rename but before journal confirmation, the
existing completion aggregate remains in flight.

`mask_completion_transaction` projects its retained prior phase/status over
the Done bytes during live scanning and after journal restore.

Dependents therefore cannot observe completion without the hash-bearing row.

Reconciliation reruns the same generation, recomputes current hashes, and can
append the missing confirmation.

Updating already-Done frontmatter is idempotent, so that recovery path does not
change ticket semantics.

No multi-file atomicity is claimed; the durable state machine supplies the
logical publication boundary.

## Scheduler routing

`State::execute_completion_effect` retains common authority, dependency,
Requested, CommandInFlight, correlation, deadline, and pending-state setup for
both tiers.

Pinned commit mode continues building and launching `lisa complete-ticket`.

Pinned journal mode bypasses command construction and performs the journal seal
synchronously inside the plugin.

It does not inspect Git availability or attempt a runtime tier switch.

`replay_in_flight_completion` has the matching seal branch while retaining the
original generation, correlation, authority, and absolute deadline.

One extracted successful-completion finalizer handles both typed receipts.

It verifies the receipt tier matches the pinned seal and verifies durable Done
frontmatter before confirming.

It then performs the established operator-modal acceptance, phase activity,
provenance emission, slot release, thread removal, DAG rebuild, and dependent
scheduling.

Commit activity retains the exact commit-oriented wording.

Journal activity reports the number of bound content hashes and makes no claim
about repository history.

## Acceptance fixture coverage

The direct repo-less seal fixture creates no `.git` directory.

It seals a Review ticket plus nested binary/text work artifacts.

It independently hashes the final visible bytes and matches every recorded
digest.

It asserts lexical project-relative path order and no temporary residue.

After sealing, it mutates `review.md`, recomputes the digest, and proves the
stored digest is detectably stale.

The unreadable fixture installs a dangling work-artifact link.

It asserts the error begins at the read-and-hash boundary, includes the artifact
name, leaves the ticket byte-for-byte in Review, and leaves no preparation file.

The interruption fixture refuses the final publisher and proves exact Review
preservation.

The serialized-row fixture proves schema 4, `seal: journal`, the complete hash
set, no commit ID, and typed restart reconstruction.

The schema compatibility fixture loads an exact schema-3 commit stream and
rejects an exact schema-4 journal confirmation with no hashes.

The scheduler fixture runs normal Reconcile admission in a repo-less project
with `completion_seal: journal`, no Git root, and no Lisa binary.

It proves synchronous confirmation, Done ticket state, four independently
verifiable hashes, journal-labelled provenance, thread release, and dependent
eligibility.

## Regression coverage

The complete native plugin suite passed with 421 tests.

This includes the existing commit transaction result, duplicate delivery,
restart reconstruction, hostile order, bounded retry, deadline parking,
operator recovery, provenance, and dependent scheduling coverage.

The final full workspace suite passed after all three ticket commits.

The plugin compiled for the production `wasm32-wasip1` target with `sha2`.

Production WASM Clippy passed with warnings denied.

Formatting and diff whitespace checks passed before the plugin commit.

## Verification commands

- `cargo test -p lisa-core completion_content_hash`
- `cargo test -p lisa-core completion_seal_receipt`
- `cargo test -p lisa-plugin completion_journal`
- `cargo test -p lisa-plugin journal_seal`
- `cargo test -p lisa-plugin`
- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- `cargo clippy -p lisa-plugin --lib --target wasm32-wasip1 -- -D warnings`
- `cargo fmt --all -- --check`

All passed.

## Worktree hygiene

`crates/lisa-core/src/completion.rs`, Cargo.lock,
`crates/lisa-plugin/Cargo.toml`,
`crates/lisa-plugin/src/completion_journal.rs`, and
`crates/lisa-plugin/src/lib.rs` are clean relative to HEAD.

The ordinary index is empty.

Remaining modified/untracked entries belong to Lisa runtime state, ticket
frontmatter transitions, attempt artifacts, and concurrent T-049-04-02 work.

They were not included in this ticket's source commits.

## Open concerns and limits

The journal is not hash-chained; the story explicitly leaves that for future
hardening.

Content is read into memory before hashing and publication. This matches the
existing artifact workflow and is sufficient for the ticket, but a future
large-artifact policy may prefer streaming hashes.

Journal sealing is tamper-evident, not tamper-preventing: mutation is discovered
by comparing current content to the recorded digest.

The tier remains intentionally lesser than commit sealing and is labelled as
such on every row and provenance record.

No acceptance gap or blocking concern remains.
