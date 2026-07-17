# Structure — T-049-03-01 hash-stamped journal seal

## Change inventory

Modify `crates/lisa-core/src/completion.rs`.

Modify `crates/lisa-plugin/src/completion_journal.rs`.

Modify `crates/lisa-plugin/src/lib.rs`.

Modify `crates/lisa-plugin/Cargo.toml`.

Do not modify CLI completion transaction files.

Do not modify ticket frontmatter.

Do not modify provenance schemas.

Do not create a separate journal-seal module; the durable receipt and hashing
logic remain adjacent to completion-journal validation.

## `crates/lisa-core/src/completion.rs`

### New content-hash value

Add public `CompletionContentHash` near the runtime seal types.

Fields remain private:

- `path: String`;
- `sha256: String`.

Derive Debug, Clone, PartialEq, Eq, ordering, hashing, Serialize, and Deserialize.

Expose a validating constructor:

`CompletionContentHash::new(path, sha256) -> Result<Self, String>`.

Expose read-only accessors:

- `path(&self) -> &str`;
- `sha256(&self) -> &str`.

Constructor invariants:

- trimmed path is not empty;
- digest length is exactly 64;
- every digest byte is ASCII lowercase hexadecimal.

The value stores the original non-empty path spelling and normalized contract
requires callers to provide the lowercase digest directly.

### New seal receipt

Add public `CompletionSealReceipt` after `CompletionContentHash`.

Variants:

- `Commit { commit_id: String }`;
- `Journal { content_hashes: Vec<CompletionContentHash> }`.

Derive Debug, Clone, PartialEq, Eq, Serialize, and Deserialize.

Expose constructors:

- `CompletionSealReceipt::commit(commit_id) -> Result<Self, String>`;
- `CompletionSealReceipt::journal(content_hashes) -> Result<Self, String>`.

Commit constructor rejects empty or whitespace-only identifiers.

Journal constructor rejects an empty set.

Journal constructor requires strict ascending path order.

Strict ordering simultaneously rejects duplicate paths.

Expose:

- `seal(&self) -> CompletionSeal`;
- `commit_id(&self) -> Option<&str>`;
- `content_hashes(&self) -> &[CompletionContentHash]`.

The empty slice for commit receipts lets aggregate callers inspect receipts
without matching when only a count is needed.

### Core tests

Add tests within the existing `#[cfg(test)]` module.

Cover valid lowercase SHA-256 input.

Reject empty paths, short/long digests, uppercase hex, and non-hex bytes.

Cover non-empty commit receipts and accessors.

Cover sorted unique journal receipts and seal identity.

Reject empty, duplicate, and unsorted journal hash sets.

No filesystem fixtures belong in core.

## `crates/lisa-plugin/Cargo.toml`

Add direct dependency `sha2 = "0.10"`.

Keep it with normal dependencies because journal sealing runs in the WASM
plugin, not only in native tests.

No Cargo.lock edit is expected because `lisa-cli` already locks the same
package/version family.

## `crates/lisa-plugin/src/completion_journal.rs`

### Imports and constants

Import `CompletionContentHash` and `CompletionSealReceipt` from core.

Import `sha2::{Digest, Sha256}`.

Import `std::collections::BTreeMap` or a sorted vector helper in addition to the
existing aggregate map.

Import the core ticket updater for preparing Done bytes.

Advance `SCHEMA_VERSION` from 3 to 4.

Retain the existing legacy minimum and acceptance of earlier schema values.

Add sibling temporary prefixes for Done-byte preparation and publication.

### Transition representation

Change `CompletionJournalTransition::Confirmed` from:

`commit_id: String`

to:

`receipt: CompletionSealReceipt`.

The transition's key routing remains unchanged.

All commit call sites construct a typed commit receipt.

Journal sealing constructs a typed journal receipt.

### Aggregate representation

Replace `confirmed_commit_id: Option<String>` with:

`confirmed_receipt: Option<CompletionSealReceipt>`.

Add `confirmed_receipt(&self) -> Option<&CompletionSealReceipt>` for scheduler
and tests.

Retain test-only `confirmed_commit_id()` as a compatibility projection through
the receipt accessor.

Requested, command-in-flight, failure, and rejection transitions clear the
confirmed receipt exactly where they currently clear the commit ID.

Confirmed stores the validated receipt.

### Serialized confirmed row

Change `JournalRecordBody::Confirmed` fields to:

- existing completion identity fields;
- existing `correlation_id`;
- `commit_id: Option<String>` with default/omission behavior;
- `content_hashes: Vec<CompletionContentHash>` with default/omission behavior.

New commit transitions serialize only `commit_id`.

New journal transitions serialize only `content_hashes`.

The common envelope continues serializing `seal`.

`JournalRecord::from_transition` projects the receipt variant into those fields.

`JournalRecord::into_transition` reconstructs a receipt according to the row's
seal and calls the validating constructors.

Invalid combinations produce precise restore errors.

The fold also rejects a receipt whose seal differs from the transition envelope
even if deserialization somehow bypassed construction.

### SHA-256 helper

Add an internal byte helper:

`sha256(bytes: &[u8]) -> String`.

It returns lowercase hexadecimal using the `sha2` digest bytes.

No streaming helper is necessary for initial correctness; `fs::read` must
already obtain complete artifact bytes and the ticket artifacts are bounded by
the workflow.

### Recorded path helper

Add an internal helper that strips `project_root` from a required path.

Reject paths outside the project root with a named message.

