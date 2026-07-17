# Research — T-049-03-01 hash-stamped journal seal

## Ticket boundary

T-049-03-01 implements the runtime mechanics for `CompletionSeal::Journal`.

The ticket does not change how a run chooses its seal.

Seal configuration and startup resolution landed in T-049-01-01.

Seal visibility and the additive `seal` field landed in T-049-01-02.

Bounded completion failure handling and parking landed in T-049-04-01.

T-049-03-02 owns explicit-commit enforcement fixtures, mid-run degradation
coverage, and the Chromebook no-repository runbook leg.

This ticket must preserve commit-sealed behavior while making a pinned journal
seal independently capable of completing a ticket.

The required durable evidence is one journal confirmation row labelled
`seal: journal` with SHA-256 hashes for the completed ticket file and every
artifact beneath the ticket's canonical work directory.

An unreadable required path must stop completion with a path-specific reason.

## Runtime seal domain

`crates/lisa-core/src/completion.rs` contains the provider-neutral completion
domain.

`CompletionSeal` has exactly `Commit` and `Journal` runtime variants.

`Auto` exists only in `CompletionSealMode`; it cannot appear as a runtime seal.

`CompletionGenerationId` binds ticket, attempt, and generation identities.

`CompletionState` advances through Eligible, Requested, CommandInFlight, and
Confirmed, with Rejected as the failure branch.

The reducer treats successful completion as a correlated event and deliberately
does not know whether success came from a repository commit or another seal.

The core module performs no filesystem I/O and has no scheduler or Zellij
dependency.

It is the natural location for stable, serializable seal-receipt vocabulary,
but not for walking directories or publishing ticket bytes.

## Pinned plugin configuration

`PluginConfig.completion_seal` is populated in the generated KDL at loop start.

Malformed or legacy plugin configuration fails closed to the commit seal.

The plugin therefore receives one immutable tier for the entire run.

The completion path must branch on this pinned field rather than probing the
environment again.

That constraint prevents a commit-sealed run from silently downgrading after a
repository failure.

## Completion dispatch

`State::execute_completion_effect` in `crates/lisa-plugin/src/lib.rs` is the
single initial completion launch boundary.

It verifies current attempt or operator authority and complete dependencies.

It retains the ticket's prior phase and status before completion publication.

It appends `Requested` and `CommandInFlight` journal transitions before any
external completion side effect.

It then installs `PendingCompletion`, launches the native completion command,
and waits for a correlated result.

Native tests may omit the command while still exercising the state machine.

The current command is always built by `State::build_completion_command`.

That builder resolves ticket and work paths relative to `git_root` and invokes
the hidden `lisa complete-ticket` CLI subcommand.

This unconditional repository-relative conversion is incompatible with a
repo-less journal-sealed project.

## Commit-sealed completion

`crates/lisa-cli/src/commit_transaction.rs` owns the Tier 1 transaction.

`complete_ticket` validates the completion key and repository-relative paths.

It snapshots the original ticket bytes, writes Done frontmatter, and invokes
the isolated commit transaction for the ticket and canonical work directory.

The isolated transaction uses `.lisa-commit.lock`, an alternate index,
`commit-tree`, and an atomic `update-ref`.

Failure restores the exact original non-Done ticket bytes.

A completion-key trailer makes replay idempotent after a lost command result.

The command prints a 40- or 64-hex commit ID on success.

`handle_completion_result` verifies that shape and then rescans durable Done
frontmatter before appending the journal confirmation.

Commit semantics must remain unchanged for a pinned commit seal.

## Durable completion journal

`crates/lisa-plugin/src/completion_journal.rs` owns `.lisa/completion-journal.jsonl`.

Schema version 3 currently supports requested, command-in-flight,
failure-observed, rejected, and confirmed rows.

Every new row already contains the pinned `seal` field.

Rows without `seal` default to `CompletionSeal::Commit` for pre-ladder history.

The journal is not appended in place.

Each append reads and folds all existing rows, validates the new transition,
serializes the complete new body, and atomically replaces the file through a
sibling temporary.

The fold rejects torn final records, malformed JSON, invalid transition order,
mixed seals inside one generation, and inconsistent failure counters.

`CompletionJournalAggregate` retains the completion key, seal, reducer state,
prior phase/status, optional confirmed commit ID, and bounded failure facts.

The confirmed row currently requires a non-empty `commit_id`.

There is no receipt representation for journal hashes yet.

The schema can evolve additively because deserialization already carries
legacy-version compatibility and defaulted fields.

## Logical publication atomicity

One filesystem rename can atomically publish one file but cannot atomically
replace both the ticket and the completion journal.

The existing completion protocol supplies the cross-file safety boundary.

`Requested` and `CommandInFlight` are durable before Done can appear.

`State::mask_completion_transaction` projects the retained prior phase/status
whenever an aggregate remains unresolved but the ticket file contains Done.

The mask is applied both for a live pending command and after journal replay on
restart.

Consequently an interruption after an atomic ticket flip but before a confirmed
row remains fail-closed: the scheduler does not expose the ticket as completed.

Reconciliation can retry the exact generation until it produces the missing
confirmation or reaches the bounded failure policy.

This is the same observable discipline used around the commit command result,
without requiring a multi-file filesystem transaction.

## File publication primitive

`crates/lisa-plugin/src/publication.rs` centralizes sibling-temporary writes.

