# Design — T-047-01-02 probe rematch on RC surface

## Decision frame

This ticket is an evidence-producing task with an external execution boundary.

The repository contains the candidate surface and the scoring rubric.

It does not contain the required rematch output or its run metadata.

The design must preserve the difference between preparation and observation.

It must also preserve the published series as an archive of real runs.

The primary decision is what can be admitted when the human-operated run has
not occurred or has not been supplied to the attempt.

## Evaluation criteria

Any viable approach is evaluated against these constraints:

1. It must use the published rubric without reinterpretation.
2. It must identify the actual method, model, CLI, surface, and fixture.
3. It must compare one changed axis against the 2026-07-16 pair.
4. It must retain the generated page as the scored artifact.
5. It must not infer agent comprehension from source strings alone.
6. It must not fabricate a human-operated or metered execution.
7. It must not modify the series if no admissible run exists.
8. It must make the unblock action concrete for the operator.
9. It must preserve unrelated concurrent worktree changes.
10. It must use Lisa's isolated transaction for any eventual shared changes.

## Option A — Generate a page in this assignment session

Under this option, the ticket agent would read the intended surface, create a
new `lisa-tour.html`, score its own output, and append it to the series.

### Advantages

- Produces the two files named in criterion 1 immediately.
- Can deliberately include the three required concepts.
- Requires no external coordination.
- Is mechanically easy to validate.

### Disadvantages

- It is not the specified human-operated container and agent-CLI run.
- The assignment already supplies the ticket's desired outcome and rubric.
- The resulting page would be contaminated by direct knowledge of the scores.
- It would measure this review session, not the installed/loop-injected surface.
- Model, CLI, fixture, and hands-off status would not match the intended test.
- Scoring a deliberately compliant self-authored page would be circular.
- Calling it a rematch would fabricate experiment provenance.

### Assessment

Rejected.

This option creates an artifact but not the evidence the artifact is meant to
represent.

## Option B — Score the source code as a proxy page

Under this option, the canonical purpose paragraph and run-summary strings
would be mapped directly to rubric columns.

The source contains actors, benefit, and evidence vocabulary, so columns 1–3
could be marked yes without running the probe.

### Advantages

- Uses verifiable repository content.
- Confirms the prerequisite implementation is present.
- Provides confidence that a future probe has the intended inputs.
- Avoids inventing the actual wording in the source.

### Disadvantages

- The rubric grades the generated landing page, not Lisa source.
- Presence in source does not prove that injected context reaches the agent.
- Presence in runtime output does not prove that the page quotes or explains it.
- It cannot establish purpose-before-mechanism in the generated page.
- It cannot identify the model, CLI, or fixture for a nonexistent run.
- It bypasses the story's explicit field-measurement goal.

### Assessment

Rejected as a completion strategy.

Retained only as preparation evidence that the intended candidate surface is
present before a human spends a metered run.

## Option C — Reuse one of the 2026-07-16 pages

Under this option, an existing HTML artifact would receive a new row describing
the new source surface.

### Advantages

- The existing pages are authentic agent-generated artifacts.
- Entry b already provides the desired loop-built comparison method.
- The archived HTML can be rescored deterministically.

### Disadvantages

- Both pages were generated from Lisa 0.3.0.
- Neither observed T-046-07 or T-047-01-01.
- Renaming or relabeling one would falsify the surface version.
- The known scores do not meet this ticket's acceptance criterion.
- A second row for the same old artifact would not be a rematch.

### Assessment

Rejected.

Historical artifacts remain baselines only.

## Option D — Infer a passing run from prerequisite tests

Under this option, T-047-01-01's 972 passing tests and focused string fixtures
would be treated as proof of the landing-probe result.

### Advantages

- The tests are real and already published.
- They prove purpose-first context composition.
- They prove factual summary behavior across clean and adverse fixtures.
- They reduce the risk of wasting a manual run on a broken candidate.

### Disadvantages

- The tests validate input and reporting contracts, not agent comprehension.
- They do not execute a real Claude or Codex landing-page task.
- They do not preserve a generated HTML page.
- They do not exercise the exact human-operated loop fixture.
- Passing software tests and passing comprehension scores are different claims.

### Assessment

Rejected as field evidence.

Accepted as readiness evidence only.

## Option E — Run the external probe autonomously from this session

Under this option, this ticket agent would create a fresh project/container,
invoke another metered coding-agent CLI, allow `lisa loop` to build the page,
then score it.

### Advantages

- Could produce genuine nested-agent output.
- Could hold the method constant against entry b.
- Could capture metadata and post-run narrative in one pass.

