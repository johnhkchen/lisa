# Structure — T-046-06-02 baseline-run-record

## Change boundary

This ticket's implementation is primarily evidence documentation.

No Rust crate, fixture Dockerfile, shared runbook, release configuration, or
test program changes are required to describe the current blocked state.

One shared ticket file is created for an uncovered product finding.

All RDSPI and evidence artifacts are written to the attempt-private work
directory specified by the assignment.

The shared `docs/active/work/T-046-06-02/` directory is not edited.

The ticket frontmatter phase and status are not edited.

## Private work directory

Root:

`.lisa/attempts/T-046-06-02/1/work/`

Existing control files remain unchanged:

- `assignment-1-1784243119844602000.md`
- `.lisa-launch-2.sh`

The following artifacts are created:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `baseline-probe.md`
- `progress.md`
- `review.md`
- `review-disposition.json`

## `research.md`

Purpose: descriptive map of ticket requirements, story/epic context, fixture,
runbook, retained container, provider transcript, observed chain, evidence gaps,
and existing ticket ownership.

It distinguishes observed values from required-but-missing measurements.

It contains no proposed product implementation.

It records the repository dirty-state boundary so later phases do not sweep
unrelated paths.

## `design.md`

Purpose: compare evidence-handling options and select the partial-probe plus
blocked-disposition approach.

It rejects reconstruction of missing time/disk measurements.

It rejects autonomous provider reruns from this implementation session.

It defines admission terminology and the unblock contract.

It decides to file the xz prerequisite as T-046-03-03.

## `structure.md`

Purpose: define file ownership, artifact boundaries, evidence schema, source
transaction boundary, and validation surface.

It does not contain result claims beyond those needed to explain the structure.

## `plan.md`

Purpose: order the implementation into independently verifiable steps.

It identifies the new ticket as the only shared source unit.

It defines checks for JSON shape, prohibited success wording, exact-path source
cleanliness, and evidence completeness.

## `baseline-probe.md`

Purpose: preserve the sanitized retained Codex/mini probe in a reviewer-readable
form.

This is not named `baseline-results.md` because that would imply the acceptance
instrument was completed.

The file opens with a prominent non-admission statement.

It uses the runbook template fields where evidence exists.

Unavailable required fields are set to `NOT RECORDED`, not guessed.

It records:

- date and fixture class;
- container, image, base, architecture, and caps;
- Codex CLI version and exact mini model;
- ChatGPT device auth provenance from predecessor evidence;
- exact tested prompt and its deviation;
- derived transcript interval with its narrower meaning;
- unavailable disk delta;
- observed command chain;
- exact README/installer/doctor/shell strings;
- visible artifacts;
- visible negative facts;
- missing acceptance checks; and
- findings mapped to tickets.

It contains no credentials, auth output, environment dump, or copied provider
configuration.

## `progress.md`

Purpose: implementation ledger for this ticket attempt.

It records phase completion, evidence extraction, deviations, finding filing,
validation, source commit receipt, and remaining human work.

It explicitly states whether any ticket-owned source path remains staged,
modified, or untracked.

## `review.md`

Purpose: final reviewer handoff.

It lists every created file and the shared ticket commit.

It assesses both acceptance criteria separately.

It summarizes what the retained probe proves and does not prove.

It documents validation coverage and gaps.

It names the exact human actions required to unblock.

It ends without starting or describing work on a dependent ticket.

## `review-disposition.json`

Purpose: machine-readable Review decision.

The exact schema is:

```json
{"disposition":"block","reason":"<non-empty actionable reason>"}
```

No additional keys, whitespace-dependent annotations, or Markdown wrapper are
allowed.

The reason must identify both missing primary legs and their missing runbook
records.

## Shared finding ticket

New path:

`docs/active/tickets/T-046-03-03.md`

Frontmatter shape:

```yaml
---
id: T-046-03-03
story: S-046-03
title: xz-free-shell-installer
type: bug
status: open
priority: critical
phase: ready
depends_on: [T-046-03-01]
---
```

