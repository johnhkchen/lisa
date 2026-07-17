# Review — T-049-03-02

## Review outcome

Ready to complete. The completion ladder now has executable proof at both honesty boundaries,
and the Chromebook protocol has a scripted, independently gradeable no-Git leg. No production
downgrade path was added because the existing architecture already pins the resolved tier and
parks native completion failures.

The ticket's manual field leg was intentionally not run. The ticket explicitly owns the fixture
flag and runbook diff, while authenticated execution remains a separately budgeted operator act.

## Acceptance criterion 1 — enforce mode and no silent switch

### Explicit commit plus missing identity

Covered by the compiled CLI integration fixture
`doctor_explicit_commit_uses_shared_missing_identity_hard_failure`.

It creates an identityless Git repository, configures explicit commit completion, invokes the
real CLI doctor command, and asserts:

- hard nonzero exit;
- named completion-seal preflight failure;
- explicit `[guards].completion = "commit"` context;
- the missing identity reason;
- exact `git config user.name` and `git config user.email` remedies; and
- the `lisa init` history-offer alternative.

This fixture passed both focused and full integration runs.

### Auto-resolved commit plus mid-run Git breakage

Added
`auto_pinned_commit_with_mid_run_repository_loss_parks_without_journal_seal` in
`crates/lisa-plugin/src/lib.rs`.

The test resolves auto against available commit support, pins commit, starts with a real healthy
temporary repository, dispatches completion, deletes only the temporary `.git`, and invokes the
real native completion transaction. It then verifies that the plugin:

- keeps the pinned tier at commit;
- records the real repository discovery failure;
- parks the ticket in blocked Review;
- publishes a nonempty operator ask;
- releases the occupied scheduler seat;
- records commit-labeled failure/rejection evidence; and
- writes zero journal-sealed/confirmed/content-hash rows.

This directly covers the temporal scenario named by the ticket. The transaction does not get a
second chance to resolve auto, and no code path switches to journal.

## Acceptance criterion 2 — no-Git Chromebook leg

### Fixture flag

`docker/chromebook-test/bin/prepare` now accepts `--no-git`.

The flag:

- requires Git absence before measurement;
- refuses to overwrite prior evidence;
- creates `~/no-git-demo` without `.git`;
- seeds fixed ticket `T-NOGIT-001`;
- makes that ticket evidence-only and explicitly source-free;
- retains the live README install section in the tested prompt;
- requires `lisa init --no-history`;
- requires the matching authenticated Lisa client;
- requires a real `lisa loop` through Done; and
- stamps `no_git: 1` into leg metadata.

The evidence-only task is important. Lisa's ordinary workflow tells agents to commit meaningful
source units, but this ticket has no source unit; all work is phase artifacts that Lisa can
journal-seal. The field leg therefore tests repository-less completion without asking the agent
to perform an impossible repository transaction.

### Independent grading

`docker/chromebook-test/bin/grade` recognizes the metadata marker and grades the actual project.
PASS requires:

- Git absent and no project `.git`;
- project-local doctor exit zero;
- exact quoted line `completion seal: journal-only — finished work is recorded but not
  undoable`;
- fixed ticket status and phase both Done;
- a confirmed `seal: journal` row for the fixed ticket;
- no `commit_id`;
- a nonempty content-hash array;
- safe, unique, project-relative paths;
- lowercase 64-hex SHA-256 values;
- every digest recomputed against current bytes;
- an explicit binding for the final ticket;
- no compiler/source-build negatives; and
- completion within the named 1,200-second full-loop hard stop.

The existing 600-second install-only bound remains unchanged for ordinary legs. The record names
which bound applied, preventing the full RDSPI workload from silently changing the install score.

### Evidence collection

`just cbt-collect` now copies only these additional project paths when the no-Git journal exists:

```text
no-git-demo/.lisa/completion-journal.jsonl
no-git-demo/docs/active/tickets/T-NOGIT-001.md
no-git-demo/docs/active/work/T-NOGIT-001/
```

Authentication directories and the rest of the home directory remain excluded.

### Standing runbook

`docs/knowledge/chromebook-install-test.md` now:

- lists `prepare --no-git` in the ritual;
- adds metered leg N to the matrix;
- narrows ordinary install claims and names the new live-loop claim;
- documents the prepared project and exact task;
- states that this ticket does not run the leg;
- quotes the exact journal-only doctor line;
- requires confirmed hash evidence and no commit id;
- documents the 20-minute hard stop;
- documents sanitized evidence collection; and
- adds seal/hash fields to the run-record template.