### Disadvantages

- The ticket assigns operation to John.
- A metered provider invocation is an external, intentionally human-driven
  action, not an implicit repository edit.
- Authentication, model selection, and fixture choice are operator inputs.
- The exact release surface is ambiguous: installed and workspace versions are
  `0.4.0-rc.8`, while the ticket names `0.4.1-rc`.
- Launching the experiment without those operator choices would change the
  comparison design.
- It could spend tokens on the wrong release candidate.

### Assessment

Rejected for the present attempt.

The ticket text makes the operator boundary explicit.

## Option F — Evidence-gated publication with a named block

Under this option, the agent verifies the candidate implementation and defines
the exact admission/scoring path, but makes no shared series change until the
manual run evidence exists.

The phase artifacts record what was inspected, what evidence is absent, and the
minimum operator action required to resume.

Review ends with a block disposition.

### Advantages

- Exactly follows the ticket's honest-boundary instruction.
- Preserves the series as a record of observed runs.
- Avoids contaminating model output with direct rubric coaching.
- Keeps unknown scores unknown.
- Provides a precise restart point after the operator run.
- Leaves unrelated source and concurrent work untouched.
- Makes the release-version mismatch visible before tokens are spent.

### Disadvantages

- Acceptance criteria remain unmet in this attempt.
- The ticket cannot complete until a human supplies the run.
- No new public knowledge artifact lands yet.

### Assessment

Chosen.

The disadvantages are the truthful representation of the current state, not a
failure of the design.

## Chosen evidence contract

The eventual run package must include enough information to establish:

- date and timezone;
- exact Lisa executable version;
- source revision or release identifier;
- confirmation that T-046-07-* and T-047-01-01 are on the surface;
- exact model name;
- coding-agent CLI name and version;
- loop-built method;
- fixture or container identity;
- the prompt used to initiate the experience;
- the generated `lisa-tour.html` bytes;
- whether the run remained hands-off after launch;
- captured final `lisa loop` or `lisa status` summary;
- any manual questions or approval prompts;
- location of per-ticket work and ledger evidence.

Secrets, auth tokens, full provider account data, and unrelated transcripts are
not part of the evidence contract.

## Comparison design

The preferred rematch holds the following facts from entry b constant:

- loop-built method;
- Claude Code as the agent CLI, if the operator can select it;
- a small ticket chain whose product is `lisa-tour.html`;
- a fresh fixture;
- the short landing-probe intent without rubric coaching.

The changed axis is the Lisa surface:

- baseline: Lisa 0.3.0 plus Zellij 0.44.3 on 2026-07-16;
- candidate: the exact RC executable/revision carrying T-046-07-* and
  T-047-01-01.

If the model cannot be held constant, the row must say so and the comparison
must acknowledge that remaining confound.

The version field must report the executable's actual value.

It must not silently label `0.4.0-rc.8` as `0.4.1-rc`.

## Scoring design

The archived HTML is scored in a fixed order.

1. Inspect the headline and first paragraph for Claude Code or Codex as coding
   agents.
2. Search the whole page for an operator benefit equivalent to not babysitting,
   not approving each step, walking away, or returning to completed work.
3. Search for an audit trail naming at least one concrete evidence mechanism
   and explaining that it can be reviewed afterward.
4. Find the first purpose statement and the first DAG/scheduling/Zellij
   mechanism statement, then compare their document order.

Columns remain binary for the new row.

If evidence language is merely a vague claim such as “transparent,” column 3
does not pass without an identifiable audit trail.

If any of columns 1–3 is no, the page and row remain useful field evidence, but
this ticket cannot pass.

The miss is converted into a copy ticket tied to the exact absent or misplaced
language.

## Publication design after evidence arrives

The generated HTML is copied without content edits to a dated, descriptive
filename in `docs/knowledge/landing-probes/`.

The README gains exactly one series row for that artifact.

The row records model/method, exact surface, four scores, and the comparison
axis or a concise pointer to it.

Any longer run-method note belongs near the series or in an adjacent metadata
record only if the actual evidence needs it.

Both shared paths form one meaningful documentation unit and are committed by
`lisa commit-ticket` with exact include paths.

## Current disposition design

Because no admissible run package is present, Implement performs only the
evidence gate and ownership verification.

It creates no synthetic HTML.

It makes no README series edit.

It makes no isolated source commit because there is no ticket-owned shared
source change.

Review records a block with one actionable reason: John must run the loop-built
probe on the intended RC surface and place the generated HTML plus run metadata
in this attempt's work directory for scoring and publication.
