# Design: T-031-03 provider contract and live regression

## Decision summary

Align the bundled workflow, common provider prompts, and user documentation on
one explicit two-boundary contract:

1. During Implement, an agent commits each meaningful ticket-owned source unit
   only through `lisa commit-ticket --include <exact-path>...`.
2. The agent never uses or hands off the ordinary Git index, never edits
   phase/status, writes `review.md`, and waits.
3. Lisa keeps the seat assigned while `lisa complete-ticket` atomically commits
   Done frontmatter and the complete work-artifact directory.
4. Only confirmed completion releases the seat or unblocks a dependent.

Add the outgoing workflow as a known legacy template so clean installations
upgrade while customized workflows remain protected. Add a deterministic
external-repository harness, invoked by a Cargo integration test, that composes
the real CLI transaction commands across five Codex tickets and a mixed-provider
ticket while preserving a foreign staged entry and recording diagnostic evidence.

## Interpretation of “one final completion commit”

The final completion commit is the single authority for Done publication. It
contains the Done ticket and all six work artifacts. Its tree also contains the
final implementation code, which was made durable through prior isolated
ticket-owned implementation commits.

Rewriting all implementation work into one diff-level commit is incompatible
with an interleaved shared branch: another ticket's commits may land between a
ticket's implementation units and completion. Squashing across those commits
would rewrite or absorb foreign history. The enforceable invariant is therefore
one final completion commit that closes a chain of isolated ticket commits,
never an ordinary-index staged handoff.

## Option 1: tell agents never to run Git

The prompt could forbid all agent commits and leave all source capture to the
final scheduler command.

Advantages:

- Very simple agent instructions.
- One visible commit per ticket.
- No risk that an agent mistypes `commit-ticket`.

Disadvantages:

- The scheduler receives only ticket/work paths at completion.
- It cannot infer which shared source changes belong to this ticket.
- A repository-wide scan could steal concurrent or human modifications.
- Adding every dirty path would directly violate T-031-01's ownership boundary.

Decision: rejected. Safe shared-worktree ownership must be explicit.

## Option 2: use ordinary Git commits under a file lock

Agents could retain familiar `git add` and `git commit` commands while relying
on `.lisa-commit.lock` for serialization.

Advantages:

- Familiar workflow for agents and humans.
- No new instruction vocabulary.

Disadvantages:

- Ticket files become staged in the shared ordinary index between commands.
- A foreign `git commit` can consume those staged entries.
- A lock cannot protect against tools that do not honor it.
- This recreates the exact E-002 field failure.

Decision: rejected. The ordinary index cannot be a cross-command mailbox.

## Option 3: add a source-ownership manifest

Agents could record modified paths in `.lisa/transactions/<ticket>.paths`, and
the final completion command could include them.

Advantages:

- The final completion diff could include outstanding source changes.
- Ownership remains explicit.

Disadvantages:

- A new persistent protocol, CLI command, cleanup lifecycle, and conflict model.
- Long tickets would leave implementation changes undurable until Review.
- Crash recovery becomes dependent on a mutable manifest and working tree.
- It duplicates the already shipped explicit-include transaction.
- Concurrent edits to a registered path remain ambiguous.

Decision: rejected for this convergence ticket. Incremental isolated commits
already provide durable explicit ownership without inventing another registry.

## Option 4: amend or squash ticket commits at completion

The scheduler could try to collapse earlier ticket implementation commits into
the final completion commit.

Advantages:

- A single diff-level commit could represent a ticket.
- History looks compact in non-concurrent cases.

Disadvantages:

- Ticket commits are interleaved on one shared branch.
- Rewriting would affect unrelated ticket or human descendants.
- The compare-and-swap transaction deliberately advances, not rewrites, HEAD.
- Recovery and provenance hashes would become invalid.

Decision: rejected. Atomic completion must not imply shared-history rewriting.

## Option 5: explicit isolated implementation commits plus final completion

Agents use `lisa commit-ticket` after meaningful implementation units and pass
only exact ticket-owned paths. They then write Review and stop. The scheduler
uses `complete-ticket` for the final ticket/work transaction.

Advantages:

- Reuses the shipped alternate-index transaction.
- Keeps source ownership explicit.
- Preserves foreign staged entries throughout the ticket.
- Makes implementation durable before Review completion.
- Allows the scheduler to remain provider-neutral.
- Matches T-031-02's pending completion state machine.

Disadvantages:

- A ticket may have more than one commit.
- Agents must know the Lisa command syntax.
- The final completion diff does not repeat already committed source changes.

Decision: chosen. It is the only current design compatible with both explicit
ownership and concurrent shared-branch history.

## Prompt design

Keep `ticket_prompt` as the source for both provider initial and reuse prompts.
Add a compact contract that states:

- do not update phase or status;
- do not run ordinary-index `git add`, `git add -A`, or `git commit`;
- commit meaningful implementation units only with `lisa commit-ticket` and
  exact repository-relative `--include` paths;
