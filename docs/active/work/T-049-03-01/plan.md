# Plan — T-049-03-01 hash-stamped journal seal

## Step 1: add typed completion evidence

Modify `crates/lisa-core/src/completion.rs`.

Add `CompletionContentHash` with validating construction and read-only accessors.

Require non-empty paths and exactly 64 lowercase hexadecimal digest bytes.

Add `CompletionSealReceipt` with distinct commit and journal variants.

Require a non-empty commit identifier.

Require a non-empty, path-sorted, path-unique journal hash set.

Expose the receipt seal, optional commit ID, and content-hash slice.

Add focused unit coverage for valid and invalid values.

Verification:

`cargo test -p lisa-core completion_content_hash`

`cargo test -p lisa-core completion_seal_receipt`

Expected result:

All constructor invariants are enforced without filesystem or adapter state.

## Step 2: commit the pure domain unit

Inspect the exact diff for `crates/lisa-core/src/completion.rs`.

Confirm no ticket/status frontmatter was edited manually.

Commit only the core file through:

`lisa commit-ticket --ticket-id T-049-03-01 --message "Add typed completion seal receipts" --include crates/lisa-core/src/completion.rs`

Verify the file is no longer modified while unrelated worktree changes remain.

## Step 3: migrate journal confirmation to typed receipts

Modify `crates/lisa-plugin/src/completion_journal.rs`.

Advance the new-row schema version to 4.

Replace transition and aggregate commit-ID storage with
`CompletionSealReceipt`.

Keep the test compatibility projection for commit IDs.

Change serialized confirmed rows to optional `commit_id` plus defaulted
`content_hashes`.

Map new commit receipts to the existing field.

Map journal receipts to the new hash field.

On replay, use the envelope seal to choose the only valid receipt shape.

Reject mismatched, missing, mixed, empty, or malformed receipt evidence.

Update all existing journal fixtures to use typed commit receipts.

Update new-row schema assertions while retaining legacy literal schemas.

Add a journal-row round-trip test that asserts `seal: journal`, the hash set,
absence of a commit ID, and aggregate reconstruction.

Verification:

`cargo test -p lisa-plugin completion_journal`

Expected result:

All pre-ladder and schema-2/3 commit history remains readable, new commit rows
retain their old evidence spelling, and new journal rows require hashes.

## Step 4: add plugin SHA-256 support

Modify `crates/lisa-plugin/Cargo.toml`.

Add `sha2 = "0.10"` as a runtime dependency.

Run Cargo metadata/check once and inspect Cargo.lock.

Expected result:

Cargo.lock does not change because the workspace already locks the dependency.

If Cargo.lock changes only to add the plugin's dependency edge, inspect whether
that lockfile representation is ticket-owned before including it.

## Step 5: implement deterministic hash collection

Continue in `crates/lisa-plugin/src/completion_journal.rs`.

Add lowercase SHA-256 computation for complete byte slices.

Add strict project-relative path labeling.

Add sorted recursive work-directory traversal.

Hash regular files and readable symbolic-link targets.

Reject directory enumeration failures with the directory path.

Reject metadata failures with the entry path.

Reject read failures with `cannot read and hash completion artifact` plus the
entry path.

Reject FIFO/device/socket and other special entries without opening them.

Return hashes sorted by recorded path.

Verification:

Add a nested artifact fixture with deliberately non-sorted creation order.

Assert recorded order is lexical by relative path.

Assert every digest independently matches `Sha256::digest` over visible bytes.

Expected result:

The same tree produces the same ordered receipt regardless of directory entry
enumeration order.

## Step 6: prepare and atomically publish Done ticket bytes

Add private Done-byte preparation in `completion_journal.rs`.

Read the original ticket.

Write it to a nonce-bearing sibling preparation path.

Run `ticket::update_ticket_done` only against that preparation path.

Read the complete prepared bytes.

Remove preparation residue on success and handled error paths.

Hash the prepared ticket bytes before publishing them.

Build and validate the full journal receipt before authoritative publication.

Publish the Done bytes through `RustPublication` with a sibling nonce temporary.

Return the typed journal receipt only after rename success.

Add an injectable publication closure for deterministic interruption testing.

Verification fixture A:

Use a project directory with no `.git` child.

Create a Review ticket plus nested canonical artifacts.

Call the journal seal operation.

Assert status and phase are Done and all hashes match the final visible bytes.

Verification fixture B:

Mutate one artifact after sealing.

Recompute its digest and assert it no longer equals the recorded digest.

Assert the recorded digest remains the value for the original sealed bytes.

Verification fixture C:

Create a dangling symbolic link under the work directory.

Assert completion returns a named read-and-hash failure containing its path.

Assert ticket bytes remain exactly Review and no receipt exists.

Verification fixture D:

Inject a publisher that observes the original Review destination and returns a
hostile interruption error.

Assert the exact original ticket survives and preparation temporaries are gone.

Expected result:

Every prerequisite fails before the status flip, and the flip itself is one
atomic sibling replacement.

## Step 7: branch completion dispatch by pinned seal

Modify `crates/lisa-plugin/src/lib.rs`.

Import `CompletionSealReceipt`.

In `execute_completion_effect`, retain all common authority and durable-journal
steps.

Construct a native command only for `CompletionSeal::Commit`.

For `CompletionSeal::Journal`, avoid `lisa_bin`, `git_root`, and repository path
normalization.

