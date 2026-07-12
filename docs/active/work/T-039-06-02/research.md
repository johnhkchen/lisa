# Research: T-039-06-02

## Ticket boundary

This ticket is the closing field-report ticket for story `S-039-06`.

Its input is the already-completed Codex-seat execution of epic `E-039`.

It must not rerun predecessor tickets or launch another provider harness.

The field report must distinguish deterministic proof from live observations.

It must account for assignment/reuse failures, retries, timeouts, stale panes,
false delivery errors, staged/index residue, done-not-committed residue, and
provenance integrity.

Any behavior change or unexplained anomaly blocks Done under the acceptance
criterion.

The ticket is documentation-only unless the evidence itself is missing.

The story explicitly says newly discovered defects are reported rather than
fixed in this slice.

## Execution population

The completed pass begins with `T-039-01-01` and ends with `T-039-06-01`.

There are 14 completed predecessor tickets in that interval:

1. `T-039-01-01` clears the test-only Clippy baseline.
2. `T-039-02-01` characterizes signal consumers.
3. `T-039-02-02` introduces typed signal ingestion.
4. `T-039-02-03` locks the ingestion regression.
5. `T-039-03-01` maps the failure state machine.
6. `T-039-03-02` names failure transition outcomes.
7. `T-039-03-03` locks bounded recovery behavior.
8. `T-039-04-01` characterizes deadline policies.
9. `T-039-04-02` introduces the clock-injected evaluator.
10. `T-039-04-03` locks cross-policy deadline behavior.
11. `T-039-05-01` characterizes publication sites.
12. `T-039-05-02` introduces atomic publication.
13. `T-039-05-03` locks hostile publication and provenance paths.
14. `T-039-06-01` rebuilds the release WASM and CLI and runs final gates.

Every ticket frontmatter currently reports `status: done` and `phase: done`.

Each ticket has an admitted six-artifact work directory under
`docs/active/work/<ticket-id>/`.

Each ticket has exactly one attempt-private directory with attempt ID 1.

No second attempt directory exists for an E-039 predecessor.

## Provenance ledger

`.lisa/provenance.jsonl` contains 14 schema-version-2 rows for the population.

Each row has the expected matching `ticket_id` and attempt lease.

Every attempt lease has `attempt_id: 1`.

Every row reports `outcome: "done"`.

Every row reports `authoritative: true`.

Every row reports `fenced: false`.

Every requested route is `method: codex`, `provider: openai`.

Every actual route is also `method: codex`, `provider: openai`.

No route substitution appears in this pass.

No E-039 ledger row reports `failed` or `timed-out`.

The recorded pane IDs alternate between pane 0 and pane 1 after the first row.

The first two records are contiguous at the completion/start boundary.

Most later records are likewise contiguous at their dependency boundary.

The single material scheduling gap is between `T-039-02-01` ending at
`1783888390` and `T-039-02-02` starting at `1783889419`.

That gap is 1,029 seconds.

The ledger has no failure row explaining this interval.

## Usage evidence

`.lisa/codex/` contains one usage JSON file for each of the 14 predecessors.

The `key` in each file equals its ticket ID.

The files contain non-null input and output token counts.

Their aggregate is 31,193,999 input tokens and 304,533 output tokens.

The usage files establish that real Codex executions occurred.

The provenance rows themselves have null token fields, so usage attribution is
split between the per-ticket usage files and the terminal JSONL ledger.

No cost value is available in the ledger.

## Git completion chain

The pass is represented on the first-parent history from the pre-pass revision
`ebc0cf237d0f7aebb12212152dd8e195dfcf2398` to current predecessor completion
`c18efaa8b9fc2ab9a79e3e82d22a76642ca65222`.

There are 14 commits titled `Complete T-039-...`, one per predecessor.

Ticket-owned source commits precede their corresponding completion commits.

The completion commits publish ticket frontmatter and admitted work artifacts.

`T-039-06-01` has no source commit because it was a build-only verification.

Current HEAD is `c18efaa`, `Complete T-039-06-01`.

The ordinary index is currently empty.

Current visible modifications are Lisa-owned lifecycle state only:

- `.lisa/provenance.jsonl` contains the uncommitted ledger tail;
- `docs/active/tickets/T-039-06-02.md` records the active Research phase.

No crate, manifest, lockfile, or predecessor work artifact is currently dirty.

## Intervening behavior change

One non-ticket completion/source commit appears inside the execution chain:

`0f850b3e5b6cae90f933c828d05286d1db522303`

Its subject is `fix(codex): relaunch between ticket assignments`.