The file belongs to the static-linux-artifacts story because it concerns the
format and extraction requirements of the Linux shell-install artifact.

The context records the actual fixture observation:

- README installer resolved v0.3.0 aarch64 GNU archive;
- `tar` attempted to invoke absent `xz`;
- installation stopped before Lisa was placed;
- the fixture intentionally excludes xz; and
- Python could work around the failure, but the user path must not require that
  improvisation.

The new ticket is a bug report, not an implementation of a chosen archive
format.

Its acceptance criteria remain behavioral:

- the README one-liner installs on the no-xz fixture;
- no xz/toolchain package is installed as a prerequisite;
- both Linux architectures are covered; and
- release verification protects the behavior.

The ticket file does not modify S-046-03's currently untracked story document.

That avoids merging this attempt into an unrelated untracked planning file.

The finding remains discoverable through its `story` frontmatter and ticket ID.

## Source commit boundary

The new ticket is the only ticket-owned shared source path.

It is committed as one meaningful unit with:

```text
lisa commit-ticket --ticket-id T-046-06-02 \
  --message "docs: file xz-free installer finding" \
  --include docs/active/tickets/T-046-03-03.md
```

No phase artifact is included in that command because Lisa owns attempt-artifact
publication and completion.

No ordinary Git index operation is used.

No broad path or directory include is used.

Before Review, the exact source path must be absent from ordinary index entries,
working-tree modifications, and untracked output.

## Evidence source boundaries

Sanitized sources used read-only:

- `docs/knowledge/chromebook-install-test.md`
- `docs/active/work/T-046-06-01/review.md`
- Docker metadata for `cbt-0716-144625`
- Docker changed-path listing for the same container
- one Codex session JSONL streamed from the stopped container
- selected shell-history command names

Credential-bearing paths are not artifact inputs:

- `/home/tester/.codex/auth.json`
- `/home/tester/.claude/.credentials.json`
- full environment variables
- login logs
- provider configuration directories

The provider transcript is not copied wholesale.

Only ticket-relevant prompt, response, command, and error text is summarized or
quoted.

## Interfaces and invariants

The runbook remains the normative interface for future human reruns.

The private `baseline-probe.md` is an evidence supplement, not a runbook
replacement.

The new bug ticket is an ownership interface for the xz finding.

The Review JSON is the machine interface to Lisa's completion logic.

The following invariants apply:

- no missing value is represented as zero;
- no derived value is labeled as directly measured;
- no probe is labeled a completed leg;
- no credential material is persisted;
- no unrelated dirty file is staged or committed;
- no ticket phase/status is manually advanced; and
- no dependent ticket is started.

## Validation structure

Markdown whitespace validation:

```text
git diff --check -- <new-ticket-path>
```

Private artifacts are also checked for trailing whitespace using a read-only
search or diff against `/dev/null`.

JSON shape validation checks exact parsed equality with:

```json
{"disposition":"block","reason":"..."}
```

Evidence terminology validation searches for accidental pass claims such as
"baseline complete" or "leg B passed."

Completeness validation confirms all eight required attempt artifacts exist and
are nonempty.

Source ownership validation checks:

- `git status --short -- docs/active/tickets/T-046-03-03.md`
- ordinary index entries for that exact path;
- the receipt commit's file list; and
- the commit message.

No workspace Rust test is structurally required because no executable product
code changes.

Repository ticket validation is appropriate if it can be scoped without
mutating unrelated planning files.

## Ordering constraints

Research precedes the evidence-handling decision.

Design precedes file and ticket structure.

Structure precedes the execution plan.

The probe artifact is written before the new ticket so the bug context is based
on preserved evidence.

The new ticket is validated before its isolated commit.

Progress is updated after the receipt exists.

Review and disposition are written last.

The disposition remains block regardless of successful local documentation
checks because the missing human evidence is the acceptance boundary.

## Structure conclusion

The implementation has one small shared source unit and a complete private
evidence/handoff set.

The structure prevents a partial historical transcript from masquerading as a
measured baseline while ensuring the unique failure chain and uncovered xz gap
are not lost.
