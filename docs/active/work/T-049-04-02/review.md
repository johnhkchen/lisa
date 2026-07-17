# Review — T-049-04-02

## Disposition

Pass.

The implementation satisfies all ticket acceptance criteria and is ready for
Lisa's completion transaction.

## Reviewed commits

- `148523b076a7fa5385de987a1617e84fdf66706c`
  — `fix(cli): clean completion locks and handle unborn history`
- `49f98a36cb24ea4500536e5b6ad9373c34befec1`
  — `test(plugin): replay the bounded completion incident`
- `8a69720001c169365aea15e7dd6b73a16827df3b`
  — `test(cli): make stale lock age assertion robust`

Each commit was created with `lisa commit-ticket` and an exact
repository-relative include path.

The production commit owns only
`crates/lisa-cli/src/commit_transaction.rs`.

The field replay commit owns only test additions in
`crates/lisa-plugin/src/lib.rs`.

The final small commit stabilizes one CLI test assertion.

## Change summary

The commit transaction now separates serialization from the visible lock
marker.

A persistent advisory guard under the Git directory remains the stable inode
that updated Lisa processes serialize on.

The root `.lisa-commit.lock` is now ephemeral and owner-described.

It carries JSON with schema version, PID, and acquisition time.

Updated processes acquire the guard first and the visible marker second.

The second advisory lock preserves compatibility with older Lisa processes and
external fixtures that know only the root path.

Every owned completion path removes the visible marker before releasing the
stable guard.

The explicit finish path and RAII Drop path share cleanup logic.

Cleanup always attempts marker removal, marker unlock, and guard unlock, then
combines any errors.

An existing marker whose recorded PID is absent is diagnosed as stale.

The returned reason names its age, PID, absence/no-such-process fact, and
successful recovery.

A live holder's marker is never truncated, removed, or stolen.

Completion-key discovery now explicitly detects a symbolic HEAD whose branch
ref does not exist.

That exact condition returns `Ok(None)`.

All other discovery failures remain strict.

After empty discovery, the transaction reaches and names the real Tier-1
parent-commit precondition.

The field replay reconstructs an unborn, identity-less repository and feeds
actual CLI transaction errors into the production scheduler failure policy.

## Acceptance criterion: lock hygiene

Pass.

Representative success and failure tests now assert the visible marker is
absent after the transaction owns it.

Covered paths include:

- successful ticket commit;
- successful full completion commit;
- idempotent prior-key short-circuit;
- no requested changes;
- staged-path overlap;
- commit identity failure;
- completion rollback and ticket-byte restoration;
- nested-project completion;
- compensating rollback;
- unborn-HEAD precondition failure.

The `finish` implementation is not dependent on one control-flow branch.

Any future `?` or unwind after acquisition still reaches Drop cleanup.

The stale fixture writes a real owner record for a child process that has been
reaped.

It dates that record approximately 120 seconds in the past.

The first call asserts the reason names stale state, measured age, exact PID,
absent holder, no such process, and recovery.

The root marker is absent after that failure.

The second call succeeds, proving the transaction is recovered rather than
merely diagnosed.

The live fixture writes owner metadata, takes the advisory lock, and snapshots
the marker.

A competitor returns a live-holder error.

The bytes and pathname remain unchanged until the fixture owner releases them.

This proves a live holder is respected.

The repository's own legacy empty marker was also encountered during the first
source transaction.

The new binary reported its age, reported that no owner was recorded, removed
it, and succeeded on the bounded retry.

No root marker remains after final verification.

## Acceptance criterion: empty-history discovery

Pass.

The new test directly calls `discover_completion_commit` on an initialized
repository with no commits.

It receives `Ok(None)`.

The subsequent full completion attempt fails at `resolve HEAD for ticket
commit`, not at `discover prior completion commit`.

The error retains the explicit “current branch does not have any commits yet”
fact used by the bounded history/identity classifier.

The failed completion restores the exact original ticket bytes and removes its
visible lock marker.

The preexisting repeated-key fixture still creates a history containing the
generation trailer, replays the same key, and receives the earlier commit ID
without creating another commit.

A later different generation remains independent.

The fix therefore changes only empty current-branch history and preserves
idempotency for born repositories.

## Acceptance criterion: preserved field replay

