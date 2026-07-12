# Structure: T-039-06-02

## Change set

This ticket creates six attempt-private RDSPI artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

All six live under:

`.lisa/attempts/T-039-06-02/1/work/`

Lisa publishes admitted copies under `docs/active/work/T-039-06-02/`.

The agent does not write that shared path directly.

No production source file is created, modified, or deleted.

No manifest, lockfile, configuration file, fixture, ticket, story, or provenance
ledger is edited by the agent.

## Artifact responsibilities

### `research.md`

Maps the evidence population and repository state.

Records the 14-ticket pass, provenance schema, usage files, commit chain, final
gates, live assignment gap, and intervening behavior change.

Describes evidence limitations without proposing remediation.

### `design.md`

Evaluates report strategies.

Selects a forensic two-track report rather than a rerun or green-only summary.

Defines the evidence hierarchy and blocking semantics.

### `structure.md`

Defines the documentary change set and report organization.

Establishes that `progress.md` is the canonical field report.

### `plan.md`

Sequences evidence validation, report writing, and review.

Defines exact verification criteria and the no-rerun boundary.

### `progress.md`

Serves as the canonical live Codex-seat field report.

Contains the verdict, scope, evidence taxonomy, timeline, deterministic proof,
live observations, required concern matrix, repository safety assessment,
provenance assessment, anomaly analysis, and acceptance mapping.

### `review.md`

Summarizes what was produced and how it was verified.

Repeats the blocking issue prominently for the human/Lisa handoff.

## Canonical field-report layout

`progress.md` will use the following top-level sections in order:

1. Verdict.
2. Scope and non-actions.
3. Evidence classes.
4. Population summary.
5. Execution timeline.
6. Deterministic proof.
7. Live execution observations.
8. Required concern matrix.
9. Assignment/reuse anomaly.
10. Retries and timeouts.
11. Stale panes and marker residue.
12. Delivery-error evidence.
13. Repository safety.
14. Provenance integrity.
15. Behavior-change assessment.
16. Acceptance mapping.
17. Follow-up boundary.

The verdict appears first so a green test list cannot hide the blocking result.

## Evidence classes

The report uses explicit labels rather than mixing claim types.

`DETERMINISTIC` means a test/build/check on controlled repository state.

`LIVE` means a fact produced by real Codex ticket execution.

`REPOSITORY` means Git, ledger, file, or lifecycle state observable after the run.

`INFERENCE` means a conclusion connecting retained facts where the initiating
runtime event itself is absent.

`NOT OBSERVED` means retained evidence contains no instance of the concern.

`UNAVAILABLE` means the evidence needed for a definitive classification was not
persisted.

These labels are local report vocabulary, not new serialized application types.

## Population summary component

The report will include exact totals:

- 14 completed predecessor tickets;
- 14 attempt-private attempt-1 directories;
- 14 usage JSON files;
- 14 schema-v2 provenance rows;
- 14 authoritative `done` outcomes;
- 14 completion commits;
- 29 total commits in the pass range, including source/completion work and the
  intervening hotfix;
- 31,193,999 input tokens;
- 304,533 output tokens.

The report will not add the active field-report attempt to predecessor totals.

## Timeline component

The timeline will focus on boundaries material to the anomaly:

- `T-039-01-01` start/end;
- `T-039-02-01` end;
- 1,029-second gap;
- `0f850b3` timestamp inside the gap;
- `T-039-02-02` start after the change;
- uninterrupted later predecessor chain;
- final `T-039-06-01` completion.

It need not print every epoch value for all 14 tickets.

The per-ticket ledger remains the detailed source.

## Deterministic-proof component

This component summarizes final-tree gates.

It includes:

- 768 passing executed workspace tests;
- zero failures;
- one intentionally ignored real-Zellij integration test;
- passing formatting;
- passing warning-strict native Clippy;
- passing warning-strict WASM Clippy;
- passing release WASM build;
- passing release CLI build;
- matching release/embedded WASM hash;
- passing canonical `just check`.

It also maps predecessor regression families to the required concerns.

## Live-observation component

This component records real execution evidence only.

It includes:

- per-ticket Codex usage capture;
- requested/actual OpenAI Codex route equality;
- attempt ID 1 for every predecessor;
- authoritative terminal outcomes;
- pane 0/pane 1 execution alternation;
- absence of live failed/timed-out terminal records;
- the scheduling gap and mid-pass hotfix;
- successful later completion after the behavior change.

It explicitly avoids treating deterministic injected-clock failures as live
failure observations.

## Concern matrix structure

The matrix columns are:

| Concern | Deterministic proof | Live/repository observation | Disposition |

Rows match the ticket language exactly.

The behavior-change row is added even though it is a general acceptance clause.

Disposition values include `clear`, `limited evidence`, and `BLOCKING`.

No concern is omitted merely because its live count is zero.

## Assignment anomaly component

This section identifies commit `0f850b3` by full hash and subject.

It lists the six changed paths at a useful category level:

- `.lisa.toml`;
- workspace/Cargo version files;
- Codex adapter;
- scheduler/plugin implementation.

It describes the semantic transition:

`ClearHandshake` resident reuse becomes `ExitThenFresh` for Codex.

It states exact timing relative to ledger events.

It distinguishes fact from inference:

- fact: the gap exists;
- fact: the hotfix landed inside it;
- fact: the hotfix changes reuse behavior;
- fact: the next ticket starts after it;
- inference: the hotfix was intervention for the failed reuse boundary;
- unavailable: the original dashboard/provider error event.

## Repository-safety component

The report separates three Git surfaces:

1. ticket-owned source commits;
2. Lisa completion commits;
3. the ordinary index/worktree.

It verifies cited source commits are ancestors of final HEAD.

It verifies one completion commit per predecessor.

It records the current empty cached diff.

It explains why Lisa-owned active lifecycle changes are not source residue.

It explicitly answers staged/index and done-not-committed requirements.

## Provenance component

The component checks:

- record count;
- schema version;
- ticket/lease identity;
- outcome;
- authoritative/fenced flags;
- requested/actual route;
- timing continuity;
- attempt multiplicity;
- token-field boundary.

It notes that usage metrics exist in separate files while terminal ledger token
fields remain null.

It identifies the missing failure row for the assignment intervention as a
provenance observability gap.

It does not call the successful terminal rows corrupt.

## Review structure

`review.md` will begin with `Outcome: BLOCKING DONE`.

It will enumerate only artifact changes.

It will list read-only verification performed.

It will assess report coverage against every concern.

It will state that no source commit exists because no ticket-owned source changed.

It will distinguish the open blocker from non-blocking limitations.

It will stop on this ticket after writing Review.

## Commit boundary

There is no meaningful ticket-owned source unit to commit with
`lisa commit-ticket`.

Private attempt artifacts are managed by Lisa's publication/completion flow.

Creating an empty source commit would misrepresent the ticket.

No ordinary `git add` or `git commit` operation is permitted or needed.

## Verification boundary

Read-only Git, JSON, and text queries are allowed.

`git diff --check` validates artifact whitespace.

`git diff --cached --name-only` validates ordinary-index cleanliness.

No builds or tests need repetition because the report consumes the final gate
record and the assignment prohibits redundant predecessor execution.

No Zellij command, Codex command, or live provider process is launched.

## Final structure

The resulting deliverable is a self-contained documentary packet.

`progress.md` contains the detailed field evidence.

`review.md` contains the concise blocking handoff.

The production tree remains unchanged.
