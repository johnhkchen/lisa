# Progress — T-046-06-02 baseline-run-record

## Current status

- Research: complete.
- Design: complete.
- Structure: complete.
- Plan: complete.
- Implement: complete for all work possible without new human-operated runs.
- Review: pending at the time of this entry.
- Expected disposition: block.

## Ownership baseline

The shared worktree was dirty before this attempt.

Unrelated state included modified Lisa provenance/journal files, a modified
`justfile`, and numerous untracked epic, story, ticket, work, and knowledge
documents.

This ticket did not edit, stage, reset, stash, or commit any of those paths.

The attempt-private directory initially contained only the assignment and Lisa
launch helper.

`docs/active/tickets/T-046-03-03.md` did not exist at the ownership baseline.

## Phase artifacts completed

Created `research.md` with the fixture, runbook, retained-container, transcript,
evidence-gap, and E-046 ownership map.

Created `design.md` comparing reconstruction, acceptance, rerun, and blocked
handoff options.

Created `structure.md` defining private artifact ownership, the evidence schema,
and one shared bug-ticket source unit.

Created `plan.md` with ordered extraction, preservation, filing, commit,
validation, and Review steps.

All phase artifacts are in:

`.lisa/attempts/T-046-06-02/1/work/`

No phase/status frontmatter field was edited.

No artifact was written to `docs/active/work/T-046-06-02/`.

## Retained evidence inspection

Inspected stopped container `cbt-0716-144625` without starting it.

Recorded:

- fixture image ID;
- Debian base digest;
- arm64 architecture;
- image size;
- 4 GiB memory cap;
- two-CPU cap;
- empty host mount list;
- stopped state;
- final writable-layer size; and
- changed-path listing.

No credential file, login log, environment dump, or provider configuration was
copied into the repository.

## Codex probe extraction

Identified retained Codex session
`019f6ce8-6bc9-71c1-ac92-f37697183459`.

Its metadata records:

- Codex CLI 0.144.5;
- model `gpt-5.4-mini`;
- medium reasoning effort;
- `/home/tester` working directory; and
- 2026-07-16 execution.

Streamed only ticket-relevant session messages, function calls, and command
outputs.

Recovered the actual prompt, command chronology, exact public strings, agent
success declaration, and operator PATH contradiction.

The main provider turn lasted approximately 234.7 seconds by transcript
timestamps.

That duration is explicitly not labeled as runbook wall time.

## Measurement audit

Checked the stopped container for the runbook's prompt and snapshot files.

The following were absent:

- `/tmp/install-section.md`;
- `/tmp/instruction.txt`;
- `/tmp/disk.before`;
- `/tmp/disk.after`;
- `/tmp/t.before`; and
- `/tmp/t.after`.

The final Docker writable-layer size cannot isolate the install because the box
also contains later auth, plugin, and tour data.

No wall-clock or disk-delta value was reconstructed.

Repository-wide search found no completed runbook-template baseline record.

No Claude/Haiku install-session record was found.

## Partial evidence artifact

Created `baseline-probe.md`.

It is prominently marked as not admitted.

It records unavailable measurement fields as `NOT RECORDED`.

It records the exact prompt deviation.

It preserves the actual chain:

- latest resolved v0.3.0;
- prebuilt Zellij was installed proactively;
- the Lisa installer required missing xz;
- Python/lzma was used as a manual workaround;
- one archive-root path attempt was wrong;
- a second direct install made doctor green under a local PATH override;
- the agent declared success; and
- plain `lisa` failed immediately afterward in the operator shell.

It maps known findings to existing E-046 tickets.

## New finding filed

The no-xz installer failure was not fully owned by existing ticket acceptance
criteria.

Created:

`docs/active/tickets/T-046-03-03.md`

Title: `xz-free-shell-installer`.

Type/status/phase: bug / open / ready.

Story: S-046-03.

Dependency: T-046-03-01.

The ticket requires the exact README shell installer to work on fresh arm64 and
amd64 Chromebook-test containers without xz or toolchain installation.

It preserves static-musl and release-integrity requirements while leaving the
archive/extraction implementation open.

## Ticket validation

Before commit:

- `git diff --check -- docs/active/tickets/T-046-03-03.md` passed.
- `lisa validate` passed.
- Lisa reported 137 tickets, 5 ready, and a valid DAG.
- the exact new ticket path was untracked and absent from the ordinary index.

After commit:

- `lisa validate` passed again with the same ticket and DAG counts.
- the ticket path is clean.
- the ordinary index has no entry for the path.

## Isolated source transaction

Committed the one meaningful ticket-owned shared source unit with:

```text
lisa commit-ticket --ticket-id T-046-06-02 \
  --message "docs: file xz-free installer finding" \
  --include docs/active/tickets/T-046-03-03.md
```

Receipt commit:

`4dcc94c352969af794341497746df9515df2894b`

Commit inspection shows exactly one file:

`docs/active/tickets/T-046-03-03.md`

The commit adds 48 lines and contains no unrelated path.

No ordinary `git add`, `git add -A`, or `git commit` command was used.

## Plan deviations

No substantive deviation occurred.

The new finding ticket was validated successfully without editing the currently
untracked S-046-03 story document.

The retained provider transcript had enough detail to preserve a useful probe,
but not enough to change the planned blocked disposition.

## Acceptance status

Criterion 1 is unmet.

There is no complete Claude/Haiku runbook record.

There is no admissible Codex/mini runbook record because its prompt deviated and
its required measurements/checks are missing.

Criterion 2 is addressed for every finding visible in the retained probe:

- T-046-03-02: release-channel skew;
- T-046-03-01: install location and PATH guidance;
- T-046-02-01/T-046-02-02: managed Zellij;
- T-046-04-01: source-build documentation exposure;
- T-046-04-02: cargo-first/remedy string scope; and
- T-046-03-03: missing-xz shell installer.

The ticket cannot pass while criterion 1 is unmet.

## Remaining work requiring John

Run a fresh Claude/Haiku instruction-A baseline leg in its own capped container.

Run a separate fresh Codex/mini instruction-A baseline leg in its own capped
container.

For each, preserve:

- exact container/image/model/auth identity;
- before/after epoch snapshots;
- before/after disk-used snapshots;
- exact instruction bytes;
- independent PATH/doctor/init/validate/dry-run exits;
- final negative checks;
- sudo/apt actions;
- questions;
- exact Lisa/docs strings followed; and
- sanitized changed-path/artifact summary.

After those two records are available, this ticket can re-enter Review and be
assessed for pass.

## Implementation conclusion

All safe agent-side preparation, verification, preservation, finding routing,
and exact-path source work is complete.

The remaining condition is intentionally outside autonomous implementation.

Review must block rather than treat the partial probe as a completed matrix leg.
