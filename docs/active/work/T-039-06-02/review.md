# Review: T-039-06-02

## Outcome: BLOCKING DONE

The requested field report is complete.

It separates deterministic final-tree proof from live Codex-seat observations,
repository state, inference, and unavailable evidence.

It accounts for every concern named in the ticket.

The report must not receive an unconditional Done verdict.

The completed E-039 pass required a mid-pass production behavior change at the
native Codex reuse boundary.

The exact initiating runtime failure was not persisted in terminal provenance or
an admitted operator-event record.

The ticket expressly requires a behavior change or unexplained anomaly to block
Done, so this Review preserves that verdict.

## Deliverable

The canonical report is `progress.md` in this attempt's private work directory.

It contains:

- an upfront blocking verdict;
- scope and no-rerun statement;
- evidence vocabulary;
- predecessor population and live token totals;
- terminal provenance summary;
- execution timeline;
- mid-pass behavior-change analysis;
- final deterministic gate evidence;
- a matrix for every required concern;
- detailed repository-safety and provenance assessments;
- acceptance mapping;
- follow-up boundary.

## Files created

This ticket authored six attempt-private artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

They live under:

`.lisa/attempts/T-039-06-02/1/work/`

Lisa admitted artifacts to `docs/active/work/T-039-06-02/` as they appeared.

The agent did not write the shared work path directly.

No production source, test, fixture, manifest, lockfile, configuration file,
ticket, story, or provenance ledger was edited by the agent.

## Evidence reviewed

The report reviewed all 14 completed E-039 predecessors from
`T-039-01-01` through `T-039-06-01`.

Sources included:

- predecessor ticket frontmatter;
- admitted predecessor progress/review artifacts;
- `.lisa/provenance.jsonl`;
- `.lisa/codex/T-039-*.usage.json`;
- attempt-directory structure;
- first-parent Git history and commit objects;
- the intervening hotfix diff;
- current ordinary-index/worktree state;
- current lease marker files;
- final build/test evidence from `T-039-06-01`.

No external source was required.

## Deterministic proof review

The final rc.7 tree has a strong green verification record.

`T-039-06-01` recorded successful release builds for the WASM plugin and native
CLI.

The release plugin and CLI `OUT_DIR` copy matched at 1,411,000 bytes and SHA-256:

```text
7098c00d1558d6b861842b133fe15067e98f52985df57134147bd35e55d55d5f
```

Formatting passed.

Native all-target/all-feature Clippy passed with warnings denied.

WASM Clippy passed with warnings denied.

The direct workspace suite executed 768 passing tests and zero failures.

One pre-existing real-Zellij integration test remained intentionally ignored.

The canonical `just check` passed.

Predecessor regressions cover exact signal admission, bounded failure outcomes,
deadline policies, stale-attempt rejection, atomic publication, isolated-index
transactions, rollback, and provenance attribution.

This coverage is suitable deterministic proof of the final tree.

It is not evidence that every live negative path fired during the pass.

## Live execution review

The report found 14 per-ticket Codex usage files.

They contain 31,193,999 input tokens and 304,533 output tokens in aggregate.

Each usage key matches one predecessor ticket.

The report found 14 schema-version-2 provenance rows.

Every row has:

- the expected ticket/attempt-1 lease;
- `outcome: done`;
- `authoritative: true`;
- `fenced: false`;
- requested and actual `codex/openai` routing.

No predecessor has a second attempt, failed row, timed-out row, duplicate
terminal result, or route substitution.

This is complete successful terminal evidence.

## Blocking assignment/reuse finding

`T-039-02-01` completed at epoch `1783888390`.

`T-039-02-02` did not begin until `1783889419`.

The dependent-start gap is 1,029 seconds.

At epoch `1783889301`, 911 seconds into that gap, commit
`0f850b3e5b6cae90f933c828d05286d1db522303` landed.

Its subject is:

```text
fix(codex): relaunch between ticket assignments
```

It changed native Codex reset semantics from resident `ClearHandshake` reuse to
`ExitThenFresh`.

It bumped the checkout from rc.6 to rc.7 and changed scheduler/adapter production
code plus associated tests.

The next dependent began 118 seconds later.

The commit's wording, comments, semantics, and placement strongly support the
inference that the resident `/clear` reuse boundary blocked unattended
assignment and required intervention.

The exact provider event, UI alert, or scheduler error is unavailable.

No failure/timed-out ledger row explains the interval.

This is both a behavior change and an incompletely reconstructed anomaly.

It is therefore blocking by the ticket's own rule.

## Required concern coverage review