Convert the relative path through `Path::display()` for stable row text.

Reject an empty relative path.

### Work traversal

Add recursive internal traversal that accepts project root, current directory,
and a mutable hash vector.

Read each directory through `fs::read_dir` and name enumeration failures.

Collect and sort entries by path before processing.

For directories, recurse.

For regular files and symbolic links, call `fs::read`; name failures as
`cannot read and hash completion artifact <path>`.

Reject other filesystem types with a named unsupported-artifact error.

Build `CompletionContentHash` from the project-relative label and digest.

### Done-byte preparation

Add an internal helper that reads the original ticket and prepares final Done
bytes without changing the authoritative ticket.

Create a sibling preparation path containing a nonce.

Write original bytes to the preparation path.

Invoke `ticket::update_ticket_done` against the preparation path.

Read the resulting complete bytes.

Remove the preparation path on success and on every handled failure.

Return named preparation/read/cleanup errors.

### Journal seal transaction

Add `complete_with_journal_seal` as the plugin-facing operation.

Inputs:

- project root;
- ticket file;
- canonical work directory.

Return `Result<CompletionSealReceipt, String>`.

Operation order:

1. prepare final Done ticket bytes;
2. create ticket content hash;
3. recursively hash work artifacts;
4. sort and validate the receipt;
5. publish final ticket bytes via `RustPublication`;
6. return the receipt.

Use a private `complete_with_journal_seal_and_publish` generic helper so tests
can inject an interruption before authoritative publication.

The production closure calls `RustPublication` with a nonce sibling temporary.

The injected closure receives destination and prepared bytes but cannot mutate
the hash set.

### Journal tests

Update existing confirmed-transition fixtures to construct commit receipts.

Update exact schema assertions from 3 to 4 for newly written rows.

Retain literal legacy rows and their current schema numbers.

Add a journal confirmed-row test that checks:

- `seal: journal`;
- no `commit_id`;
- complete `content_hashes`;
- restart reconstruction of the typed receipt.

Add invalid receipt/envelope fixture tests.

Add direct repo-less journal transaction tests for hash correctness,
post-seal mutation, unreadable/dangling artifact failure, and injected
publication interruption.

## `crates/lisa-plugin/src/lib.rs`

### Command construction boundary

Preserve `build_completion_command` exactly for commit mode.

In `execute_completion_effect`, build the command only when
`self.config.completion_seal == CompletionSeal::Commit`.

For journal mode, retain the already resolved absolute ticket file and canonical
work directory paths.

Journal mode must not consult `git_root`, require `lisa_bin`, or launch a native
command.

### Dispatch branch

Keep authority validation, dependency validation, Requested append,
CommandInFlight append, and pending insertion common to both tiers.

Keep `launched_completion_effects` test tracing common because the reducer still
emits one completion effect.

After pending insertion:

- commit: launch the existing command and return pending;
- journal: call `completion_journal::complete_with_journal_seal` synchronously.

On journal success, invoke the common success helper.

On journal error, keep the pending/in-flight fence, log a named completion
rejection, rebuild the DAG if needed, and return without confirmation.

### Replay branch

`replay_in_flight_completion` must branch on the pinned seal too.

Commit replay builds and launches the same native command.

Journal replay reinstalls pending state and reruns the idempotent journal seal
transaction without appending another Requested or CommandInFlight row.

Both reuse the original correlation and deadline.

### Common success helper

Extract the lower half of `handle_completion_result` into a method accepting:

- ticket ID;
- cloned `PendingCompletion`;
- `CompletionSealReceipt`.

The helper verifies receipt seal equals pinned run seal.

It rescans durable Done frontmatter.

It appends Confirmed with the exact receipt and correlation.

It handles the operator-modal accepted state.

It removes pending completion state and rebuilds the DAG.

It logs phase completion and phase transition.

It emits seal-specific informational activity.

It completes/removes the thread, emits Done provenance, releases the slot, and
schedules newly ready tickets.

### Commit result handler

Leave authority validation and all T-049-04-01 failure classification unchanged.

On success, parse the existing stdout commit ID and construct a core commit
receipt.

Call the common success helper.

No journal fallback is reachable from this method.

### Scheduler fixture tests

Keep existing commit-path tests and exact argv assertions.

Add a journal-mode completion fixture with no repository and no Lisa binary.

Assert the completion effect is handled synchronously, the ticket becomes Done,
the aggregate is Confirmed with a journal receipt, provenance says journal,
the seat is released, and the dependent becomes schedulable.

Assert no native completion command context is required in journal mode.

Add or adapt restart coverage for the Done-before-confirmation masked state.

## Ordering constraints

Implement and test core receipt vocabulary first.

Then migrate journal serialization and aggregate state while all call sites
still fail to compile visibly.

Then add hash construction and atomic ticket publication.

Then branch initial dispatch and replay.

Then extract common completion success and restore commit callers.

Finally add end-to-end repo-less scheduler coverage and run the full suite.

## Commit units

First meaningful unit:

- core receipt domain;
- exact include `crates/lisa-core/src/completion.rs`.

Second meaningful unit:

- plugin journal schema, hashing, atomic Done publication, scheduler routing,
  and dependency declaration;
- exact includes `crates/lisa-plugin/Cargo.toml`,
  `crates/lisa-plugin/src/completion_journal.rs`, and
  `crates/lisa-plugin/src/lib.rs`.

Cargo.lock is included only if Cargo actually changes it and the diff is solely
owned by this ticket.
