# Plan — T-046-06-02 baseline-run-record

## Goal

Preserve all admissible facts from the retained Codex/mini install probe, file
the uncovered xz prerequisite, verify the evidence boundary, and produce a
blocked Review handoff that names the two human-operated reruns required for
completion.

## Step 1 — freeze the ownership baseline

Inspect the ordinary worktree before creating shared source.

Record unrelated modifications and untracked files without changing them.

Confirm `docs/active/tickets/T-046-03-03.md` does not already exist.

Confirm this ticket's attempt directory contains no preexisting baseline result
that should be preserved.

Verification:

- exact new ticket path is absent;
- no current-ticket source file is staged;
- unrelated `justfile`, Lisa state, stories, epics, tickets, and work directories
  remain outside ownership.

## Step 2 — identify the retained evidence object

Inspect stopped container `cbt-0716-144625` read-only.

Capture container state, image ID, architecture, resource caps, mount list,
image size, base digest, and final writable-layer size.

List changed paths without copying provider auth directories.

Identify the Codex session containing the install probe.

Verification:

- image matches the predecessor's tested fixture;
- memory is 4 GiB;
- CPUs are capped at two;
- mount list is empty;
- session CLI is Codex 0.144.5;
- session model is `gpt-5.4-mini`.

## Step 3 — extract only sanitized probe facts

Stream the install session JSONL from the stopped container.

Read user/assistant messages, tool command arguments, and tool outputs relevant
to the Lisa installation.

Do not read or persist `auth.json`, Claude credential JSON, login logs, or
environment dumps.

Build a chronological chain from the prompt through the agent declaration and
the operator's immediate PATH check.

Record the exact tested prompt.

Record exact public README, installer, doctor, and shell strings.

Verification:

- every quote is present in the retained session;
- the prompt deviation from instruction A is explicit;
- timestamps are labeled transcript timestamps;
- no credentials appear in notes or artifacts.

## Step 4 — audit runbook-required measurements

Check for:

- `/tmp/install-section.md`;
- `/tmp/instruction.txt`;
- `/tmp/disk.before`;
- `/tmp/disk.after`;
- `/tmp/t.before`; and
- `/tmp/t.after`.

Check repository and attempt artifacts for a completed result-template section.

Check for a separate Claude/Haiku install session.

Verification:

- absent files are reported as absent, never zero;
- final Docker writable size is not substituted for disk delta;
- transcript duration is not substituted for runbook wall time;
- later Claude tour state is not substituted for an install leg.

## Step 5 — write `baseline-probe.md`

Create the partial evidence artifact in the attempt-private work directory.

Lead with a non-admission banner.

Use runbook-template field names where practical.

Set missing required values to `NOT RECORDED`.

Include the derived transcript interval only in a separately labeled context
field.

Include container artifacts and changed-path summary.

Map each observed finding to an existing E-046 ticket or the new finding.

Verification:

- artifact does not say PASS;
- artifact does not claim a completed baseline leg;
- missing disk and wall fields are visibly incomplete;
- exact strings and deviations are included;
- no secret-bearing path content is included.

## Step 6 — file the no-xz installer finding

Create `docs/active/tickets/T-046-03-03.md` with ready/open bug frontmatter.

Place it under S-046-03 because the defect is in the static Linux shell-install
artifact delivery path.

Describe the real v0.3.0 fixture failure and current cargo-dist `.tar.xz`
boundary.

Keep the acceptance criteria behavioral rather than prescribing tar.gz,
embedded decompression, or another implementation.

Require both supported Linux architectures to be verified.

Require the no-xz/no-toolchain fixture invariant to remain intact.

Verification:

- frontmatter parses;
- title and context identify one bounded bug;
- acceptance can be tested on the fixture;
- ticket does not absorb stable release, managed runtime, or docs-remedy work;
- `git diff --check` passes for the exact path.

## Step 7 — commit the shared source unit

Run exactly:

```text
lisa commit-ticket --ticket-id T-046-06-02 \
  --message "docs: file xz-free installer finding" \
  --include docs/active/tickets/T-046-03-03.md
```

Do not run `git add`, `git add -A`, ordinary `git commit`, reset, checkout, or
stash.

Record the receipt commit hash in `progress.md` and `review.md`.

Verification:

- receipt reports success;
- receipt commit contains exactly one path;
- exact new ticket path is clean afterward;
- ordinary index has no entry for it;
- unrelated worktree state remains untouched.

## Step 8 — assess the acceptance criteria

Criterion 1 requires one runbook record for each of the two primary legs.

Mark it unmet because:

- the Codex probe used a deviating prompt;
- Codex wall and disk snapshots are absent;
- Codex independent acceptance results are incomplete; and
- no Claude/Haiku baseline leg exists.

Criterion 2 requires all findings to have ownership.

Map channel skew to T-046-03-02.

Map local-bin/PATH behavior to T-046-03-01.

Map managed Zellij to T-046-02-01/T-046-02-02.

Map documentation/remedy steering to T-046-04-01/T-046-04-02.

Map no-xz extraction to new T-046-03-03.

Treat this criterion as structurally addressed for the observed probe, while
the overall ticket remains blocked by criterion 1.

## Step 9 — update `progress.md`

Record phase completion and each implementation step.

Record the lack of product source changes.

Record the new-ticket commit receipt.

Record validation commands and results.

Record the exact remaining human actions.

Record any deviation from this plan before Review.

Verification:

- progress has a current status section;
- source ownership is explicit;
- no completed-human-run claim appears.

## Step 10 — validate the artifact set

Confirm all phase/evidence artifacts exist and are nonempty.

Check Markdown for trailing whitespace.

Search for forbidden overclaims:

- `baseline complete`;
- `leg B passed`;
- `acceptance satisfied`; and
- numeric disk delta presented without qualification.

Validate the new ticket through Lisa's repository validator if the shared dirty
planning state permits it without mutation.

Inspect exact-path Git status and receipt contents.

Verification:

- documentation checks pass;
- new ticket is clean and committed;
- any validator failure is classified as ticket-owned or unrelated.

## Step 11 — write Review artifacts

Write `review.md` last among Markdown artifacts.

Summarize created files, retained evidence, source commit, finding links,
validation, acceptance gaps, and unblock actions.

Write `review-disposition.json` with exactly a block disposition and one
non-empty actionable reason.

The reason must ask John for:

- a fresh Claude/Haiku instruction-A run; and
- a separate fresh Codex/mini instruction-A run;

both with complete snapshots, independent checks, strings, and artifact fields.

Verification:

- JSON parses;
- keys are exactly `disposition` and `reason`;
- disposition is exactly `block`;
- reason is a non-empty string;
- Review makes no request to advance phase/status manually.

## Testing strategy

No Rust unit or integration tests are needed because no executable code changes.

The shared change is a Markdown ticket and is checked structurally.

Evidence verification is source-to-artifact traceability:

- Docker metadata supports fixture identity;
- transcript metadata supports model/CLI identity;
- transcript messages support the exact strings and sequence;
- changed paths support artifact observations;
- missing snapshot paths support the incompleteness finding.

The Review JSON receives a machine parse/equality test.

The source transaction receives exact commit-path and clean-status checks.

## Stop condition

After both Review artifacts exist and validations complete, remain on
T-046-06-02.

Do not start T-046-06-03.

Do not edit T-046-06-02 frontmatter.

Do not publish private artifacts into `docs/active/work/`.

Do not mark the ticket complete.

The expected final state for this attempt is a documented, actionable block.