After pending insertion, call the journal seal transaction synchronously.

On failure, log a named completion rejection and retain the in-flight fence.

Do not append Confirmed, release the seat, or schedule dependents on failure.

Verification:

An existing commit-mode argv test must remain byte-for-byte unchanged.

A journal-mode state without a Lisa binary or Git root must reach the journal
operation rather than command construction.

## Step 8: converge successful seals on one finalizer

Extract a common completion-success helper from `handle_completion_result`.

Accept the ticket ID, cloned pending facts, and typed receipt.

Reject a receipt whose seal differs from `PluginConfig.completion_seal`.

Keep durable Done rescan before confirmation.

Append Confirmed with the typed receipt.

Retain operator modal handling, phase activity, provenance emission, thread
completion, slot release, and dependent scheduling.

Render existing commit verification text unchanged for commit receipts.

Render journal verification text with the number of hashes and no claim of a
commit.

Change commit result success to construct a validated commit receipt and call
the helper.

Leave all nonzero/malformed result handling from T-049-04-01 intact.

Verification:

Run existing completion success, duplicate result, failure retry, deadline, and
parking tests.

Expected result:

Tier 1 behavior and diagnostics do not regress, while Tier 2 uses honest typed
evidence.

## Step 9: make journal reconciliation idempotent

Branch `replay_in_flight_completion` on the pinned seal.

Retain stored correlation, deadline, authority, and generation checks.

For commit mode, build and launch the same native command as today.

For journal mode, reinstall pending state and rerun hash preparation/publication
without adding Requested or CommandInFlight rows.

Send successful replay through the common finalizer.

Leave failures in the same bounded in-flight/deadline state.

Verification:

Create a journal aggregate with in-flight state and a ticket already containing
Done, representing interruption after ticket publication.

Restore state and confirm the DAG masks the ticket to its prior Review state.

Replay journal sealing and assert exactly one Confirmed row is added.

Assert the final receipt hashes the visible Done ticket and artifacts.

## Step 10: add the repo-less scheduler acceptance fixture

Build a complete temporary project with:

- no repository metadata;
- predecessor Review ticket;
- passing disposition and canonical work artifacts;
- active current lease and occupied slot;
- dependent open ticket;
- completion journal and provenance paths;
- pinned `CompletionSeal::Journal`;
- no configured Lisa binary.

Dispatch completion through the normal scheduler input.

Assert:

- one completion effect is observed;
- no native host command is required;
- ticket status and phase are Done;
- journal rows all say `seal: journal`;
- Confirmed contains ticket plus every artifact hash;
- aggregate receipt is journal-typed;
- provenance Done record says journal;
- pending state and thread are removed;
- the slot is released;
- the dependent is eligible/scheduled according to the fixture's slot setup.

Independently recompute hashes after completion.

Mutate one artifact and show the stored digest is stale.

Expected result:

The acceptance claim is proven through the real scheduler boundary in a folder
that has never been initialized as a repository.

## Step 11: run focused verification

Run:

`cargo fmt --all -- --check`

If formatting fails only for ticket-owned files, run `cargo fmt --all` and
inspect unrelated effects before retaining changes.

Run:

`cargo test -p lisa-core completion_content_hash`

`cargo test -p lisa-core completion_seal_receipt`

`cargo test -p lisa-plugin completion_journal`

`cargo test -p lisa-plugin journal_seal`

`cargo test -p lisa-plugin completion`

Expected result:

Receipt invariants, persistence, acceptance fixtures, and the broader completion
boundary all pass.

## Step 12: run workspace verification

Run:

`cargo test --workspace`

Run:

`cargo check -p lisa-plugin --target wasm32-wasip1`

Run Clippy if the workspace's normal check path requires it or focused warnings
appear.

Expected result:

All native tests pass and the plugin remains valid for its production WASM
target with the hashing dependency.

## Step 13: inspect ticket ownership

Run `git status --short`.

Run exact diffs for:

- `crates/lisa-plugin/Cargo.toml`;
- `crates/lisa-plugin/src/completion_journal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `Cargo.lock` only if changed.

Confirm unrelated dirty files remain untouched and unstaged.

Update `progress.md` with implementation, tests, deviations, and remaining work.

## Step 14: commit the plugin unit through Lisa

Commit exact ticket-owned plugin paths:

`lisa commit-ticket --ticket-id T-049-03-01 --message "Implement hash-stamped journal completion" --include crates/lisa-plugin/Cargo.toml --include crates/lisa-plugin/src/completion_journal.rs --include crates/lisa-plugin/src/lib.rs`

Add `--include Cargo.lock` only if the inspected lockfile diff belongs solely to
this dependency edge.

Do not use ordinary `git add` or `git commit`.

Verify no ticket-owned source remains modified, staged, or untracked.

## Step 15: Review handoff

Write `review.md` in the attempt-private work directory.

Summarize the pure receipt contract, journal schema compatibility, hash
collection, atomic publication order, scheduler routing, recovery behavior,
tests, commits, and open concerns.

Re-run a read-only status and source diff check.

If every acceptance criterion is met and ticket-owned source is clean, write
exactly:

`{"disposition":"pass","reason":null}`

to `review-disposition.json`.

If a ticket-owned defect remains, write a structured block with an actionable
reason rather than claiming completion.

After both Review artifacts exist, remain on T-049-03-01 and stop.
