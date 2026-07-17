# Structure — T-046-06-03 closing acceptance run

## Structural outcome

This ticket changes attempt-private evidence and workflow artifacts only.

No Rust, release workflow, fixture, README, ticket frontmatter, or shared work
artifact is modified.

The retained runs tested a pre-fix surface and cannot justify product changes.

## Attempt-private directory

All created files live under:

`.lisa/attempts/T-046-06-03/1/work/`

The directory already contains:

- `assignment-1-1784257876315230000.md`;
- `.lisa-launch-0.sh`.

This attempt creates:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `closing-attempts-2026-07-16.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

Lisa may later publish admitted work artifacts after lease verification.

The agent does not write directly to `docs/active/work/T-046-06-03/`.

## `research.md`

Purpose: descriptive map of the evidence gate.

It records:

- ticket and story boundaries;
- runbook controls;
- prerequisite fix ownership;
- landing-probe baseline;
- retained container inventory;
- protocol-file audit;
- Claude and Codex retained-run facts;
- missing seeded/tour evidence;
- existing finding ownership;
- evidence conclusion.

It does not prescribe the disposition until its conclusion.

## `design.md`

Purpose: compare viable ways to handle incomplete field evidence.

It evaluates:

- admitting container exit zero;
- reconstructing measurements;
- filing duplicate bugs;
- changing the runbook;
- autonomously rerunning;
- blocking without preservation;
- preserving evidence with an actionable block.

It chooses evidence preservation plus operator-owned block.

It defines the privacy and verification posture.

## `structure.md`

Purpose: define file ownership and artifact boundaries.

It names every file created by the attempt.

It keeps shared source ownership empty.

It defines the evidence schema and Review JSON shape.

It establishes ordering for Implement and Review.

## `plan.md`

Purpose: sequence evidence extraction, documentation, and validation.

Each step has an independent verification condition.

No step requires ordinary Git staging.

No step launches a metered provider run.

## `closing-attempts-2026-07-16.md`

Purpose: retain sanitized failed-attempt evidence.

The filename deliberately uses `attempts`, not `results`.

Its top-level status is `NOT ADMITTED`.

The file contains one section for each stopped container.

### Shared header

The header states:

- date;
- ticket ID;
- evidence source;
- privacy exclusions;
- acceptance meaning.

### Claude section

The Claude record contains:

- container name;
- image identity abbreviation;
- architecture;
- resource caps;
- mount count;
- container state;
- writable-layer size with caveat;
- actual CLI/model;
- actual prompt;
- README source;
- chronological install chain;
- sudo/apt actions;
- relevant exact public strings;
- post-run shell-history observations;
- changed-path summary;
- missing fields;
- non-admission reasons.

### Codex section

The Codex record contains the same categories.

It additionally records:

- Codex CLI version;
- mini model and effort;
- mistyped README filename;
- second operator message;
- initial doctor omission;
- later Zellij recovery chain.

### Acceptance matrix

A compact matrix maps both runs to:

- correct model;
- correct public surface;
- exact instruction;
- hands-off behavior;
- measurements;
- positive exits;
- negative assertions;
- time threshold;
- disk threshold;
- admission status.

Unknown values remain `NOT RECORDED`.

No inferred values are presented as measurements.

### Finding map

The final section links observed pre-fix behavior to:

- T-046-03-02;
- T-046-03-03;
- T-046-02-*;
- T-046-04-*;
- T-047-01-02.

It says no new product bug was established.

## `progress.md`

Purpose: implementation ledger.

It records:

- phase completion;
- ownership baseline;
- read-only Docker inspection;
- privacy boundary;
- evidence artifact creation;
- finding-routing decision;
- source transaction status;
- verification results;
- acceptance mapping;
- remaining operator work.

Because no shared source unit changes, it records that no `lisa commit-ticket`
transaction was needed.

That is distinct from skipping a required source commit.

## `review.md`

Purpose: human handoff and self-assessment.

It opens with the blocked disposition.

It summarizes exactly what was inspected and created.

It explains why the retained runs fail admission.

It maps all three acceptance criteria.

It describes verification coverage and gaps.

It names the exact operator remedy.

It states that no source path is left modified by the ticket.

## `review-disposition.json`

The file is a single JSON object.

Its required fields are:

```json
{
  "disposition": "block",
  "reason": "<non-empty actionable reason>",
  "remedy_owner": "operator",
  "ask": "<one sentence>",
  "steps": ["<exact action>"],
  "check": "<read-only command>"
}
```

The exact file will remain one line to avoid ambiguity.

The disposition is not `pass` because no criterion is fully evidenced.

The owner is not `agent` because rerunning requires human authentication and
metered operation.

The owner is not `world` because external publication is not the only missing
condition; John must perform controlled runs.

The check observes artifact presence only.

It never performs the remedy.

## Evidence source boundaries

Docker container inspection is read-only.

`docker diff` supplies changed-path facts.

`docker cp ... -` streams selected files through stdout.

No credential directory is copied to the repository.

Claude extraction is limited to the relevant session JSONL.

Codex extraction is limited to the relevant session JSONL.

Auth JSON, settings, tokens, and environment state are excluded.

Shell history is filtered to installation and verification commands.

The evidence document paraphrases rather than embeds full transcripts.

Short exact error/output strings are retained because they establish routing.

## Shared source boundary

No new ticket is created.

No existing ticket is edited.

No story ticket list is edited.

No runbook is edited.

No landing-probe series file is edited.

No source commit is created.

This avoids duplicating existing owners from evidence that did not exercise the
fixed surface.

## Ordering

1. Complete Research from read-only repository and container inspection.

2. Complete Design and select non-admission plus structured block.

3. Complete this Structure blueprint.

4. Write Plan with exact validation steps.

5. During Implement, create the failed-attempt evidence artifact.

6. Write Progress after evidence content is stable.

7. Validate Markdown, JSON, ownership, and index state.

8. During Review, create `review.md` and disposition JSON.

9. Re-run final validation including Review artifacts.

10. Stop on this ticket and let Lisa handle lifecycle publication.

## Future evidence admission boundary

A future attempt may replace the block only when new artifacts contain:

- current live README bytes;
- exact weak-model identities;
- exact one-shot prompts;
- before/after time and disk values;
- independent positive exit matrix;
- independent negative results;
- sudo/apt transcript summary;
- 0.40.1 seeded recovery;
- landing-probe page and comparison record.

The current failed-attempt artifact remains historical evidence.

It must not be relabeled as a passing result after future runs.

## Structure conclusion

The ticket's implementation shape is deliberately documentation-only and
attempt-private.

It preserves real failed evidence, keeps source ownership accurate, and gives
the operator a precise route to create the missing acceptance record.
