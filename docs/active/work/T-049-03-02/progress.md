# Progress — T-049-03-02

## Outcome

Implementation is complete. The enforcement boundaries are covered, the no-Git Chromebook
fixture and grader are scripted, the standing runbook scores the claim, all ticket-owned changes
are durable through Lisa commits, and automated verification is green.

The authenticated/manual Chromebook leg was not executed, as required by the ticket boundary.

## Completed phases

- [x] Research mapped seal resolution, loop pinning, plugin completion, journal evidence,
  parking, and the Chromebook scripts.
- [x] Design selected boundary tests rather than duplicate production branches.
- [x] Structure fixed the five owned source/document paths and three commit units.
- [x] Plan sequenced scenario coverage, fixture changes, documentation, and verification.
- [x] Implement completed every planned source unit.
- [x] Review artifacts prepared after final automated verification.

## Baseline enforcement verification

Before editing source, ran the existing compiled CLI fixture:

```text
cargo test -p lisa-cli --test seal_visibility \
  doctor_explicit_commit_uses_shared_missing_identity_hard_failure -- --exact --nocapture
```

Result:

```text
1 passed; 0 failed
```

The fixture creates an actual identityless repository under explicit
`[guards].completion = "commit"`. It requires nonzero doctor status plus:

- `Completion seal preflight failed`;
- the explicit guard spelling;
- the missing identity reason;
- the two-line `git config` remedy; and
- the alternative `lisa init` history offer.

No new preflight production branch was needed.

## Enforcement scenario implementation

Modified `crates/lisa-plugin/src/lib.rs` with:

```text
auto_pinned_commit_with_mid_run_repository_loss_parks_without_journal_seal
```

The scenario:

1. resolves `Auto + CommitSealSupport::Available` through the public core resolver;
2. pins the resulting commit tier into plugin config;
3. initializes a real temporary Git repository with local identity and HEAD;
4. dispatches a real Review completion effect;
5. removes only that temporary fixture's `.git` directory;
6. invokes the real native `complete_ticket` transaction;
7. feeds the repository-discovery failure to the plugin result boundary;
8. verifies Review/blocked parking, seat release, and an operator ask; and
9. verifies the journal has only commit-labeled failure/rejection rows.

Negative evidence asserted explicitly:

- no `seal: journal` row;
- no confirmed row;
- no content hashes;
- no commit id;
- no Done ticket.

Focused test result:

```text
1 passed; 0 failed
```

Commit:

```text
5b23903 Test pinned commit failure without downgrade
```

Exact include:

```text
crates/lisa-plugin/src/lib.rs
```

## No-Git fixture implementation

Modified `docker/chromebook-test/bin/prepare`:

- added `--no-git` parsing and help;
- fails if Git is available before the measured leg;
- refuses to overwrite an existing `~/no-git-demo`;
- creates the fixed `T-NOGIT-001` evidence-only ticket;
- asserts no `.git` entry exists;
- writes a measured instruction that keeps Git absent;
- directs the tested agent through `lisa init --no-history` and a real `lisa loop`;
- requires the Lisa client to match the authenticated outer CLI;
- records `no_git: 0|1` in leg metadata.

The fixture ticket explicitly requires phase artifacts but prohibits source changes. This avoids
inventing a source commit obligation in the repository-less scenario while still exercising all
RDSPI phases and final completion.

Modified `docker/chromebook-test/bin/grade`:

- detects no-Git metadata;
- grades the actual `~/no-git-demo` project;
- keeps 600 seconds for install-only legs and uses 1,200 seconds for the full no-Git loop;
- captures the exact doctor completion line;
- requires exact journal-only copy;
- requires Git and `.git` absence;
- requires `T-NOGIT-001` Done frontmatter;
- parses completion JSONL with Node;
- selects the final confirmed journal row for the fixed ticket;
- rejects commit evidence, missing hashes, malformed digests, duplicate paths, absolute paths,
  traversal, and project escapes;
- recomputes every SHA-256 binding;
- requires the final ticket binding;
- writes the verifier summary into the run record.