Pass.

The regression embeds the exact preserved journal named by the ticket.

It parses the source and requires exactly 80 T-001 rejection rows carrying the
old discovery failure and unborn-branch reason.

The reconstructed repository is initialized but has no HEAD commit.

Local name and email are explicitly empty.

The fixture verifies both `rev-parse --verify HEAD` and
`git var GIT_AUTHOR_IDENT` fail before the replay begins.

The first actual CLI transaction returns the real HEAD precondition error.

Scheduler handling records failure 1/2 and launches exactly one retry.

The second actual CLI transaction returns the same class of real error.

Scheduler handling records failure 2/2 and parks.

The regression asserts:

- exactly two command launch effects total;
- exactly two `failure-observed` journal rows;
- first consequence `retry-scheduled`;
- second consequence `park`;
- exactly one Park provenance record;
- retry count and limit both persisted as 2/2;
- the canonical block is structured and operator-owned;
- the ask equals `HISTORY_IDENTITY_ASK` exactly;
- the ticket is Review/Blocked;
- the thread and seat are released;
- no pending completion remains;
- later reconciliation launches no third attempt;
- `.lisa-commit.lock` is absent after each real CLI failure and at the end.

The old 80-rejection churn is therefore closed by an executable reproduction,
not a string-only classifier fixture.

## Correctness review

### Serialization safety

Pass.

The stable guard removes the replacement-inode race that would result from
deleting the only lock inode.

Marker removal happens while the stable guard is held.

New processes cannot concurrently create or inspect another marker.

The visible advisory lock keeps the live-old-process boundary conservative.

### Stale ownership policy

Pass.

Only `ESRCH` proves a recorded PID absent on Unix.

`EPERM` remains present, avoiding a false takeover when the caller lacks
permission to signal the process.

Unknown liveness is not auto-stolen.

The marker's age is reported as evidence but is not used to override a live
holder.

### Discovery scope

Pass.

The implementation did not add `git log --all`.

Prior-key discovery remains scoped to current HEAD ancestry.

It did not convert arbitrary exit 128 failures to absence.

Corruption and other real Git failures still fail closed.

### Transaction rollback

Pass.

No ordinary-index, alternate-index, ref-update, compensating rollback, or
ticket-byte restoration semantics were weakened.

All existing isolation assertions continue to pass.

## Test coverage

Passed focused CLI transaction tests.

Passed the focused field replay.

Passed completion-failure policy tests.

Passed the complete lisa-cli suite.

Passed `cargo test --workspace` with all executed native tests green.

The final workspace run included 422 plugin tests and the new replay.

The existing real-Zellij test remains ignored under its declared external
environment requirements; this ticket adds no dependency on it.

Passed `cargo check -p lisa-plugin --target wasm32-wasip1`.

Passed `just check`, including a second workspace run.

Passed formatting and whitespace checks.

Passed warning-denying clippy for the CLI library.

Passed warning-denying clippy for the plugin WASM library.

## Concurrency and ownership review

Pass.

T-049-03-01 concurrently modified the plugin coordinator during this ticket.

The overlap was detected before either ticket used a broad commit.

This ticket first committed its independent CLI module.

Its plugin test hunks were then removed while T-049-03-01 published its exact
units.

The test was reapplied and verified against that new baseline before its own
exact-path commit.

No other ticket's uncommitted source entered these commits.

The ordinary index is empty.

Remaining worktree state is Lisa-owned bookkeeping or another active ticket.

## Open concerns

No blocking concerns.

The stable `<git-dir>/lisa-commit.guard` inode intentionally persists.

It is repository metadata, not the operator-visible transaction marker.

On stale recovery the transaction returns one named failure after cleaning the
marker instead of silently continuing.

This is intentional: it provides the operator-facing reason required by the
ticket and lets the existing bounded scheduler account for the recovery.

PID liveness recovery is Unix-specific.

On non-Unix targets, parsed owners are treated conservatively as unknown rather
than stolen; Lisa's Zellij host/runtime and CI targets are Unix-like.

## Final assessment

The incident's transaction residue is now attributable and self-cleaning, an
unborn repository no longer fails in idempotency discovery, and the exact field
environment converges from 80 rejections to one bounded retry sequence and one
plain-language park.

Disposition: pass.
