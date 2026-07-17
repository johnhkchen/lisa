# Plan — T-049-01-02 seal visibility and ledger field

## Step 1 — Add provenance seal fields

Import `CompletionSeal` in core provenance.

Add a defaulted `seal` field to all three concrete provenance records.

Update core fixtures to serialize explicit commit and journal values.

Retain old JSON fixtures without a seal.

Assert each old fixture parses with `CompletionSeal::Commit`.

Update the provenance ledger reference document.

Run focused core provenance tests.

Commit exact paths:

`lisa commit-ticket --ticket-id T-049-01-02 --message "Label provenance with completion seals" --include crates/lisa-core/src/provenance.rs --include docs/knowledge/provenance-ledger.md`

## Step 2 — Propagate the plugin seal to provenance writers

Update assignment-transition construction.

Update retry, park, and unpark construction.

Update execution-record construction.

Update plugin and ownership fixtures that construct records directly.

Run plugin library tests sufficient to compile every constructor.

Keep this source unit with journal changes if both touch `lib.rs`.

## Step 3 — Seal completion-journal rows and replay

Add a defaulted seal to `JournalRecord`.

Pass seal into `append` and record conversion.

Retain seal in reconstructed aggregates.

Reject mixed seals within an active generation.

Allow a new generation after terminal state to use a different tier.

Update all journal fixtures with explicit new-write seals.

Add a legacy missing-field classification assertion.

Add a serialized-row assertion for both tiers.

Run focused completion-journal tests.

Commit exact paths:

`lisa commit-ticket --ticket-id T-049-01-02 --message "Carry seals through plugin audit rows" --include crates/lisa-plugin/src/completion_journal.rs --include crates/lisa-plugin/src/lib.rs --include crates/lisa-plugin/src/ownership.rs`

## Step 4 — Add shared visibility copy

Add the two exhaustive copy strings to `completion_seal.rs`.

Add observational tier resolution for doctor/status.

Ensure explicit journal does not probe.

Ensure auto selects the environment-supported tier.

Ensure explicit commit remains displayed as commit even if diagnostics fail.

Test exact punctuation and em dash text.

Test that journal copy has no lowercase or uppercase `git`.

## Step 5 — Surface the line in doctor and status

Resolve the inspection tier in doctor.

Append the shared line without disrupting dependency reporting.

Resolve and print the same line in status.

Add fixtures for explicit commit and journal in both modules.

Where practical, use output helpers so tests prove inclusion, not only copy.

Run focused CLI completion-seal, doctor, and status tests.

Run the existing black-box missing-Git doctor test to protect diagnostics.

Commit exact paths:

`lisa commit-ticket --ticket-id T-049-01-02 --message "Show completion seals in doctor and status" --include crates/lisa-cli/src/completion_seal.rs --include crates/lisa-cli/src/doctor.rs --include crates/lisa-cli/src/status.rs`

## Step 6 — Broad verification

Run `cargo fmt --all -- --check` after formatting.

Run `cargo test -p lisa-core`.

Run focused `lisa-plugin` completion-journal tests.

Run focused `lisa-cli` doctor/status tests.

Run `cargo test --workspace` if focused suites pass in available time.

Inspect exact source-path status after each ticket commit.

Do not stage or commit Lisa-managed ledger, ticket, or other-ticket files.

## Step 7 — Review

Compare the final diff against both acceptance criteria.

Confirm every ticket-owned source path is clean.

Record commits, test results, compatibility behavior, and concerns in review.md.

Write a pass disposition only if all required behavior and verification hold.
