# Review — T-046-06-03 closing acceptance run

## Disposition summary

**Block — operator remedy required.**

The retained closing-named containers do not satisfy the closing protocol.

Both exercised the pre-fix v0.3.0 installation surface rather than the current
fixed surface.

Neither preserved the required snapshots or complete operator-graded matrix.

The seeded Zellij 0.40.1 recovery and landing-probe rematch are also absent.

No acceptance criterion is fully evidenced.

## Work completed

All six RDSPI phases were completed in this attempt-private directory.

Created:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `closing-attempts-2026-07-16.md`;
- `progress.md`;
- this `review.md`;
- `review-disposition.json`.

No executable source, release workflow, fixture, README, runbook, shared ticket,
or shared landing-probe artifact was changed by this attempt.

No file was deleted.

No ticket phase/status field was manually edited.

## Evidence inspected

Read the ticket, story, full Chromebook test runbook, RDSPI workflow, prerequisite
reviews, and landing-probe benchmark.

Searched repository and attempt artifacts for closing primary records, seeded
recovery, and tour-rematch evidence.

Inspected two stopped containers read-only:

- `cbt-0716-182723-claude-a`;
- `cbt-0716-184858-codex-b`.

Both have the required 4 GiB/two-CPU caps and zero host mounts.

Both remained stopped throughout inspection.

Selected Docker metadata, changed paths, provider-session rows, and filtered
shell-history commands were used.

No authentication/config directory, token, provider account detail, or
environment dump was copied into ticket artifacts.

## Failed-attempt artifact

`closing-attempts-2026-07-16.md` preserves the useful facts.

It labels both runs `NOT ADMITTED`.

It labels unavailable measurements `NOT RECORDED`.

It records exact relevant installer strings, apt actions, model/prompt facts,
changed-path summaries, and missing checks.

It does not present final Docker layer size or container lifetime as a runbook
measurement.

It does not retroactively change either prompt or model.

## Claude attempt assessment

The Claude session used `claude-sonnet-5`.

The ticket requires a Haiku-class model.

Its prompt referenced an entire README file rather than embedding exact
install-section bytes.

Shell history shows the README came from fixed pre-fix commit
`b5af5fa9d2ac304edfad2e9992ae11bd04834e98`.

The installer resolved:

> downloading lisa-cli 0.3.0 aarch64-unknown-linux-gnu

It failed because xz was absent.

The agent installed `xz-utils` with sudo apt.

It installed Lisa and manually downloaded Zellij into `~/.cargo/bin`.

The old doctor printed “All dependencies satisfied.”

That is a useful reproduction of the baseline chain, not a closing pass.

The attempt has no before/after time files.

It has no before/after disk files.

It has no complete PATH/doctor/init/validate/dry-run exit record.

Shell history contains no validate or dry-run command.

It has no independent final negative-check record.

## Codex attempt assessment

The Codex session used CLI 0.144.5 with `gpt-5.4-mini`.

Its first prompt referenced `/tmp/lisa-README.md`.

Only mistyped `/tmp/lisa-README.m` existed.

That file contained the pre-fix mechanism-first README.

The installer again resolved v0.3.0 GNU Linux and failed on xz.

The agent installed `xz-utils` with sudo apt.

It installed Lisa into `~/.cargo/bin` and stopped before doctor.

The operator then sent a second message asking for Zellij help.

That breaks the one-instruction hands-off control.

The agent attempted apt Zellij installation, then downloaded Zellij 0.44.3.

The attempt has no runbook time or disk snapshots.

It has no complete operator-graded positive/negative matrix.

Shell history contains no validate or dry-run command.

## Acceptance criterion 1

> A passing closing run is recorded for both legs with every positive and
> negative criterion holding.

**Unmet.**

The Claude model tier is wrong.

Both surfaces and prompts are wrong.

The Codex run includes a second human hint.

Both installed xz.

Both lack measurements and complete exit/negative records.

Neither run can be admitted.

## Acceptance criterion 2

> The seeded 0.40-Zellij variant records recovery through Lisa's strings alone.

**Unmet.**

No container, transcript, or result artifact records Zellij 0.40.1 seeding.

The manual Zellij work visible in the retained attempts came from the pre-fix
README path, not current Lisa floor diagnostics.

No new failure-specific product ticket was needed because no current fixed
surface was tested.

If a conforming seeded rerun fails, that run must file a concrete new bug and
keep this gate open.

## Acceptance criterion 3

> The short-prompt tour rematch names coding agents and the don't-babysit
> purpose unprompted, with a baseline comparison.

**Unmet.**

No new `lisa-tour.html` exists in this attempt.

No dated metadata/score comparison exists.

T-047-01-02 already owns the full loop-built rematch and remains blocked on the
same human-operated evidence boundary.

This attempt did not create a duplicate tour ticket or surrogate page.

## Finding ownership

The retained attempts reproduce already-owned pre-fix behavior:

- v0.3.0 stable-channel skew → T-046-03-02;
- xz-dependent old archive → T-046-03-03;
- unmanaged/manual Zellij → T-046-02-*;
- source-oriented old documentation → T-046-04-*;
- purpose-first surface → T-046-07-*;
- landing-probe publication → T-047-01-02.

Because these runs did not exercise the fixed current surface, opening duplicate
bugs would misstate the evidence.

## Verification coverage

Confirmed all required phase and evidence artifacts are present and nonempty.

Checked private Markdown artifacts for trailing whitespace.

Confirmed the failed-attempt artifact contains both container names and explicit
`NOT ADMITTED`/`NOT RECORDED` markers.

Confirmed the disposition JSON parses and carries an operator owner, nonempty
reason, one-sentence ask, concrete steps, and read-only check.

Confirmed both inspected containers remain stopped.

Confirmed no ticket-owned shared source entry exists in the ordinary Git index.

No Rust tests were run because the ticket changed no executable source.

## Source transaction review

There was no ticket-owned shared source unit to commit.

No `lisa commit-ticket` invocation was required.

No ordinary `git add`, broad add, or ordinary commit command was used.

Unrelated journal, provenance, lifecycle, and concurrent worktree state was not
staged, reset, or included.

## Open concerns

The current evidence says nothing about whether the latest fixed public surface
passes the protocol.

It only proves that two closing-named runs accidentally repeated the pre-fix
path.

The operator must use live `main`, not the preserved baseline commit.

The operator must create snapshot files before launching the agent.

The operator must run dry-run, not a real loop, for the primary acceptance
matrix.

The primary Claude leg must use an exact Haiku-class model.

The Codex leg must receive no second hint.

## Required handoff

John must run the exact current runbook from fresh containers and place five
sanitized artifacts in this attempt directory:

1. `closing-primary-claude.md` for the Haiku-class instruction-A leg.

2. `closing-primary-codex.md` for the mini-class instruction-A leg.

3. `closing-seeded-zellij-0.40.1.md` for recovery using Lisa strings alone.

4. `lisa-tour-closing.html` as the unedited short-prompt output.

5. `landing-probe-closing.md` with model/method/surface metadata, rubric scores,
   and the 2026-07-16 comparison.

The two primary records must contain exact model IDs, live README identity,
prompt bytes, before/after snapshots, all exit values, all negative results,
sudo/apt actions, relevant strings, and changed-path summaries.

Only after all five artifacts exist and their contents pass the criteria can a
future Review change disposition to pass.

## Conclusion

Agent-owned inspection and documentation are complete.

The ticket is not ready for completion.

`review-disposition.json` blocks on the exact human-operated evidence needed to
close the epic honestly.

Remain on T-046-06-03 and let Lisa retain/park the ticket; do not start another
ticket.