The normal grader's noninteractive init now uses the current explicit
`lisa init --no-history` contract.

Modified `justfile`:

- `cbt-collect` detects the fixed no-Git journal;
- copies only the completion journal, fixed ticket, and fixed work directory;
- retains existing run-record/tour/docker-diff collection;
- never copies `.claude`, `.codex`, or a whole home directory.

Commit:

```text
66aeabc Add scripted no-Git completion fixture
```

Exact includes:

```text
docker/chromebook-test/bin/prepare
docker/chromebook-test/bin/grade
justfile
```

## Standing protocol implementation

Modified `docs/knowledge/chromebook-install-test.md`:

- added the `prepare --no-git` ritual command;
- separated ordinary install/dry-run claims from the full completion claim;
- added manual metered leg N to the matrix;
- documented the prepared directory and fixed ticket;
- documented the same-client requirement;
- stated explicitly that this ticket does not execute the leg;
- added the 1,200-second full-loop hard stop;
- made Git absence a scored condition;
- quoted the exact doctor journal-only line;
- required Done frontmatter and a confirmed journal row;
- required recomputation of every content hash and absence of commit id;
- documented sanitized collected evidence paths;
- extended the run-record template with wall limit, seal line, and hash verification.

Commit:

```text
1526eeb Score repository-less completion in Chromebook protocol
```

Exact include:

```text
docs/knowledge/chromebook-install-test.md
```

## Automated verification

### Plugin package

```text
cargo test -p lisa-plugin
```

Result:

```text
423 passed; 0 failed
```

This includes the new temporal breakage scenario and existing repository-less hash-seal tests.

### Compiled CLI seal fixtures

```text
cargo test -p lisa-cli --test seal_visibility
```

Result:

```text
5 passed; 0 failed
```

This includes explicit commit/missing identity and all auto/commit/journal visibility cases.

### Workspace

```text
cargo test --workspace
```

Result: all unit, integration, and doc tests passed with zero failures.

### Formatting and fixture syntax

Passed:

```text
cargo fmt --all -- --check
sh -n docker/chromebook-test/bin/prepare
sh -n docker/chromebook-test/bin/grade
node --check <extracted embedded verifier>
just --list
git diff --check
```

The embedded verifier was also exercised against a generated matching confirmed journal row and
then against a post-seal mutation:

```text
matching evidence accepted
mutated Review artifact rejected with SHA-256 mismatch
```

`just --fmt --check` was attempted but is not a usable ticket-local gate: the repository's
existing justfile differs globally from the current formatter's spacing style. The file parses
successfully with `just --list`, and the ticket diff passes whitespace validation. The ticket did
not mechanically reformat unrelated recipes.

## Deviations from plan

- No production completion code changed because Research confirmed both hard enforcement and
  pinned-tier parking already exist.
- The scenario composes the public pure auto resolver with plugin config instead of launching
  Zellij; it then crosses the real native transaction and plugin result boundaries.
- The synthetic hash-verifier test was implemented as a temporary Node harness rather than a
  persistent test file, keeping the field-only grader free of repository test scaffolding.
- `just --fmt --check` was replaced by parser evaluation for the reason above.

## Shared-worktree note

After commit `5b23903`, active ticket `T-049-05-01` began a separate uncommitted edit in
`crates/lisa-plugin/src/lib.rs`. Its diff changes attempt high-water/orphaned-block functions and
does not overlap this ticket's committed test block. This ticket did not stage, revert, include,
or otherwise consume that work. The full workspace suite passed while the concurrent diff was
present.

All changes authored by T-049-03-02 are committed through Lisa. The remaining modified ledger,
ticket-state, work-artifact, and concurrent source paths belong to Lisa or other active tickets.

## Remaining manual action

Run leg N in a fresh authenticated Chromebook fixture when field tokens are intentionally
budgeted:

```text
/cbt/prepare --no-git
/cbt/run claude|codex <exact-low-end-model-id>
/cbt/grade
just cbt-collect <container-name>
```

That execution is the future evidence record, not a blocker for this ticket, whose acceptance
scope explicitly ships the runbook diff and fixture flag rather than the run.