It was committed at epoch `1783889301`, inside the 1,029-second scheduling gap.

It changed six production/configuration files and moved the version from
`0.4.0-rc.6` to `0.4.0-rc.7`.

It added `ResetStrategy::ExitThenFresh` for native Codex.

It changed native Codex from resident `/clear` reuse to exiting the resident TUI
and launching a fresh process before assigning the next ticket.

It changed Codex signal capabilities so a clear handshake is no longer required.

It rewrote the related assignment/recovery tests around fresh-process delivery.

Its author and committer are John Chen rather than the ticket agent identity.

Its message and code comments state that interactive Codex `/clear` was not a
reliable unattended delivery boundary.

The commit is an actual provider lifecycle behavior change, not a report-only or
test-only adjustment.

The temporal correlation connects it to the observed assignment gap, although
the repository does not contain a terminal failure ledger row for the failed
reuse before the hotfix.

## Assignment and reuse observations

`T-039-01-01` completed on pane 0.

`T-039-02-01` then ran on pane 1 and completed normally.

The next dependent ticket did not start for 1,029 seconds.

The relaunch hotfix landed 911 seconds after predecessor completion.

`T-039-02-02` began 118 seconds after that hotfix.

The remaining 12 predecessor tickets completed on alternating panes without a
second comparable gap or an attempt-level failure record.

Repository evidence therefore records one live reuse/assignment failure that
required an out-of-band behavior change.

The exact UI error text or provider event that initiated intervention is not
persisted in the admitted artifacts or terminal ledger.

## Retries and timeouts

No predecessor has attempt ID 2.

No predecessor has multiple provenance rows.

No predecessor has a `failed` or `timed-out` terminal row.

The live pass therefore shows no Lisa-level attempt retry or terminal timeout.

The unexplained scheduling gap is not represented as a timeout outcome.

Deterministic tests separately cover one bounded chat retry, one assignment
successor in the legacy path, one startup successor/relaunch, timeout reclaim,
and stale-thread reclaim.

Those tests prove policy behavior; they are not live observations that those
failure branches fired during this pass.

## Stale panes and markers

The ledger shows pane reuse between pane 0 and pane 1 across completed tickets.

The live pass has no fenced outcome and no stale-thread failure outcome.

Current signal state retains lease marker files for completed panes, including
pane 1 at `T-039-06-01` attempt 1 and older pane 2/pane 3 markers from E-038.

The active ticket lease is on pane 0.

Lease marker presence alone does not prove a pane is scheduled or authoritative;
the current lease and running-thread state are the scheduler authority.

No admitted report claims that an old pane advanced a newer E-039 attempt.

No provenance duplication indicates stale-pane completion.

## Delivery errors

No E-039 provenance row reports delivery failure.

No predecessor review records a false-positive `lisa commit-ticket` failure.

All ticket-owned source commit hashes cited by predecessor reviews resolve to
commits in the final first-parent chain.

The live reuse failure is visible as a scheduling gap plus hotfix, not as a
persisted delivery-error outcome.

Consequently, the evidence cannot classify the missing UI/error event as a true
or false delivery error.

That missing event detail is an evidence limitation, not proof of absence.

## Index and completion residue

Every predecessor review reports the ordinary index empty after its source
transaction.

The final rebuild independently found the ordinary index empty.

Current `git diff --cached --name-only` is empty.

All ticket-owned source changes are committed in the final history.

Every predecessor has a completion commit and `done/done` frontmatter.

No predecessor remains done in runtime state without a completion commit.

The uncommitted provenance tail is Lisa-managed aggregate lifecycle state and is
not ticket-owned staged residue.

## Deterministic proof boundary

The final direct workspace run executed 768 passing tests with zero failures.

One real-Zellij integration test remained intentionally ignored.

Native and WASM Clippy passed with warnings denied.

Formatting, release plugin build, release CLI build, WASM check, and `just check`
all passed.

The release WASM and CLI build-script copy shared SHA-256
`7098c00d1558d6b861842b133fe15067e98f52985df57134147bd35e55d55d5f`.

This deterministic proof covers the final tree after the intervening rc.7 fix.

It does not erase the fact that the pass changed behavior midway.

## Research conclusion

The repository provides strong deterministic proof and a complete successful
terminal record for all 14 predecessors.

It also provides direct evidence of one mid-pass native Codex reuse failure and
an out-of-band production behavior change used to continue the pass.

The original live failure event is not represented by a failed/timed-out ledger
row, so its exact transition and error classification cannot be reconstructed.

Under this ticket's acceptance criterion, the behavior change and missing
failure provenance are blocking evidence for Done.