`RustPublication` writes complete bytes to a sibling temporary and renames it
over the destination.

Rename failures remove the temporary and preserve the prior destination.

Temporary-name policies reject directory traversal and non-sibling paths.

Tests cover replacement, rename failure, hostile filenames, long paths, and
temporary cleanup.

The helper does not create parent directories, serialize payloads, or coordinate
multiple destinations.

The journal already uses this primitive for its complete JSONL publication.

The journal seal can use the same primitive for prepared Done ticket bytes.

## Ticket frontmatter update

`lisa_core::ticket::update_ticket_done` changes both `status` and `phase` to
`done`.

The function operates on a path and writes the changed document to that path.

It does not itself publish through a sibling rename.

Commit completion can tolerate the direct write because a failed transaction
restores the original bytes and Git supplies the final atomic authority.

Journal completion needs to prepare Done bytes away from the live ticket and
then publish those complete bytes through `RustPublication`.

The existing function can operate on a private preparation file, after which
the plugin can read the prepared bytes and atomically replace the real ticket.

## Canonical work artifacts

Review phase artifacts begin in the attempt-private directory.

Lisa admits phase artifacts to `PluginConfig.work_dir/<ticket-id>` only after
validating the current lease.

Completion eligibility already requires the current private Review evidence
and valid pass disposition.

The completion transaction includes the canonical work directory, not the
attempt-private directory.

Journal hashes therefore need to walk the canonical ticket work directory.

"Every artifact under" includes nested files, so a recursive traversal is
required rather than a fixed six-artifact list.

Stable recorded paths should be relative to the project root where possible,
so rows remain inspectable without embedding host-specific absolute prefixes.

Sorted paths make fixtures and audits deterministic.

Directories themselves have no content hash; regular file bytes are the
content-bearing records.

Failures to enumerate a directory, inspect an entry, or read a regular file are
all ways the required hash set cannot be computed and must be named.

## SHA-256 support

The workspace already uses `sha2` version 0.10 in `lisa-cli`.

The package and its transitive dependencies are already present in Cargo.lock.

`lisa-plugin` does not currently depend on `sha2`.

Adding the same dependency to the plugin avoids a second implementation and
does not introduce a new lockfile package family.

Hashes are conventionally rendered as lowercase 64-character hexadecimal
strings in the existing CLI runtime code.

## Completion success handling

`State::handle_completion_result` currently combines command-result validation,
durable Done verification, confirmation journaling, scheduler cleanup,
provenance, seat release, and dependent scheduling.

Its activity text and local names assume every success has a commit ID.

Journal completion has no honest commit ID.

The common post-publication path can instead accept a typed seal receipt,
persist that receipt, and render seal-specific activity text.

Commit callers can continue validating the exact existing stdout contract.

Journal callers can complete synchronously after the in-flight row because the
plugin already performs filesystem reads and atomic publications.

The same pending authority and correlation remain available to both branches.

## Failure and recovery behavior

T-049-04-01 classifies failed native completion commands and bounds retries.

A journal hashing or publication error occurs inside the plugin rather than as
native stderr, so it needs an equivalent named rejection path.

The core acceptance requirement is immediate fail-closed blocking of completion,
not fabrication of a partial hash row.

Leaving the aggregate in flight while reporting the exact path-specific error
preserves the mask and enables the existing deadline/reconciliation machinery.

No confirmed journal row may be written until every required byte is read, the
Done ticket bytes are ready, and the Done ticket is atomically visible.

A later replay is idempotent because applying Done frontmatter again yields the
same completed ticket content and recomputes the full current hash set.

## Provenance boundary

`emit_provenance` writes the existing execution record after confirmation.

Those records already carry `seal` from pinned plugin configuration.

This ticket requires the hash set in the completion journal row, not a duplicate
hash set in provenance.

No provenance schema change is required for the acceptance criteria.

## Relevant test patterns

Completion-journal unit tests inspect exact JSONL fields and reconstruct state
after reopening the file.

Scheduler tests create temporary ticket/work trees, dispatch completion, and
inspect durable ticket, journal, provenance, pending state, and dependents.

Commit-path tests verify nested project paths, exact work retention, original
ticket restoration, completion-key replay, and unrelated ordinary-index state.

Publication tests simulate hostile filenames and rename failures while checking
that destinations remain complete and sibling temporaries are absent.

The ticket adds three distinctive journal fixtures: successful repo-less
completion with independently recomputed hashes, post-seal artifact mutation
showing a stale recorded digest, and an unreadable/unhashable artifact with a
named failure.

The unreadable case should not depend solely on Unix permission bits because
privileged test runners can still read mode-000 files.

A filesystem object that cannot be read as a regular artifact or an injectable
test reader provides a deterministic failure boundary.

## Worktree and ownership constraints

The ordinary worktree already contains Lisa-owned journal/ticket edits and
unrelated T-049-02-01 source changes.

None of those changes belongs to this ticket.

Likely ticket-owned source paths are `crates/lisa-core/src/completion.rs`,
`crates/lisa-plugin/src/completion_journal.rs`, `crates/lisa-plugin/src/lib.rs`,
and `crates/lisa-plugin/Cargo.toml`.

Cargo.lock needs no package update if the plugin reuses the already locked
`sha2` version.

Source units must be committed only through `lisa commit-ticket` with exact
repository-relative includes.

Phase artifacts remain under this attempt-private work directory for Lisa to
admit and publish.
