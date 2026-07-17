# Structure — T-049-03-02

## Change set overview

This ticket changes no public Rust API and adds no production completion branch. The source
shape is one Rust scenario test plus the Chromebook fixture/runbook surface.

Owned repository paths:

- `crates/lisa-plugin/src/lib.rs`
- `docker/chromebook-test/bin/prepare`
- `docker/chromebook-test/bin/grade`
- `docs/knowledge/chromebook-install-test.md`
- `justfile`

Existing verification-only path:

- `crates/lisa-cli/tests/seal_visibility.rs`

Attempt-private artifacts:

- `.lisa/attempts/T-049-03-02/1/work/research.md`
- `.lisa/attempts/T-049-03-02/1/work/design.md`
- `.lisa/attempts/T-049-03-02/1/work/structure.md`
- `.lisa/attempts/T-049-03-02/1/work/plan.md`
- `.lisa/attempts/T-049-03-02/1/work/progress.md`
- `.lisa/attempts/T-049-03-02/1/work/review.md`
- `.lisa/attempts/T-049-03-02/1/work/review-disposition.json`

## Rust test structure

### Location

Add one `#[test]` beside the existing completion failure fixtures in
`crates/lisa-plugin/src/lib.rs`.

This location already has access to:

- `completion_failure_fixture`;
- `install_current_attempt` and Review fixture helpers;
- `State::dispatch_completion`;
- `State::handle_completion_result`;
- ticket scanning and disposition parsing;
- the completion journal path;
- the native `lisa_cli::commit_transaction::complete_ticket` boundary.

No new test module or fixture file is required.

### Scenario name

Use a name that records both temporal facts and the forbidden fallback, for example:

`auto_pinned_commit_with_mid_run_repository_loss_parks_without_journal_seal`

### Setup block

The test will:

1. create `completion_failure_fixture("T-PINNED-COMMIT")`;
2. call the public core resolver with `Auto` and commit support `Available`;
3. assign the resolved `Commit` tier to `state.config.completion_seal`;
4. initialize Git inside the fixture temp root;
5. configure a local test name and email;
6. stage and commit the fixture files for a valid initial history.

The Git setup stays local to the temporary directory. A small closure may run commands and
assert their success. It is test-local rather than a reusable production helper.

### Completion block

The test dispatches a Reconcile completion while Git is healthy. That must create a pending
commit effect and journal requested/in-flight state.

It then deletes exactly `dir.path().join(".git")`. The path is explicit, temporary, and asserted
to be a directory before removal.

Construct `CompleteTicketRequest` using:

- the fixture root as `repo_root`;
- `T-PINNED-COMMIT` as ticket id;
- the ticket file relative path;
- the canonical work directory relative path;
- generation 1 for the current attempt lease.

The native call must return an error. Feed `Error: {error}` into the plugin result handler with
nonzero exit.

### Assertions block

Scheduler and ticket assertions:

- pinned tier is `CompletionSeal::Commit` before and after breakage;
- completion was dispatched while the repository existed;
- no pending completion remains after the failure;
- no active thread remains for the ticket;
- the owned seat is released;
- ticket phase is Review;
- ticket status is Blocked;
- Review disposition is Block with a nonempty reason/ask.

Journal assertions:

- contains a failure-observed row;
- contains an action-required rejection;
- contains the actual repository discovery failure;
- all serialized seal labels are commit;
- contains no `"seal":"journal"`;
- contains no `"state":"confirmed"`;
- contains no `content_hashes`;
- contains no `commit_id`.

This directly proves zero journal-sealed rows for the affected ticket.

## Prepare script structure

### Flag parsing

Add integer state `NO_GIT=0` near the existing variant states.

Recognize:

```text
--no-git
```

Update both usage lines and the descriptive flag list. The flag remains combinable with
`--pin`; it is independent of ancient-Zellij/XDG variants, though the runbook recommends a
fresh uncombined leg.

### Project constant

The no-Git project path is fixed:

```text
$HOME/no-git-demo
```

The fixed ticket id is:

```text
T-NOGIT-001
```

These constants are shared by prose and grader expectations, not exposed as environment
configuration.

### Fixture creation

Only when `NO_GIT=1`:

- require `git` to be absent from PATH;
- refuse to overwrite an existing `~/no-git-demo`;
- create `docs/active/tickets` below it;
- write one ready, dependency-free evidence-only ticket;
- assert `.git` is absent.

The ticket's acceptance criteria require only phase-artifact content. It explicitly prohibits
source changes, so the agent never has a meaningful Implement source unit that would require
`lisa commit-ticket` in a repository-less project.

### Instruction creation

Normal and discovery instructions keep their current content.

No-Git instruction uses the fetched README install section and adds exact actions:

- install Lisa;
- do not install Git;
- enter the prepared project;
- run `lisa init --no-history`;
- configure `[agent].client` to the same CLI conducting the leg;
- run `lisa loop` through Done for `T-NOGIT-001`;
- require project-local doctor success.

The instruction never mentions `/cbt` or the grader.

### Metadata

Append:

