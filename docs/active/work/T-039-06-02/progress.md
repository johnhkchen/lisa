# Field Report: T-039-06-02

## Verdict: BLOCKING DONE

The field report is complete, but the live pass does not qualify for an
unconditional Done verdict.

The final repository tree has strong green deterministic proof.

All 14 predecessor tickets also have authoritative successful terminal records.

However, the live Codex-seat pass required a production behavior change between
`T-039-02-01` and `T-039-02-02` before assignment could continue.

Commit `0f850b3e5b6cae90f933c828d05286d1db522303`, titled
`fix(codex): relaunch between ticket assignments`, landed inside a 1,029-second
gap between those tickets.

It changed native Codex from resident `/clear` reuse to exiting and launching a
fresh process for each new ticket.

The initiating reuse failure has no persisted failed/timed-out provenance row
and no retained exact dashboard/provider error event.

The behavior change and incomplete failure record satisfy the ticket's explicit
condition for blocking Done.

No repair is attempted in this reporting ticket.

## Scope and non-actions

This report aggregates the completed E-039 Codex-seat pass from
`T-039-01-01` through `T-039-06-01`.

It uses admitted predecessor artifacts, Lisa provenance, per-ticket Codex usage,
Git history, and current repository state.

It does not include the active `T-039-06-02` attempt in predecessor totals.

No predecessor ticket was rerun.

No Codex command was launched.

No Zellij command was launched.

No provider harness or ignored live integration test was launched.

No build, Clippy, format, or workspace test gate was repeated.

No production source was edited.

## Evidence vocabulary

`DETERMINISTIC` identifies controlled tests, builds, hashes, and injected-clock
checks recorded by predecessor work.

`LIVE` identifies records produced by actual Codex ticket executions.

`REPOSITORY` identifies Git, ledger, lifecycle, index, or marker state.

`INFERENCE` identifies a conclusion drawn by connecting retained facts when the
original runtime event was not persisted.

`NOT OBSERVED` means retained evidence has no instance; it does not mean the
behavior is proved impossible.

`UNAVAILABLE` means the pass did not retain enough evidence for a definitive
classification.

## Population summary

The completed live population contains 14 tickets:

- one lint-baseline ticket;
- three signal-ingestion tickets;
- three failure-transition tickets;
- three deadline-evaluator tickets;
- three atomic-publication tickets;
- one final rebuild/gate ticket.

All 14 ticket files currently report `status: done` and `phase: done`.

All 14 have admitted six-phase artifacts.

All 14 have exactly one attempt-private `attempt_id: 1` directory.

All 14 have one per-ticket Codex usage JSON file.

All 14 have one schema-version-2 terminal provenance row.

All 14 have one `Complete T-039-*` Git commit.

The Git interval contains 29 commits total, including ticket source commits,
completion commits, and the intervening rc.7 hotfix.

## Live execution totals

The 14 usage files contain:

```text
input tokens:  31,193,999
output tokens:    304,533
```

Every usage key exactly matches one predecessor ticket ID.

These records establish that the population was executed by real Codex seats.

The terminal provenance rows report null token fields, so token metrics are
retained in the per-ticket usage files rather than duplicated into the ledger.

No cost figure is available.

## Terminal provenance summary

For all 14 predecessor rows:

```text
schema_version: 2
attempt_id:     1
outcome:        done
authoritative:  true
fenced:         false
requested:      codex / openai
actual:         codex / openai
```

There are no duplicate predecessor ticket rows.

There are no missing predecessor ticket rows.

There is no requested/actual provider substitution.

There is no E-039 predecessor `failed` row.

There is no E-039 predecessor `timed-out` row.

There is no E-039 predecessor attempt ID 2.

The terminal ledger is internally consistent for successful outcomes.

## Execution timeline

`T-039-01-01` began at `1783887652` on pane 0 and completed at `1783887898`.

`T-039-02-01` began immediately at `1783887898` on pane 1 and completed at
`1783888390`.

`T-039-02-02` did not begin until `1783889419` on pane 0.

The interval between predecessor completion and dependent start is 1,029 seconds.

Inside that interval, at `1783889301`, the following commit landed:

```text
0f850b3e5b6cae90f933c828d05286d1db522303
fix(codex): relaunch between ticket assignments
```

The hotfix landed 911 seconds after `T-039-02-01` completed.

The dependent began 118 seconds after the hotfix landed.