Together the flag, grader, collector, and prose make “a bare folder finishes its tickets” a
scored sentence rather than an informal observation.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

- Added one scenario test only.
- No production scheduler/completion behavior changed.

### `docker/chromebook-test/bin/prepare`

- Added no-Git flag, fixed fixture ticket, prompt, invariant checks, and metadata.

### `docker/chromebook-test/bin/grade`

- Added no-Git branch, exact seal capture, Done checks, and complete JSONL/hash verifier.
- Updated noninteractive normal init to pass the current explicit no-history choice.

### `justfile`

- Added path-specific no-Git evidence collection.

### `docs/knowledge/chromebook-install-test.md`

- Added the full no-Git leg protocol and scoring rules.

No files were deleted. No schema, dependency, public API, or Docker package changed.

## Ticket commits

```text
5b23903 Test pinned commit failure without downgrade
66aeabc Add scripted no-Git completion fixture
1526eeb Score repository-less completion in Chromebook protocol
```

Every commit used `lisa commit-ticket --ticket-id T-049-03-02` with exact includes. No ordinary
index staging or ordinary commit command was used for ticket work.

## Test coverage

### Passed

- Focused explicit commit/missing identity compiled CLI test: 1/1.
- Focused mid-run repository-loss plugin test: 1/1.
- `cargo test -p lisa-cli --test seal_visibility`: 5/5.
- `cargo test -p lisa-plugin`: 423/423.
- `cargo test --workspace`: all tests and doc tests, zero failures.
- `cargo fmt --all -- --check`.
- POSIX shell syntax for prepare and grade.
- Node syntax for the embedded verifier.
- Just recipe parsing/listing.
- Repository whitespace diff check.
- Synthetic verifier positive case.
- Synthetic post-seal mutation negative case.

### Coverage strengths

- The explicit preflight case crosses the compiled CLI boundary.
- The mid-run case crosses the pure resolver, real native transaction, plugin adapter, durable
  journal, ticket state, disposition, seat, and provenance boundaries.
- Existing journal completion tests independently cover hashing the final ticket and all nested
  artifacts.
- The field verifier recomputes bytes rather than trusting row shape.
- Normal and no-Git grader paths remain visibly distinct.

### Not executed

- Docker image rebuild.
- Authenticated Claude/Codex no-Git leg.
- Live release download during that leg.
- Real 20-minute metered completion run.

These omissions match the ticket's explicit scope. The runbook makes the future manual execution
and resulting evidence unambiguous.

## Open concerns and limitations

### Manual field evidence remains pending

The scripts are syntactically and logically verified, but only the real leg can establish how a
current low-end authenticated CLI follows the longer nested-loop instruction. Failure of that
future leg should produce a new product or protocol ticket; it does not invalidate the delivered
instrument.

### Same-client instruction is agent-visible, not prewritten config

Prepare runs before `/cbt/run` selects the CLI, so the measured prompt tells the agent to set the
Lisa client to the CLI conducting the leg. This preserves one prepare flag and works for either
provider, but it is an observed agent action. The grader's full completion requirement catches a
failure to configure it.

### Field-only verifier has no permanent shell test file

The embedded Node verifier was extracted and exercised through a temporary harness during
implementation. Its positive and mutation-negative paths passed. Keeping it embedded avoids a
second implementation drifting from the actual field grader.

### Existing justfile formatter drift

`just --fmt --check` reports repository-wide spacing differences that predate this ticket. The
modified recipe parses under `just --list`, and ticket diffs pass whitespace checks. This ticket
did not reformat unrelated recipes.

### Concurrent shared-file edit

After this ticket committed its isolated test, T-049-05-01 began editing other functions in the
same `crates/lisa-plugin/src/lib.rs` worktree file. The changes do not overlap this test and were
not staged or committed by this ticket. The workspace suite passed with them present. The
ticket-owned test itself is durable in `5b23903`.

## Final assessment

The acceptance criteria are satisfied. Enforce mode fails hard with remedy, a pinned commit run
cannot journal-downgrade after Git breakage, and the standing Chromebook protocol can now produce
and independently verify a real repository-less completion record. The implementation is ready
for Lisa's completion publication.