```text
no_git: 0|1
```

to `/tmp/leg-meta`. The grader treats this as the variant selector.

## Grader structure

### Variant selection

Near startup, derive:

```text
no_git=0|1
project_root=$HOME/demo|$HOME/no-git-demo
wall_limit=600|1200
```

Use exact metadata matching (`^no_git: 1$`). Older prepared legs without the field remain normal
legs.

### Shared positive checks

Keep these common:

- Lisa on login PATH;
- doctor invocation;
- init/validate/dry-run health;
- resource measurements;
- prohibited compiler checks;
- apt history capture;
- run record generation.

Normal mode continues creating `~/demo`. Its noninteractive init command becomes explicit
`lisa init --no-history`, matching the current CLI contract.

No-Git mode uses the agent-completed project rather than creating a second smoke project.

### Seal capture

Capture the exact first line containing `completion seal:` from doctor output into
`doctor_seal_line`.

For no-Git mode, require exactly:

```text
completion seal: journal-only — finished work is recorded but not undoable
```

Record this full line, not a paraphrase.

### Ticket assertions

For `T-NOGIT-001.md`, require exact frontmatter lines:

- `status: done`
- `phase: done`

Require the project and its parents within the fixture boundary to have no `.git` entry, and
require `command -v git` to fail.

### Embedded Node verifier

Add one heredoc-driven Node program inside the no-Git branch. Inputs:

- project root from argv;
- fixed journal relative path;
- fixed completion id.

Algorithm:

1. read nonempty JSONL lines;
2. parse each line as JSON;
3. select confirmed, journal-sealed rows for `T-NOGIT-001`;
4. choose the final matching row;
5. reject a present commit id;
6. require a nonempty `content_hashes` array;
7. validate each binding's path and lowercase 64-hex digest;
8. reject absolute paths and any `..` component;
9. resolve the path and prove it remains below project root;
10. reject duplicate bindings;
11. read every bound file and recompute SHA-256;
12. require a binding for the final ticket file;
13. print a one-line confirmation with attempt, generation, and binding count.

The shell captures verifier stdout/stderr and exit code. Nonzero status calls `failhard`; success
calls `note`. The exact summary is included in the run record.

### Record additions

Add these lines:

- applied wall limit;
- exact doctor completion-seal line;
- journal verification summary or failure.

Normal records use `(not a no-git leg)` for journal verification.

## Runbook structure

### Scripted ritual

Add `/cbt/prepare --no-git` to the command synopsis and explain that it is a full completion
variant rather than an install-only smoke.

### Claims section

Expand “What a run proves” narrowly: only the scored no-Git leg proves that a bare folder can
finish a ticket with journal evidence. Ordinary install legs still do not prove live loop
quality.

### Matrix

Add a named N leg:

- one selected low-end authenticated CLI;
- bookworm primary fixture;
- no Git;
- auto resolution to journal;
- full ticket completion.

The leg is manual and metered. This ticket does not execute it.

### Preparation and instruction

Document the exact command and seeded project. Explain the evidence-only task rationale and
same-client configuration requirement.

### Acceptance

Add a separate no-Git pass subsection requiring:

- Git absent before and after;
- project-local doctor zero;
- exact journal-only line quoted in the record;
- ticket Done;
- confirmed journal row;
- all path hashes recomputed successfully;
- no commit id;
- no compile/source-build negatives;
- completion within the 20-minute hard stop.

### Evidence and recording

Describe the extra collected paths and add record-template fields for seal line and journal hash
verification.

## Justfile structure

Extend `cbt-collect` after the existing `/tmp` evidence loop:

- create a `no-git-demo` evidence subtree only when the source journal exists;
- copy the journal to its `.lisa` location;
- copy the fixed ticket;
- copy the fixed work artifact directory;
- retain current tour and docker-diff collection.

Every copied source path is explicit. Authentication directories remain excluded.

## Commit units

Unit 1 — enforcement regression:

- include only `crates/lisa-plugin/src/lib.rs`.

Unit 2 — no-Git fixture instrumentation:

- include `docker/chromebook-test/bin/prepare`;
- include `docker/chromebook-test/bin/grade`;
- include `justfile`.

Unit 3 — standing protocol:

- include only `docs/knowledge/chromebook-install-test.md`.

Each unit is committed with `lisa commit-ticket --ticket-id T-049-03-02` and exact includes.

## Verification boundaries

- Rust formatting for modified Rust.
- Focused plugin test for mid-run breakage.
- Existing compiled seal visibility integration tests.
- Full `cargo test -p lisa-plugin` if focused tests pass.
- Full `cargo test -p lisa-cli --test seal_visibility`.
- `sh -n` for prepare and grade.
- A synthetic temporary no-Git project/journal fixture for the embedded hash verifier path if it
  can be invoked without weakening script structure.
- `just --fmt --check` or `just --evaluate` as available for justfile syntax.
- `cargo test --workspace` for final regression coverage, subject to unrelated concurrent edits.
- Final Git status checked only for owned paths; unrelated dirty paths are preserved.