From `T-039-02-02` onward, the 12 remaining predecessor records proceed through
pane 0/pane 1 alternation without another comparable dependency-boundary gap.

`T-039-06-01` completed at `1783895486` on pane 1.

## Intervening behavior change

The hotfix changed six production/configuration paths:

- `.lisa.toml`;
- `Cargo.toml`;
- `Cargo.lock`;
- `crates/lisa-cli/Cargo.toml`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/lib.rs`.

It bumped Lisa from `0.4.0-rc.6` to `0.4.0-rc.7`.

It added the provider reset strategy `ExitThenFresh`.

It assigned that strategy to native Codex.

It disabled Codex dependence on the clear-handshake capability.

It made a resident Codex seat enter the exit/relaunch path before accepting the
next ticket.

It rewrote associated scheduler tests from clear-based reuse/fresh fallback to
fresh-process delivery plus one same-generation chat retry.

The commit comments explicitly say interactive Codex `/clear` was not a reliable
unattended delivery boundary.

The author/committer is John Chen, not a ticket-owned source transaction from an
E-039 predecessor agent.

### Evidence classification

`REPOSITORY FACT`: the gap, commit, timestamp, diff, and later start are exact.

`LIVE FACT`: the dependent was not assigned a recorded attempt until after the
behavior change.

`INFERENCE`: the hotfix was manual intervention for the failed resident-seat
reuse boundary. This inference is strongly supported by its subject, semantics,
and placement but is not represented as a terminal failure row.

`UNAVAILABLE`: exact UI error text, provider hook event, or scheduler alert that
caused the intervention.

## Deterministic proof on the final tree

The final predecessor `T-039-06-01` rebuilt the release artifacts after all
checked-in changes, including rc.7.

### Release artifacts

The release WASM build passed:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

The release CLI build passed:

```text
cargo build -p lisa-cli --release
```

The release plugin and CLI build-script copy were both 1,411,000 bytes.

They shared SHA-256:

```text
7098c00d1558d6b861842b133fe15067e98f52985df57134147bd35e55d55d5f
```

This proves the freshly rebuilt WASM crossed the CLI embedding boundary.

### Format and lint

Workspace formatting passed.

Native Clippy passed for workspace/all-target/all-feature scope with warnings
denied.

WASM Clippy passed for `lisa-plugin` on `wasm32-wasip1` with warnings denied.

### Tests and canonical check

The direct workspace run executed 768 passing tests with zero failures.

The component totals were:

- 274 CLI unit tests;
- one provider-contract integration test;
- three help-surface integration tests;
- 157 core unit tests;
- 333 plugin unit tests.

One real-Zellij integration test remained intentionally ignored by its existing
environment gate.

`just check` passed the ordinary WASM check and repeated workspace tests.

### Relevant deterministic regression families

Signal regressions prove exact current-attempt admission and stale evidence
rejection across typed consumers.

Failure regressions prove assignment delivery gets one bounded chat retry before
a named `AssignmentDeliveryFailed` terminal state.

Legacy recovery regressions prove at most one successor attempt and no infinite
retry/babysitting path.

Startup regressions prove bounded relaunch and fencing behavior.

Deadline regressions prove injected-clock acknowledgement, transition, review,
health, session, and stale-thread policies with distinct exemptions/actions.

Publication regressions prove exact-byte atomic replacement, hostile-path
cleanup, attribution, and provenance integrity.

Commit transaction regressions prove foreign-index preservation, overlap
rejection, rollback, non-Done restoration, and verified completion publication.

These are deterministic final-tree contracts, not claims that each negative
branch occurred during the live pass.

## Required concern matrix

| Concern | Deterministic proof | Live/repository observation | Disposition |
|---|---|---|---|
| Assignment/reuse failures | Reset, delivery, acknowledgement, and bounded failure tests exist | One 1,029-second reuse gap; rc.7 changes Codex from `/clear` reuse to exit/fresh before the next start | **Observed; BLOCKING** |
| Retries | One chat retry and finite legacy recovery/startup budgets are tested | All predecessor terminal leases remain attempt 1; no duplicate row or Lisa-level attempt retry | Clear for observed terminal history; negative branches not exercised live |
| Timeouts | Injected deadline and hard-silence timeout tests pass | No predecessor ledger row is `timed-out`; the reuse gap is not classified as a timeout | Not observed live; do not infer one |
| Stale panes | Stale-attempt rejection and stale-thread fencing tests pass | No fenced/duplicate E-039 terminal result; old lease marker files remain for completed panes | No stale authority observed; marker residue alone is non-authoritative |
| False delivery errors | Delivery outcomes and isolated commit transactions are deterministic | No retained predecessor report identifies a false-positive delivery/commit error; original reuse error event is unavailable | Not observed in retained evidence; limited by missing event |
| Staged/index residue | Commit fixtures cover foreign stage preservation, overlap, rollback, and cleanup | Every predecessor review records an empty ordinary index; current cached diff is empty | Clear |
| Done-not-committed residue | Completion fixtures cover failure restoration and verified ref publication | Every done predecessor has one completion commit in the final chain | Clear |
| Provenance integrity | Append, hostile target, lease attribution, and current-ticket tests pass | 14 unique authoritative done rows with exact attempt and route identity | Terminal integrity clear; intermediate failure observability gap is **BLOCKING** |
| Behavior change/anomaly | Final tree gates pass after rc.7 | Unscoped provider lifecycle hotfix landed mid-pass; initiating failure is not fully reconstructable | **BLOCKING** |

## Assignment and reuse field observation

The pass did not validate one unchanged provider contract end to end.

It began on rc.6 with native Codex configured for resident clear-handshake reuse.

After `T-039-02-01`, assignment did not progress to a recorded dependent attempt
for 1,029 seconds.

The checkout was changed to rc.7 with exit-then-fresh Codex semantics.

The next dependent then started and the remaining pass completed.

The final green run therefore validates the post-hotfix tree.

It cannot be cited as live proof that the pre-hotfix resident reuse behavior was
correct.

This distinction is the central field result.

## Retry field observation

Every predecessor terminal row is attempt ID 1.

Every predecessor has one usage file and one terminal row.

There is no automatic Lisa-level ticket retry in the retained E-039 history.

There is no evidence of duplicate ownership or two authoritative outcomes for a
single E-039 ticket.

The rc.7 same-process chat retry budget is proven deterministically.

The ledger does not expose internal chat submission counts, so the report does
not claim whether an individual live assignment prompt was submitted twice.

## Timeout field observation

No terminal provenance row is `timed-out`.

No predecessor attempt is fenced.

The 1,029-second assignment interval is a scheduling gap, not a terminal timeout
record.

Calling it a timeout would exceed the retained evidence.

The final tree's timeout behavior remains covered by injected deterministic
tests, including active-session and awaiting-human exemptions.

## Stale-pane field observation

Pane IDs alternate between 0 and 1 across the completed E-039 ledger.

No E-039 row is fenced and no duplicate terminal result appears.

No admitted artifact reports an old pane advancing or completing a replacement
attempt.

Current `.lisa/signals/` state includes lease markers for completed/older panes:

- pane 1 retains `T-039-06-01`, attempt 1;
- pane 2 retains an E-038 lease;
- pane 3 retains an E-038 lease.

The active field-report attempt lease is on pane 0.

These files are marker residue, not proof of current scheduler authority.

The current lease/running-thread relation, not marker presence alone, gates
admission.

The evidence supports “no stale authority observed,” not “no stale file exists.”

## False delivery error field observation

No E-039 terminal record reports delivery failure.

No predecessor review says `lisa commit-ticket` falsely returned failure after a
successful commit.

Every cited ticket-owned source hash resolves and participates in the final
first-parent history.

The reuse intervention is not classified as a false delivery error because the
exact scheduler/provider error was not retained.

Accordingly:

- false delivery error: `NOT OBSERVED` in retained evidence;
- exact initial reuse error: `UNAVAILABLE`;
- absence of a ledger row: an observability limitation, not proof no error fired.

## Staged/index residue field observation

Each predecessor source transaction reports exact include paths and an empty
ordinary index afterward.

The final rebuild independently checked `git diff --cached --name-only` and found
no path.

This report repeated that read-only check and found:

```text
cached_paths=0
```

No crate, manifest, lockfile, or predecessor artifact is currently dirty.

Visible status is limited to Lisa-managed lifecycle state for this active run:

- modified `.lisa/provenance.jsonl`;
- modified `docs/active/tickets/T-039-06-02.md`;
- auto-published `docs/active/work/T-039-06-02/`.

No staged/index residue from predecessor ticket work exists.

## Done-not-committed field observation

Every predecessor marked done has a corresponding `Complete T-039-*` commit.

There are exactly 14 completion commits for the 14-ticket population.

The final completion commit is:

```text
c18efaa8b9fc2ab9a79e3e82d22a76642ca65222
Complete T-039-06-01
```

Ticket-owned source commits are parents/ancestors of their completion commits.

No predecessor is done only in working-tree state.

No done-not-committed predecessor residue is present.

The active field-report ticket is intentionally not done and is outside this
predecessor result.

## Provenance integrity assessment

### Strengths

Terminal coverage is complete: one row per predecessor.

Lease identity is exact: ticket ID and attempt ID agree throughout.

Requested and actual routing agree throughout.

Authority is unambiguous: every successful terminal row is authoritative and
unfenced.

There are no duplicate outcomes.

Git completion history and terminal ledger contain the same predecessor set.

Per-ticket usage keys contain the same predecessor set.

### Limitations

Terminal provenance does not capture the failed reuse transition that prompted
the rc.7 change.

No failed or timed-out row exists for that intervention.

No durable activity log or exact operator alert is part of the admitted evidence.

Provenance token fields are null even though separate usage files contain counts.

These limitations do not corrupt the 14 successful terminal rows.

They do prevent the field report from reconstructing the full causal chain for
the assignment failure.

Because the acceptance criterion requires unexplained anomalies to block Done,
this missing intermediate evidence is material.

## Repository consistency checks

The following read-only checks were performed during this report:

- JSON aggregation of E-039 provenance rows;
- JSON aggregation of E-039 usage files;
- Git first-parent history and completion count;
- hotfix metadata and diff inspection;
- ordinary cached-diff inspection;
- worktree classification;
- `git diff --check`.

Results:

```text
provenance rows:    14
usage files:        14
completion commits: 14
range commits:      29
cached paths:        0
diff check:       pass
```

No mutating Git command was used.

## Source commit disposition

This ticket made no production source change.

It therefore has no meaningful unit for `lisa commit-ticket`.

No empty implementation commit was created.

No ordinary `git add`, `git add -A`, or `git commit` was used.

The six RDSPI artifacts are private attempt artifacts that Lisa admits and
publishes through its lifecycle transaction.

## Acceptance mapping

### Field report exists

Satisfied by this `progress.md`, with the concise handoff in `review.md`.

### Derived from predecessor live execution, provenance, and repository state

Satisfied by the 14 usage files, 14 terminal rows, 14 completion commits,
admitted predecessor artifacts, final gates, and current residue checks.

### Deterministic proof separated from live observations

Satisfied by the dedicated sections and evidence vocabulary above.

### Assignment/reuse failures accounted for

Satisfied and blocking: the rc.6 reuse boundary stalled; rc.7 changed Codex to
exit-then-fresh before the next ticket began.

### Retries accounted for

Satisfied: deterministic budgets are recorded; no live Lisa-level attempt retry
or duplicate authoritative terminal result occurred.

### Timeouts accounted for

Satisfied: deterministic policy tests are recorded; no live terminal timeout is
claimed or observed.

### Stale panes accounted for

Satisfied: deterministic rejection/fencing coverage is distinct from live marker
residue; no stale authority or duplicate terminal result is observed.

### False delivery errors accounted for

Satisfied with limitation: none is retained; the original reuse error event is
unavailable and is not mislabeled.

### Staged/index residue accounted for

Satisfied: predecessor and current checks show an empty ordinary index.

### Done-not-committed residue accounted for

Satisfied: every done predecessor has its completion commit.

### Provenance integrity accounted for

Satisfied: terminal integrity is complete, while the missing intermediate reuse
failure record is explicitly identified.

### No redundant rerun

Satisfied: no provider, Zellij, predecessor, build, or test rerun was launched.

### Behavior change or unexplained anomaly blocks Done

Satisfied: this report's verdict is `BLOCKING DONE`.

## Required follow-up boundary

This ticket does not authorize the follow-up itself.

Human triage should decide whether to create a separate defect/investigation for:

- the rc.6 `/clear` reuse failure reproduction;
- durable intermediate assignment/reuse failure provenance;
- preservation of exact operator/provider error evidence;
- explicit validation of rc.7 exit-then-fresh behavior in an authorized live run.

Until that decision resolves the behavior change and provenance gap, this ticket
must not be certified Done.

## Implementation status

The requested report is implemented.

No source work remains.

The only remaining RDSPI phase is Review.