### Assignment/reuse failures

Covered.

One live failure is evidenced by the assignment gap and mid-gap provider
lifecycle hotfix.

It is marked blocking rather than normalized as successful reuse.

### Retries

Covered.

Deterministic tests prove finite retry budgets.

The live terminal set has attempt ID 1 throughout and no duplicate outcome.

No unsupported claim is made about internal live chat submission count.

### Timeouts

Covered.

Deterministic injected-clock timeout behavior is recorded.

No live predecessor terminal timeout occurred.

The wall-clock reuse gap is not mislabeled as a timeout.

### Stale panes

Covered.

Deterministic stale-attempt/stale-thread tests are distinct from live evidence.

No fenced or duplicate E-039 terminal row exists.

Old lease marker residue is reported as non-authoritative file state, not as
proof that stale panes executed work.

### False delivery errors

Covered with an explicit evidence limitation.

No retained predecessor report identifies a false-positive delivery or commit
error.

All cited source commits resolve in final history.

The unavailable initiating reuse error is not speculatively classified.

### Staged/index residue

Covered and clear.

Every predecessor review records source cleanliness after its isolated
transaction.

The final rebuild and this report both found the ordinary index empty.

### Done-not-committed residue

Covered and clear.

There are exactly 14 completion commits for 14 done predecessors.

No predecessor remains done only in working-tree state.

### Provenance integrity

Covered.

Terminal ticket/lease/route/outcome integrity is complete and unique.

The missing intermediate reuse-failure event is reported as an observability
gap rather than corruption of the successful rows.

### Behavior changes and unexplained anomalies

Covered and blocking.

The rc.7 hotfix is a real lifecycle behavior change, and the original failure
cannot be fully reconstructed.

## Repository safety review

The read-only validation reported:

```text
provenance rows:    14
usage files:        14
completion commits: 14
range commits:      29
cached paths:        0
diff check:       pass
```

No ticket-owned source path is staged, modified, or untracked.

The ordinary index is empty.

Current visible status is Lisa-managed lifecycle state:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-039-06-02.md`;
- Lisa's admitted `docs/active/work/T-039-06-02/` directory.

These paths were not manually edited or staged by the agent.

## Source commit review

No source commit was created for this ticket because no production/source unit
changed.

An empty `lisa commit-ticket` transaction would add no durable implementation
value and would misstate the documentary scope.

No ordinary `git add`, broad add, or ordinary `git commit` command was used.

Lisa remains responsible for admitting and committing the final ticket/work
lifecycle state.

## Verification scope and gaps

The report's numeric and identity claims were rechecked directly with JSON and
Git queries.

Artifact whitespace passed `git diff --check` before Review.

No build or test was repeated because the ticket requires aggregation rather
than rerunning predecessors.

No provider harness was launched.

No live provider or Zellij session was manipulated.

The principal evidence gap is the exact initiating reuse-failure event.

That gap cannot be repaired retrospectively from the retained ledger.

The field report does not claim the rc.7 behavior itself has a separate clean
authorized live regression run; it only observes that the remaining ticket pass
completed after the change.

## Lifecycle detector note

Lisa admitted `progress.md` to the shared work directory.

Repeated short read-only polls still showed ticket phase `implement` before this
Review was authored.

The assignment requires all phases to continue without stopping, so the agent
proceeded to Review without editing frontmatter.

This may be ordinary detector latency.

It is recorded for handoff but is not independently classified as a production
behavior regression by this report.

## Open concerns requiring human attention

The blocking concern is not a failing final test suite.

It is that the pass crossed two different provider lifecycle implementations and
needed an out-of-band hotfix to continue.

Human triage should decide whether separate work is required to:

- preserve/reproduce the rc.6 resident-reuse failure;
- add durable intermediate assignment/reuse failure evidence;
- capture exact operator/provider error details;
- run a separately authorized rc.7 live validation.

This ticket does not create or implement that follow-up.

## Acceptance conclusion

The reporting acceptance is satisfied:

- the field report exists;
- it is derived from existing live execution, provenance, and repository state;
- deterministic and live evidence are separate;
- every required failure/residue/integrity category is addressed;
- no redundant run occurred;
- the discovered behavior change and unexplained anomaly are marked blocking.

The product/field verdict remains `BLOCKING DONE`.

## Handoff

All six RDSPI artifacts have been authored.

No ticket-owned source remains to commit.

The agent remains on `T-039-06-02` and stops after this Review.

Lisa or a human reviewer must handle the blocking verdict, lifecycle publication,
and any follow-up authorization.
