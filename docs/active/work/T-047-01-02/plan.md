# Plan — T-047-01-02 probe rematch on RC surface

## Goal

Record and score one authentic landing-probe rematch on the intended RC
surface, or terminate this attempt in the ticket's required named blocked state
when the human-operated run evidence is absent.

The plan never converts preparation into an observed result.

## Step 1 — Establish repository and attempt ownership

Read:

- `AGENTS.md`;
- `CLAUDE.md`;
- the assignment file;
- `docs/active/tickets/T-047-01-02.md`; and
- `docs/knowledge/rdspi-workflow.md`.

Record:

- attempt-private artifact destination;
- prohibition on direct shared-work publication;
- prohibition on phase/status edits;
- isolated commit requirement; and
- pre-existing dirty worktree paths.

Verification:

- all phase artifacts resolve beneath the attempt directory;
- no command uses ordinary staging or commit;
- unrelated worktree paths are not changed.

Commit: none.

## Step 2 — Map the benchmark and historical baseline

Read:

- `docs/knowledge/landing-probes/README.md`;
- `2026-07-16-a-direct-codex-mini.html`; and
- `2026-07-16-b-loop-built-claude.html`.

Extract:

- prompt variants;
- four rubric definitions;
- required row metadata;
- entry a method, model, surface, and scores;
- entry b method, model, surface, and scores; and
- the confound note.

Define the priority comparison as entry b versus a new loop-built run, with the
Lisa surface as the intended changed axis.

Verification:

- the new comparison does not claim attribution if model or method also changes;
- columns 1–3 remain governed by the published definitions.

Commit: none.

## Step 3 — Verify prerequisite candidate surface

Confirm that the branch contains the completion and source commits for:

- T-046-07-01;
- T-046-07-02; and
- T-047-01-01.

Inspect:

- canonical `PURPOSE_PARAGRAPH`;
- template composition;
- managed assignment composition;
- run-summary truth rules;
- status and post-loop integration; and
- T-047-01-01's published verification record.

Record exact workspace and installed Lisa versions.

Verification:

- intended source commits are reachable from the current branch;
- the exact executable version is not inferred from ticket prose;
- the `0.4.1-rc` train label and `0.4.0-rc.8` observed version are not silently
  conflated.

Commit: none.

## Step 4 — Search for admissible manual evidence

Inspect the attempt directory and landing-probe knowledge directory for:

- a new generated HTML page;
- a run metadata record;
- exact model and CLI identity;
- exact Lisa version/revision;
- fixture/container identity;
- method;
- prompt identity;
- hands-off/intervention record; and
- captured post-run summary.

Search repository references to T-047-01-02 and recent probe names.

Do not treat the current assignment agent's Lisa provenance row as the nested
landing-probe execution.

Verification:

- every required run fact comes from supplied evidence;
- no old 0.3.0 page is relabeled;
- no missing field is guessed.

Commit: none.

## Step 5 — Apply the evidence gate

### Branch A: evidence absent or incomplete

Record the absent fields in `progress.md`.

Do not:

- author a surrogate landing page;
- append a speculative series row;
- assign rubric scores;
- start a metered provider run on John's behalf;
- change the rubric;
- modify source code; or
- call `lisa commit-ticket` with an empty or private-only unit.

Continue directly to ownership checks and Review.

Expected disposition: block.

### Branch B: evidence present and complete

Continue to Steps 6 through 11.

Expected disposition depends on the scored page.

## Step 6 — Validate and preserve supplied evidence

Record hashes or byte counts for the supplied HTML before copying.

Inspect for accidentally captured:

- tokens;
- credentials;
- provider cookies;
- unrelated account identifiers; or
- private transcript content.

Confirm the HTML is the measured output, not a later edited version.

Confirm the run metadata identifies:

- date;
- model;
- CLI and version;
- loop-built method;
- actual Lisa version and revision;
- fixture;
- prompt; and
- interventions.

If required identity is missing, return to Branch A with an actionable list.

Commit: none.

## Step 7 — Score Actors

Render or inspect the page's visible document order.

Read the headline and first paragraph.

Mark yes only if they identify Lisa as running coding agents and name Claude
Code or Codex.

Record the exact supporting location in progress notes.

Do not count model metadata outside the page.

## Step 8 — Score Benefit

Inspect the complete visible page.

Mark yes only if it communicates the operator outcome:

