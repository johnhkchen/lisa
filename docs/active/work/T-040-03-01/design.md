# Design: deterministic T-039-06-02 regression

## Decision summary

Add one dedicated native plugin test in `crates/lisa-plugin/src/lib.rs` that
constructs the T-039-06-02 boundary with a current attempt, Review assignment,
`review.md`, explicit blocking disposition, provenance ledger path, and a real
dependent ticket.

Drive the production artifact poll once, then assert the entire refusal state:
no pending completion, no authoritative Done provenance, unchanged Review
frontmatter, retained thread/slot/lease ownership, and an unsatisfied dependent.

The pending-completion assertion is explicitly documented as the historical
failure discriminator for the pre-T-040-01-03 unconditional path.

## Option 1: extend the existing table-driven gate test

The existing `review_disposition_gates_artifact_completion_and_dependents`
already has a block row. It could gain a temporary ledger path and provenance
assertions inside every case or only the block case.

Advantages:

- minimal additional fixture setup;
- avoids repeating ticket, DAG, thread, slot, and lease construction;
- keeps all disposition outcomes in one location.

Disadvantages:

- obscures the field incident inside a generic parser-policy matrix;
- pass semantics differ because pass legitimately prepares completion, while
  block and invalid do not;
- a ledger nonexistence assertion is meaningful only before a completion result
  and does not add useful differentiation to every table row;
- future refactoring of the generic table could accidentally weaken the named
  historical contract without making the loss conspicuous;
- the acceptance criterion specifically requests a test reproducing the
  T-039-06-02 scenario.

This option is viable but rejected because discoverability and regression
intent are more valuable than saving a modest fixture repetition.

## Option 2: add a dedicated state-level regression test

Create a single test named for blocking Review and the T-039-06-02 incident.
Use temporary real ticket files and the production `Dag`, `State`, artifact
admission, disposition parser, and artifact poll.

Advantages:

- test name makes historical evidence searchable;
- all acceptance-criterion assertions live in one focused scenario;
- uses the precise production consumer that was formerly unconditional;
- deterministic and host-independent;
- makes the pre-fix discriminator explicit;
- can configure an isolated ledger without affecting unrelated cases;
- preserves the existing table test as broader policy coverage.

Disadvantages:

- repeats roughly the same state fixture as the table test;
- stays in the large monolithic plugin source file;
- observes the transaction boundary rather than launching a real Git commit.

This is the chosen option. The repeated setup is acceptable because it makes a
field regression self-contained and resistant to incidental changes in generic
test helpers.

## Option 3: exercise a real `complete-ticket` subprocess and Git repository

Build a disposable repository, run the plugin path far enough to launch the
CLI, and inspect commit history plus provenance afterward.

Advantages:

- literal proof that no Git commit was created;
- covers CLI and repository plumbing in addition to scheduler policy;
- resembles field execution more closely.

Disadvantages:

- a correct block never invokes the command, so most harness complexity is
  unused on the passing regression path;
- native plugin tests intentionally stub host command execution;
- would require additional binary/build/environment coordination;
- higher flake and runtime risk for a pure authorization branch;
- duplicates existing completion transaction coverage;
- makes it harder to identify that the scheduler request itself is forbidden.

Rejected. The cleanest proof that a command cannot commit is to prove the
command request was never created at its only scheduler entry point.

## Option 4: parser-only test in lisa-core

Add a test that verifies the block JSON parses as `ReviewDisposition::Block`.

Advantages:

- small and fast;
- directly locks the JSON contract.

Disadvantages:

- parser behavior is already covered;
- cannot observe scheduler assignment, pending completion, provenance, or DAG
  readiness;
- would have passed before the scheduler gate existed.

Rejected because it cannot discriminate the historical bug.

## Fixture shape

The temporary ticket directory contains:

1. `T-REVIEW.md`, with `status: review` and `phase: review`;
2. `T-DEPENDENT.md`, with `phase: ready` and
   `depends_on: [T-REVIEW]`.

Scanning those files through `lisa_core::ticket::scan_tickets` and
`Dag::from_tickets` exercises the real dependency model.

The state config points `ticket_dir` and `work_dir` at temporary paths.
`ledger_path` points at a temporary `provenance.jsonl` path that does not exist
before the poll.

An `AgentSlot` on pane 39 owns `T-REVIEW`. A running `Thread` for the same
ticket begins in `Phase::Review`. `install_current_attempt` synchronizes the
thread, slot, and `current_leases` map.

The attempt-private directory receives both:

- `review.md` with representative review text;
- `review-disposition.json` containing a nonempty actionable block reason.

The test invokes `state.check_artifact_advances()` once. A single poll is
sufficient: Review is already the current phase, and both artifacts exist.

## Assertions

### No Done preparation

Assert `pending_completions` does not contain `T-REVIEW`.

This is the regression's load-bearing assertion. The old unconditional
Review-to-Done branch admitted `review.md` and inserted this entry even when a
block file existed.

### No commit-visible state

Read the reviewed ticket and assert it still contains both `status: review` and
`phase: review`. Assert the DAG's ticket remains non-Done.

The asynchronous test boundary means this alone is not the historical
discriminator, but it protects the durable publication contract.

### No Done provenance

Assert the configured ledger path does not exist. Because the fixture begins
with no ledger and block cannot call the successful completion-result path,
no row of any kind should be produced.

The assertion message should name authoritative Done provenance, preserving
the acceptance language. Avoid calling `read_ledger` because that helper is
declared later in the same module and would add ordering/coupling for no gain;
file nonexistence is stronger here.

### Assignment retained

Assert:

- the thread exists and remains `Running` in `Phase::Review`;
- the pane slot still owns `T-REVIEW`;
- the slot lease equals the installed lease;
- `current_leases` still holds that lease.

Together these prevent silent seat release or attempt fencing.

### Dependent blocked

Assert `all_dependencies_done(T-DEPENDENT)` is false and no dependent thread
exists. The first is the real readiness predicate; the second prevents an
unexpected scheduling side effect from being overlooked.

### Actionability

Although not the only acceptance criterion, assert the activity log contains
the exact block reason. This ensures the retained assignment has an actionable
explanation rather than becoming an unexplained stall.

## Why no production change

T-040-01-03 already routes artifact, idle, and stopped automated completion
through `request_review_completion`. The block branch performs no transaction
request and logs the reason. Adding another policy layer would duplicate the
fix rather than pin it.

This ticket advances regression evidence, so source scope remains one test in
the existing plugin module.

## Commit design

The meaningful source unit is the dedicated test in
`crates/lisa-plugin/src/lib.rs`. After focused and full verification, commit
that exact path with:

```text
lisa commit-ticket --ticket-id T-040-03-01 \
  --message "Pin blocking Review completion regression" \
  --include crates/lisa-plugin/src/lib.rs
```

Attempt-private RDSPI artifacts are excluded; Lisa owns their later admission
and final completion transaction.
