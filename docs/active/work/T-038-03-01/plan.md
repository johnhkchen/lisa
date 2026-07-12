# Plan: materialize and verify the repetition inventory

## Goal

Create an acceptance-facing list that classifies every surveyed repetition
family as either `small demonstrated-value cleanup` or
`too-large → future epic`, gives a one-line justification for each, identifies
proof seams for small candidates, and lands no source changes.

## Preconditions

- The ticket is in Research and is an inventory spike.
- `CLAUDE.md`, the ticket, and the RDSPI workflow have been read.
- The active scheduler is `crates/lisa-plugin/src/lib.rs`.
- The adapter seam is `crates/lisa-plugin/src/adapter.rs`.
- Maintained harnesses are under `crates/lisa-cli/tests/fixtures/`.
- Hook and fixture writing patterns have been included in the survey.
- Existing unrelated worktree changes must remain untouched.

## Step 1: lock the survey boundary

Record in `inventory.md` that the inventory covers:

- scheduler signal scanning;
- scheduler failure/recovery and timeout/liveness paths;
- scheduler atomic publication;
- native provider adapters;
- maintained and historical shell harnesses;
- lifecycle-hook JSON writing and merge enumeration;
- scheduler regression fixture writing.

Verification:

- Each boundary maps to a section in `research.md`.
- No claim is made that every repeated token in the repository is inventoried.
- “Every repetition site” is satisfied at the semantic-family level within the
  story’s named scope.

## Step 2: write the classification key

At the top of `inventory.md`, state the exact two allowed labels.

Clarify that harmless or intentionally independent repetition receives the
too-large label when its safe removal would require future-epic scope. Its
justification must say whether an epic is recommended.

Verification:

- Exact label spelling matches the ticket.
- There is no third ambiguous bucket such as “maybe” or “harmless.”

## Step 3: write the small-candidate table

Add C-01 through C-04:

1. Pane signal filename parser.
2. Adapter clear-reset trait default.
3. Adapter review-follow-up trait default.
4. Deterministic harness event-count primitive.

For each row, include sites, classification, one-line justification, and proof.

Verification:

- C-01 is limited to filename grammar; scanner behavior stays outside it.
- C-02 and C-03 retain overridable trait methods.
- C-04 stays within one script and does not create a shared shell dependency.
- Each row names a focused existing or added test.

## Step 4: write the deferred-candidate table

Add C-05 through C-14:

1. Whole signal scanner abstraction.
2. Failure/reclaim unification.
3. Timeout/liveness unification.
4. Atomic publication helper.
5. Cross-harness Zellij helper library.
6. Historical harness consolidation.
7. Declarative hook schema.
8. Scheduler test-fixture builders.
9. Provider assignment construction.
10. Adapter compatibility assertions.

For each row, name exact sites or boundaries, use the exact too-large label,
give a one-line reason, and name the future epic theme or explicitly state that
no epic is recommended.

Verification:

- Deferred rows do not accidentally instruct the successor to edit them.
- Reasons name concrete semantic differences rather than merely saying “large.”
- The final report can copy the deferred family names without re-surveying.

## Step 5: state successor scope

Add a short handoff that `T-038-03-02` may implement only C-01 through C-04,
subject to its clean-tree prerequisite and its own RDSPI review.

State that C-05 through C-14 remain in place.

Verification:

- The inventory does not itself authorize broad cleanup.
- Stable provider, lease, scheduler, and CLI contracts remain explicit.

## Step 6: write implementation progress

Create `progress.md` after the inventory exists.

Record:

- all phase artifacts completed so far;
- the inventory candidate counts;
- source edits: none;
- commits: none, by design;
- deviations: none or a precise explanation;
- remaining work: structural checks and Review.

Verification:

- Progress does not claim source implementation.
- It explains why `lisa commit-ticket` was not run.

## Step 7: verify artifact structure

Run read-only checks against the private attempt directory:

- list expected files;
- count candidate ids in `inventory.md`;
- count exact classification strings;
- search for one-line justification and proof columns;
- inspect candidate ordering and uniqueness.

Expected results:

- C-01 through C-14 each appear exactly once in the inventory tables.
- Four rows are small.
- Ten rows are too-large.
- Every small row names a proof seam.
- No source change is present from this ticket.

## Step 8: inspect repository state

Run `git status --short`.

Expected results:

- Pre-existing modifications may remain in provenance and other tickets.
- This ticket’s private attempt artifacts may be ignored by Git.
- No Rust, shell fixture, Cargo, or shared work artifact is changed by this
  ticket.
- Nothing is staged by this ticket.

Do not clean, reset, stage, or commit unrelated changes.

## Step 9: decide whether tests are required now

This spike changes documentation only, so it does not run workspace tests as an
implementation proof. The classification is verified structurally and against
existing test names.

The successor implementation must run, at minimum:

- focused plugin unit tests for C-01 through C-03;
- the ignored real-Zellij integration test for C-04 when environment
  prerequisites are available;
- `cargo test --workspace`;
- workspace clippy according to the clean-gate ticket’s command;
- the WASM check required by the story dependency.

If C-04 cannot run in a contributor environment, it should not be landed merely
on shell syntax; the successor must record the missing prerequisite or omit
that cleanup.

## Step 10: write Review and stop

Create `review.md` with:

- artifact summary;
- candidate count and selected ids;
- acceptance criterion evaluation;
- test-coverage assessment;
- open concerns and future-epic boundary;
- confirmation of no source changes and no ticket frontmatter edit.

After writing Review, remain on `T-038-03-01`. Do not start
`T-038-03-02`; Lisa owns publication, completion commit, and seat release.

## Atomicity and commits

There is no meaningful ticket-owned source unit to commit in this spike.
Therefore:

- do not call ordinary `git add`;
- do not call ordinary `git commit`;
- do not call `lisa commit-ticket` with documentation that Lisa is expected to
  admit and publish;
- let Lisa’s completion transaction publish all attempt artifacts together.

## Risks and mitigations

Risk: the inventory treats syntax similarity as policy identity.

Mitigation: only four candidates pass the semantic and focused-proof gate.

Risk: a “small” scheduler helper expands into scanner refactoring.

Mitigation: C-01’s interface owns only filename parsing.

Risk: trait defaults hide provider differences.

Mitigation: only identical reset and follow-up methods default; assignment,
signals, readiness, and launch remain explicit.

Risk: shell deduplication creates a fragile shared library.

Mitigation: C-04 is local to one script; cross-script helpers are deferred.

Risk: historical evidence is modified as cleanup.

Mitigation: C-10 explicitly forbids rewriting admitted harness artifacts.

Risk: the successor lands everything marked small without rechecking the clean
gate or actual diff.

Mitigation: inventory language says “may implement,” and T-038-03-02 retains
its own acceptance, RDSPI, exact-path commits, and verification obligations.