- never leave ticket-owned files staged;
- after `review.md`, stop and remain on the current ticket;
- do not start another ticket until Lisa confirms the completion commit.

Update `finish_up_prompt` with the same terminal behavior. The follow-up need
not repeat the full CLI syntax because it is sent after Implement, but it must
forbid frontmatter/index publication and tell the agent to wait for Lisa.

Use phrase-level tests for every safety rule. Since Claude and Codex adapters
delegate to the same prompt functions, these tests cover initial and reuse text
for both clients; adapter equality tests continue to cover delivery wiring.

## Workflow design

Replace generic “commit incrementally” with an “Atomic Git contract” subsection
under Implement. Explain that:

- meaningful units remain desirable;
- all agent commits use the isolated Lisa command;
- exact include paths are mandatory;
- broad or ordinary-index staging is forbidden;
- `progress.md` records transaction commit IDs where useful;
- Review ends by writing the artifact and waiting;
- Lisa alone prepares and commits Done.

Revise Phase Rules and Concurrency so file locking is described as part of the
isolated transaction, not as sufficient protection for ordinary staging.

## Upgrade ownership design

Copy the exact outgoing six-phase workflow into a new versioned file below
`crates/lisa-cli/data/legacy/`. Add it to `LEGACY_RDSPI_WORKFLOWS` alongside the
existing v0.2 file.

`plan_owned_template` behavior remains unchanged:

- current new template: no-op;
- exact v0.2 or exact outgoing template: update;
- unknown/customized content: safety skip.

Extend tests to iterate every legacy entry and explicitly test the outgoing
template upgrade. Keep the unknown-content preservation test byte-exact.

## Harness alternatives

### Real paid Claude/Codex sessions in nested Zellij

This most closely resembles a field loop, but it requires provider credentials,
network access, installed interactive clients, a terminal host, and nondeterministic
model behavior. It is unsuitable as a mandatory workspace regression.

### Plugin-only state simulation

This covers seat/DAG logic but cannot execute the native alternate-index Git
boundary or inspect a real commit tree and index.

### CLI-only transaction sequence

This provides real repository durability but needs an explicit activity model
to represent route, seat reuse, and dependency-start ordering.

### Deterministic composed harness

Chosen: scaffold a temporary external Git repository, route six fixture tickets
through a one-seat activity sequence, and invoke the real Lisa CLI for all
implementation and completion commits. Five tickets use Codex; a sixth uses
Claude. One Codex ticket depends on an earlier Codex completion. The harness
asserts the activity gate before starting the dependent.

The scheduler's actual pending/reuse state machine remains covered by T-031-02
plugin tests. The new harness joins that modeled scheduling sequence to real CLI
Git effects and durable evidence without pretending to run a model.

## Harness evidence design

Create a temporary root containing separate `repo/` and `evidence/` directories.
The repository is outside the Lisa source tree. Evidence remains outside the
fixture repository so it cannot create loop-owned residue.

Record:

- `activity.jsonl`: route, seat, start, implementation commit, pending, confirmed;
- `provenance.jsonl`: one confirmed completion row per ticket and commit hash;
- `commits.txt`: implementation and completion hashes;
- `index.before` and `index.after`: exact foreign stage tuples;
- per-ticket completion-tree listings;
- per-ticket ticket/work blobs from the completion commit;
- dependency prerequisite/start ordering checks;
- fixture `git status --porcelain=v1` after completion.

On failure, print both fixture and evidence paths. By default retain failures and
clean successful temporary roots unless a keep flag requests inspection.

## Harness assertions

- At least five routed rows identify Codex and one seat ID.
- The seat is reused sequentially for all five Codex tickets.
- The mixed Claude row goes through the same transaction functions.
- The dependent start row follows existence of its prerequisite completion hash.
- Every Done ticket blob is introduced by its completion commit.
- Every artifact exists in that completion commit.
- Every source file exists in the completion commit tree.
- No ticket/work/source path remains modified, staged, or untracked.
- The foreign staged tuple is byte-identical before and after each completion.
- The foreign blob is absent from every ticket commit diff.
- Provenance has exactly one confirmed row per ticket.
- Activity contains commit and confirmation hashes sufficient for diagnosis.

## Documentation design

Update README's workflow and concurrency sections to six phases and the atomic
contract. Add a short recovery paragraph: a failed completion keeps the ticket,
seat, and dependents in place; inspect the surfaced Git error, repair the exact
conflict, and let Lisa retry.

Update the setup guide's Agent Lifecycle and artifact tree. Remove instructions
that agents or humans advance normal phase frontmatter during a running ticket.
Describe the final completion receipt and foreign-index guarantee.

## Verification decision

Run focused prompt/adapter tests, init ownership tests, the harness integration
test, `lisa validate` through the harness, workspace tests, plugin Clippy, the
WASM release build, `just check`, and path-scoped `git diff --check`.

No production command interface needs to change. The implementation is contract,
upgrade data, tests/harness, and documentation layered on the T-031-01/02 runtime.