- no babysitting;
- no per-step approvals;
- walk away and return; or
- clear equivalent wording.

Mechanism claims such as concurrency or automation alone do not pass.

Record the exact supporting or missing location.

## Step 9 — Score Evidence trail

Inspect the complete visible page.

Mark yes only if it presents an auditable aftermath and identifies concrete
evidence such as:

- `.lisa/provenance.jsonl`;
- `.lisa/completion-journal.jsonl`; or
- per-ticket work documents.

A vague “transparent” or “traceable” adjective without an audit mechanism does
not pass.

Record the exact supporting or missing location.

## Step 10 — Score purpose-before-mechanism

Locate the page's first purpose/benefit explanation.

Locate the first visible DAG, scheduling, or Zellij explanation.

Mark yes when purpose precedes mechanism.

Consider the title and hero copy because the historical b failure appeared in
the title before the purpose.

Record both locations.

## Step 11 — Select pass or miss publication branch

### Passing page

Required condition:

- Actors yes;
- Benefit yes; and
- Evidence yes.

Order is still recorded factually but is not listed as a columns 1–3 block in
this ticket's second acceptance criterion.

Proceed to Step 12.

### Rubric miss

Preserve the authentic page and factual scores.

Create a concrete copy ticket for every distinct communication failure or one
well-scoped ticket covering tightly related misses.

The follow-up ticket names:

- observed missing concept;
- page location;
- source surface likely responsible, if established;
- required copy behavior; and
- dependency relationship.

Do not reinterpret the rubric.

Keep T-047-01-02 open and use a block disposition after committing admissible
evidence and the follow-up ticket.

## Step 12 — Create the public measurement unit

Copy the generated HTML without editorial changes to a unique dated filename
under `docs/knowledge/landing-probes/`.

Append exactly one row to the README series.

The row contains:

- artifact filename;
- model and CLI/method;
- exact surface version and relevant revision/runtime;
- Actors score;
- Benefit score;
- Evidence score;
- Order score; and
- notes stating the comparison axis.

If model or runtime differs from entry b, state that remaining confound.

Do not rewrite the two historical rows.

## Step 13 — Validate documentation changes

Run scoped checks:

- `git diff --check` on exact ticket-owned shared paths;
- verify the README row contains the exact artifact filename;
- verify the artifact exists and is non-empty;
- verify all Markdown table rows have the same column count;
- search the new row for model, method, surface, and four scores;
- confirm the comparison-axis note is explicit;
- inspect the diff for changes to prior scores or rubric wording.

If a follow-up ticket exists:

- run `lisa validate`;
- confirm its ID and dependencies are valid; and
- include only its exact path in its isolated transaction.

Cargo build/test is not required for pure knowledge artifacts.

## Step 14 — Commit each shared source unit

For a passing page, run:

```text
lisa commit-ticket --ticket-id T-047-01-02 \
  --message "T-047-01-02: record RC landing probe" \
  --include docs/knowledge/landing-probes/<exact-new-file>.html \
  --include docs/knowledge/landing-probes/README.md
```

Use the real exact filename.

For a miss ticket, commit its exact path in a meaningful isolated unit with an
accurate ticket-owned message.

Never use ordinary `git add`, `git add -A`, or `git commit`.

After every isolated commit:

- inspect the commit file list;
- confirm no unrelated path was included;
- confirm the ordinary index has no ticket-owned entry;
- confirm no ticket-owned shared file remains modified or untracked.

## Step 15 — Write progress

Record:

- phase completion;
- evidence inventory;
- version and prerequisite verification;
- gate outcome;
- scores, if evidence exists;
- files changed;
- validation results;
- isolated commit receipts;
- deviations; and
- remaining work.

For the current evidence-absent branch, explicitly state that no source commit
was warranted.

## Step 16 — Review

Evaluate both acceptance criteria independently.

Summarize:

- files created, modified, or intentionally left unchanged;
- test/verification coverage;
- evidence gaps;
- version ambiguity;
- worktree ownership; and
- exact unblock action.

Write `review.md`.

Write `review-disposition.json` with exactly one allowed JSON shape.

Current expected content:

```json
{"disposition":"block","reason":"John must run the loop-built landing probe on the intended RC surface and place the unedited generated HTML plus exact model, CLI, Lisa version/revision, fixture, prompt, intervention, and post-run-summary metadata in this attempt's work directory so it can be scored and published."}
```

After Review, remain on T-047-01-02 and stop.
